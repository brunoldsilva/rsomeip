//! User Datagram Protocol.
//!
//! This module provides a [`Socket`] implementation of the User Datagram Protocol, which is a
//! connectionless communication protocol of the Internet Protocol suite.

mod sender;
use sender::Sender;

mod receiver;
use receiver::Receiver;

mod connection;
use connection::Connection;

mod listener;
use listener::Listener;

mod handle;
use handle::{Handle, Message};

mod socket;
pub use socket::Socket;
