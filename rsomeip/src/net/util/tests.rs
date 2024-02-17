use super::*;
use std::time::Duration;
use tokio::time::timeout;

#[test]
fn buffer_split() {
    let mut buffer = Buffer::from([1u8, 2, 3, 4, 5]);
    let buffer_half = buffer.split(2).expect("should split the buffer");
    assert_eq!(&buffer_half[..], &[1u8, 2]);
    assert_eq!(&buffer[..], &[3u8, 4, 5]);
}

#[test]
fn buffer_get_mut() {
    let mut buffer = Buffer::new(Arc::new([0u8; 5]), 1..4);
    for byte in buffer
        .get_mut()
        .expect("should get a reference to the inner buffer")
    {
        *byte = 1;
    }
    assert_eq!(&buffer.inner[..], &[0u8, 1, 1, 1, 0]);
}

#[test]
fn buffer_deref() {
    let buffer = Buffer::from([1u8; 8]);
    assert_eq!(buffer[..], [1u8; 8]);
}

#[test]
fn pool_new() {
    let pool = BufferPool::new();
    assert_eq!(pool.buffers.len(), 0);
}

#[test]
fn pool_push() {
    let mut pool = BufferPool::new();
    assert_eq!(pool.buffers.len(), 0);
    let buffer = Arc::new([0u8; 8]);
    pool.push(buffer);
    assert_eq!(pool.buffers.len(), 1);
}

#[test]
fn pool_try_pull() {
    let mut pool = BufferPool::new();
    assert!(pool.try_pull().is_none());
    let buffer = Arc::new([0u8; 8]);
    pool.push(buffer.clone());
    assert!(pool.try_pull().is_none());
    std::mem::drop(buffer);
    assert!(pool.try_pull().is_some());
}

#[tokio::test]
async fn response_send() {
    let (sender, receiver) = response_channels::<(), i32>();
    let handle = tokio::spawn(async move {
        sender.send(Ok(()));
    });
    let response = timeout(Duration::from_millis(10), receiver.get())
        .await
        .expect("should not timeout");
    assert_eq!(response, Some(Ok(())));
    timeout(Duration::from_millis(10), handle)
        .await
        .expect("should not timeout")
        .expect("should complete successfully");
}

#[tokio::test]
async fn response_ok() {
    let (sender, receiver) = response_channels::<(), i32>();
    let handle = tokio::spawn(async move {
        sender.ok(());
    });
    let response = timeout(Duration::from_millis(10), receiver.get())
        .await
        .expect("should not timeout");
    assert_eq!(response, Some(Ok(())));
    timeout(Duration::from_millis(10), handle)
        .await
        .expect("should not timeout")
        .expect("should complete successfully");
}

#[tokio::test]
async fn response_err() {
    let (sender, receiver) = response_channels::<(), i32>();
    let handle = tokio::spawn(async move {
        sender.err(-1);
    });
    let response = timeout(Duration::from_millis(10), receiver.get())
        .await
        .expect("should not timeout");
    assert_eq!(response, Some(Err(-1)));
    timeout(Duration::from_millis(10), handle)
        .await
        .expect("should not timeout")
        .expect("should complete successfully");
}

#[tokio::test]
async fn response_none() {
    let (_, receiver) = response_channels::<(), i32>();
    let response = timeout(Duration::from_millis(10), receiver.get())
        .await
        .expect("should not timeout");
    assert_eq!(response, None);
}
