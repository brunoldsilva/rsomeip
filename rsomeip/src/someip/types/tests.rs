use self::bytes::{Deserialize, Serialize};
use super::*;

#[test]
fn message_new() {
    let message = Message::<u32>::new(0x1234, 0x5678, 0x90ab_1234);
    assert_eq!(message.service(), 0x1234);
    assert_eq!(message.method(), 0x5678);
    assert_eq!(*message.payload(), 0x90ab_1234);
}

#[test]
fn message_default() {
    let message = Message::<u32>::default();
    assert_eq!(message.service(), 0);
    assert_eq!(message.method(), 0);
    assert_eq!(*message.payload(), 0);
}

#[test]
fn message_accessors() {
    let mut message = Message::<u32>::default();
    message.set_service(0x1234);
    message.set_method(0x5678);
    message.set_payload(0x90ab_1234);
    assert_eq!(message.service(), 0x1234);
    assert_eq!(message.method(), 0x5678);
    assert_eq!(message.id(), 0x1234_5678);
    assert_eq!(*message.payload(), 0x90ab_1234);
    message.set_id(0x90ab_1234);
    assert_eq!(message.id(), 0x90ab_1234);
}

const SERIALIZED_MESSAGE: [u8; 12] = [
    0x12, 0x34, // ServiceId
    0x56, 0x78, // MethodId
    0x00, 0x00, 0x00, 0x04, // Payload Length
    0x11, 0x11, 0x22, 0x22, // Payload
];

#[test]
fn message_serialize() {
    let mut buffer = [0u8; 12];
    let mut ser = bytes::Serializer::new(buffer.as_mut());
    let message = Message::new(0x1234, 0x5678, 0x1111_2222_u32);
    assert_eq!(message.serialize(&mut ser), Ok(()));
    assert_eq!(buffer, SERIALIZED_MESSAGE);
}

#[test]
fn message_deserialize() {
    let buffer = SERIALIZED_MESSAGE;
    let mut de = bytes::Deserializer::new(buffer.as_ref());
    let message = Message::<u32>::deserialize(&mut de);
    assert_eq!(message, Ok(Message::new(0x1234, 0x5678, 0x1111_2222_u32)));
}

#[test]
fn message_display() {
    let message = Message::<u32>::new(0x1234, 0x5678, 0x1111_2222);
    assert_eq!(message.to_string(), "[M.1234.5678]");
}

#[test]
fn request_new() {
    let request = Request::new(0x1234_5678_u32);
    assert_eq!(
        request,
        Request {
            client: 0,
            session: 0,
            protocol: 1,
            interface: 0,
            message_type: MessageType::Request,
            return_code: ReturnCode::Ok,
            payload: 0x1234_5678_u32
        }
    );
}

#[test]
fn request_build() {
    assert_eq!(
        Request::build(0x1234_5678_u32).get(),
        Request::new(0x1234_5678_u32)
    );
}

#[test]
fn request_accessors() {
    let mut request = Request::new(0x1234_5678_u32);
    request.set_client(0x1234);
    request.set_session(0x5678);
    request.set_protocol(0xab);
    request.set_interface(0xcd);
    request.set_message_type(MessageType::Response);
    request.set_return_code(ReturnCode::NotOk);
    assert_eq!(request.client(), 0x1234);
    assert_eq!(request.session(), 0x5678);
    assert_eq!(request.protocol(), 0xab);
    assert_eq!(request.interface(), 0xcd);
    assert_eq!(request.message_type(), MessageType::Response);
    assert_eq!(request.return_code(), ReturnCode::NotOk);
    request.set_id(0x1111_2222);
    assert_eq!(request.id(), 0x1111_2222);
}

const SERIALIZED_REQUEST: [u8; 12] = [
    0x12, 0x34, // ClientId
    0x56, 0x78, // SessionId
    0xab, // Protocol Version
    0xcd, // Interface Version
    0x80, // MessageType
    0x01, // ReturnCode
    0x11, 0x11, 0x22, 0x22, // Payload
];

#[test]
fn request_serialize() {
    let mut buffer = [0u8; 12];
    let mut ser = bytes::Serializer::new(buffer.as_mut());
    let request = RequestBuilder::new(0x1111_2222_u32)
        .client(0x1234)
        .session(0x5678)
        .protocol(0xab)
        .interface(0xcd)
        .message_type(MessageType::Response)
        .return_code(ReturnCode::NotOk)
        .get();
    assert_eq!(request.serialize(&mut ser), Ok(()));
    assert_eq!(buffer, SERIALIZED_REQUEST);
}

#[test]
fn request_deserialize() {
    let buffer = SERIALIZED_REQUEST;
    let mut de = bytes::Deserializer::new(buffer.as_ref());
    let request = Request::<u32>::deserialize(&mut de);
    assert_eq!(
        request,
        Ok(RequestBuilder::new(0x1111_2222_u32)
            .client(0x1234)
            .session(0x5678)
            .protocol(0xab)
            .interface(0xcd)
            .message_type(MessageType::Response)
            .return_code(ReturnCode::NotOk)
            .get())
    );
}

#[test]
fn request_display() {
    let request = RequestBuilder::new(0x1234_5678_u32)
        .client(0x1234)
        .session(0x5678)
        .message_type(MessageType::Response)
        .return_code(ReturnCode::NotOk)
        .get();
    assert_eq!(request.to_string(), "[R.1234.5678.80.01]");
}

#[test]
fn request_builder_new() {
    let request = RequestBuilder::new(0x1234_5678_u32).get();
    assert_eq!(
        request,
        Request {
            client: 0,
            session: 0,
            protocol: 1,
            interface: 0,
            message_type: MessageType::Request,
            return_code: ReturnCode::Ok,
            payload: 0x1234_5678_u32
        }
    );
}

#[test]
fn request_builder_build() {
    let request = RequestBuilder::new(0x1234_5678_u32)
        .client(0x1234)
        .session(0x5678)
        .protocol(0x9a)
        .interface(0xbc)
        .message_type(MessageType::Response)
        .return_code(ReturnCode::NotOk)
        .payload(0x1111_2222_u32)
        .get();
    assert_eq!(
        request,
        Request {
            client: 0x1234,
            session: 0x5678,
            protocol: 0x9a,
            interface: 0xbc,
            message_type: MessageType::Response,
            return_code: ReturnCode::NotOk,
            payload: 0x1111_2222_u32
        }
    );
}

#[test]
fn message_type_serialize() {
    let mut buffer = [0u8; 1];
    let mut ser = bytes::Serializer::new(buffer.as_mut());
    assert_eq!(MessageType::Response.serialize(&mut ser), Ok(()));
    assert_eq!(buffer, [0x80_u8]);
}

#[test]
fn message_type_deserialize() {
    let buffer = [0x80_u8];
    let mut de = bytes::Deserializer::new(buffer.as_ref());
    assert_eq!(MessageType::deserialize(&mut de), Ok(MessageType::Response));
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
fn return_code_serialize() {
    let mut buffer = [0u8; 1];
    let mut ser = bytes::Serializer::new(buffer.as_mut());
    assert_eq!(ReturnCode::NotOk.serialize(&mut ser), Ok(()));
    assert_eq!(buffer, [0x01_u8]);
}

#[test]
fn return_code_deserialize() {
    let buffer = [0x01_u8];
    let mut de = bytes::Deserializer::new(buffer.as_ref());
    assert_eq!(ReturnCode::deserialize(&mut de), Ok(ReturnCode::NotOk));
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
