//! SOME/IP Transport Protocol
//!
//! This module provides an implementation of the Transport Protocol - an extension of the SOME/IP
//! protocol design to segment large messages.

use crate::Message;
use rsomeip_bytes::{
    Buf, BufMut, Bytes, BytesMut, Deserialize, DeserializeError, Serialize, serialize_into,
    size_hint,
};

/// SOME/IP-TP header.
///
/// Used to identify a segment of a larger SOME/IP message.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TpHeader {
    /// Offset from the start of the original message in multiples of 16-bytes.
    pub offset: usize,
    /// Whether more segments are expected after this one.
    pub more_segments: bool,
    /// The contents of this segment.
    pub body: Bytes,
}

impl TpHeader {
    /// Size of the serialized [`TpHeader`] in bytes.
    pub const HEADER_SIZE: usize = 4;

    /// The offset is measured in units of 16 bytes.
    pub const OFFSET_UNIT: usize = 16;

    pub const MAX_OFFSET: usize = 0xffff_fff0;

    /// Creates a new [`TpHeader`] with the given `body`.
    pub fn new(body: Bytes) -> Self {
        Self {
            body,
            ..Default::default()
        }
    }

    /// Splits this header into two at the given limit.
    ///
    /// Returns a header with the body from `[0..limit]` while `self` remains with `[limit..len]`.
    ///
    /// Returns [`None`] if the body is already below the limit.
    ///
    /// The `limit` must be a multiple of 16. If not, it will be rounded to the nearest multiple less
    /// than the `limit`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::tp::TpHeader;
    /// use rsomeip_bytes::{Bytes, BytesMut, BufMut};
    ///
    /// let buffer = {
    ///     let mut buffer = BytesMut::with_capacity(32);
    ///     buffer.put_bytes(0, 16);
    ///     buffer.put_bytes(1, 16);
    ///     buffer.freeze()
    /// };
    /// let mut header = TpHeader::new(buffer);
    ///
    /// assert_eq!(
    ///     header.split(16),
    ///     Some(TpHeader {
    ///         offset: 0,
    ///         more_segments: true,
    ///         body: Bytes::copy_from_slice([0u8; 16].as_slice())
    ///     })
    /// );
    /// assert_eq!(
    ///     header,
    ///     TpHeader {
    ///         offset: 1,
    ///         more_segments: false,
    ///         body: Bytes::copy_from_slice([1u8; 16].as_slice())
    ///     }
    /// );
    /// ```
    pub fn split(&mut self, limit: usize) -> Option<Self> {
        // Limit must be greater than the offset unit size.
        if limit < Self::OFFSET_UNIT {
            return None;
        }

        // Round the limit to the nearest multiple of 16.
        let limit = limit / Self::OFFSET_UNIT * Self::OFFSET_UNIT;

        // Check if we are already below the limit.
        if self.body.len() <= limit {
            self.more_segments = false;
            return None;
        }

        // Split the header at the limit.
        let header = Self {
            offset: self.offset,
            more_segments: true,
            body: self.body.split_to(limit),
        };

        // Update our offset.
        self.offset += limit / Self::OFFSET_UNIT;

        // Return the new segment.
        Some(header)
    }

    /// Joins the bodies of `self` and `other`.
    ///
    /// # Errors
    ///
    /// Returns the following errors:
    ///
    /// - [`TpError::UnexpectedSegment`] if `self` is not expecting any more segments.
    /// - [`TpError::WrongOffset`] if the `offset` of `other` does not match the expected offset of
    ///   `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::tp::TpHeader;
    /// use rsomeip_bytes::{Bytes, BytesMut};
    ///
    /// let mut header = TpHeader {
    ///     offset: 0,
    ///     more_segments: true,
    ///     body: Bytes::copy_from_slice([0u8; 16].as_slice()),
    /// };
    /// assert_eq!(
    ///     header.join(TpHeader {
    ///         offset: 1,
    ///         more_segments: false,
    ///         body: Bytes::copy_from_slice([0u8; 16].as_slice())
    ///     }),
    ///     Ok(())
    /// );
    /// assert_eq!(
    ///     header,
    ///     TpHeader {
    ///         offset: 0,
    ///         more_segments: false,
    ///         body: Bytes::copy_from_slice([0u8; 32].as_slice())
    ///     }
    /// );
    /// ```
    pub fn join(&mut self, other: Self) -> Result<(), TpError> {
        // Check if we are expecting other segments.
        if !self.more_segments {
            return Err(TpError::UnexpectedSegment(other));
        }

        // Get our current offset.
        let current_offset = self.body.len() / Self::OFFSET_UNIT;

        // Check if the other header's offset is correct.
        if current_offset != other.offset {
            return Err(TpError::WrongOffset {
                expected: current_offset,
                other,
            });
        }

        // Join both headers.
        self.body = {
            // Try to reclaim the BytesMut to avoid having to make an extra copy.
            let mut body = BytesMut::from(std::mem::take(&mut self.body));
            body.put(other.body);
            body.freeze()
        };
        self.more_segments = other.more_segments;

        Ok(())
    }
}

impl Serialize for TpHeader {
    fn serialize(&self, buffer: &mut impl BufMut) -> Result<usize, rsomeip_bytes::SerializeError> {
        if self.offset > Self::MAX_OFFSET {
            return Err(rsomeip_bytes::SerializeError::Other(format!(
                "offset exceeds max value: max={} actual={}",
                Self::MAX_OFFSET,
                self.offset
            )));
        }
        let offset = u32::try_from(self.offset << 4).unwrap_or(0xffff_fff0);
        let header = offset | u32::from(self.more_segments);
        serialize_into!(buffer, header, &self.body)
    }

    fn size_hint(&self) -> usize {
        // Both `offset` and `more_segments` occupy the same `u32` field.
        size_hint!(0u32, &self.body)
    }
}

impl Deserialize for TpHeader {
    type Output = Self;

    fn deserialize(buffer: &mut impl Buf) -> Result<Self::Output, DeserializeError> {
        const OFFSET_MASK: u32 = 0xffff_fff0;
        const MORE_SEGMENTS_MASK: u32 = 0x0000_0001;

        let header = u32::deserialize(buffer)?;
        Ok(Self {
            offset: usize::try_from((header & OFFSET_MASK) >> 4)
                .map_err(|error| DeserializeError::Other(format!("{error}")))?,
            more_segments: header & MORE_SEGMENTS_MASK == 1,
            body: buffer.copy_to_bytes(buffer.remaining()),
        })
    }
}

/// Represents an error during SOME/IP-TP operations.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TpError {
    /// Tried to deserialize an invalid TP segment.
    #[error("deserialization error: {0}")]
    InvalidData(DeserializeError),
    /// Tried to join a segment while the [`TpHeader::more_segments`] was set to `false`.
    #[error("not expecting more segments")]
    UnexpectedSegment(TpHeader),
    /// Tried to join a header with the wrong offset.
    #[error("tried to join header with wrong offset: expected={expected} actual={}", other.offset)]
    WrongOffset { expected: usize, other: TpHeader },
}

impl<T> Message<T>
where
    T: Buf,
{
    /// Converts a [`Message<T>`] into a [`Message<TpHeader>`] by deserializing `T` into a
    /// [`TpHeader`].
    ///
    /// # Errors
    ///
    /// Returns a [`DeserializeError`] if the segment could not be deserialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, tp::TpHeader};
    /// use rsomeip_bytes::Bytes;
    ///
    /// let buffer = Bytes::copy_from_slice(&[1u8; 5]);
    /// let message = Message::new(Header::default(), buffer);
    /// assert!(message.into_segment().is_ok());
    /// ```
    pub fn into_segment(mut self) -> Result<Message<TpHeader>, DeserializeError> {
        let header = TpHeader::deserialize(&mut self.body)?;
        Ok(self.map_body(|_| header))
    }

    /// Converts a [`Message<T>`] into a [`Message<TpHeader>`] by wrapping `T` in a [`TpHeader`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, tp::TpHeader};
    /// use rsomeip_bytes::Bytes;
    ///
    /// let buffer = Bytes::copy_from_slice(&[1u8]);
    /// let message = Message::new(Header::default(), buffer);
    /// assert_eq!(message.wrap_segment().body.body[0], 1u8);
    /// ```
    pub fn wrap_segment(self) -> Message<TpHeader> {
        self.map_body(|mut body| TpHeader::new(body.copy_to_bytes(body.remaining())))
    }
}

impl Message<TpHeader> {
    /// Unwraps a [`Message<TpHeader>`] into a [`Message<Bytes>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rsomeip_proto::{Message, Header, tp::TpHeader};
    /// use rsomeip_bytes::Bytes;
    ///
    /// let header = TpHeader {
    ///     offset: 0,
    ///     more_segments: true,
    ///     body: Bytes::copy_from_slice(&[1u8])
    /// };
    /// let message = Message::new(Header::default(), header);
    /// assert_eq!(message.unwrap_segment().body, Bytes::copy_from_slice(&[1u8]));
    /// ```
    pub fn unwrap_segment(self) -> Message<Bytes> {
        self.map_body(|body| body.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tp_header_is_split() {
        let buffer = {
            let mut buffer = BytesMut::with_capacity(48);
            buffer.put_bytes(0, 16);
            buffer.put_bytes(1, 16);
            buffer.put_bytes(2, 16);
            buffer.freeze()
        };
        let mut header = TpHeader::new(buffer);
        assert_eq!(
            header.split(16),
            Some(TpHeader {
                offset: 0,
                more_segments: true,
                body: Bytes::copy_from_slice([0u8; 16].as_slice())
            })
        );
        assert_eq!(
            header.split(16),
            Some(TpHeader {
                offset: 1,
                more_segments: true,
                body: Bytes::copy_from_slice([1u8; 16].as_slice())
            })
        );
        assert_eq!(header.split(16), None);
        assert_eq!(
            header,
            TpHeader {
                offset: 2,
                more_segments: false,
                body: Bytes::copy_from_slice([2u8; 16].as_slice())
            }
        );
    }

    #[test]
    fn limit_is_multiple_of_16() {
        let mut header = TpHeader::new(Bytes::copy_from_slice([0u8; 32].as_slice()));
        assert_eq!(
            header.split(15), // Must be greater than 15.
            None
        );
        assert_eq!(
            header.split(31), // Should round down.
            Some(TpHeader {
                offset: 0,
                more_segments: true,
                body: Bytes::copy_from_slice([0u8; 16].as_slice())
            })
        );
        assert_eq!(
            header,
            TpHeader {
                offset: 1,
                more_segments: false,
                body: Bytes::copy_from_slice([0u8; 16].as_slice())
            }
        );
    }

    #[test]
    fn tp_header_is_joined() {
        let mut header = TpHeader {
            offset: 0,
            more_segments: true,
            body: Bytes::copy_from_slice([0u8; 16].as_slice()),
        };
        assert_eq!(
            header.join(TpHeader {
                offset: 1,
                more_segments: false,
                body: Bytes::copy_from_slice([0u8; 16].as_slice())
            }),
            Ok(())
        );
        assert_eq!(
            header,
            TpHeader {
                offset: 0,
                more_segments: false,
                body: Bytes::copy_from_slice([0u8; 32].as_slice())
            }
        );
    }

    #[test]
    fn extra_segments_are_not_joined() {
        let mut header = TpHeader::new(Bytes::copy_from_slice([0u8; 16].as_slice()));
        let other = TpHeader {
            offset: 1,
            more_segments: false,
            body: Bytes::copy_from_slice([0u8; 16].as_slice()),
        };
        assert_eq!(
            header.join(other.clone()),
            Err(TpError::UnexpectedSegment(other))
        );
    }

    #[test]
    fn offset_is_checked() {
        let mut header = TpHeader {
            offset: 0,
            more_segments: true,
            body: Bytes::copy_from_slice([0u8; 16].as_slice()),
        };
        let other = TpHeader {
            offset: 2,
            more_segments: false,
            body: Bytes::copy_from_slice([0u8; 16].as_slice()),
        };
        assert_eq!(
            header.join(other.clone()),
            Err(TpError::WrongOffset { expected: 1, other })
        );
    }

    // offset=1 more_segments=true body=1
    const SERIALIZED_HEADER: [u8; 5] = [0u8, 0, 0, 0b0001_0001, 1];

    #[test]
    fn header_is_serializable() {
        let header = TpHeader {
            offset: 1,
            more_segments: true,
            body: Bytes::copy_from_slice(&[1u8]),
        };
        let mut buffer = BytesMut::with_capacity(5);
        assert_eq!(header.size_hint(), 5);
        assert_eq!(header.serialize(&mut buffer), Ok(5));
        assert_eq!(&buffer.freeze()[..], &SERIALIZED_HEADER);
    }

    #[test]
    fn header_is_deserializable() {
        let mut buffer = Bytes::copy_from_slice(&SERIALIZED_HEADER);
        assert_eq!(
            TpHeader::deserialize(&mut buffer),
            Ok(TpHeader {
                offset: 1,
                more_segments: true,
                body: Bytes::copy_from_slice(&[1u8])
            })
        );
    }

    #[test]
    fn max_offset_is_checked() {
        let header = TpHeader {
            offset: 0xffff_ffff,
            more_segments: true,
            body: Bytes::copy_from_slice(&[1u8]),
        };
        let mut buffer = BytesMut::with_capacity(5);
        assert!(header.serialize(&mut buffer).is_err());
    }
}
