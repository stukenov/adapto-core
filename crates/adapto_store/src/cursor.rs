use crate::document::Document;

/// A lazy iterator over query results.
///
/// Provides both an explicit `next()` method and implements `Iterator`
/// so callers can use `for doc in cursor { ... }` or `.collect()`.
pub struct Cursor {
    documents: Vec<Document>,
    position: usize,
}

impl Cursor {
    /// Create a cursor wrapping a pre-resolved list of documents.
    pub(crate) fn new(documents: Vec<Document>) -> Self {
        Self {
            documents,
            position: 0,
        }
    }

    /// Peek at the next document without advancing.
    pub fn peek(&self) -> Option<&Document> {
        self.documents.get(self.position)
    }

    /// Consume the cursor and collect all remaining documents.
    pub fn collect_docs(self) -> Vec<Document> {
        self.documents.into_iter().skip(self.position).collect()
    }

    /// Number of remaining documents.
    pub fn count(&self) -> usize {
        self.documents.len().saturating_sub(self.position)
    }

    /// Consume the cursor and return the first document (if any).
    pub fn first(mut self) -> Option<Document> {
        if self.position < self.documents.len() {
            Some(self.documents.swap_remove(self.position))
        } else {
            None
        }
    }
}

impl Iterator for Cursor {
    type Item = Document;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.documents.len() {
            let doc = self.documents[self.position].clone();
            self.position += 1;
            Some(doc)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.documents.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Cursor {}
