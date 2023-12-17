use thiserror::Error;

#[cfg(test)]
mod tests;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("operation failed unexpectedly: {0}")]
    Failure(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
