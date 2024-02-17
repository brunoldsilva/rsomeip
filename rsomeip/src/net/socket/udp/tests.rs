use super::*;
use crate::{net::util::response_channels, testing::ipv4};

/// Checks that the socket can bind to an address.
///
/// Steps:
/// 1. Create a [`Socket`] and bind to an address.
///
/// Checks:
/// - The socket should bind to the address.
#[tokio::test]
async fn bind() {
    let _socket = Socket::bind(ipv4!([127, 0, 0, 1]))
        .await
        .expect("should bind to the address");
}

/// Checks that the socket can send data.
///
/// Steps:
/// 1. Create a [`Socket`] and start processing messages.
/// 2. Create a native socket to receive the data.
/// 3. Send the data from the [`Socket`].
/// 4. Receive the data on the native socket.
///
/// Checks:
/// - The socket should send the data.
/// - The socket should send the data to the correct address.
/// - The socket should send the correct data.
#[tokio::test]
async fn send() {
    // Create a new [`Socket`] and start processing messages.
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let (mut socket, _packets) = Socket::bind(src).await.expect("should bind to the address");
    let (tx_messages, messages) = mpsc::channel(8);
    tokio::spawn(async move { socket.process(messages).await });

    // Create a native socket to receive the data.
    let socket = UdpSocket::bind(dst)
        .await
        .expect("should bind to the address");

    // Send the data.
    let data = Arc::new([1u8; 16]);
    let (sender, receiver) = response_channels::<(), io::Error>();
    let message = Message::new(Operation::Send((dst, data.clone())), sender);
    tx_messages
        .send(message)
        .await
        .expect("should send the data to the socket");
    let response = receiver.get().await.expect("should receive a response");
    assert!(response.is_ok());

    // Receive the data.
    let mut buffer = [0u8; 16];
    let (_size, address) = socket
        .recv_from(&mut buffer)
        .await
        .expect("should receive the data");
    assert_eq!(address, src);
    assert_eq!(buffer[..16], data[..16]);
}

/// Checks that the socket can receive data.
///
/// Steps:
/// 1. Create a [`Socket`] and start processing messages.
/// 2. Create a native socket to send the data.
/// 3. Send the data from the native socket.
/// 4. Receive the data on the [`Socket`].
///
/// Checks:
/// - The socket should receive the data.
/// - The socket should receive the data from the correct address.
/// - The socket should receive the correct data.
#[tokio::test]
async fn recv() {
    // Create a new [`Socket`] and start processing messages.
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let (mut socket, mut packets) = Socket::bind(src).await.expect("should bind to the address");
    let (_tx_messages, messages) = mpsc::channel(8);
    tokio::spawn(async move { socket.process(messages).await });

    // Create a native socket to send the data.
    let socket = UdpSocket::bind(dst)
        .await
        .expect("should bind to the address");

    // Send the data.
    let buffer = [1u8; 16];
    socket
        .send_to(buffer.as_ref(), src)
        .await
        .expect("should send the data to the socket");

    // Receive the data.
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
