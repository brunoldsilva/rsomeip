use crate::testing::ipv4;
use std::time::Duration;

use super::*;

#[test]
fn new() {
    let (tx, _rx) = mpsc::channel(1);
    let _controller = Controller::new(tx);
}

#[tokio::test]
async fn connect() {
    let (tx, mut rx) = mpsc::channel::<Command>(1);
    let target = ipv4!([127, 0, 0, 1]);
    let target_copy = target;
    let handle = tokio::spawn(async move {
        let command = rx
            .recv()
            .await
            .expect("should receive a command from the controller");
        let (request, tx) = command.into_parts();
        let Request::Connect(target) = request else {
            panic!("should receive a Connect request");
        };
        assert_eq!(target, target_copy);
        tx.send(Ok(()))
            .await
            .expect("should send the response to the controller");
    });
    let controller = Controller::new(tx);
    controller
        .connect(target)
        .await
        .expect("should receive an Ok response");
    tokio::time::timeout(Duration::from_millis(10), handle)
        .await
        .expect("task should not timeout")
        .expect("task should succeed");
}

#[tokio::test]
async fn disconnect() {
    let (tx, mut rx) = mpsc::channel::<Command>(1);
    let target = ipv4!([127, 0, 0, 1]);
    let target_copy = target;
    let handle = tokio::spawn(async move {
        let command = rx
            .recv()
            .await
            .expect("should receive a command from the controller");
        let (request, tx) = command.into_parts();
        let Request::Disconnect(target) = request else {
            panic!("should receive a Disconnect request");
        };
        assert_eq!(target, target_copy);
        tx.send(Ok(()))
            .await
            .expect("should send the response to the controller");
    });
    let controller = Controller::new(tx);
    controller
        .disconnect(target)
        .await
        .expect("should receive an Ok response");
    tokio::time::timeout(Duration::from_millis(10), handle)
        .await
        .expect("task should not timeout")
        .expect("task should succeed");
}

#[tokio::test]
async fn send() {
    let (tx, mut rx) = mpsc::channel::<Command>(1);
    let target = ipv4!([127, 0, 0, 1]);
    let target_copy = target;
    let handle = tokio::spawn(async move {
        let command = rx
            .recv()
            .await
            .expect("should receive a command from the controller");
        let (request, tx) = command.into_parts();
        let Request::Send { msg, dst } = request else {
            panic!("should receive a Send request");
        };
        assert_eq!(msg.message_id(), 0x_1234_5678_u32);
        assert_eq!(dst, target_copy);
        tx.send(Ok(()))
            .await
            .expect("should send the response to the controller");
    });
    let msg = Message::builder()
        .service_id(0x1234)
        .method_id(0x5678)
        .build();
    let controller = Controller::new(tx);
    controller
        .send(msg, target)
        .await
        .expect("should receive an Ok response");
    tokio::time::timeout(Duration::from_millis(10), handle)
        .await
        .expect("task should not timeout")
        .expect("task should succeed");
}

#[tokio::test]
async fn relay() {
    let (tx, mut rx) = mpsc::channel::<Command>(1);
    let message_id = 0x1234_5678_u32;
    let handle = tokio::spawn(async move {
        let command = rx
            .recv()
            .await
            .expect("should receive a command from the controller");
        let (request, tx_response) = command.into_parts();
        #[allow(unused_variables)]
        let Request::Relay { id, tx } = request
        else {
            panic!("should receive a Relay request");
        };
        assert_eq!(id, message_id);
        tx_response
            .send(Ok(()))
            .await
            .expect("should send the response to the controller");
    });
    let controller = Controller::new(tx);
    let (tx, mut _rx) = mpsc::channel(1);
    controller
        .relay(message_id, tx)
        .await
        .expect("should receive an Ok response");
    tokio::time::timeout(Duration::from_millis(10), handle)
        .await
        .expect("task should not timeout")
        .expect("task should succeed");
}

#[tokio::test]
async fn shutdown() {
    let (tx, mut rx) = mpsc::channel::<Command>(1);
    let handle = tokio::spawn(async move {
        let command = rx
            .recv()
            .await
            .expect("should receive a command from the controller");
        let (request, tx) = command.into_parts();
        let Request::Shutdown = request else {
            panic!("should receive a Shutdown request");
        };
        tx.send(Ok(()))
            .await
            .expect("should send the response to the controller");
    });
    let controller = Controller::new(tx);
    controller
        .shutdown()
        .await
        .expect("should receive an Ok response");
    tokio::time::timeout(Duration::from_millis(10), handle)
        .await
        .expect("task should not timeout")
        .expect("task should succeed");
}
