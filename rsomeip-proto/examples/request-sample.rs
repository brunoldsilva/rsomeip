//! Request sample.
//!
//! Example of a client that sends request to a server.

#![expect(clippy::expect_used, reason = "helps to reduce verbosity")]

use rsomeip_bytes::{BytesMut, Serialize as _};
use rsomeip_proto::{ClientId, Endpoint, Interface, SessionId};
use std::{net::UdpSocket, thread, time::Duration};

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
    #[expect(clippy::infinite_loop, reason = "user must manually stop the process")]
    #[expect(clippy::print_stdout, reason = "used to show events to the user")]
    #[expect(clippy::use_debug, reason = "Vec doesn't implement Display")]
    loop {
        // Clone the request and increment the session id.
        let request = request.clone().with_session(session_id.increment());
        println!("> {request} {:02x?}", request.body);

        // Process the request into bytes.
        let mut write_buffer = BytesMut::with_capacity(request.size_hint());
        endpoint
            .process(request, &mut write_buffer)
            .expect("should process the request");

        // Send the data through the socket.
        socket
            .send_to(&write_buffer.freeze(), stub_address())
            .expect("should send the data.");

        // Wait for a response.
        let mut read_buffer = BytesMut::zeroed(64);
        let (size, _) = socket
            .recv_from(&mut read_buffer)
            .expect("should receive the data");

        // Process the response.
        let response: Message = endpoint
            .poll(&mut read_buffer.split_to(size).freeze())
            .expect("should process the response");
        println!("< {response} {:02x?}", response.body);

        // Wait before sending another request.
        thread::sleep(Duration::from_secs(1));
    }
}
