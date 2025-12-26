//! Fluent builder for Runtime construction.
//!
//! Provides a builder pattern interface for creating and configuring Runtime instances.

use crate::runtime::Runtime;

/// Builder for constructing Runtime instances with fluent API.
///
/// Allows customizable runtime instantiation following the builder pattern.
/// Currently creates a basic runtime, but designed to support future configuration options.
///
/// # Example
/// ```ignore
/// let rt = RuntimeBuilder::new().build();
/// ```
pub struct RuntimeBuilder {}

impl RuntimeBuilder {
    /// Creates a new runtime builder.
    ///
    /// Initializes a builder instance for constructing a Runtime.
    ///
    /// # Example
    /// ```ignore
    /// let builder = RuntimeBuilder::new();
    /// ```
    #[allow(clippy::new_without_default)] // TODO: Enable when configuration options are added
    pub fn new() -> Self {
        Self {}
    }

    /// Builds and returns a configured Runtime instance.
    ///
    /// Consumes the builder and constructs a Runtime with the current configuration.
    /// Currently creates a default runtime but designed to support configuration options.
    ///
    /// # Returns
    /// A newly constructed Runtime instance
    ///
    /// # Example
    /// ```ignore
    /// let rt = RuntimeBuilder::new().build();
    /// ```
    pub fn build(self) -> Runtime {
        Runtime::default()
    }
}
