//! `rsomeip` error types.

/// A specialized result type for `rsomeip` operations.
pub type Result<T> = std::result::Result<T, Error>;

/// An error associated with `rsomeip` operations.
pub type Error = Box<dyn std::error::Error>;
