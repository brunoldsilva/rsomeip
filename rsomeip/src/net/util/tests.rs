use super::*;

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
