use std::{future::IntoFuture, time::Duration};

use tokio::{net::TcpListener, time::timeout};

use crate::testing::ipv4;

use super::*;

#[test]
fn new() {
    let address = ipv4!([127, 0, 0, 1]);
    let socket = Socket::new(address);
    assert_eq!(socket.address, address);
}

#[tokio::test]
async fn connect() {
    let local_addr = ipv4!([127, 0, 0, 1]);
    let peer_addr = ipv4!([127, 0, 0, 2]);
    let (tx_messages, rx_messages) = mpsc::channel(1);
    let (tx_packets, rx_packets) = mpsc::channel(1);
    let mut socket = Socket::new(local_addr);
    let handle = tokio::spawn(async move { socket.process(rx_messages, tx_packets).await });
    let listener = TcpListener::bind(peer_addr)
        .await
        .expect("should bind to the local address");
    let (message, response) = Message::new(Operation::Connect(peer_addr));
    tx_messages
        .send(message)
        .await
        .expect("should send the message to the socket");
    let (_stream, address) = timeout(Duration::from_millis(50), listener.accept())
        .await
        .expect("should not timeout")
        .expect("should accept a connection from the socket");
    timeout(Duration::from_millis(50), response.into_future())
        .await
        .expect("should not timeout")
        .expect("should receive a response")
        .expect("should receive an Ok");
    assert_eq!(local_addr, address);
    std::mem::drop(tx_messages);
    std::mem::drop(rx_packets);
    handle.await.expect("task should complete successfully");
}

#[tokio::test]
async fn connect_v2() {}
