//! Connection state and data structures for I/O management.

/// Represents the current state of a client connection.
pub enum ConnexionState {
    Reading,
    Writing,
}

/// Represents an active client connection with buffered output.
pub(crate) struct Connexion {
    pub(crate) state: ConnexionState,
    pub(crate) out: Vec<u8>,
}

impl Connexion {
    /// Creates a new connection in the Reading state.
    pub(crate) fn new() -> Self {
        Self {
            state: ConnexionState::Reading,
            out: Vec::new(),
        }
    }
}
