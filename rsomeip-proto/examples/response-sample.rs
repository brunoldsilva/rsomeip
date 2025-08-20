#![allow(clippy::expect_used, reason = "helps to reduce verbosity")]

use rsomeip_bytes::{BytesMut, Serialize};
use rsomeip_proto::{Endpoint, Interface, MessageType};
use std::net::UdpSocket;

mod common;
use common::{SAMPLE_METHOD_ID, SAMPLE_SERVICE_ID, stub_address};

// A SOME/IP message.
type Message = rsomeip_proto::Message<Vec<u8>>;

fn main() {
    // Bind an UDP socket.
    let socket = UdpSocket::bind(stub_address()).expect("should bind the socket");

    // Create the SOME/IP endpoint.
    let endpoint = Endpoint::new().with_interface(
        SAMPLE_SERVICE_ID,
        Interface::default()
            .with_method(SAMPLE_METHOD_ID)
            .into_stub(),
    );

    // Process requests and send back responses.
    send_responses(&socket, &endpoint);
}

fn send_responses(socket: &UdpSocket, endpoint: &Endpoint) {
    // Continuously process requests from service consumers.
    loop {
        // Wait for a request.
        let mut buffer = BytesMut::zeroed(64);
        let (size, remote_address) = socket
            .recv_from(&mut buffer[..])
            .expect("should receive the data");

        // Process the request.
        let request: Message = endpoint
            .poll(&mut buffer.split_to(size).freeze())
            .expect("should process the request");
        println!("< {request} {:02x?}", request.body);

        // Create a response.
        let response = request.clone().with_message_type(MessageType::Response);
        println!("> {response} {:02x?}", response.body);

        // Process the response into bytes.
        let mut buffer = BytesMut::with_capacity(request.size_hint());
        endpoint
            .process(response, &mut buffer)
            .expect("should process the response");

        // Send the data through the socket.
        socket
            .send_to(&buffer.freeze(), remote_address)
            .expect("should send the data.");
    }
}
