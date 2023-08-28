mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub service_id: u16,
    pub method_id: u16,
    pub length: u32,
    pub client_id: u16,
    pub session_id: u16,
    pub protocol_version: u8,
    pub interface_version: u8,
    pub message_type: MessageType,
    pub return_code: ReturnCode,
    pub payload: Vec<u8>,
}

impl Message {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn builder() -> MessageBuilder {
        MessageBuilder::new()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            service_id: Default::default(),
            method_id: Default::default(),
            length: 8,
            client_id: Default::default(),
            session_id: Default::default(),
            protocol_version: 1,
            interface_version: Default::default(),
            message_type: MessageType::default(),
            return_code: ReturnCode::default(),
            payload: Vec::default(),
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:04x}.{:04x}.{:04x}.{:04x}]",
            self.service_id, self.method_id, self.client_id, self.session_id
        )
    }
}

pub struct MessageBuilder {
    msg: Message,
}

#[allow(clippy::missing_const_for_fn)]
impl MessageBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            msg: Message::default(),
        }
    }

    pub fn service_id(mut self, value: u16) -> Self {
        self.msg.service_id = value;
        self
    }

    pub fn method_id(mut self, value: u16) -> Self {
        self.msg.method_id = value;
        self
    }

    pub fn length(mut self, value: u32) -> Self {
        self.msg.length = value;
        self
    }

    pub fn client_id(mut self, value: u16) -> Self {
        self.msg.client_id = value;
        self
    }

    pub fn session_id(mut self, value: u16) -> Self {
        self.msg.session_id = value;
        self
    }

    pub fn protocol_version(mut self, value: u8) -> Self {
        self.msg.protocol_version = value;
        self
    }

    pub fn interface_version(mut self, value: u8) -> Self {
        self.msg.interface_version = value;
        self
    }

    pub fn message_type(mut self, value: MessageType) -> Self {
        self.msg.message_type = value;
        self
    }

    pub fn return_code(mut self, value: ReturnCode) -> Self {
        self.msg.return_code = value;
        self
    }

    #[must_use]
    pub fn build(self) -> Message {
        self.msg
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageType {
    #[default]
    Request = 0x00,
    RequestNoReturn = 0x01,
    Notification = 0x02,
    Response = 0x80,
    Error = 0x81,
    TpRequest = 0x20,
    TpRequestNoReturn = 0x21,
    TpNotification = 0x22,
    TpResponse = 0xa0,
    TpError = 0xa1,
    Unknown(u8),
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Request,
            0x01 => Self::RequestNoReturn,
            0x02 => Self::Notification,
            0x80 => Self::Response,
            0x81 => Self::Error,
            0x20 => Self::TpRequest,
            0x21 => Self::TpRequestNoReturn,
            0x22 => Self::TpNotification,
            0xa0 => Self::TpResponse,
            0xa1 => Self::TpError,
            x => Self::Unknown(x),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReturnCode {
    #[default]
    Ok = 0x00,
    NotOk = 0x01,
    UnknownService = 0x02,
    UnknownMethod = 0x03,
    NotReady = 0x04,
    NotReachable = 0x05,
    Timeout = 0x06,
    WrongProtocolVersion = 0x07,
    WrongInterfaceVersion = 0x08,
    MalformedMessage = 0x09,
    WrongMessageType = 0x0a,
    E2eRepeated = 0x0b,
    E2eWrongSequence = 0x0c,
    E2e = 0x0d,
    E2eNotAvailable = 0x0e,
    E2eNoNewData = 0x0f,
    Reserved(u8),
    Unknown(u8),
}

impl From<u8> for ReturnCode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Ok,
            0x01 => Self::NotOk,
            0x02 => Self::UnknownService,
            0x03 => Self::UnknownMethod,
            0x04 => Self::NotReady,
            0x05 => Self::NotReachable,
            0x06 => Self::Timeout,
            0x07 => Self::WrongProtocolVersion,
            0x08 => Self::WrongInterfaceVersion,
            0x09 => Self::MalformedMessage,
            0x0a => Self::WrongMessageType,
            0x0b => Self::E2eRepeated,
            0x0c => Self::E2eWrongSequence,
            0x0d => Self::E2e,
            0x0e => Self::E2eNotAvailable,
            0x0f => Self::E2eNoNewData,
            n if (0x10..=0x5e).contains(&n) => Self::Reserved(n),
            n => Self::Unknown(n),
        }
    }
}
