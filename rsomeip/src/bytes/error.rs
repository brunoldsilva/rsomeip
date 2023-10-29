pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Message(String),
    Failure,
    BufferOverflow,
    LengthOverflow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{msg}"),
            Self::Failure => write!(f, "operation failed unexpectedly"),
            Self::BufferOverflow => write!(f, "writing exceeds buffer capacity"),
            Self::LengthOverflow => write!(f, "length exceeds maximum value"),
        }
    }
}

impl std::error::Error for Error {}
