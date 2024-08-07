//! A Rust implementation of AUTOSTAR's [Scalable service-Oriented MiddlewarE over IP
//! (SOME/IP)](https://some-ip.com/).
//!
//! ## Overview
//!
//! This library enables Rust applications to communicate with other applications using
//! SOME/IP, a message-based Inter-process Communication (IPC) protocol developed for the
//! automotive industry.

#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]

pub mod bytes;
pub mod net;
pub mod socket;
pub mod someip;
pub(crate) mod testing;
