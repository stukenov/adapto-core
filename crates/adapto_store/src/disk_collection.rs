use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cursor::Cursor;
use crate::document::Document;
use crate::error::StoreError;
use crate::index::{BTreeIndex, IndexInfo, IndexKey};
use crate::query::{Filter, Query};

const RECORD_MAGIC: [u8; 2] = [0xAD, 0xA0];

#[derive(Serialize, Deserialize)]
struct PersistedIndex {
    offsets: Vec<u64>,
    indexes: Vec<PersistedBTree>,
}

#[derive(Serialize, Deserialize)]
struct PersistedBTree {
    name: String,
    fields: Vec<String>,
    unique: bool,
    entries: Vec<(Vec<u8>, Vec<String>)>,
}

pub struct DiskCollectionInner {
    name: String,
    data_path: PathBuf,
    index_path: PathBuf,
    mmap: Option<Mmap>,
    offsets: Vec<u64>,
    indexes: Vec<BTreeIndex>,
}

impl DiskCollectionInner {
    pub fn open(name: &str, base_path: &Path) -> Result<Self, StoreError> {
        let dir = base_path.join("disk");
        fs::create_dir_all(&dir)?;

        let data_path = dir.join(format!("{name}.dat"));
        let index_path = dir.join(format!("{name}.idx"));

        let mut coll = Self {
            name: name.to_string(),
            data_path,
            index_path,
            mmap: None,
            offsets: Vec::new(),
            indexes: Vec::new(),
        };

        if coll.data_path.exists() && fs::metadata(&coll.data_path)?.len() > 0 {
            coll.remap()?;
            coll.load_or_rebuild_index()?;
        }

        Ok(coll)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn count_all(&self) -> u64 {
        self.offsets.len() as u64
    }

    pub fn bulk_insert(&mut self, docs: Vec<Value>) -> Result<u64, StoreError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.data_path)?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);

        self.offsets.clear();
        let mut pos: u64 = 0;

        for val in &docs {
            let doc = Document::new(val.clone(), None);
            let payload = rmp_serde::to_vec(&doc)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let len = payload.len() as u32;

            self.offsets.push(pos);

            writer.write_all(&RECORD_MAGIC)?;
            writer.write_all(&len.to_le_bytes())?;
            writer.write_all(&payload)?;

            pos += 2 + 4 + payload.len() as u64;
        }

        writer.flush()?;
        drop(writer);

        self.remap()?;

        for idx in &mut self.indexes {
            *idx = BTreeIndex::new(idx.name.clone(), idx.fields.clone(), idx.unique);
        }
        self.rebuild_all_indexes()?;
        self.persist_index()?;

        Ok(docs.len() as u64)
    }

    pub fn create_index(&mut self, field: &str, unique: bool) -> Result<(), StoreError> {
        let name = format!("idx_{}", field);
        if self.indexes.iter().any(|i| i.name == name) {
            return Ok(());
        }

        let mut idx = BTreeIndex::new(name, vec![field.to_string()], unique);

        if self.mmap.is_some() {
            for (i, &offset) in self.offsets.iter().enumerate() {
                let doc = self.read_doc_at(offset)?;
                let key = idx.key_for(&doc);
                idx.insert(key, &i.to_string())?;
            }
        }

        self.indexes.push(idx);
        self.persist_index()?;
        Ok(())
    }

    pub fn find_one(&self, query: &Query) -> Result<Option<Document>, StoreError> {
        if let Some(indices) = self.index_candidates(&query.filter) {
            for idx in indices {
                if idx < self.offsets.len() {
                    let doc = self.read_doc_at(self.offsets[idx])?;
                    if query.filter.matches(&doc) {
                        return Ok(Some(doc));
                    }
                }
            }
            return Ok(None);
        }

        for &offset in &self.offsets {
            let doc = self.read_doc_at(offset)?;
            if query.filter.matches(&doc) {
                return Ok(Some(doc));
            }
        }
        Ok(None)
    }

    pub fn find(&self, query: &Query) -> Cursor {
        let mut results = Vec::new();

        if let Some(indices) = self.index_candidates(&query.filter) {
            for idx in indices {
                if idx < self.offsets.len() {
                    if let Ok(doc) = self.read_doc_at(self.offsets[idx]) {
                        if query.filter.matches(&doc) {
                            results.push(doc);
                        }
                    }
                }
            }
            return Cursor::new(results);
        }

        for &offset in &self.offsets {
            if let Ok(doc) = self.read_doc_at(offset) {
                if query.filter.matches(&doc) {
                    results.push(doc);
                }
            }
        }

        Cursor::new(results)
    }

    pub fn indexes(&self) -> Vec<IndexInfo> {
        self.indexes.iter().map(|i| i.info()).collect()
    }

    pub fn index_keys(&self, field: &str) -> Vec<String> {
        let name = format!("idx_{}", field);
        for idx in &self.indexes {
            if idx.name == name {
                return idx.export_entries().into_iter().filter_map(|(k, _)| {
                    match k {
                        crate::index::IndexKey::Single(v) => {
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else {
                                Some(v.to_string())
                            }
                        }
                        _ => None,
                    }
                }).collect();
            }
        }
        Vec::new()
    }

    fn read_doc_at(&self, offset: u64) -> Result<Document, StoreError> {
        let mmap = self
            .mmap
            .as_ref()
            .ok_or_else(|| StoreError::DiskError("data file not mapped".into()))?;

        let off = offset as usize;
        if off + 6 > mmap.len() {
            return Err(StoreError::DiskError("offset past end of file".into()));
        }

        if mmap[off] != RECORD_MAGIC[0] || mmap[off + 1] != RECORD_MAGIC[1] {
            return Err(StoreError::DiskError(format!(
                "bad magic at offset {offset}"
            )));
        }

        let len = u32::from_le_bytes([
            mmap[off + 2],
            mmap[off + 3],
            mmap[off + 4],
            mmap[off + 5],
        ]) as usize;

        let start = off + 6;
        let end = start + len;
        if end > mmap.len() {
            return Err(StoreError::DiskError("record extends past file end".into()));
        }

        rmp_serde::from_slice(&mmap[start..end])
            .map_err(|e| StoreError::Serialization(e.to_string()))
    }

    fn remap(&mut self) -> Result<(), StoreError> {
        let file = File::open(&self.data_path)?;
        let meta = file.metadata()?;
        if meta.len() == 0 {
            self.mmap = None;
            return Ok(());
        }
        let mmap = unsafe { Mmap::map(&file)? };
        self.mmap = Some(mmap);
        Ok(())
    }

    fn index_candidates(&self, filter: &Filter) -> Option<Vec<usize>> {
        match filter {
            Filter::Eq(field, val) => {
                let idx = self
                    .indexes
                    .iter()
                    .find(|i| i.fields.len() == 1 && i.fields[0] == *field)?;
                let key = IndexKey::Single(val.clone());
                let ids = idx.find_eq(&key);
                Some(
                    ids.iter()
                        .filter_map(|s| s.parse::<usize>().ok())
                        .collect(),
                )
            }
            Filter::And(filters) => {
                for f in filters {
                    if let Some(candidates) = self.index_candidates(f) {
                        return Some(candidates);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn rebuild_all_indexes(&mut self) -> Result<(), StoreError> {
        if self.mmap.is_none() {
            return Ok(());
        }
        for (i, &offset) in self.offsets.iter().enumerate() {
            let doc = self.read_doc_at(offset)?;
            let id_str = i.to_string();
            for idx in &mut self.indexes {
                let key = idx.key_for(&doc);
                let _ = idx.insert(key, &id_str);
            }
        }
        Ok(())
    }

    fn persist_index(&self) -> Result<(), StoreError> {
        let mut persisted_indexes = Vec::new();
        for idx in &self.indexes {
            let mut entries = Vec::new();
            for (key, ids) in idx.export_entries() {
                let key_bytes = rmp_serde::to_vec(key)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                entries.push((key_bytes, ids.clone()));
            }
            persisted_indexes.push(PersistedBTree {
                name: idx.name.clone(),
                fields: idx.fields.clone(),
                unique: idx.unique,
                entries,
            });
        }

        let data = PersistedIndex {
            offsets: self.offsets.clone(),
            indexes: persisted_indexes,
        };

        let bytes = rmp_serde::to_vec(&data)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        let mut f = BufWriter::new(File::create(&self.index_path)?);
        f.write_all(&bytes)?;
        f.flush()?;
        Ok(())
    }

    fn load_or_rebuild_index(&mut self) -> Result<(), StoreError> {
        if self.index_path.exists() {
            if let Ok(bytes) = fs::read(&self.index_path) {
                if let Ok(persisted) = rmp_serde::from_slice::<PersistedIndex>(&bytes) {
                    self.offsets = persisted.offsets;
                    self.indexes.clear();
                    for pi in persisted.indexes {
                        let mut idx =
                            BTreeIndex::new(pi.name, pi.fields, pi.unique);
                        for (key_bytes, ids) in pi.entries {
                            if let Ok(key) = rmp_serde::from_slice::<IndexKey>(&key_bytes) {
                                for id in &ids {
                                    let _ = idx.insert(key.clone(), id);
                                }
                            }
                        }
                        self.indexes.push(idx);
                    }
                    return Ok(());
                }
            }
        }

        self.rebuild_offsets()?;
        self.rebuild_all_indexes()?;
        self.persist_index()?;
        Ok(())
    }

    fn rebuild_offsets(&mut self) -> Result<(), StoreError> {
        let mmap = match &self.mmap {
            Some(m) => m,
            None => return Ok(()),
        };

        self.offsets.clear();
        let mut pos = 0usize;
        let len = mmap.len();

        while pos + 6 <= len {
            if mmap[pos] != RECORD_MAGIC[0] || mmap[pos + 1] != RECORD_MAGIC[1] {
                break;
            }
            let rec_len = u32::from_le_bytes([
                mmap[pos + 2],
                mmap[pos + 3],
                mmap[pos + 4],
                mmap[pos + 5],
            ]) as usize;

            if pos + 6 + rec_len > len {
                break;
            }

            self.offsets.push(pos as u64);
            pos += 6 + rec_len;
        }

        Ok(())
    }
}

