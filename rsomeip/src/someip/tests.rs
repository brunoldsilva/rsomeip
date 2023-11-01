#![cfg(test)]

use super::*;

#[test]
fn new() {
    let message = Message::new();
    assert_eq!(message, Message::default());
}

#[test]
fn default() {
    let message = Message::default();
    assert_eq!(message.service_id, 0);
    assert_eq!(message.method_id, 0);
}

#[test]
fn build() {
    let message = Message::builder()
        .service_id(0x1234)
        .method_id(0x5678)
        .build();
    assert_eq!(message.service_id, 0x1234);
    assert_eq!(message.method_id, 0x5678);
}

#[test]
fn display() {
    let payload = Payload::builder()
        .client_id(0x0034)
        .session_id(0x00cd)
        .data(vec![1, 2, 3, 4])
        .build();
    let message = Message::builder()
        .service_id(0x0012)
        .method_id(0x00ab)
        .payload(payload)
        .build();
    assert_eq!(message.to_string(), "[0012.00ab][0034.00cd]{4}");
}

const SERIALIZED_MESSAGE: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x00, 0x00, 0x00, 0x08, // Service, Method and Length
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // Payload
];

#[test]
fn serialize() {
    let mut buf = [0u8; 16];
    let mut ser = bytes::Serializer::new(&mut buf);
    let message = Message::builder()
        .service_id(0x1234)
        .method_id(0x5678)
        .build();
    assert_eq!(message.serialize(&mut ser), Ok(()));
    assert_eq!(buf, SERIALIZED_MESSAGE);
}

#[test]
fn deserialize() {
    let buf = SERIALIZED_MESSAGE;
    let mut de = bytes::Deserializer::new(&buf);
    let result = Message::deserialize(&mut de);
    let expected = Message::builder()
        .service_id(0x1234)
        .method_id(0x5678)
        .build();
    assert_eq!(result, Ok(expected));
}
