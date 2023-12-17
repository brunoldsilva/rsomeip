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
