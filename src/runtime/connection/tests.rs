#![cfg(test)]

use super::*;
use crate::testing::{ipv4, port};
use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn connect_udp() {
    let src = ipv4!([127, 0, 0, 1]);
    let dst = Some(ipv4!([127, 0, 0, 2]));
    assert!(
        Factory::connect(Protocol::Udp, src, dst).await.is_ok(),
        "Should create an UDP connection."
    );
}
