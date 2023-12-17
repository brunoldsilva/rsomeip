pub mod error;
pub use error::{Error, Result};

pub mod socket;

// Re-export here for convenience.
pub use std::net::SocketAddr;
