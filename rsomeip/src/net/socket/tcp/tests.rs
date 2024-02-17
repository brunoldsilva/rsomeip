use crate::{net::socket::Socket, testing::ipv4};
use std::{sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpSocket},
    time::timeout,
};

#[tokio::test]
async fn connect() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);

    let (socket, packets) = Socket::tcp(local_addr);

    let listener = TcpListener::bind(peer_addr)
        .await
        .expect("should bind to the local address");
    let handle = tokio::spawn(async move {
        let (_stream, address) = timeout(Duration::from_millis(50), listener.accept())
            .await
            .expect("should not timeout")
            .expect("should accept a connection from the socket");
        assert_eq!(local_addr, address);
    });

    timeout(Duration::from_millis(50), socket.connect(peer_addr))
        .await
        .expect("should not timeout")
        .expect("should connect to the peer");
    timeout(Duration::from_millis(50), handle)
        .await
        .expect("task should complete successfully")
        .expect("should complete successfully");

    std::mem::drop(packets);
}

#[tokio::test]
async fn send() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);

    let (socket, packets) = Socket::tcp(local_addr);

    let listener = TcpListener::bind(peer_addr)
        .await
        .expect("should bind to the local address");
    let handle = tokio::spawn(async move {
        let (stream, address) = timeout(Duration::from_millis(50), listener.accept())
            .await
            .expect("should not timeout")
            .expect("should accept a connection from the socket");
        assert_eq!(local_addr, address);
        stream.readable().await.expect("should become readable");
        let mut buffer = [0u8; 8];
        let len = stream
            .try_read(buffer.as_mut())
            .expect("should be able to read");
        assert_eq!(len, 8);
        assert_eq!(buffer, [1u8; 8]);
    });

    timeout(Duration::from_millis(50), socket.connect(peer_addr))
        .await
        .expect("should not timeout")
        .expect("should connect to the peer");
    timeout(
        Duration::from_millis(50),
        socket.send(peer_addr, Arc::new([1u8; 8])),
    )
    .await
    .expect("should not timeout")
    .expect("should send message to peer");
    timeout(Duration::from_millis(50), handle)
        .await
        .expect("task should complete successfully")
        .expect("should complete successfully");

    std::mem::drop(packets);
}

#[tokio::test]
async fn disconnect() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);

    let (socket, packets) = Socket::tcp(local_addr);

    let listener = TcpListener::bind(peer_addr)
        .await
        .expect("should bind to the local address");
    let handle = tokio::spawn(async move {
        let (stream, address) = timeout(Duration::from_millis(50), listener.accept())
            .await
            .expect("should not timeout")
            .expect("should accept a connection from the socket");
        assert_eq!(local_addr, address);
        stream.readable().await.expect("should become readable");
        let mut buffer = [0u8; 8];
        let len = stream
            .try_read(buffer.as_mut())
            .expect("should be able to read");
        assert_eq!(len, 0);
    });

    timeout(Duration::from_millis(50), socket.connect(peer_addr))
        .await
        .expect("should not timeout")
        .expect("should connect to the peer");
    timeout(Duration::from_millis(50), socket.disconnect(peer_addr))
        .await
        .expect("should not timeout")
        .expect("should disconnect from peer");
    timeout(Duration::from_millis(50), handle)
        .await
        .expect("task should complete successfully")
        .expect("should complete successfully");

    std::mem::drop(packets);
}

#[tokio::test]
async fn open() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);

    let (socket, mut packets) = Socket::tcp(local_addr);

    timeout(Duration::from_millis(50), socket.open())
        .await
        .expect("should not timeout")
        .expect("should open the socket to incoming connections");
    let handle = tokio::spawn(async move {
        let (address, data) = timeout(Duration::from_millis(50), packets.recv())
            .await
            .expect("should not timeout")
            .expect("should receive data from the peer");
        assert_eq!(peer_addr, address);
        assert_eq!(&data[..8], &[1u8; 8]);
    });

    let socket = TcpSocket::new_v4()
        .and_then(|socket| {
            socket.bind(peer_addr)?;
            Ok(socket)
        })
        .expect("should bind to the address");
    let stream = timeout(Duration::from_millis(50), socket.connect(local_addr))
        .await
        .expect("should not timeout")
        .expect("should connect to the local");
    timeout(Duration::from_millis(50), stream.writable())
        .await
        .expect("should not timeout")
        .expect("should become writable");
    let buffer = [1u8; 8];
    stream
        .try_write(buffer.as_ref())
        .expect("should send data to the local");

    timeout(Duration::from_millis(50), handle)
        .await
        .expect("task should complete successfully")
        .expect("should complete successfully");
}

#[tokio::test]
async fn close() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);

    let (socket, mut packets) = Socket::tcp(local_addr);

    timeout(Duration::from_millis(50), socket.open())
        .await
        .expect("should not timeout")
        .expect("should open the socket to incoming connections");
    timeout(Duration::from_millis(50), socket.close())
        .await
        .expect("should not timeout")
        .expect("should close the socket to incoming connections");
    let handle = tokio::spawn(async move {
        let _ = timeout(Duration::from_millis(50), packets.recv())
            .await
            .expect_err("should timeout");
    });

    let socket = TcpSocket::new_v4()
        .and_then(|socket| {
            socket.bind(peer_addr)?;
            Ok(socket)
        })
        .expect("should bind to the address");
    let _ = timeout(Duration::from_millis(50), socket.connect(local_addr))
        .await
        .expect("should not timeout")
        .expect_err("should not connect to the local");

    timeout(Duration::from_millis(100), handle)
        .await
        .expect("task should complete successfully")
        .expect("should complete successfully");
}
