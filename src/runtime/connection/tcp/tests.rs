#![cfg(test)]

use super::*;
use crate::testing::ipv4;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn connect() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let socket = TcpSocket::new_v4()
        .map(|s| {
            s.bind(dst).expect("Should bind to destination address");
            s
        })
        .expect("Should create a TCP socket.");
    let listener = socket.listen(1024).expect("Should create a TCP listener.");
    let handle = tokio::spawn(async move {
        let (_, addr) = listener
            .accept()
            .await
            .expect("Should create a TCP stream.");
        assert_eq!(addr, src, "Should establish a connection to the source.");
    });
    assert!(
        Factory::connect(src, dst).await.is_ok(),
        "Should establish a connection."
    );
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn connect_send() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let socket = TcpSocket::new_v4()
        .map(|s| {
            s.bind(dst).expect("Should bind to destination address");
            s
        })
        .expect("Should create a TCP socket.");
    let listener = socket.listen(1024).expect("Should create a TCP listener.");
    let handle = tokio::spawn(async move {
        let (stream, addr) = listener
            .accept()
            .await
            .expect("Should create a TCP stream.");
        assert_eq!(addr, src, "Should establish a connection to the source.");
        stream.readable().await.expect("Should become readable.");
        let mut buf = [0u8; 1024];
        stream
            .try_read(&mut buf[..])
            .expect("Should read a buffer from the stream.");
        assert_eq!(buf[0], 1, "Buffer should not be empty.");
    });
    let connection = Factory::connect(src, dst)
        .await
        .expect("Should establish a connection.");
    let buf = [1u8; 1024];
    connection
        .sender
        .expect("Should have a sender.")
        .send(buf)
        .await
        .expect("Should sender the buffer.");
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn connect_recv() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let socket = TcpSocket::new_v4()
        .map(|s| {
            s.bind(dst).expect("Should bind to destination address");
            s
        })
        .expect("Should create a TCP socket.");
    let listener = socket.listen(1024).expect("Should create a TCP listener.");
    let handle = tokio::spawn(async move {
        let (stream, addr) = listener
            .accept()
            .await
            .expect("Should create a TCP stream.");
        assert_eq!(addr, src, "Should establish a connection to the source.");
        stream.writable().await.expect("Should become writable.");
        let buf = [1u8; 1024];
        stream
            .try_write(&buf[..])
            .expect("Should write the buffer to the stream.");
    });
    let mut connection = Factory::connect(src, dst)
        .await
        .expect("Should establish a connection.");
    let buf = connection
        .receiver
        .recv()
        .await
        .expect("Should receive a buffer.");
    assert_eq!(buf[0], 1, "Buffer should not be empty.");
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn listen() {
    let src = ipv4!([127, 0, 0, 1]);
    assert!(
        Factory::listen(src).await.is_ok(),
        "Should create a listener."
    );
}

#[tokio::test]
async fn listen_recv() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let mut listener = Factory::listen(src)
        .await
        .expect("Should create a handle for the listener.");
    let handle = tokio::spawn(async move {
        let (addr, buf) = listener
            .receiver
            .recv()
            .await
            .expect("Should receive a buffer");
        assert_eq!(
            addr, dst,
            "Remote address should match destination address."
        );
        assert_ne!(buf[0], 0, "Buffer should not be empty.");
    });
    let socket = TcpSocket::new_v4()
        .map(|s| {
            s.bind(dst).expect("Should bind to the remote port");
            s
        })
        .expect("Should create a remote socket.");
    let stream = socket
        .connect(src)
        .await
        .expect("Should connect to the listener.");
    let buf = [1u8; 1024];
    stream.writable().await.expect("Should be writable.");
    stream.try_write(&buf[..]).expect("Should send the buffer.");
    timeout(Duration::from_secs(1), handle)
        .await
        .expect("Should not timeout.")
        .expect("Should complete the task.");
}

#[tokio::test]
async fn listen_reply() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let mut listener = Factory::listen(src)
        .await
        .expect("Should create a handle for the listener.");
    let handle = tokio::spawn(async move {
        let (addr, buf) = listener
            .receiver
            .recv()
            .await
            .expect("Should receive a buffer.");
        assert_eq!(
            addr, dst,
            "Remote address should match destination address."
        );
        assert_eq!(buf[0], 1, "Buffer should not be empty.");
        let buf = [2u8; 1024];
        listener
            .sender
            .send((addr, buf))
            .await
            .expect("Should send the buffer back to the sender.");
    });
    let socket = TcpSocket::new_v4()
        .map(|s| {
            s.bind(dst).expect("Should bind to the remote port");
            s
        })
        .expect("Should create a remote socket.");
    let stream = socket
        .connect(src)
        .await
        .expect("Should connect to the listener.");
    let mut buf = [1u8; 1024];
    stream.writable().await.expect("Should be writable.");
    stream.try_write(&buf[..]).expect("Should send the buffer.");
    stream.readable().await.expect("Should be readable.");
    stream
        .try_read(&mut buf[..])
        .expect("Should read a buffer.");
    assert_eq!(buf[0], 2, "Buffer should not be empty.");
    timeout(Duration::from_secs(1), handle)
        .await
        .expect("Should not timeout.")
        .expect("Should complete the task.");
}
