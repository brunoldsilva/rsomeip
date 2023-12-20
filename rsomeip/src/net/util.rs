use std::sync::Arc;

#[cfg(test)]
mod tests;

/// A pool of byte buffers that uses reference counting to keep track of available buffers.
///
/// Buffers can be added to the pool with `push()` and retrieved with `try_pull()`.
///
/// A buffer can only be pulled from the pool if it's not referenced anywhere else. This ensures
/// that the buffer can be safely modified without introducing undefined behavior. Then, you just
/// add it back to the pool (usually cloning it first) and it will become available once it's no
/// longer being used outside of the pool.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # fn awesome_function(_buffer: Arc<[u8]>) {}
/// use rsomeip::net::util::BufferPool;
/// use std::sync::Arc;
///
/// // The pool starts empty.
/// let mut pool = BufferPool::new();
///
/// // Pulling will only work if there are available buffers in the pool.
/// let mut buffer: Arc<[u8]> = pool.try_pull().unwrap_or(Arc::new([0u8; 8]));
///
/// // By using `get_mut()`, we are able to modify the contents of the buffer.
/// if let Some(buffer) = Arc::get_mut(&mut buffer) {
///     buffer.iter_mut().for_each(|elem| *elem += 1);
/// }
///
/// // Add the buffer back to the pull to be used later, when it becomes available.
/// pool.push(buffer.clone());
///
/// // Send the buffer to wherever you want. You can then retrieve it from the pool
/// // once you're done using it.
/// awesome_function(buffer);
/// ```
#[derive(Debug, Default)]
pub struct BufferPool {
    buffers: Vec<Arc<[u8]>>,
}

impl BufferPool {
    /// Creates a new [`BufferPool`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an available buffer, if it exists.
    ///
    /// A buffer is considered available if there are no external references to it.
    pub fn try_pull(&mut self) -> Option<Arc<[u8]>> {
        for (index, buffer) in self.buffers.iter().enumerate() {
            if Arc::strong_count(buffer) + Arc::weak_count(buffer) == 1 {
                return Some(self.buffers.swap_remove(index));
            }
        }
        None
    }

    /// Adds the `buffer` to the pool.
    pub fn push(&mut self, buffer: Arc<[u8]>) {
        self.buffers.push(buffer);
    }
}
