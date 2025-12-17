//! Integration tests for the rsomeip crate.
//!
//! These tests verify end-to-end communication between SOME/IP endpoints
//! using both UDP and TCP transports.

use rsomeip::{
    endpoint::{InterfaceId, Proxy as _, Server as _, Stub as _},
    socket::{tcp::TcpSocket, udp::UdpSocket, SocketAddr},
    someip::{self, MessageType, ReturnCode},
};
use rsomeip_bytes::Bytes;
use std::sync::atomic::{AtomicU16, Ordering};

/// Generates a unique port for each test to avoid conflicts.
fn unique_port() -> u16 {
    static PORT: AtomicU16 = AtomicU16::new(50000);
    PORT.fetch_add(1, Ordering::Relaxed)
}

/// Creates a unique IPv4 socket address for testing.
fn test_addr() -> SocketAddr {
    format!("127.0.0.1:{}", unique_port()).parse().unwrap()
}

mod udp {
    use super::*;
    use rsomeip::endpoint::v1::Endpoint;

    #[tokio::test]
    async fn client_sends_request_server_responds() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();
                let client_addr = test_addr();

                // Create server endpoint
                let server_socket = UdpSocket::bind(server_addr)
                    .await
                    .expect("server should bind");
                let mut server = Endpoint::spawn(server_socket);

                // Create client endpoint
                let client_socket = UdpSocket::bind(client_addr)
                    .await
                    .expect("client should bind");
                let mut client = Endpoint::spawn(client_socket);

                // Server serves an interface
                let interface = InterfaceId::new(0x1234, 1);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");

                // Client creates a proxy to the server
                let mut proxy = client
                    .proxy(interface, server_addr)
                    .await
                    .expect("client should create proxy");

                // Client sends a request
                let request = someip::Message::new(Bytes::copy_from_slice(&[1, 2, 3, 4]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_type(MessageType::Request);
                proxy.send(request).await.expect("client should send request");

                // Server receives the request
                let (received, source) = stub
                    .recv_from()
                    .await
                    .expect("server should receive request");
                assert_eq!(source, client_addr);
                assert_eq!(received.message_type, MessageType::Request);
                assert_eq!(&received.payload[..], &[1, 2, 3, 4]);

                // Server sends a response
                let response = received
                    .with_type(MessageType::Response)
                    .with_code(ReturnCode::Ok);
                stub.send_to(source, response)
                    .await
                    .expect("server should send response");

                // Client receives the response
                let response = proxy.recv().await.expect("client should receive response");
                assert_eq!(response.message_type, MessageType::Response);
                assert_eq!(response.return_code, ReturnCode::Ok);
                assert_eq!(&response.payload[..], &[1, 2, 3, 4]);
            })
            .await;
    }

    #[tokio::test]
    async fn server_sends_notification_to_client() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();
                let client_addr = test_addr();

                // Create server endpoint
                let server_socket = UdpSocket::bind(server_addr)
                    .await
                    .expect("server should bind");
                let mut server = Endpoint::spawn(server_socket);

                // Create client endpoint
                let client_socket = UdpSocket::bind(client_addr)
                    .await
                    .expect("client should bind");
                let mut client = Endpoint::spawn(client_socket);

                // Server serves an interface
                let interface = InterfaceId::new(0x5678, 2);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");

                // Client creates a proxy to the server
                let mut proxy = client
                    .proxy(interface, server_addr)
                    .await
                    .expect("client should create proxy");

                // Server sends a notification
                let notification =
                    someip::Message::new(Bytes::copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]))
                        .with_service(interface.service)
                        .with_interface(interface.version)
                        .with_type(MessageType::Notification);
                stub.send_to(client_addr, notification)
                    .await
                    .expect("server should send notification");

                // Client receives the notification
                let received = proxy
                    .recv()
                    .await
                    .expect("client should receive notification");
                assert_eq!(received.message_type, MessageType::Notification);
                assert_eq!(&received.payload[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
            })
            .await;
    }

    #[tokio::test]
    async fn multiple_request_response_exchanges() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();
                let client_addr = test_addr();

                // Create endpoints
                let server_socket = UdpSocket::bind(server_addr)
                    .await
                    .expect("server should bind");
                let mut server = Endpoint::spawn(server_socket);

                let client_socket = UdpSocket::bind(client_addr)
                    .await
                    .expect("client should bind");
                let mut client = Endpoint::spawn(client_socket);

                // Setup interface
                let interface = InterfaceId::new(0xABCD, 1);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");
                let mut proxy = client
                    .proxy(interface, server_addr)
                    .await
                    .expect("client should create proxy");

                // Send multiple requests and responses
                for i in 0..5u8 {
                    let request = someip::Message::new(Bytes::copy_from_slice(&[i]))
                        .with_service(interface.service)
                        .with_interface(interface.version)
                        .with_session(i as u16)
                        .with_type(MessageType::Request);
                    proxy.send(request).await.expect("should send request");

                    let (received, source) = stub.recv_from().await.expect("should receive request");
                    assert_eq!(&received.payload[..], &[i]);

                    let response = received
                        .with_type(MessageType::Response)
                        .with_payload(Bytes::copy_from_slice(&[i, i]));
                    stub.send_to(source, response)
                        .await
                        .expect("should send response");

                    let response = proxy.recv().await.expect("should receive response");
                    assert_eq!(&response.payload[..], &[i, i]);
                }
            })
            .await;
    }

    #[tokio::test]
    async fn multiple_clients_single_server() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();

                // Create server
                let server_socket = UdpSocket::bind(server_addr)
                    .await
                    .expect("server should bind");
                let mut server = Endpoint::spawn(server_socket);

                let interface = InterfaceId::new(0x1111, 1);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");

                // Create multiple clients
                let client1_addr = test_addr();
                let client1_socket = UdpSocket::bind(client1_addr)
                    .await
                    .expect("client1 should bind");
                let mut client1 = Endpoint::spawn(client1_socket);
                let mut proxy1 = client1
                    .proxy(interface, server_addr)
                    .await
                    .expect("client1 should create proxy");

                let client2_addr = test_addr();
                let client2_socket = UdpSocket::bind(client2_addr)
                    .await
                    .expect("client2 should bind");
                let mut client2 = Endpoint::spawn(client2_socket);
                let mut proxy2 = client2
                    .proxy(interface, server_addr)
                    .await
                    .expect("client2 should create proxy");

                // Client 1 sends request
                let request1 = someip::Message::new(Bytes::copy_from_slice(&[1]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_client(1)
                    .with_type(MessageType::Request);
                proxy1.send(request1).await.expect("client1 should send");

                // Client 2 sends request
                let request2 = someip::Message::new(Bytes::copy_from_slice(&[2]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_client(2)
                    .with_type(MessageType::Request);
                proxy2.send(request2).await.expect("client2 should send");

                // Server receives both requests and responds
                for _ in 0..2 {
                    let (received, source) = stub.recv_from().await.expect("should receive request");
                    let response = received.with_type(MessageType::Response);
                    stub.send_to(source, response)
                        .await
                        .expect("should send response");
                }

                // Both clients receive responses
                let resp1 = proxy1.recv().await.expect("client1 should receive response");
                assert_eq!(resp1.message_type, MessageType::Response);

                let resp2 = proxy2.recv().await.expect("client2 should receive response");
                assert_eq!(resp2.message_type, MessageType::Response);
            })
            .await;
    }

    #[tokio::test]
    async fn error_response_handling() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();
                let client_addr = test_addr();

                // Create endpoints
                let server_socket = UdpSocket::bind(server_addr)
                    .await
                    .expect("server should bind");
                let mut server = Endpoint::spawn(server_socket);

                let client_socket = UdpSocket::bind(client_addr)
                    .await
                    .expect("client should bind");
                let mut client = Endpoint::spawn(client_socket);

                // Setup interface
                let interface = InterfaceId::new(0x2222, 1);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");
                let mut proxy = client
                    .proxy(interface, server_addr)
                    .await
                    .expect("client should create proxy");

                // Client sends request
                let request = someip::Message::new(Bytes::copy_from_slice(&[0xFF]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_type(MessageType::Request);
                proxy.send(request).await.expect("should send request");

                // Server receives and sends error response
                let (received, source) = stub.recv_from().await.expect("should receive request");
                let error_response = received
                    .with_type(MessageType::Error)
                    .with_code(ReturnCode::NotOk);
                stub.send_to(source, error_response)
                    .await
                    .expect("should send error");

                // Client receives error
                let response = proxy.recv().await.expect("should receive error");
                assert_eq!(response.message_type, MessageType::Error);
                assert_eq!(response.return_code, ReturnCode::NotOk);
            })
            .await;
    }
}

mod tcp {
    use super::*;
    use rsomeip::endpoint::v1::Endpoint;

    #[tokio::test]
    async fn client_connects_and_exchanges_messages() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let server_addr = test_addr();
                let client_addr = test_addr();

                // Create server endpoint with TCP
                let server_socket = TcpSocket::new(server_addr);
                let mut server = Endpoint::spawn(server_socket);

                // Server serves an interface
                let interface = InterfaceId::new(0x3333, 1);
                let mut stub = server
                    .serve(interface)
                    .await
                    .expect("server should serve interface");

                // Create client endpoint with TCP
                let client_socket = TcpSocket::new(client_addr);
                let mut client = Endpoint::spawn(client_socket);

                // Client creates a proxy (this establishes TCP connection)
                let mut proxy = client
                    .proxy(interface, server_addr)
                    .await
                    .expect("client should create proxy");

                // Client sends a request
                let request = someip::Message::new(Bytes::copy_from_slice(&[0xCA, 0xFE]))
                    .with_service(interface.service)
                    .with_interface(interface.version)
                    .with_type(MessageType::Request);
                proxy.send(request).await.expect("client should send request");

                // Server receives the request
                let (received, _source) = stub
                    .recv_from()
                    .await
                    .expect("server should receive request");
                assert_eq!(received.message_type, MessageType::Request);
                assert_eq!(&received.payload[..], &[0xCA, 0xFE]);
            })
            .await;
    }
}
