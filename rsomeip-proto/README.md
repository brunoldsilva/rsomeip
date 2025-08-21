# rsomeip-proto

Sans-IO implementation of the SOME/IP protocol.

## Overview

This crate provides an implementation of the core SOME/IP protocol, mainly
focused on message processing.

It provides the basic primitives used by the protocol as well as endpoint and
interface abstractions to check messages for correctness.

## Features

- **tp**: Implementation of the SOME/IP Transport Protocol for segmenting large
  messages.

## Getting Started

1. Add `rsomeip-proto` as a dependency to your project.

   ```toml
   # Cargo.toml

   [dependencies]
   rsomeip-proto = "0.1.0"
   ```

2. Create an [`Endpoint`] and serve an [`Interface`]. Use the
   [`Endpoint::process`] and [`Endpoint::poll`] methods to serialize and
   deserialize messages.

   ```rust
   use rsomeip_proto::{Endpoint, ServiceId, Interface, MethodId, MessageType};
   use rsomeip_bytes::{Bytes, BytesMut};

   type Message = rsomeip_proto::Message<Bytes>;

   // Create an endpoint with a stub and a proxy.
   let endpoint = Endpoint::new()
       .with_interface(
           ServiceId::new(0x0001),
           Interface::default().with_method(MethodId::default()),
       )
       .with_interface(
           ServiceId::new(0x0002),
           Interface::default().with_method(MethodId::default()).into_proxy(),
       );

   // Poll for incoming messages.
   let mut buffer = Message::default()
       .with_service(ServiceId::new(0x0001))
       .to_bytes()
       .expect("should serialize the message");
   assert_eq!(
       endpoint.poll(&mut buffer),
       Ok(Message::default().with_service(ServiceId::new(0x0001)))
   );

   // Process outgoing messages.
   let mut buffer = BytesMut::with_capacity(16);
   assert_eq!(
       endpoint.process(
           Message::default().with_service(ServiceId::new(0x0002)),
           &mut buffer
       ),
       Ok(16)
   );
   assert_eq!(
       buffer.freeze(),
       Message::default()
           .with_service(ServiceId::new(0x0002))
           .to_bytes()
           .expect("should serialize")
   );
   ```

## Motivation

This crate is intended to be used as the core for developing more complex crates
based on the SOME/IP protocol.

## License

This project is licensed under either the [Apache-2.0 License] or [MIT License],
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

[Apache-2.0 License]: http://www.apache.org/licenses/LICENSE-2.0
[MIT License]: http://opensource.org/licenses/MIT
