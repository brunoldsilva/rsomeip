use super::{Operation, Socket};
use crate::testing::ipv4;
use tokio::sync::mpsc;

#[tokio::test]
async fn new() {
    let (sender, _) = mpsc::channel(1);
    let _socket = Socket::new(sender);
}

#[tokio::test]
async fn send_messages() {
    let (sender, mut receiver) = mpsc::channel(1);
    let socket = Socket::new(sender);
    tokio::spawn(async move {
        let message = receiver.recv().await.expect("should receive a message");
        let (op, response) = message.into_parts();
        assert!(matches!(op, Operation::Send(_)));
        response.ok(());
        let message = receiver.recv().await.expect("should receive a message");
        let (op, response) = message.into_parts();
        assert!(matches!(op, Operation::Connect(_)));
        response.ok(());
        let message = receiver.recv().await.expect("should receive a message");
        let (op, response) = message.into_parts();
        assert!(matches!(op, Operation::Disconnect(_)));
        response.ok(());
        let message = receiver.recv().await.expect("should receive a message");
        let (op, response) = message.into_parts();
        assert!(matches!(op, Operation::Open));
        response.ok(());
        let message = receiver.recv().await.expect("should receive a message");
        let (op, response) = message.into_parts();
        assert!(matches!(op, Operation::Close));
        response.ok(());
    });
    socket
        .send(ipv4!([127, 0, 0, 1]), std::sync::Arc::new([1u8; 16]))
        .await
        .expect("should get an Ok response");
    socket
        .connect(ipv4!([127, 0, 0, 1]))
        .await
        .expect("should get an Ok response");
    socket
        .disconnect(ipv4!([127, 0, 0, 1]))
        .await
        .expect("should get an Ok response");
    socket.open().await.expect("should get an Ok response");
    socket.close().await.expect("should get an Ok response");
}

#[tokio::test]
async fn closed_socket() {
    let (sender, mut receiver) = mpsc::channel(1);
    let socket = Socket::new(sender);
    receiver.close();
    socket
        .open()
        .await
        .expect_err("should get an error sending a message when the channel is closed");
}
