pub mod error;
pub use error::{Error, Result};

pub mod socket;

pub mod util;

// Re-export here for convenience.
pub use std::net::SocketAddr;
