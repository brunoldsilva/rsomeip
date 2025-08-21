//! SOME/IP service interface.

use crate::{InterfaceVersion, Message, MessageType, MethodId, ReturnCode};
use std::collections::HashMap;

/// SOME/IP service interface.
///
/// An interface defines a series of methods which determine the types of messages that it can send or
/// receive. These methods can differ between successive versions of the interface.
///
/// This struct is used to manage the state of a service interface by defining its methods, and
/// checking incoming and outgoing messages for correctness.
///
/// # Stub vs Proxy
///
/// An interface can behave either as a stub which serves methods to be "called", or as a proxy which
/// "calls" these methods. This determines what kind of message types the interface considers valid
/// for a given method type.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Interface {
    /// Major version of the service interface.
    pub version: InterfaceVersion,
    /// Whether the interface should behave as a stub or proxy.
    pub flavor: InterfaceType,
    /// Methods registered with the interface.
    pub methods: HashMap<MethodId, MethodType>,
}

impl Interface {
    /// Returns `self` as a [`Stub`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, InterfaceType};
    ///
    /// let interface = Interface::default().into_stub();
    /// assert_eq!(interface.flavor, InterfaceType::Stub);
    /// ```
    ///
    /// [`Stub`]: InterfaceType::Stub
    #[inline]
    #[must_use]
    pub const fn into_stub(mut self) -> Self {
        self.flavor = InterfaceType::Stub;
        self
    }

    /// Returns `self` as a [`Proxy`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, InterfaceType};
    ///
    /// let interface = Interface::default().into_proxy();
    /// assert_eq!(interface.flavor, InterfaceType::Proxy);
    /// ```
    ///
    /// [`Proxy`]: InterfaceType::Proxy
    #[inline]
    #[must_use]
    pub const fn into_proxy(mut self) -> Self {
        self.flavor = InterfaceType::Proxy;
        self
    }

    /// Returns `self` with the given [`InterfaceVersion`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, InterfaceVersion};
    ///
    /// let interface = Interface::default().with_version(InterfaceVersion::new(1));
    /// assert_eq!(interface.version, InterfaceVersion::new(1));
    /// ```
    #[inline]
    #[must_use]
    pub const fn with_version(mut self, version: InterfaceVersion) -> Self {
        self.version = version;
        self
    }

    /// Returns `self` with the given method.
    ///
    /// This is the same as inserting a method with [`MethodType::Method`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let interface = Interface::default().with_method(MethodId::new(1));
    /// assert_eq!(interface.get(MethodId::new(1)), Some(MethodType::Method));
    /// ```
    #[inline]
    #[must_use]
    pub fn with_method(mut self, id: MethodId) -> Self {
        _ = self.insert(id, MethodType::Method);
        self
    }

    /// Returns `self` with the given procedure.
    ///
    /// This is the same as inserting a method with [`MethodType::Procedure`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let interface = Interface::default().with_procedure(MethodId::new(1));
    /// assert_eq!(interface.get(MethodId::new(1)), Some(MethodType::Procedure));
    /// ```
    #[inline]
    #[must_use]
    pub fn with_procedure(mut self, id: MethodId) -> Self {
        _ = self.insert(id, MethodType::Procedure);
        self
    }

    /// Returns `self` with the given event.
    ///
    /// This is the same as inserting a method with [`MethodType::Event`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let interface = Interface::default().with_event(MethodId::new(1));
    /// assert_eq!(interface.get(MethodId::new(1)), Some(MethodType::Event));
    /// ```
    #[inline]
    #[must_use]
    pub fn with_event(mut self, id: MethodId) -> Self {
        _ = self.insert(id, MethodType::Event);
        self
    }

    /// Inserts a method with the given `id` and `flavor`.
    ///
    /// Returns the previous flavor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let mut interface = Interface::default();
    /// assert_eq!(interface.insert(MethodId::new(1), MethodType::Method), None);
    /// assert_eq!(interface.insert(MethodId::new(1), MethodType::Procedure), Some(MethodType::Method));
    /// ```
    #[inline]
    pub fn insert(&mut self, id: MethodId, flavor: MethodType) -> Option<MethodType> {
        self.methods.insert(id, flavor)
    }

    /// Removes the method with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let mut interface = Interface::default().with_method(MethodId::new(1));
    /// assert_eq!(interface.remove(MethodId::new(1)), Some(MethodType::Method));
    /// assert_eq!(interface.remove(MethodId::new(1)), None);
    /// ```
    #[inline]
    pub fn remove(&mut self, id: MethodId) -> Option<MethodType> {
        self.methods.remove(&id)
    }

    /// Returns the flavor of the method with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let interface = Interface::default().with_method(MethodId::new(1));
    /// assert_eq!(interface.get(MethodId::new(1)), Some(MethodType::Method));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, id: MethodId) -> Option<MethodType> {
        self.methods.get(&id).copied()
    }

    /// Returns a mutable reference to the method with the given `id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Interface, MethodId, MethodType};
    ///
    /// let mut interface = Interface::default().with_method(MethodId::new(1));
    /// assert_eq!(interface.get_mut(MethodId::new(1)), Some(&mut MethodType::Method));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, id: MethodId) -> Option<&mut MethodType> {
        self.methods.get_mut(&id)
    }
}

/// Shorthand for making an error message.
macro_rules! drop {
    ($reason:expr) => {
        Err((MessageError::Dropped($reason)))
    };
}

/// Shorthand for making an error message.
macro_rules! invalid {
    ($reason:expr) => {
        Err((MessageError::Invalid($reason)))
    };
}

/// Shorthand for making an error message.
macro_rules! invalid_or_dropped {
    ($message_type:expr, $reason:expr) => {
        Err((MessageError::invalid_or_dropped($message_type, $reason)))
    };
}

impl Interface {
    /// Checks if the `message` is valid in the given `direction`.
    ///
    /// This is used to confirm that the message parameters are correctly configured for this
    /// interface when sending ([`Direction::Outgoing`]) or receiving it ([`Direction::Incoming`]).
    ///
    /// # Errors
    ///
    /// Returns a [`MessageError`] if the message is invalid.
    pub(crate) fn check<T>(
        &self,
        message: &Message<T>,
        direction: Direction,
    ) -> Result<(), MessageError> {
        // Check if interface version is correct.
        if message.interface() != self.version {
            return invalid_or_dropped!(message.message_type(), ReturnCode::WrongInterfaceVersion);
        }

        // Check if the method is registered.
        let Some(method) = self.methods.get(&message.method) else {
            return invalid_or_dropped!(message.message_type(), ReturnCode::UnknownMethod);
        };

        // Check if the message type is correct.
        match direction {
            Direction::Incoming => match (self.flavor, method) {
                (InterfaceType::Stub, MethodType::Procedure) => {
                    if message.message_type() != MessageType::RequestNoReturn {
                        return drop!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Stub, MethodType::Method) => {
                    if message.message_type() != MessageType::Request {
                        return invalid!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Proxy, MethodType::Method) => {
                    if !matches!(
                        message.message_type(),
                        MessageType::Response | MessageType::Error
                    ) {
                        return drop!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Proxy, MethodType::Event) => {
                    if message.message_type() != MessageType::Notification {
                        return drop!(ReturnCode::WrongMessageType);
                    }
                }
                _ => return drop!(ReturnCode::WrongMessageType),
            },
            Direction::Outgoing => match (self.flavor, method) {
                (InterfaceType::Stub, MethodType::Method) => {
                    if !matches!(
                        message.message_type(),
                        MessageType::Response | MessageType::Error
                    ) {
                        return invalid!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Stub, MethodType::Event) => {
                    if message.message_type() != MessageType::Notification {
                        return invalid!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Proxy, MethodType::Procedure) => {
                    if message.message_type() != MessageType::RequestNoReturn {
                        return invalid!(ReturnCode::WrongMessageType);
                    }
                }
                (InterfaceType::Proxy, MethodType::Method) => {
                    if message.message_type() != MessageType::Request {
                        return invalid!(ReturnCode::WrongMessageType);
                    }
                }
                _ => return invalid!(ReturnCode::WrongMessageType),
            },
        }
        Ok(())
    }
}

/// The type of a SOME/IP service interface.
///
/// This defines how it should process SOME/IP messages.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// A service interface being served on a local endpoint.
    #[default]
    Stub,
    /// A service interface being served on a remote endpoint.
    Proxy,
}

/// The type of a SOME/IP method.
///
/// This defines how it should process SOME/IP messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodType {
    /// A callable function on a service interface that does not return a value.
    Procedure,
    /// A callable function on a service interface that returns a value.
    Method,
    /// A non-callable event on the service interface.
    Event,
}

/// The direction an event is taking through the endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The event is coming from a remote endpoint.
    Incoming,
    /// The event is going to a remote endpoint.
    Outgoing,
}

/// A message processing error.
///
/// A SOME/IP endpoint may occasionally try to send or receive messages which are incorrectly
/// configured for the service interface that processes them.
///
/// This enum serves as a way to represent the reason for a message to not be processed, as well as
/// whether the error should be reported to the user or not.
///
/// # Error Handling
///
/// Depending on the error variant, either an error response should be sent back to the source or the
/// whole message should be dropped.
///
/// If the error is [`MessageError::Invalid`], then an error response should be sent with the
/// contained [`ReturnCode`]. Otherwise, the message should be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("invalid message: {0}")]
    Invalid(ReturnCode),
    #[error("message dropped: {0}")]
    Dropped(ReturnCode),
}

impl MessageError {
    /// Creates a new [`MessageError::Invalid`] or [`MessageError::Dropped`] depending on the
    /// [`MessageType`].
    ///
    /// This is used to reduce the verbosity of having to specify whether a message should be
    /// responded to or dropped in case of an error.
    fn invalid_or_dropped(message_type: MessageType, return_code: ReturnCode) -> Self {
        if message_type == MessageType::Request {
            Self::Invalid(return_code)
        } else {
            Self::Dropped(return_code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A SOME/IP message.
    type Message = crate::Message<rsomeip_bytes::Bytes>;

    #[test]
    fn interface_version_is_checked() {
        // Create a new interface with a specific version.
        let interface = Interface::default().with_version(InterfaceVersion::new(1));

        // Check incoming and outgoing messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Requests should not be dropped.
            if value == MessageType::Request {
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Incoming
                    ),
                    invalid!(ReturnCode::WrongInterfaceVersion)
                );
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Outgoing
                    ),
                    invalid!(ReturnCode::WrongInterfaceVersion)
                );
            } else {
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Incoming
                    ),
                    drop!(ReturnCode::WrongInterfaceVersion)
                );
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Outgoing
                    ),
                    drop!(ReturnCode::WrongInterfaceVersion)
                );
            }
        }
    }

    #[test]
    fn method_id_is_checked() {
        // Create a new interface without any methods.
        let interface = Interface::default();

        // Check incoming and outgoing messages.
        for value in [
            MessageType::Request,
            MessageType::Response,
            MessageType::RequestNoReturn,
            MessageType::Error,
            MessageType::Notification,
            MessageType::Unknown(0xff),
        ] {
            // Requests should not be dropped.
            if value == MessageType::Request {
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Incoming
                    ),
                    invalid!(ReturnCode::UnknownMethod)
                );
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Outgoing
                    ),
                    invalid!(ReturnCode::UnknownMethod)
                );
            } else {
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Incoming
                    ),
                    drop!(ReturnCode::UnknownMethod)
                );
                assert_eq!(
                    interface.check(
                        &Message::default().with_message_type(value),
                        Direction::Outgoing
                    ),
                    drop!(ReturnCode::UnknownMethod)
                );
            }
        }
    }

    #[test]
    fn stub_method_valid_message_types() {
        // Create a service interface stub.
        let interface = Interface::default().with_method(MethodId::default());

        // Check incoming messages.
        assert_eq!(
            interface.check(&Message::default(), Direction::Incoming),
            Ok(())
        );

        // Check outgoing messages.
        for value in [MessageType::Response, MessageType::Error] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn stub_method_invalid_message_types() {
        // Create a service interface stub.
        let interface = Interface::default().with_method(MethodId::default());

        // Check incoming messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Request,
            MessageType::Notification,
            MessageType::RequestNoReturn,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }

    #[test]
    fn stub_procedure_valid_message_types() {
        // Create a service interface stub.
        let interface = Interface::default().with_procedure(MethodId::default());

        // Check incoming messages.
        assert_eq!(
            interface.check(
                &Message::default().with_message_type(MessageType::RequestNoReturn),
                Direction::Incoming
            ),
            Ok(())
        );
    }

    #[test]
    fn stub_procedure_invalid_message_types() {
        // Create a service interface stub.
        let interface = Interface::default().with_procedure(MethodId::default());

        // Check incoming messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::Request,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                drop!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }

    #[test]
    fn stub_event_valid_message_types() {
        // Create a service interface stub.
        let interface = Interface::default().with_event(MethodId::default());

        // Check outgoing messages.
        assert_eq!(
            interface.check(
                &Message::default().with_message_type(MessageType::Notification),
                Direction::Outgoing
            ),
            Ok(())
        );
    }

    #[test]
    fn stub_event_only_handles_notifications() {
        // Create a service interface stub.
        let interface = Interface::default().with_event(MethodId::default());

        // Check incoming messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                drop!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Error,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }

    #[test]
    fn proxy_method_valid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_method(MethodId::default())
            .into_proxy();

        // Check incoming messages.
        for value in [MessageType::Response, MessageType::Error] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                Ok(())
            );
        }

        // Check outgoing messages.
        assert_eq!(
            interface.check(
                &Message::default().with_message_type(MessageType::Request),
                Direction::Outgoing
            ),
            Ok(())
        );
    }

    #[test]
    fn proxy_method_invalid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_method(MethodId::default())
            .into_proxy();

        // Check incoming messages.
        for value in [
            MessageType::Request,
            MessageType::Notification,
            MessageType::RequestNoReturn,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                drop!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Response,
            MessageType::Error,
            MessageType::Notification,
            MessageType::RequestNoReturn,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }

    #[test]
    fn proxy_procedure_valid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_procedure(MethodId::default())
            .into_proxy();

        // Check outgoing messages.
        assert_eq!(
            interface.check(
                &Message::default().with_message_type(MessageType::RequestNoReturn),
                Direction::Outgoing
            ),
            Ok(())
        );
    }

    #[test]
    fn proxy_procedure_invalid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_procedure(MethodId::default())
            .into_proxy();

        // Check incoming messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                drop!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Error,
            MessageType::Notification,
            MessageType::Request,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }

    #[test]
    fn proxy_event_valid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_event(MethodId::default())
            .into_proxy();

        // Check incoming messages.
        assert_eq!(
            interface.check(
                &Message::default().with_message_type(MessageType::Notification),
                Direction::Incoming
            ),
            Ok(())
        );
    }

    #[test]
    fn proxy_event_invalid_message_types() {
        // Create a service interface proxy.
        let interface = Interface::default()
            .with_event(MethodId::default())
            .into_proxy();

        // Check incoming messages.
        for value in [
            MessageType::Error,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Incoming
                ),
                drop!(ReturnCode::WrongMessageType)
            );
        }

        // Check outgoing messages.
        for value in [
            MessageType::Error,
            MessageType::Request,
            MessageType::RequestNoReturn,
            MessageType::Notification,
            MessageType::Response,
            MessageType::Unknown(0xff),
        ] {
            assert_eq!(
                interface.check(
                    &Message::default().with_message_type(value),
                    Direction::Outgoing
                ),
                invalid!(ReturnCode::WrongMessageType)
            );
        }
    }
}
