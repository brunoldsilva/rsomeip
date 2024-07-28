//! Transmission Control Protocol.
//!
//! TCP is a connection-oriented, reliable, ordered, and error-checked communication protocol of
//! the Internet Protocol suite.
//!
//! This module provides a [`Socket`] implementation of TCP.

mod outgoing;

mod incoming;

mod connection;

mod listener;

mod stream;
