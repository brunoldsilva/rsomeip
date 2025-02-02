#![cfg(test)]

/// Generates a random port in the range `49_152..65_535`.
macro_rules! port {
    () => {
        crate::testing::gen_range(49_152..65_535)
    };
}
pub(crate) use port;

/// Creates a new Ipv4 address.
///
/// This macro has three versions:
///
/// - `ipv4!([127, 0, 0, 1], 12345)` will create the address `127.0.0.1:12345`.
/// - `ipv4!([127, 0, 0, 1])` will create the address `127.0.0.1` with a random port in the range
///   `49_152..65_535`.
/// - `ipv4!()` will create a random address in the range `127.0.0.1-127.0.0.255` with a random
///    port in the range `49_152..65_535`.
macro_rules! ipv4 {
    ([$($addr:tt),*], $port:tt) => {
        ::std::net::SocketAddr::new(::std::net::IpAddr::V4(::std::net::Ipv4Addr::new($($addr),*)), $port)
    };
    ([$($addr:tt),*]) => {
        ::std::net::SocketAddr::new(::std::net::IpAddr::V4(::std::net::Ipv4Addr::new($($addr),*)),
        crate::testing::port!())
    };
    () => {
        ::std::net::SocketAddr::new(
            ::std::net::IpAddr::V4(
                ::std::net::Ipv4Addr::new(127, 0, 0, crate::testing::gen_range(1..255))
            ),
            crate::testing::port!()
        )
    }
}
pub(crate) use ipv4;

/// Generates a random value in the given range.
pub fn gen_range<T, R>(range: R) -> T
where
    T: rand::distributions::uniform::SampleUniform,
    R: rand::distributions::uniform::SampleRange<T>,
{
    use rand::Rng;
    rand::thread_rng().gen_range(range)
}
