#![cfg(test)]

use super::*;

#[test]
fn new() {
    assert_eq!(Payload::new(), Payload::default());
}

#[test]
fn default() {
    let payload = Payload::default();
    assert_eq!(payload.client_id, 0);
    assert_eq!(payload.session_id, 0);
    assert_eq!(payload.protocol_version, 1);
    assert_eq!(payload.interface_version, 0);
    assert_eq!(payload.message_type, MessageType::Request);
    assert_eq!(payload.return_code, ReturnCode::Ok);
}

#[test]
fn display() {
    let payload = Payload::builder()
        .client_id(0x0012)
        .session_id(0x00ab)
        .data(vec![1, 2, 3, 4])
        .build();
    assert_eq!(payload.to_string(), "[0012.00ab]{4}");
}

#[test]
fn build() {
    let payload = Payload::builder()
        .client_id(0xdef0)
        .session_id(0x1234)
        .protocol_version(2)
        .interface_version(3)
        .message_type(MessageType::Error)
        .return_code(ReturnCode::NotOk)
        .data(vec![1, 2, 3, 4])
        .build();
    assert_eq!(payload.client_id, 0xdef0);
    assert_eq!(payload.session_id, 0x1234);
    assert_eq!(payload.protocol_version, 2);
    assert_eq!(payload.interface_version, 3);
    assert_eq!(payload.message_type, MessageType::Error);
    assert_eq!(payload.return_code, ReturnCode::NotOk);
    assert_eq!(payload.data, vec![1, 2, 3, 4]);
}

const SERIALIZED_PAYLOAD: [u8; 12] = [
    0x12, 0x34, 0x56, 0x78, 0x02, 0x03, 0x81, 0x01, 0x01, 0x02, 0x03, 0x04,
];

#[test]
fn serialize_payload() {
    let mut buf = [0u8; 12];
    let mut ser = bytes::Serializer::new(&mut buf);
    let payload = Payload::builder()
        .client_id(0x1234)
        .session_id(0x5678)
        .protocol_version(2)
        .interface_version(3)
        .message_type(MessageType::Error)
        .return_code(ReturnCode::NotOk)
        .data(vec![1, 2, 3, 4])
        .build();
    assert_eq!(payload.serialize(&mut ser), Ok(()));
    assert_eq!(buf, SERIALIZED_PAYLOAD);
}

#[test]
fn deserialize_payload() {
    let buf = SERIALIZED_PAYLOAD;
    let mut de = bytes::Deserializer::new(&buf);
    assert_eq!(de.set_limit(Some(12)), Ok(()));
    let result = Payload::deserialize(&mut de);
    let expected = Payload::builder()
        .client_id(0x1234)
        .session_id(0x5678)
        .protocol_version(2)
        .interface_version(3)
        .message_type(MessageType::Error)
        .return_code(ReturnCode::NotOk)
        .data(vec![1, 2, 3, 4])
        .build();
    assert_eq!(result, Ok(expected));
}

#[test]
fn message_type_from_u8() {
    assert_eq!(MessageType::from(0x00), MessageType::Request);
    assert_eq!(MessageType::from(0x01), MessageType::RequestNoReturn);
    assert_eq!(MessageType::from(0x02), MessageType::Notification);
    assert_eq!(MessageType::from(0x80), MessageType::Response);
    assert_eq!(MessageType::from(0x81), MessageType::Error);
    assert_eq!(MessageType::from(0x20), MessageType::TpRequest);
    assert_eq!(MessageType::from(0x21), MessageType::TpRequestNoReturn);
    assert_eq!(MessageType::from(0x22), MessageType::TpNotification);
    assert_eq!(MessageType::from(0xa0), MessageType::TpResponse);
    assert_eq!(MessageType::from(0xa1), MessageType::TpError);
    (u8::MIN..=u8::MAX)
        .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
        .for_each(|x| assert_eq!(MessageType::Unknown(x), MessageType::from(x)));
}

#[test]
fn u8_from_message_type() {
    assert_eq!(0x00, u8::from(MessageType::Request));
    assert_eq!(0x01, u8::from(MessageType::RequestNoReturn));
    assert_eq!(0x02, u8::from(MessageType::Notification));
    assert_eq!(0x80, u8::from(MessageType::Response));
    assert_eq!(0x81, u8::from(MessageType::Error));
    assert_eq!(0x20, u8::from(MessageType::TpRequest));
    assert_eq!(0x21, u8::from(MessageType::TpRequestNoReturn));
    assert_eq!(0x22, u8::from(MessageType::TpNotification));
    assert_eq!(0xa0, u8::from(MessageType::TpResponse));
    assert_eq!(0xa1, u8::from(MessageType::TpError));
    (u8::MIN..=u8::MAX)
        .filter(|x| ![0x00, 0x01, 0x02, 0x80, 0x81, 0x20, 0x21, 0x22, 0xa0, 0xa1].contains(x))
        .for_each(|x| assert_eq!(u8::from(MessageType::Unknown(x)), x));
}

#[test]
fn return_code_from_u8() {
    assert_eq!(ReturnCode::from(0x00), ReturnCode::Ok);
    assert_eq!(ReturnCode::from(0x01), ReturnCode::NotOk);
    assert_eq!(ReturnCode::from(0x02), ReturnCode::UnknownService);
    assert_eq!(ReturnCode::from(0x03), ReturnCode::UnknownMethod);
    assert_eq!(ReturnCode::from(0x04), ReturnCode::NotReady);
    assert_eq!(ReturnCode::from(0x05), ReturnCode::NotReachable);
    assert_eq!(ReturnCode::from(0x06), ReturnCode::Timeout);
    assert_eq!(ReturnCode::from(0x07), ReturnCode::WrongProtocolVersion);
    assert_eq!(ReturnCode::from(0x08), ReturnCode::WrongInterfaceVersion);
    assert_eq!(ReturnCode::from(0x09), ReturnCode::MalformedMessage);
    assert_eq!(ReturnCode::from(0x0a), ReturnCode::WrongMessageType);
    assert_eq!(ReturnCode::from(0x0b), ReturnCode::E2eRepeated);
    assert_eq!(ReturnCode::from(0x0c), ReturnCode::E2eWrongSequence);
    assert_eq!(ReturnCode::from(0x0d), ReturnCode::E2e);
    assert_eq!(ReturnCode::from(0x0e), ReturnCode::E2eNotAvailable);
    assert_eq!(ReturnCode::from(0x0f), ReturnCode::E2eNoNewData);
    (0x10..=0x5e).for_each(|x| assert_eq!(ReturnCode::Reserved(x), ReturnCode::from(x)));
    (0x5f..=u8::MAX).for_each(|x| assert_eq!(ReturnCode::Unknown(x), ReturnCode::from(x)));
}

#[test]
fn u8_from_return_code() {
    assert_eq!(0x00, u8::from(ReturnCode::Ok));
    assert_eq!(0x01, u8::from(ReturnCode::NotOk));
    assert_eq!(0x02, u8::from(ReturnCode::UnknownService));
    assert_eq!(0x03, u8::from(ReturnCode::UnknownMethod));
    assert_eq!(0x04, u8::from(ReturnCode::NotReady));
    assert_eq!(0x05, u8::from(ReturnCode::NotReachable));
    assert_eq!(0x06, u8::from(ReturnCode::Timeout));
    assert_eq!(0x07, u8::from(ReturnCode::WrongProtocolVersion));
    assert_eq!(0x08, u8::from(ReturnCode::WrongInterfaceVersion));
    assert_eq!(0x09, u8::from(ReturnCode::MalformedMessage));
    assert_eq!(0x0a, u8::from(ReturnCode::WrongMessageType));
    assert_eq!(0x0b, u8::from(ReturnCode::E2eRepeated));
    assert_eq!(0x0c, u8::from(ReturnCode::E2eWrongSequence));
    assert_eq!(0x0d, u8::from(ReturnCode::E2e));
    assert_eq!(0x0e, u8::from(ReturnCode::E2eNotAvailable));
    assert_eq!(0x0f, u8::from(ReturnCode::E2eNoNewData));
    (0x10..=0x5e).for_each(|x| assert_eq!(u8::from(ReturnCode::Reserved(x)), x));
    (0x5f..=u8::MAX).for_each(|x| assert_eq!(u8::from(ReturnCode::Unknown(x)), x));
}
