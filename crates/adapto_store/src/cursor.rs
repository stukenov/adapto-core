use std::collections::VecDeque;

use crate::document::Document;

/// A lazy iterator over query results.
///
/// Provides both an explicit `next()` method and implements `Iterator`
/// so callers can use `for doc in cursor { ... }` or `.collect()`.
pub struct Cursor {
    documents: VecDeque<Document>,
}

impl Cursor {
    /// Create a cursor wrapping a pre-resolved list of documents.
    pub(crate) fn new(documents: Vec<Document>) -> Self {
        Self {
            documents: VecDeque::from(documents),
        }
    }

    /// Peek at the next document without advancing.
    pub fn peek(&self) -> Option<&Document> {
        self.documents.front()
    }

    /// Consume the cursor and collect all remaining documents.
    pub fn collect_docs(self) -> Vec<Document> {
        self.documents.into()
    }

    /// Number of remaining documents.
    pub fn count(&self) -> usize {
        self.documents.len()
    }

    /// Consume the cursor and return the first document (if any).
    pub fn first(mut self) -> Option<Document> {
        self.documents.pop_front()
    }
}

impl Iterator for Cursor {
    type Item = Document;

    fn next(&mut self) -> Option<Self::Item> {
        self.documents.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.documents.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for Cursor {}
