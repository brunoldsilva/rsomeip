use crate::{
    endpoint::{Error, Result},
    someip::Message,
};
use std::net::SocketAddr;
use tokio::sync::mpsc;

#[cfg(test)]
mod tests;

/// Provides asynchronous control over a SOME/IP endpoint.
pub struct Controller {
    tx: mpsc::Sender<Command>,
}

impl Controller {
    /// Creates a new [`Controller`].
    pub fn new(sender: mpsc::Sender<Command>) -> Self {
        Self { tx: sender }
    }

    /// Requests that the endpoint connects to the given target.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint failed to connect to the target.
    pub async fn connect(&self, target: SocketAddr) -> Response {
        self.request(Request::Connect(target)).await
    }

    /// Requests that the endpoint disconnects from the given target.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint failed to disconnect from the target.
    pub async fn disconnect(&self, target: SocketAddr) -> Response {
        self.request(Request::Disconnect(target)).await
    }

    /// Requests that the endpoint sends the message to the destination.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be sent, either from a serialization error or
    /// connection issue.
    pub async fn send(&self, msg: Message, dst: SocketAddr) -> Response {
        self.request(Request::Send { msg, dst }).await
    }

    /// Requests that the endpoint relay messages with the given message id.
    ///
    /// # Errors
    ///
    /// Returns an error if the relay could not be set.
    pub async fn relay(&self, id: u32, tx: mpsc::Sender<Result<Message>>) -> Response {
        self.request(Request::Relay { id, tx }).await
    }

    /// Requests the endpoint to shut down.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint failed to shutdown.
    pub async fn shutdown(&self) -> Response {
        self.request(Request::Shutdown).await
    }

    /// Sends a request to the endpoint and awaits the response.
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be sent to the endpoint, or if the response could
    /// not be received.
    async fn request(&self, request: Request) -> Response {
        let (tx, mut rx) = mpsc::channel(1);
        self.tx
            .send(Command { request, tx })
            .await
            .map_err(|_| Error::Failure("could not send request"))?;
        rx.recv()
            .await
            .ok_or(Error::Failure("could not receive response"))?
    }
}

/// Encapsulates a Request with a channel for sending the Response.
pub struct Command {
    request: Request,
    tx: mpsc::Sender<Response>,
}

impl Command {
    /// Consumes the command to return the response channel and the request.
    pub fn into_parts(self) -> (Request, mpsc::Sender<Response>) {
        (self.request, self.tx)
    }
}

/// Operations that an endpoint can be requested to perform.
#[derive(Debug)]
pub enum Request {
    /// Requests that the endpoint connects to the given address.
    Connect(SocketAddr),
    /// Requests that the endpoint disconnects from the given address.
    Disconnect(SocketAddr),
    /// Requests that the endpoint sends the message to the destination.
    Send { msg: Message, dst: SocketAddr },
    /// Requests that the endpoint relay messages with the given message id.
    Relay {
        id: u32,
        tx: mpsc::Sender<Result<Message>>,
    },
    /// Requests the endpoint to shut down.
    Shutdown,
}

/// A response to a [`Request`].
pub type Response = Result<()>;
