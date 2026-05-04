//! Criopolis cascade orchestrator daemon.
//!
//! The crate watches Gas City bead events, filters cascade-chain beads,
//! and dispatches cascade transitions through `gc`.

pub mod error;

pub use error::{Error, Result};
