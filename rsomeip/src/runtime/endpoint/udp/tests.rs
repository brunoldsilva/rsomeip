#![cfg(test)]

use super::*;
use crate::testing::ipv4;

#[tokio::test]
async fn bind() {
    let addr = ipv4!([127, 0, 0, 1]);
    let (_endpoint, _receiver) = UdpEndpoint::bind(addr)
        .await
        .expect("Should bind to the address.");
}

#[tokio::test]
async fn send() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let (endpoint, _receiver) = UdpEndpoint::bind(src)
        .await
        .expect("Should bind to the address.");
    let task = tokio::spawn(async move {
        let socket = UdpSocket::bind(dst)
            .await
            .expect("Should bind to the address.");
        let mut buf = [0u8; 128];
        let (_size, addr) = socket
            .recv_from(&mut buf[..])
            .await
            .expect("Should receive a message.");
        assert_eq!(src, addr, "Message should come from the endpoint.");
        assert_eq!(buf[0], 1, "Message should not be empty.");
    });
    let sender = endpoint.sender();
    sender
        .send((dst, [1u8; 128]))
        .await
        .expect("Should send the message.");
    task.await.expect("Should complete the task.");
}

#[tokio::test]
async fn recv() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = ipv4!([127, 0, 0, 2]);
    let (_endpoint, mut receiver) = UdpEndpoint::bind(src)
        .await
        .expect("Should bind to the address.");
    let task = tokio::spawn(async move {
        let (addr, msg) = receiver.recv().await.expect("Should receive a message.");
        assert_eq!(dst, addr, "Message should come from the remote address.");
        assert_eq!(msg[0], 1, "Message should not be empty.");
    });
    let socket = UdpSocket::bind(dst)
        .await
        .expect("Should bind to the address.");
    let buf = [1u8; 128];
    let _size = socket
        .send_to(&buf[..], src)
        .await
        .expect("Should send the message to the endpoint.");
    task.await.expect("Should complete the task.");
}
