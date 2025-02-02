//! Synchronization primitives for use in asynchronous contexts.
//!
//! - [`ResponseSender`] and [`ResponseReceiver`] make it easier to send results between tasks.

use tokio::sync::oneshot;

/// Creates a connected pair of [`ResponseSender`] and [`ResponseReceiver`].
///
/// A response can be sent from the [`ResponseSender`], using either the `ok` or `err` functions,
///  and received by the [`ResponseReceiver`] with a call to `get`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust,ignore
/// # tokio_test::block_on(async {
/// // Create a pair for sending and receiving.
/// let (sender, receiver) = crate::support::sync::response_channels::<bool, i32>();
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
mod tests {
    use super::*;
    #[tokio::test]
    async fn response_send() {
        let (sender, receiver) = response_channels::<(), i32>();
        let handle = tokio::spawn(async move {
            sender.send(Ok(()));
        });
        let response = receiver.get().await;
        assert_eq!(response, Some(Ok(())));
        handle.await.expect("should complete successfully");
    }

    #[tokio::test]
    async fn response_ok() {
        let (sender, receiver) = response_channels::<(), i32>();
        let handle = tokio::spawn(async move {
            sender.ok(());
        });
        let response = receiver.get().await;
        assert_eq!(response, Some(Ok(())));
        handle.await.expect("should complete successfully");
    }

    #[tokio::test]
    async fn response_err() {
        let (sender, receiver) = response_channels::<(), i32>();
        let handle = tokio::spawn(async move {
            sender.err(-1);
        });
        let response = receiver.get().await;
        assert_eq!(response, Some(Err(-1)));
        handle.await.expect("should complete successfully");
    }

    #[tokio::test]
    async fn response_none() {
        let (_, receiver) = response_channels::<(), i32>();
        let response = receiver.get().await;
        assert_eq!(response, None);
    }
}
