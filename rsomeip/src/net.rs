//! Network communication using SOME/IP, TCP and UDP.

/// Re-export for convenience.
pub(crate) use std::net::SocketAddr;

/// A specialized [Result] type for I/O operations.
pub(crate) type IoResult<T> = std::io::Result<T>;

pub mod util;
