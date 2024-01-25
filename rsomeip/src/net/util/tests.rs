use super::*;
use std::time::Duration;
use tokio::time::timeout;

#[test]
fn new() {
    let pool = BufferPool::new();
    assert_eq!(pool.buffers.len(), 0);
}

#[test]
fn push() {
    let mut pool = BufferPool::new();
    assert_eq!(pool.buffers.len(), 0);
    let buffer = Arc::new([0u8; 8]);
    pool.push(buffer);
    assert_eq!(pool.buffers.len(), 1);
}

#[test]
fn try_pull() {
    let mut pool = BufferPool::new();
    assert!(pool.try_pull().is_none());
    let buffer = Arc::new([0u8; 8]);
    pool.push(buffer.clone());
    assert!(pool.try_pull().is_none());
    std::mem::drop(buffer);
    assert!(pool.try_pull().is_some());
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
