#![cfg(test)]

#[allow(clippy::cast_possible_truncation)]
pub const fn hash(module_path: &'static str, file: &'static str, line: u32, column: u32) -> u16 {
    let mut hash = 0xcbf2;
    let prime = 0x7655;

    let mut bytes = module_path.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u16;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    bytes = file.as_bytes();
    i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u16;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    hash ^= line as u16; // ALLOW_LINT: Should never have a line number greater than u16::MAX.
    hash = hash.wrapping_mul(prime);
    hash ^= column as u16; // ALLOW_LINT: Should never have a column number greater than u16::MAX.
    hash = hash.wrapping_mul(prime);
    hash
}

macro_rules! port {
    () => {{
        const PORT: u16 =
            crate::testing::hash(module_path!(), file!(), line!(), column!()) % 50_000 + 10_000;
        PORT
    }};
}
pub(crate) use port;

macro_rules! ipv4 {
    ([$($addr:tt),*], $port:tt) => {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new($($addr),*)), $port)
    };
    ([$($addr:tt),*]) => {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new($($addr),*)), port!())
    }
}
pub(crate) use ipv4;
