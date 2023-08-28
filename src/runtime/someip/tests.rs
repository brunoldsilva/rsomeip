#![cfg(test)]

use super::*;

#[test]
fn new() {
    let msg = Message::new();
    assert_eq!(msg.length, 8, "Should include rest of message.");
    assert_eq!(msg.protocol_version, 1, "Should always be set to 1.");
}

#[test]
fn default() {
    let msg = Message::default();
    assert_eq!(msg.length, 8, "Should include rest of message.");
    assert_eq!(msg.protocol_version, 1, "Should always be set to 1.");
}

#[test]
fn build() {
    let msg = Message::builder()
        .service_id(0x1234)
        .method_id(0x5678)
        .length(0x9abc)
        .client_id(0xdef0)
        .session_id(0x1234)
        .protocol_version(2)
        .interface_version(3)
        .message_type(MessageType::Error)
        .return_code(ReturnCode::NotOk)
        .build();
    assert_eq!(msg.service_id, 0x1234);
    assert_eq!(msg.method_id, 0x5678);
    assert_eq!(msg.length, 0x9abc);
    assert_eq!(msg.client_id, 0xdef0);
    assert_eq!(msg.session_id, 0x1234);
    assert_eq!(msg.protocol_version, 2);
    assert_eq!(msg.interface_version, 3);
    assert_eq!(msg.message_type, MessageType::Error);
    assert_eq!(msg.return_code, ReturnCode::NotOk);
}

#[test]
fn display() {
    let msg = Message::builder()
        .service_id(0x0012)
        .method_id(0x0034)
        .client_id(0x5678)
        .session_id(0x9abc)
        .build();
    assert_eq!(
        msg.to_string(),
        "[0012.0034.5678.9abc]",
        "Should display the primary message identifiers."
    );
}

#[test]
fn message_type_from_u8() {
    assert_eq!(MessageType::Request, MessageType::from(0x00));
    assert_eq!(MessageType::RequestNoReturn, MessageType::from(0x01));
    assert_eq!(MessageType::Notification, MessageType::from(0x02));
    assert_eq!(MessageType::Response, MessageType::from(0x80));
    assert_eq!(MessageType::Error, MessageType::from(0x81));
    assert_eq!(MessageType::TpRequest, MessageType::from(0x20));
    assert_eq!(MessageType::TpRequestNoReturn, MessageType::from(0x21));
    assert_eq!(MessageType::TpNotification, MessageType::from(0x22));
    assert_eq!(MessageType::TpResponse, MessageType::from(0xa0));
    assert_eq!(MessageType::TpError, MessageType::from(0xa1));
    (u8::MIN..=u8::MAX)
        .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
        .for_each(|x| assert_eq!(MessageType::Unknown(x), MessageType::from(x)));
}

#[test]
fn return_code_from_u8() {
    assert_eq!(ReturnCode::Ok, ReturnCode::from(0x00));
    assert_eq!(ReturnCode::NotOk, ReturnCode::from(0x01));
    assert_eq!(ReturnCode::UnknownService, ReturnCode::from(0x02));
    assert_eq!(ReturnCode::UnknownMethod, ReturnCode::from(0x03));
    assert_eq!(ReturnCode::NotReady, ReturnCode::from(0x04));
    assert_eq!(ReturnCode::NotReachable, ReturnCode::from(0x05));
    assert_eq!(ReturnCode::Timeout, ReturnCode::from(0x06));
    assert_eq!(ReturnCode::WrongProtocolVersion, ReturnCode::from(0x07));
    assert_eq!(ReturnCode::WrongInterfaceVersion, ReturnCode::from(0x08));
    assert_eq!(ReturnCode::MalformedMessage, ReturnCode::from(0x09));
    assert_eq!(ReturnCode::WrongMessageType, ReturnCode::from(0x0a));
    assert_eq!(ReturnCode::E2eRepeated, ReturnCode::from(0x0b));
    assert_eq!(ReturnCode::E2eWrongSequence, ReturnCode::from(0x0c));
    assert_eq!(ReturnCode::E2e, ReturnCode::from(0x0d));
    assert_eq!(ReturnCode::E2eNotAvailable, ReturnCode::from(0x0e));
    assert_eq!(ReturnCode::E2eNoNewData, ReturnCode::from(0x0f));
    (0x10..=0x5e).for_each(|x| assert_eq!(ReturnCode::Reserved(x), ReturnCode::from(x)));
    (0x5f..=u8::MAX).for_each(|x| assert_eq!(ReturnCode::Unknown(x), ReturnCode::from(x)));
}
