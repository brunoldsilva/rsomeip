//! Collection types.
//!
//! - [`SharedMap`] is a thread-safe collection of key-value pairs.

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};

/// A thread-safe collection of key-value pairs.
///
/// This map can be cloned to create multiple instances pointing to the same underlying memory
/// allocation, which can be accessed from different threads without leading to undefined behavior.
///
/// # Examples
///
/// ```rust,ignore
/// use rsomeip::net::util::SharedMap;
/// let map: SharedMap<u32, &'static str> = SharedMap::new();
///
/// let cloned_map = map.clone();
/// let handle = std::thread::spawn(move || {
///     assert_eq!(cloned_map.insert(1, "Mercury"), None);
/// });
/// assert_eq!(map.insert(2, "Venus"), None);
///
/// handle.join();
/// assert_eq!(map.get(&1), Some("Mercury"));
/// ```
#[derive(Debug, Clone)]
pub struct SharedMap<K, V> {
    inner: Arc<Mutex<HashMap<K, V>>>,
}

impl<K, V> SharedMap<K, V>
where
    K: Hash + PartialEq + Eq,
    V: Clone,
{
    /// Creates a new [`SharedMap`].
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::support::sync::SharedMap;
    /// let map: SharedMap<u32, &'static str> = SharedMap::new();
    /// assert_eq!(map.insert(1, "Mercury"), None);
    /// assert_eq!(map.get(&1), Some("Mercury"));
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::default(),
        }
    }

    /// Inserts a new value into the map.
    ///
    /// If the key already exists, it replaces the value, and returns the old one.
    ///
    /// # Panics
    ///
    /// Panics if the inner mutex is poisoned, which happens when a different thread panics while
    /// holding a lock on the mutex.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::support::sync::SharedMap;
    /// let map: SharedMap<u32, &'static str> = SharedMap::new();
    /// assert_eq!(map.insert(1, "Mercury"), None);
    /// assert_eq!(map.get(&1), Some("Mercury"));
    /// assert_eq!(map.insert(1, "Venus"), Some("Mercury"));
    /// assert_eq!(map.get(&1), Some("Venus"));
    /// ```
    #[allow(clippy::expect_used)]
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        self.inner
            .lock()
            .expect("mutex should not be poisoned")
            .insert(key, value)
    }

    /// Returns the value corresponding to the given key.
    ///
    /// # Panics
    ///
    /// Panics if the inner mutex is poisoned, which happens when a different thread panics while
    /// holding a lock on the mutex.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::support::sync::SharedMap;
    /// let map: SharedMap<u32, &'static str> = SharedMap::new();
    /// assert_eq!(map.get(&1), None);
    /// assert_eq!(map.insert(1, "Mercury"), None);
    /// assert_eq!(map.get(&1), Some("Mercury"));
    /// ```
    #[allow(clippy::expect_used)]
    pub fn get(&self, key: &K) -> Option<V> {
        self.inner
            .lock()
            .expect("mutex should not be poisoned")
            .get(key)
            .cloned()
    }

    /// Removes an entry from the map, and returns it.
    ///
    /// # Panics
    ///
    /// Panics if the inner mutex is poisoned, which happens when a different thread panics while
    /// holding a lock on the mutex.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::support::sync::SharedMap;
    /// let map: SharedMap<u32, &'static str> = SharedMap::new();
    /// assert_eq!(map.insert(1, "Mercury"), None);
    /// assert_eq!(map.remove(&1), Some("Mercury"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    #[allow(clippy::expect_used)]
    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner
            .lock()
            .expect("mutex should not be poisoned")
            .remove(key)
    }
}

impl<K, V> Default for SharedMap<K, V>
where
    K: Hash + PartialEq + Eq,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the happy-path of all [`SharedMap`] methods.
    #[test]
    fn shared_map() {
        let map: SharedMap<u32, &'static str> = SharedMap::new();

        let cloned_map = map.clone();
        let handle = std::thread::spawn(move || {
            assert_eq!(cloned_map.insert(1, "Mercury"), None);
        });
        assert_eq!(map.insert(2, "Venus"), None);

        handle.join().expect("should complete successfully");
        assert_eq!(map.get(&1), Some("Mercury"));

        map.remove(&1);
        assert_eq!(map.get(&1), None);
    }
}
