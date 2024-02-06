use std::{
    ops::{Deref, Range},
    sync::Arc,
};
use tokio::sync::oneshot;

/// A scoped reference into a shared byte buffer.
#[derive(Debug, Clone)]
pub struct Buffer {
    inner: Arc<[u8]>,
    range: Range<usize>,
}

impl Buffer {
    /// Creates a new [`Buffer`].
    #[must_use]
    pub fn new(inner: Arc<[u8]>, range: Range<usize>) -> Self {
        Self { inner, range }
    }

    /// Splits the [`Buffer`] into two halves, returning the first half as a new [`Buffer`], and
    /// keeping the second half in the current [`Buffer`].
    ///
    /// Returns [`None`] if the given length is greater than or equal to the length of the
    /// [`Buffer`].
    pub fn split(&mut self, length: usize) -> Option<Self> {
        if length >= self.len() {
            return None;
        }
        let buffer = Self::new(
            self.inner.clone(),
            self.range.start..self.range.start + length,
        );
        self.range = self.range.start + length..self.range.end;
        Some(buffer)
    }

    /// Returns the length of the [`Buffer`] in bytes.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Returns a shared reference into the given [`Buffer`].
    pub fn get(&self) -> &[u8] {
        self.inner[self.range.clone()].as_ref()
    }

    /// Returns a mutable reference into the given [`Buffer`], if there are no other [`Buffer`]s
    /// with the same underlying allocation.
    ///
    /// Returns [`None`] otherwise, because it is not safe to mutate a shared value.
    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        Arc::get_mut(&mut self.inner).map(|buffer| buffer[self.range.clone()].as_mut())
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner[self.range.clone()].as_ref()
    }
}

impl<const N: usize> From<[u8; N]> for Buffer {
    fn from(value: [u8; N]) -> Self {
        let length = value.len();
        Self::new(Arc::new(value), 0..length)
    }
}

impl From<Arc<[u8]>> for Buffer {
    fn from(value: Arc<[u8]>) -> Self {
        let length = value.len();
        Self::new(value, 0..length)
    }
}

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
/// use rsomeip::net::util::BufferPool;
/// use std::sync::Arc;
///
/// // The pool starts empty.
/// let mut pool = BufferPool::new();
///
/// // Pulling will only work if there are available buffers in the pool.
/// let mut buffer: Arc<[u8]> = pool.pull_or_else(|| Arc::new([0u8; 8]));
///
/// // By using `get_mut()`, we are able to modify the contents of the buffer.
/// if let Some(buffer) = Arc::get_mut(&mut buffer) {
///     buffer.iter_mut().for_each(|elem| *elem += 1);
/// }
///
/// // Add the buffer back to the pool to be used later, when it becomes available.
/// pool.push(buffer.clone());
///
/// // Send the buffer to wherever you want. You can then retrieve it from the pool
/// // once you're done using it.
/// # fn awesome_function(_buffer: Arc<[u8]>) {}
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
        self.buffers
            .iter_mut()
            .position(|buffer| Arc::get_mut(buffer).is_some())
            .map(|index| self.buffers.swap_remove(index))
    }

    /// Returns an available buffer, or creates a new one with the provided closure.
    ///
    /// The closure is only called if there are no available buffers in the pool.
    pub fn pull_or_else<F>(&mut self, f: F) -> Arc<[u8]>
    where
        F: FnOnce() -> Arc<[u8]>,
    {
        self.try_pull().unwrap_or_else(f)
    }

    /// Adds the `buffer` to the pool.
    pub fn push(&mut self, buffer: Arc<[u8]>) {
        self.buffers.push(buffer);
    }
}

/// Creates a connected pair of [`ResponseSender`] and [`ResponseReceiver`].
///
/// A response can be sent from the [`ResponseSender`], using either the `ok` or `err` functions,
///  and received by the [`ResponseReceiver`] with a call to `get`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// # tokio_test::block_on(async {
/// // Create a pair for sending and receiving.
/// let (sender, receiver) = rsomeip::net::util::response_channels::<bool, i32>();
///
/// // Move one of them to a different task.
/// tokio::spawn(async move {
///     // Send the response through the sender.
///     sender.send(Ok(true));
///     // Or, sender.ok(true);
///     // Or, sender.err(-1);
/// });
///
/// // Receive the response on the receiver.
/// let response = receiver.get().await;
/// assert_eq!(response, Some(Ok(true)));
/// # });
/// ```
pub fn response_channels<T, E>() -> (ResponseSender<T, E>, ResponseReceiver<T, E>)
where
    T: Send,
    E: Send,
{
    let (sender, receiver) = oneshot::channel();
    (ResponseSender::new(sender), ResponseReceiver::new(receiver))
}

/// Receives a response from the associated [`ResponseSender`].
///
/// A pair of [`ResponseSender`] and [`ResponseReceiver`] can be created with the
/// [`response_channels`] function.
#[derive(Debug)]
pub struct ResponseReceiver<T, E> {
    inner: oneshot::Receiver<Result<T, E>>,
}

impl<T, E> ResponseReceiver<T, E>
where
    T: Send,
    E: Send,
{
    /// Creates a new [`ResponseReceiver`].
    fn new(inner: oneshot::Receiver<Result<T, E>>) -> Self {
        Self { inner }
    }

    /// Gets the response from the [`ResponseSender`].
    ///
    /// This function returns [`None`] if the sender is dropped before sending the response.
    pub async fn get(self) -> Option<Result<T, E>> {
        self.inner.await.ok()
    }
}

/// Sends a response to the associated [`ResponseReceiver`].
///
/// A pair of [`ResponseSender`] and [`ResponseReceiver`] can be created with the
/// [`response_channels`] function.
#[derive(Debug)]
pub struct ResponseSender<T, E> {
    inner: oneshot::Sender<Result<T, E>>,
}

impl<T, E> ResponseSender<T, E> {
    /// Creates a new [`ResponseSender`].
    fn new(inner: oneshot::Sender<Result<T, E>>) -> Self {
        Self { inner }
    }

    /// Send the given [`Result`] to the [`ResponseReceiver`].
    pub fn send(self, result: Result<T, E>) {
        let _ = self.inner.send(result);
    }

    /// Sends an [`Ok`] response with the given value to the [`ResponseReceiver`].
    pub fn ok(self, value: T) {
        let _ = self.inner.send(Ok(value));
    }

    /// Sends an [`Err`] response with the given error to the [`ResponseReceiver`].
    pub fn err(self, error: E) {
        let _ = self.inner.send(Err(error));
    }
}

#[cfg(test)]
mod tests;
