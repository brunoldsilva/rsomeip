#![allow(clippy::expect_used, reason = "helps to reduce verbosity")]

use rsomeip_bytes::{BytesMut, Serialize};
use rsomeip_proto::{ClientId, Endpoint, Interface, SessionId};
use std::net::UdpSocket;

mod common;
use common::{SAMPLE_METHOD_ID, SAMPLE_SERVICE_ID, proxy_address, stub_address};

// A SOME/IP message.
type Message = rsomeip_proto::Message<Vec<u8>>;

fn main() {
    // Bind an UDP socket.
    let socket = UdpSocket::bind(proxy_address()).expect("should bind the socket");

    // Create the SOME/IP endpoint.
    let endpoint = Endpoint::new().with_interface(
        SAMPLE_SERVICE_ID,
        Interface::default()
            .with_method(SAMPLE_METHOD_ID)
            .into_proxy(),
    );

    // Send requests to the service and process the responses.
    send_requests(&socket, &endpoint);
}

fn send_requests(socket: &UdpSocket, endpoint: &Endpoint) {
    // Create a request to send to the service provider.
    let mut session_id = SessionId::ENABLED;
    let request = Message::default()
        .with_service(SAMPLE_SERVICE_ID)
        .with_method(SAMPLE_METHOD_ID)
        .with_client(ClientId::new(0x0001))
        .with_body((0..10).collect::<Vec<u8>>());

    // Continuously send requests to the service provider.
    loop {
        // Clone the request and increment the session id.
        let request = request.clone().with_session(session_id.increment());
        println!("> {request} {:02x?}", request.body);

        // Process the request into bytes.
        let mut buffer = BytesMut::with_capacity(request.size_hint());
        endpoint
            .process(request, &mut buffer)
            .expect("should process the request");

        // Send the data through the socket.
        socket
            .send_to(&buffer.freeze(), stub_address())
            .expect("should send the data.");

        // Wait for a response.
        let mut buffer = BytesMut::zeroed(64);
        let (size, _) = socket
            .recv_from(&mut buffer[..])
            .expect("should receive the data");

        // Process the response.
        let response: Message = endpoint
            .poll(&mut buffer.split_to(size).freeze())
            .expect("should process the response");
        println!("< {response} {:02x?}", response.body);

        // Wait before sending another request.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
