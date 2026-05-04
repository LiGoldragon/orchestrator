//! Criopolis cascade orchestrator daemon.
//!
//! The crate watches Gas City bead events, filters cascade-chain beads,
//! and dispatches cascade transitions through `gc`.

pub mod bead;
pub mod command_line;
pub mod dispatch;
pub mod error;
pub mod event;
pub mod gc;
pub mod identifiers;
pub mod orchestrator;
pub mod state;

pub use bead::CascadeBead;
pub use command_line::CommandLine;
pub use dispatch::{CascadeAction, CascadeDecision, CascadeDispatchRecord, CascadeDispatcher};
pub use error::{Error, Result};
pub use event::{EventBatch, OrchestratorEvent, OrchestratorEventKind};
pub use gc::GcClient;
pub use identifiers::{AgentName, BeadId, CascadeId, EventSequence};
pub use orchestrator::{Orchestrator, OrchestratorConfiguration};
pub use state::EventCursor;
