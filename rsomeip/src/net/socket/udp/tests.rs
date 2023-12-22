use super::*;
use crate::testing::ipv4;

#[tokio::test]
async fn bind() {
    let _socket = Socket::bind(ipv4!([127, 0, 0, 1]))
        .await
        .expect("should bind to the address");
}

#[tokio::test]
async fn send() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let (mut socket, mut packets) = Socket::bind(src).await.expect("should bind to the address");
    let (_tx_messages, messages) = mpsc::channel(8);
    tokio::spawn(async move { socket.process(messages).await });
    let socket = UdpSocket::bind(dst)
        .await
        .expect("should bind to the address");
    let buffer = [1u8; 16];
    socket
        .send_to(buffer.as_ref(), src)
        .await
        .expect("should send the data to the socket");
    let (address, data) = packets.recv().await.expect("should receive a packet");
    assert_eq!(address, dst);
    assert_eq!(data[..16], buffer[..16]);
}

// Add a unit test for the `process_operation()` method.
// The test should create a new socket, join a multicast group, send and then receive a message from that group.
#[tokio::test]
#[ignore = "multicast tests are not supported yet"]
async fn multicast() {
    let src = ipv4!([127, 0, 0, 1], 12345);
    let dst = ipv4!([224, 0, 0, 1], 56789);
    let (mut socket, mut packets) = Socket::bind(src).await.expect("should bind to the address");
    let (_tx_messages, messages) = mpsc::channel(8);
    tokio::spawn(async move { socket.process(messages).await });
    let socket = UdpSocket::bind(dst)
        .await
        .expect("should bind to the address");
    let buffer = [1u8; 16];
    socket
        .send_to(buffer.as_ref(), dst)
        .await
        .expect("should send the data to the socket");
    let (address, data) = packets.recv().await.expect("should receive a packet");
    assert_eq!(address, src);
    assert_eq!(data[..16], buffer[..16]);
}
