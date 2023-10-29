#![cfg(test)]

macro_rules! port {
    () => {{
        use rand::Rng;
        rand::thread_rng().gen_range(10_000..=60_000)
    }};
}
pub(crate) use port;

macro_rules! ipv4 {
    ([$($addr:tt),*], $port:tt) => {
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new($($addr),*)), $port)
    };
    ([$($addr:tt),*]) => {
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new($($addr),*)),
        crate::testing::port!())
    }
}
pub(crate) use ipv4;
