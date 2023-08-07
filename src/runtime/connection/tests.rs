#![cfg(test)]

use super::*;
use crate::testing::ipv4;
use tokio::net::TcpSocket;

#[tokio::test]
async fn connect_udp() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = Some(ipv4!([127, 0, 0, 2]));
    Factory::connect(Protocol::Udp, src, dst)
        .await
        .expect("Should create an UDP connection.");
}

#[tokio::test]
async fn connect_tcp() {
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
    Factory::connect(Protocol::Tcp, src, Some(dst))
        .await
        .expect("Should create a Tcp connection.");
    handle.await.expect("Should complete the task.");
}

#[tokio::test]
async fn listen_tcp() {
    let src = ipv4!([127, 0, 0, 1]);
    Factory::listen(Protocol::Tcp, src)
        .await
        .expect("Should create a TCP connection.");
}
