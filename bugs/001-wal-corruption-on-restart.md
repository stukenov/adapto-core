# BUG-001: WAL corruption on server restart ("trailing characters")

## Severity: Medium (data loss on restart, workaround exists)

## Symptom

When restarting a server that uses `AdaptoStore` with `disk_collection()` (DB path set), the WAL replay fails:

```
WalCorrupted("line 34065: trailing characters")
```

Server panics and cannot start until WAL files are manually deleted.

## Reproduction

```bash
# 1. Start server with disk storage
MYQAZ_DB=/tmp/test-db myqaz-server &

# 2. Let it import data (inserts ~100K docs into disk_collection)
# 3. Kill server (Ctrl+C or kill)
kill $!

# 4. Restart with same DB path
MYQAZ_DB=/tmp/test-db myqaz-server
# => PANIC: WalCorrupted("line XXXXX: trailing characters")
```

## Workaround

```bash
rm -rf /tmp/test-db/*   # delete corrupted WAL before restart
```

## Root Cause Analysis

### 1. No Drop impl on WriteAheadLog

**File:** `crates/adapto_store/src/wal.rs:63-67`

`WriteAheadLog` struct has no `Drop` implementation. When the process exits (gracefully or not), there's no explicit `flush()` + `sync_data()` call. OS-level file buffers may not be flushed.

### 2. Non-atomic multi-line writes

**File:** `crates/adapto_store/src/wal.rs:84-91` (`append()`)

```rust
writeln!(self.file, "{}", line)?;  // JSON line
self.file.flush()?;
self.file.sync_data()?;
```

Each `append()` call is individually flushed, but if the process is killed between two `append()` calls (e.g., during `bulk_insert` which calls `append()` in a loop), the last line may be incomplete:

```
{"Insert":{"collection":"companies","doc_id":"123","data":{...}}}   <- complete
{"Insert":{"collection":"companies","doc_id":"124","data":{"bin":"1   <- TRUNCATED
```

### 3. No recovery on replay

**File:** `crates/adapto_store/src/wal.rs:94-120` (`replay()`)

```rust
for line in reader.lines() {
    let line = line?;
    let entry: WalEntry = serde_json::from_str(&line)
        .map_err(|e| WalError::Corrupted(format!("line {}: {}", line_num, e)))?;
    // ...
}
```

Replay fails immediately on any malformed line. The `serde_json` error "trailing characters" occurs when:
- A line contains valid JSON followed by garbage bytes (partial next write)
- Or a line is truncated mid-JSON

There is no attempt to skip or truncate incomplete trailing entries.

### 4. No close signal in StorageEngine

**File:** `crates/adapto_store/src/engine.rs:57`

`StorageEngine` holds `Arc<RwLock<WriteAheadLog>>`. When the engine is dropped, the WAL file handle is closed by the OS, but without explicit flush/sync guarantees.

## Proposed Fixes

### Fix 1: Add Drop impl (minimal)

```rust
impl Drop for WriteAheadLog {
    fn drop(&mut self) {
        let _ = self.file.flush();
        let _ = self.file.sync_all();
    }
}
```

### Fix 2: Recovery on replay (recommended)

In `replay()`, when the **last line** fails to parse, treat it as an incomplete write and truncate instead of erroring:

```rust
for line in reader.lines() {
    let line = line?;
    match serde_json::from_str::<WalEntry>(&line) {
        Ok(entry) => { /* apply */ },
        Err(e) if is_last_line => {
            eprintln!("WAL: truncating incomplete final entry at line {}: {}", line_num, e);
            // Rewrite WAL without the corrupted line
            break;
        },
        Err(e) => return Err(WalError::Corrupted(...)),
    }
}
```

### Fix 3: Commit markers (robust)

Add a checksum or length prefix to each WAL line:

```
LEN:CHECKSUM:JSON\n
```

On replay, validate length + checksum before parsing. Skip lines that don't match.

## Affected Files

| File | Line | What |
|------|------|------|
| `crates/adapto_store/src/wal.rs` | 63-67 | WriteAheadLog struct, no Drop |
| `crates/adapto_store/src/wal.rs` | 84-91 | append() — write + flush + sync |
| `crates/adapto_store/src/wal.rs` | 94-120 | replay() — no error recovery |
| `crates/adapto_store/src/wal.rs` | 123-142 | compact() — atomic swap |
| `crates/adapto_store/src/engine.rs` | 57 | StorageEngine.wal field |
| `crates/adapto_store/src/engine.rs` | 660-668 | replay_wal() |

## Test Gap

Existing tests (`tests/store_tests.rs:638-727`) test WAL persistence and compaction but do NOT simulate:
- Process crash mid-write
- Restart with corrupted WAL
- Recovery from incomplete entries

## Environment

- macOS Darwin 25.5.0 (ARM64)
- Rust stable
- Triggered during development with `disk_collection()` + `bulk_insert()` of ~739K company records
