use rsomeip_proto::{MethodId, ServiceId};
use std::net::SocketAddr;

/// ID of the service instance.
pub const SAMPLE_SERVICE_ID: ServiceId = ServiceId::new(0x1234);

/// ID of the method.
pub const SAMPLE_METHOD_ID: MethodId = MethodId::new(0x0421);

/// Returns the address of the service provider.
pub fn stub_address() -> SocketAddr {
    ([127, 0, 0, 1], 30509).into()
}

/// Returns the address of the service consumer.
#[allow(dead_code, reason = "false positive")]
pub fn proxy_address() -> SocketAddr {
    ([127, 0, 0, 2], 30509).into()
}
