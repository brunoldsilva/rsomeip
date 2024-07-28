use super::{connection, incoming, outgoing};
use crate::{
    net::util::Buffer,
    socket::{self, IoResult, SocketAddr},
};
use std::io;
use tokio::net::{TcpSocket, TcpStream};
use tokio_util::sync::CancellationToken;

pub struct Stream {
    inner: TcpStream,
}

impl Stream {
    /// Creates a new [`Stream`].
    pub fn new(inner: TcpStream) -> Self {
        Self { inner }
    }

    /// Establish a TCP connection to the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    pub async fn connect(src: SocketAddr, dst: SocketAddr) -> IoResult<Self> {
        let socket = match src {
            SocketAddr::V4(_) => TcpSocket::new_v4(),
            SocketAddr::V6(_) => TcpSocket::new_v6(),
        }?;
        socket.bind(src)?;
        let stream = socket.connect(dst).await?;
        Ok(Self::new(stream))
    }

    /// Spawns a task to concurrently process data from the socket.
    ///
    /// Returns a [`socket::Connection`] handle to allow communicating with the socket.
    pub fn spawn(mut self, token: CancellationToken) -> impl socket::Connection {
        let (tx_outgoing, rx_outgoing) = outgoing::channel(1);
        let (tx_incoming, rx_incoming) = incoming::channel(1);
        let connection = connection::Connection::new(tx_outgoing, rx_incoming);

        tokio::spawn(async move {
            self.process(tx_incoming, rx_outgoing, token).await;
        });

        connection
    }

    /// Process data from the connected socket.
    ///
    /// This includes sending coming from the `receiver` and receiving data and sending it to the
    /// `sender`.
    ///
    /// The cancellation token can be used to stop data processing.
    async fn process(
        &mut self,
        sender: incoming::Sender,
        receiver: outgoing::Receiver,
        token: CancellationToken,
    ) {
        tokio::select! {
            () = self.send(receiver) => {},
            () = self.recv(sender) => {}
            () = token.cancelled() => {},
        }
    }

    /// Sends data to the connected socket.
    ///
    /// Data is retrieved from the `receiver` channel.
    async fn send(&self, mut receiver: outgoing::Receiver) {
        'recv: loop {
            if let Some(buffer) = receiver.recv().await {
                'write: loop {
                    if let Err(error) = self.inner.writable().await {
                        eprintln!("Stream is not writable: {error}");
                        break 'recv;
                    }
                    match self.inner.try_write(buffer.as_ref()) {
                        Ok(0) => {
                            println!("Reached end-of-file.");
                            break 'recv;
                        }
                        Ok(length) => {
                            println!("Sent {length} bytes to target.");
                            continue 'recv;
                        }
                        Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                            // Wake-up was a false positive.
                            continue 'write;
                        }
                        Err(error) => {
                            eprintln!("Failed to send data to target: {error}");
                            break 'recv;
                        }
                    }
                }
            }
            println!("No more messages to send.");
            break 'recv;
        }
        println!("Done sending packets.");
    }

    /// Receives data from the connected socket,
    ///
    /// Forwards this data to the `sender` channel.
    async fn recv(&self, sender: incoming::Sender) {
        loop {
            if let Err(error) = self.inner.readable().await {
                eprintln!("Stream is not readable: {error}");
                break;
            }
            let mut buffer: Buffer = Buffer::from([0u8; 1024]);
            let Some(buffer_mut) = buffer.get_mut() else {
                eprintln!("Failed to create a buffer because it's not unique.");
                break;
            };
            match self.inner.try_read(buffer_mut.as_mut()) {
                Ok(0) => {
                    println!("Reached end-of-file.");
                    break;
                }
                Ok(length) => {
                    println!("Received {length} bytes from target.");
                    buffer.shrink_to(length);
                    if sender.send(buffer).await.is_err() {
                        println!("Sender is closed. Message dropped.");
                        break;
                    };
                }
                Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                    // Wake-up was a false positive.
                    continue;
                }
                Err(error) => {
                    eprintln!("Failed to receive data from target: {error}");
                    break;
                }
            }
        }
        println!("Done receiving packets.");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        socket::{Connection, Receiver, Sender},
        Buffer, CancellationToken, IoResult, TcpStream,
    };
    use crate::testing::ipv4;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn connect() {
        let _ = establish_connection()
            .await
            .expect("should establish a connection");
    }

    #[tokio::test]
    async fn send() {
        let (connection, stream) = establish_connection()
            .await
            .expect("should establish a connection");

        let handle = tokio::spawn(async move {
            stream.readable().await.expect("should become readable");
            let mut buffer = [0u8];
            stream
                .try_read(&mut buffer)
                .expect("should read from the stream");
            assert_eq!(buffer, VALUE);
        });

        connection
            .send(Buffer::from(VALUE))
            .await
            .expect("should send the value");

        handle.await.expect("should complete successfully");
    }

    #[tokio::test]
    async fn recv() {
        let (mut connection, stream) = establish_connection()
            .await
            .expect("should establish a connection");

        let handle = tokio::spawn(async move {
            stream.writable().await.expect("should become writable");
            stream
                .try_write(&VALUE)
                .expect("should write to the stream");
        });

        let value = connection.recv().await.expect("should receive the value");
        assert_eq!(&value[..], &VALUE);

        handle.await.expect("should complete successfully");
    }

    /// Establishes a TCP connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    async fn establish_connection() -> IoResult<(impl Connection, TcpStream)> {
        let local_address = ipv4!([127, 0, 0, 1]);
        let peer_address = ipv4!([127, 0, 0, 2]);
        let token = CancellationToken::new();

        let tcp_listener = TcpListener::bind(peer_address).await?;
        let handle = tokio::spawn(async move {
            let (stream, address) = tcp_listener
                .accept()
                .await
                .expect("should accept the connection");
            assert_eq!(address, local_address);
            stream
        });
        let connection = super::Stream::connect(local_address, peer_address)
            .await?
            .spawn(token);
        let tcp_stream = handle.await.expect("should complete successfully");
        Ok((connection, tcp_stream))
    }

    /// The value that is sent through the stream.
    const VALUE: [u8; 1] = [1u8];
}
