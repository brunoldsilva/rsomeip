pub(crate) mod message;
pub use message::{Header, Message, MessageError};

pub(crate) mod primitives;
pub use primitives::{
    ClientId, InterfaceVersion, MessageId, MessageType, MessageTypeField, MethodId,
    ProtocolVersion, RequestId, ReturnCode, ServiceId, SessionId,
};
