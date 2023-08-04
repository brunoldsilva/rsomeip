#![cfg(test)]

use super::*;
use crate::testing::{ipv4, port};
use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn connect() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = Some(ipv4!([127, 0, 0, 2]));
    let connection = Factory::connect(src, dst)
        .await
        .expect("Should create a connection.");
    assert!(
        connection.sender.is_some(),
        "Should create a sender channel."
    );
}

#[tokio::test]
async fn connect_no_dst() {
    let src = ipv4!([127, 0, 0, 1]);
    let connection = Factory::connect(src, None)
        .await
        .expect("Should create a connection.");
    assert!(
        connection.sender.is_none(),
        "Shouldn't create a sender channel."
    );
}

macro_rules! send_buf {
    ($payload:tt, $src:tt, $dst:tt) => {
        let socket = UdpSocket::bind($src)
            .await
            .expect("Should bind to the address.");
        socket
            .connect($dst)
            .await
            .expect("Should connect to the address.");
        socket
            .send(&$payload[..])
            .await
            .expect("Should send the buffer.");
    };
}

#[tokio::test]
async fn receive() {
    const PORT: u16 = port!();
    const SRC: SocketAddr = ipv4!([127, 0, 0, 1], PORT);
    const DST: SocketAddr = ipv4!([127, 0, 0, 2], PORT);
    let mut connection = Factory::connect(SRC, Some(DST))
        .await
        .expect("Should create a connection.");

    let task = tokio::spawn(async move {
        let buf = connection
            .receiver
            .recv()
            .await
            .expect("Should receive a buffer.");
        let is_empty: bool = buf[..].iter().fold(true, |acc, e| acc & (*e == 0u8));
        assert!(
            !is_empty,
            "The contents of the buffer should not be zero (empty)."
        );
    });

    let buf = [1u8; 1024];
    send_buf!(buf, DST, SRC);
    task.await.expect("Should complete the task.");
}

#[tokio::test]
async fn receive_no_dst() {
    const PORT: u16 = port!();
    const SRC: SocketAddr = ipv4!([127, 0, 0, 1], PORT);
    const DST: SocketAddr = ipv4!([127, 0, 0, 2], PORT);
    let mut connection = Factory::connect(SRC, None)
        .await
        .expect("Should create a connection.");

    let handle = tokio::spawn(async move {
        let buf = connection
            .receiver
            .recv()
            .await
            .expect("Should receive a buffer.");
        let is_empty: bool = buf[..].iter().fold(true, |acc, e| acc & (*e == 0u8));
        assert!(
            !is_empty,
            "The contents of the buffer should not be zero (empty)."
        );
    });

    let buf = [1u8; 1024];
    send_buf!(buf, DST, SRC);
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn send() {
    const PORT: u16 = port!();
    const SRC: SocketAddr = ipv4!([127, 0, 0, 1], PORT);
    const DST: SocketAddr = ipv4!([127, 0, 0, 2], PORT);
    let connection = Factory::connect(SRC, Some(DST))
        .await
        .expect("Should create a connection.");

    let handle = tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let socket = UdpSocket::bind(DST)
            .await
            .expect("Should bind to the address.");
        socket
            .connect(SRC)
            .await
            .expect("Should connect to the address.");
        socket
            .recv(&mut buf[..])
            .await
            .expect("Should send the buffer.");
        let is_empty: bool = buf[..].iter().fold(true, |acc, e| acc & (*e == 0u8));
        assert!(
            !is_empty,
            "The contents of the buffer should not be zero (empty)."
        );
    });

    let buf = [1u8; 1024];
    connection
        .sender
        .expect("Should exist.")
        .send(buf)
        .await
        .expect("Should send the buffer.");
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn drop() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = Some(ipv4!([127, 0, 0, 2]));
    let connection = Factory::connect(src, dst)
        .await
        .expect("Should create a connection.");
    let socket = Arc::downgrade(&connection.connection);
    assert!(socket.upgrade().is_some(), "Should exist.");
    std::mem::drop(connection);
    assert!(
        socket.upgrade().is_none(),
        "Should drop the socket with the connection."
    );
}
