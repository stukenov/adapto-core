use arc_swap::ArcSwapOption;
use std::sync::Arc;

/// A lock-free swappable cache for job-updated datasets.
///
/// Reads ([`Reloadable::load`]) are wait-free; a job calls [`Reloadable::store`]
/// after writing the database so changes appear without a process restart.
/// Usable in a `static` via the `const fn` [`Reloadable::empty`].
pub struct Reloadable<T> {
    cell: ArcSwapOption<T>,
}

impl<T> Default for Reloadable<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> Reloadable<T> {
    /// An empty cache (no value yet). Safe in a `static`.
    pub const fn empty() -> Self {
        Reloadable {
            cell: ArcSwapOption::const_empty(),
        }
    }
    /// A cache pre-populated with `v`.
    pub fn new(v: T) -> Self {
        Reloadable {
            cell: ArcSwapOption::from(Some(Arc::new(v))),
        }
    }
    /// Current value, or `None` if never stored.
    pub fn load(&self) -> Option<Arc<T>> {
        self.cell.load_full()
    }
    /// Atomically replace the value.
    pub fn store(&self, v: T) {
        self.cell.store(Some(Arc::new(v)));
    }
    /// True if a value has been stored.
    pub fn is_set(&self) -> bool {
        self.cell.load().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static GLOBAL: Reloadable<Vec<i32>> = Reloadable::empty();

    #[test]
    fn empty_then_store_then_load() {
        assert!(GLOBAL.load().is_none());
        GLOBAL.store(vec![1, 2, 3]);
        assert_eq!(*GLOBAL.load().unwrap(), vec![1, 2, 3]);
        GLOBAL.store(vec![9]);
        assert_eq!(*GLOBAL.load().unwrap(), vec![9]);
    }
}
