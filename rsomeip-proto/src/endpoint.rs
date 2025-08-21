use crate::{
    Interface, Message, MessageType, ProtocolVersion, ReturnCode, ServiceId,
    interface::{Direction, MessageError},
};
use rsomeip_bytes::{Buf, BufMut, Deserialize, DeserializeError, Serialize, SerializeError};
use std::collections::HashMap;

/// Collection of SOME/IP service interfaces.
///
/// This is used to process SOME/IP messages to and from serialized data.
#[derive(Debug, Default, Clone)]
pub struct Endpoint {
    interfaces: HashMap<ServiceId, Interface>,
}

impl Endpoint {
    /// Creates a new [`Endpoint`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `self` with the given service interface.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface};
    ///
    /// let endpoint = Endpoint::new().with_interface(ServiceId::new(0), Interface::default());
    /// assert_eq!(endpoint.get(ServiceId::new(0)), Some(&Interface::default()));
    /// ```
    #[must_use]
    pub fn with_interface(mut self, id: ServiceId, interface: Interface) -> Self {
        _ = self.insert(id, interface);
        self
    }

    /// Inserts the `interface` with the given `id`.
    ///
    /// Returns the previous interface, if it exists.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface};
    ///
    /// let mut endpoint = Endpoint::new();
    /// assert_eq!(endpoint.insert(ServiceId::new(0), Interface::default()), None);
    /// assert_eq!(endpoint.insert(ServiceId::new(0), Interface::default()), Some(Interface::default()));
    /// ```
    pub fn insert(&mut self, id: ServiceId, interface: Interface) -> Option<Interface> {
        self.interfaces.insert(id, interface)
    }

    /// Removes the interface with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface};
    ///
    /// let mut endpoint = Endpoint::new().with_interface(ServiceId::new(0), Interface::default());
    /// assert_eq!(endpoint.remove(ServiceId::new(0)), Some(Interface::default()));
    /// assert_eq!(endpoint.remove(ServiceId::new(0)), None);
    /// ```
    pub fn remove(&mut self, id: ServiceId) -> Option<Interface> {
        self.interfaces.remove(&id)
    }

    /// Returns a reference to the interface with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface};
    ///
    /// let endpoint = Endpoint::new().with_interface(ServiceId::new(0), Interface::default());
    /// assert_eq!(endpoint.get(ServiceId::new(0)), Some(&Interface::default()));
    /// ```
    #[must_use]
    pub fn get(&self, id: ServiceId) -> Option<&Interface> {
        self.interfaces.get(&id)
    }

    /// Returns a mutable reference to the interface with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface};
    ///
    /// let mut endpoint = Endpoint::new().with_interface(ServiceId::new(0), Interface::default());
    /// assert_eq!(endpoint.get_mut(ServiceId::new(0)), Some(&mut Interface::default()));
    /// ```
    #[must_use]
    pub fn get_mut(&mut self, id: ServiceId) -> Option<&mut Interface> {
        self.interfaces.get_mut(&id)
    }

    /// Polls the `buffer` for a [`Message`].
    ///
    /// Checks if the message is valid for any of the interfaces of this endpoint.
    ///
    /// # Errors
    ///
    /// Returns the following errors:
    ///
    /// - [`EndpointError::InvalidData`]: If the [`Message`] failed to be deserialized from the
    ///   `buffer`.
    /// - [`EndpointError::InvalidMessage`]: If the [`Message`] was invalid and an error response
    ///   should be sent back to the source.
    /// - [`EndpointError::MessageDropped`]: If the [`Message`] was invalid and should be dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface, MethodId, Message, MessageType};
    /// use rsomeip_bytes::Bytes;
    ///
    /// // Create an endpoint with a single interface.
    /// let endpoint = Endpoint::new().with_interface(
    ///     ServiceId::default(),
    ///     Interface::default().with_method(MethodId::default()),
    /// );
    ///
    /// // Create an incoming message.
    /// let message: Message<Bytes> = Message::default();
    ///
    /// // Poll the buffer for a message and check it.
    /// assert_eq!(
    ///     endpoint.poll(&mut message.to_bytes().expect("should serialize")),
    ///     Ok(message)
    /// );
    /// ```
    pub fn poll<T>(&self, buffer: &mut impl Buf) -> Result<Message<T>, EndpointError<T>>
    where
        T: Deserialize<Output = T>,
    {
        // Try to extract a message from the buffer.
        let message: Message<T> =
            Message::deserialize(buffer).map_err(EndpointError::InvalidData)?;

        // Check if the protocol version is correct.
        if message.protocol() != ProtocolVersion::new(1) {
            return Err(EndpointError::invalid_or_dropped(
                message,
                ReturnCode::WrongProtocolVersion,
            ));
        }

        // Find the interface to which the message is addressed.
        let Some(interface) = self.interfaces.get(&message.service) else {
            return Err(EndpointError::invalid_or_dropped(
                message,
                ReturnCode::UnknownService,
            ));
        };

        // Check if the message is valid for the given interface.
        match interface.check(&message, Direction::Incoming) {
            Ok(()) => {
                // Message is valid.
                Ok(message)
            }
            Err(MessageError::Invalid(return_code)) => {
                // Message is invalid and an error response should be sent back.
                Err(EndpointError::InvalidMessage {
                    message,
                    return_code,
                })
            }
            Err(MessageError::Dropped(return_code)) => {
                // Message is invalid and should be dropped.
                Err(EndpointError::MessageDropped {
                    message,
                    return_code,
                })
            }
        }
    }

    /// Processes the `message` into the `buffer`.
    ///
    /// Checks if the message is valid for any of the interfaces of this endpoint.
    ///
    /// Returns the size of the serialized message.
    ///
    /// # Errors
    ///
    /// Returns the following errors:
    ///
    /// - [`EndpointError::InvalidInput`]: If the [`Message`] failed to be serialized into the
    ///   `buffer`.
    /// - [`EndpointError::InvalidMessage`]: If the [`Message`] was invalid and an error response
    ///   should be sent back to the source.
    /// - [`EndpointError::MessageDropped`]: If the [`Message`] was invalid and should be dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Endpoint, ServiceId, Interface, MethodId, Message, MessageType};
    /// use rsomeip_bytes::{Bytes, BytesMut, Serialize};
    ///
    /// // Create an endpoint with a single interface.
    /// let endpoint = Endpoint::new().with_interface(
    ///     ServiceId::default(),
    ///     Interface::default().with_event(MethodId::default()),
    /// );
    ///
    /// // Create an outgoing message.
    /// let message: Message<Bytes> = Message::default().with_message_type(MessageType::Notification);
    ///
    /// // Process the message and write it into a buffer.
    /// let mut buffer = BytesMut::with_capacity(message.size_hint());
    /// assert_eq!(
    ///     endpoint.process(message.clone(), &mut buffer),
    ///     Ok(message.size_hint())
    /// );
    /// assert_eq!(buffer.freeze(), message.to_bytes().expect("should serialize"));
    /// ```
    pub fn process<T>(
        &self,
        message: Message<T>,
        buffer: &mut impl BufMut,
    ) -> Result<usize, EndpointError<T>>
    where
        T: Serialize,
    {
        // Check if the protocol version is correct.
        if message.protocol() != ProtocolVersion::new(1) {
            return Err(EndpointError::InvalidMessage {
                message,
                return_code: ReturnCode::WrongProtocolVersion,
            });
        }

        // Find the interface to which the message is addressed.
        let Some(interface) = self.interfaces.get(&message.service) else {
            return Err(EndpointError::InvalidMessage {
                message,
                return_code: ReturnCode::UnknownService,
            });
        };

        // Check if the message is valid for the given interface.
        match interface.check(&message, Direction::Outgoing) {
            Ok(()) => {
                // Message is valid.
            }
            Err(MessageError::Invalid(return_code)) => {
                // Message is invalid and an error response should be sent back.
                return Err(EndpointError::InvalidMessage {
                    message,
                    return_code,
                });
            }
            Err(MessageError::Dropped(return_code)) => {
                // Message is invalid and should be dropped.
                return Err(EndpointError::MessageDropped {
                    message,
                    return_code,
                });
            }
        }

        // Serialize the message into the buffer.
        message
            .serialize(buffer)
            .map_err(EndpointError::InvalidInput)
    }
}

/// Represents an error during [`Endpoint`] operations.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum EndpointError<T> {
    #[error("could not deserialize a message: {0}")]
    InvalidData(DeserializeError),
    #[error("could not serialize the message: {0}")]
    InvalidInput(SerializeError),
    #[error("invalid message: {message} reason={return_code}")]
    InvalidMessage {
        /// The message itself.
        message: Message<T>,
        /// The reason why the message is invalid.
        return_code: ReturnCode,
    },
    #[error("message dropped: {message} reason={return_code}")]
    MessageDropped {
        /// The message itself.
        message: Message<T>,
        /// The reason why the message is invalid.
        return_code: ReturnCode,
    },
    #[error("{0}")]
    Custom(String),
}

impl<T> EndpointError<T> {
    /// Creates a new [`EndpointError::InvalidMessage`] or [`EndpointError::MessageDropped`] depending
    /// on the [`MessageType`].
    ///
    /// This is used to reduce the verbosity of having to specify whether a message should be
    /// responded to or dropped in case of an error.
    fn invalid_or_dropped(message: Message<T>, return_code: ReturnCode) -> Self {
        if message.message_type() == MessageType::Request {
            Self::InvalidMessage {
                message,
                return_code,
            }
        } else {
            Self::MessageDropped {
                message,
                return_code,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsomeip_bytes::{Bytes, BytesMut};

    #[test]
    fn protocol_version_is_checked() {
        // Create a new endpoint.
        let endpoint = Endpoint::new();

        // Create a message with an invalid protocol version.
        let message: Message<Bytes> = Message::default().with_protocol(ProtocolVersion::new(0xff));

        // Check incoming messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Try to poll the message.
            let result = endpoint.poll(
                &mut message
                    .clone()
                    .with_message_type(value)
                    .to_bytes()
                    .expect("should serialize"),
            );

            // Requests should not be dropped.
            if value == MessageType::Request {
                assert_eq!(
                    result,
                    Err(EndpointError::InvalidMessage {
                        message: message.clone().with_message_type(value),
                        return_code: ReturnCode::WrongProtocolVersion
                    })
                );
            } else {
                assert_eq!(
                    result,
                    Err(EndpointError::MessageDropped {
                        message: message.clone().with_message_type(value),
                        return_code: ReturnCode::WrongProtocolVersion
                    })
                );
            }
        }

        // Check outgoing messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Try to process the message.
            assert_eq!(
                endpoint.process(
                    message.clone().with_message_type(value),
                    &mut BytesMut::new(),
                ),
                Err(EndpointError::InvalidMessage {
                    message: message.clone().with_message_type(value),
                    return_code: ReturnCode::WrongProtocolVersion
                })
            );
        }
    }

    #[test]
    fn service_id_is_checked() {
        // Create a new endpoint.
        let endpoint = Endpoint::new();

        // Create a message with an unknown service id.
        let message: Message<Bytes> = Message::default();

        // Check incoming messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Try to poll the message.
            let result = endpoint.poll(
                &mut message
                    .clone()
                    .with_message_type(value)
                    .to_bytes()
                    .expect("should serialize"),
            );

            // Requests should not be dropped.
            if value == MessageType::Request {
                assert_eq!(
                    result,
                    Err(EndpointError::InvalidMessage {
                        message: message.clone().with_message_type(value),
                        return_code: ReturnCode::UnknownService
                    })
                );
            } else {
                assert_eq!(
                    result,
                    Err(EndpointError::MessageDropped {
                        message: message.clone().with_message_type(value),
                        return_code: ReturnCode::UnknownService
                    })
                );
            }
        }

        // Check outgoing messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Try to process the message.
            assert_eq!(
                endpoint.process(
                    message.clone().with_message_type(value),
                    &mut BytesMut::new(),
                ),
                Err(EndpointError::InvalidMessage {
                    message: message.clone().with_message_type(value),
                    return_code: ReturnCode::UnknownService
                })
            );
        }
    }

    #[test]
    fn interface_errors_are_propagated() {
        // Create a new endpoint with an empty interface.
        let endpoint = Endpoint::new().with_interface(ServiceId::default(), Interface::default());

        // Create a message with an unknown method id.
        let message: Message<Bytes> = Message::default();

        // Check outgoing messages.
        assert_eq!(
            endpoint.process(message.clone(), &mut BytesMut::with_capacity(16)),
            Err(EndpointError::InvalidMessage {
                message: message.clone(),
                return_code: ReturnCode::UnknownMethod
            })
        );
        assert_eq!(
            endpoint.process(
                message
                    .clone()
                    .with_message_type(MessageType::RequestNoReturn),
                &mut BytesMut::with_capacity(16)
            ),
            Err(EndpointError::MessageDropped {
                message: message
                    .clone()
                    .with_message_type(MessageType::RequestNoReturn),
                return_code: ReturnCode::UnknownMethod
            })
        );

        // Check incoming messages.
        assert_eq!(
            endpoint.poll(&mut message.clone().to_bytes().expect("should serialize")),
            Err(EndpointError::InvalidMessage {
                message: message.clone(),
                return_code: ReturnCode::UnknownMethod
            })
        );
        assert_eq!(
            endpoint.poll(
                &mut message
                    .clone()
                    .with_message_type(MessageType::RequestNoReturn)
                    .to_bytes()
                    .expect("should serialize")
            ),
            Err(EndpointError::MessageDropped {
                message: message.with_message_type(MessageType::RequestNoReturn),
                return_code: ReturnCode::UnknownMethod
            })
        );
    }
}
