//! A Rust implementation of AUTOSTAR's [Scalable service-Oriented MiddlewarE over IP
//! (SOME/IP)](https://some-ip.com/).
//!
//! ## Overview
//!
//! This library enables Rust applications to communicate with other applications using
//! SOME/IP, a message-based Inter-process Communication (IPC) protocol developed for the
//! automotive industry.

#![warn(
    clippy::nursery,
    clippy::pedantic,
    clippy::expect_used,
    clippy::unwrap_used
)]
#![allow(
    clippy::missing_const_for_fn,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::future_not_send
)]

pub mod bytes;
pub mod endpoint;
pub mod socket;
pub mod someip;
pub(crate) mod support;
pub(crate) mod testing;

mod error;
pub use error::{Error, Result};

pub mod runtime;
pub use runtime::{Runtime, Scope};
