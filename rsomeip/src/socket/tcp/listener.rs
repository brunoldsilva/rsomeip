use crate::{
    net::IoResult,
    socket::{self, SocketAddr},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

/// Creates a channel of connected [`Sender`] and [`Listener`] pairs.
pub fn channel<T>(buffer: usize) -> (Sender<T>, Listener<T>)
where
    T: socket::Connection + From<TcpStream> + 'static,
{
    let (tx, rx) = mpsc::channel(buffer);
    (Sender::new(tx), Listener::new(rx))
}

/// Listens for incoming TCP connections.
///
/// A [`Listener`] binds to a given [`SocketAddr`], and listens for TCP connections made to that
/// address. These connections can be accepted using the [`socket::Listener::accept`] method.
pub struct Listener<T>
where
    T: socket::Connection + From<TcpStream>,
{
    inner: mpsc::Receiver<(T, SocketAddr)>,
}

impl<T> Listener<T>
where
    T: socket::Connection + From<TcpStream> + Send + 'static,
{
    /// Creates a new [`Listener<T>`].
    fn new(inner: mpsc::Receiver<(T, SocketAddr)>) -> Self {
        Self { inner }
    }

    /// Binds a [`Listener`] to the given address capable of accepting TCP connections.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the underlying TCP listener cannot be bound to the address.
    pub async fn bind(address: SocketAddr) -> IoResult<Self> {
        TcpListener::bind(address).await.map(Self::spawn)
    }

    pub fn spawn(listener: TcpListener) -> Self {
        const BUFFER_SIZE: usize = 8;
        let (sender, receiver) = mpsc::channel(BUFFER_SIZE);
        let task = ListenerTask::new(listener, sender);
        tokio::spawn(task.run());
        Self::new(receiver)
    }
}

impl<T> socket::Listener for Listener<T>
where
    T: socket::Connection + From<TcpStream>,
{
    async fn accept(&mut self) -> Option<(impl socket::Connection, SocketAddr)> {
        self.inner.recv().await
    }
}

/// Sends new connections to the [`Listener`].
pub struct Sender<T>
where
    T: socket::Connection + From<TcpStream>,
{
    inner: mpsc::Sender<(T, SocketAddr)>,
}

impl<T> Sender<T>
where
    T: socket::Connection + From<TcpStream>,
{
    /// Creates a new [`Sender<T>`].
    fn new(inner: mpsc::Sender<(T, SocketAddr)>) -> Self {
        Self { inner }
    }

    /// Sends the connection to the listener.
    pub async fn send(&self, connection: T, address: SocketAddr) -> Option<()> {
        self.inner.send((connection, address)).await.ok()
    }
}

struct ListenerTask<T>
where
    T: socket::Connection,
{
    listener: TcpListener,
    connections: mpsc::Sender<(T, SocketAddr)>,
}

impl<T> ListenerTask<T>
where
    T: socket::Connection + From<TcpStream>,
{
    fn new(listener: TcpListener, connections: mpsc::Sender<(T, SocketAddr)>) -> Self {
        Self {
            listener,
            connections,
        }
    }

    async fn run(mut self) {
        loop {
            match self.listener.accept().await {
                Ok((stream, address)) => {
                    if self
                        .connections
                        .send((T::from(stream), address))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    eprintln!("error listening for connections: {error}");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mpsc,
        socket::{Connection, Listener, Receiver, SendResult, Sender},
        SocketAddr,
    };
    use crate::{net::util::Buffer, socket, testing::ipv4};
    use tokio::net::{TcpSocket, TcpStream};

    #[tokio::test]
    async fn bind() {
        let address = ipv4!([127, 0, 0, 1]);
        let _ = super::Listener::<FakeConnection>::bind(address)
            .await
            .expect("should bind to the address");
    }

    #[tokio::test]
    async fn channel() {
        let (sender, mut listener) = super::channel::<FakeConnection>(1);
        let connection = FakeConnection;
        let address = ipv4!([127, 0, 0, 1]);

        sender
            .send(connection, address)
            .await
            .expect("should send the connection");

        let (_, addr) = listener
            .accept()
            .await
            .expect("should receive the connection");
        assert_eq!(address, addr);
    }

    #[tokio::test]
    async fn accept() {
        // Bind a Listener to a local address.
        let local_address = ipv4!([127, 0, 0, 1]);
        let mut listener = super::Listener::<FakeConnection>::bind(local_address)
            .await
            .expect("should bind to the local address");

        // Connect remotely to the Listener.
        let peer_address = ipv4!([127, 0, 0, 2]);
        let peer = connect(peer_address, local_address);

        // Accept the connection with the Listener.
        let (_, address) = listener.accept().await.expect("should accept a connection");
        assert_eq!(address, peer_address);

        // Confirm that the peer connected.
        peer.await
            .expect("should complete successfully")
            .expect("should establish connection");
    }

    #[tokio::test]
    async fn accept_none() {
        let (_, mut listener) = listen_channel::<FakeConnection>(1);
        let result = listener.accept().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn send() {
        let (sender, mut rx_connection) = sender_channel::<FakeConnection>(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = FakeConnection;

        sender
            .send(connection, address)
            .await
            .expect("should send the connection");

        let (FakeConnection, addr) = rx_connection
            .recv()
            .await
            .expect("should receive the connection");
        assert_eq!(address, addr);
    }

    #[tokio::test]
    async fn send_none() {
        let (sender, _) = sender_channel::<FakeConnection>(1);
        let address = ipv4!([127, 0, 0, 1]);
        let connection = FakeConnection;

        let result = sender.send(connection, address).await;

        assert!(result.is_none());
    }

    // Establishes a TCP connection between `src` and `dst` asynchronously.
    fn connect(
        src: SocketAddr,
        dst: SocketAddr,
    ) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
        tokio::spawn(async move {
            let socket = TcpSocket::new_v4()?;
            socket.bind(src)?;
            socket.connect(dst).await.map(|_| ())
        })
    }

    /// Creates a connected pair of [`mpsc::Sender`] and [`super::Listener`].
    fn listen_channel<T>(buffer: usize) -> (mpsc::Sender<(T, SocketAddr)>, super::Listener<T>)
    where
        T: socket::Connection + From<TcpStream> + 'static,
    {
        let (tx, rx) = mpsc::channel(buffer);
        (tx, super::Listener::new(rx))
    }

    fn sender_channel<T>(buffer: usize) -> (super::Sender<T>, mpsc::Receiver<(T, SocketAddr)>)
    where
        T: Connection + From<TcpStream>,
    {
        let (tx, rx) = mpsc::channel(buffer);
        (super::Sender::new(tx), rx)
    }

    /// A fake connection that can be sent through a listener.
    struct FakeConnection;

    impl Connection for FakeConnection {
        fn split(self) -> (impl socket::Sender, impl socket::Receiver) {
            (Self, Self)
        }
    }

    impl Sender for FakeConnection {
        async fn send(&self, _: Buffer) -> SendResult<Buffer> {
            unimplemented!()
        }
    }

    impl Receiver for FakeConnection {
        async fn recv(&mut self) -> Option<Buffer> {
            unimplemented!()
        }
    }

    impl From<TcpStream> for FakeConnection {
        fn from(_: TcpStream) -> Self {
            Self
        }
    }
}
