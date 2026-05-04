//! Error type for the orchestrator daemon.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("gc command failed: {command}; status={status:?}; stderr={stderr}")]
    GcCommandFailed {
        command: String,
        status: Option<i32>,
        stderr: String,
    },

    #[error("bead show response did not contain a bead")]
    EmptyBeadResponse,

    #[error("bead {bead_id} is not present in the active city bead store")]
    MissingBead { bead_id: String },

    #[error("invalid identifier for {kind}: {value}")]
    InvalidIdentifier { kind: &'static str, value: String },

    #[error("invalid metadata on bead {bead_id}: {field}={value}")]
    InvalidMetadata {
        bead_id: String,
        field: &'static str,
        value: String,
    },

    #[error("missing cascade target on bead {bead_id}")]
    MissingCascadeTarget { bead_id: String },

    #[error("missing next bead for dispatch from {bead_id} to {next_bead_id}")]
    MissingNextBead {
        bead_id: String,
        next_bead_id: String,
    },

    #[error("redb state error: {0}")]
    State(String),

    #[error("rkyv archive error: {0}")]
    Archive(String),
}

impl Error {
    pub fn state(error: impl std::fmt::Display) -> Self {
        Self::State(error.to_string())
    }

    pub fn archive(error: impl std::fmt::Display) -> Self {
        Self::Archive(error.to_string())
    }

    pub fn is_missing_bead_command(&self) -> bool {
        matches!(
            self,
            Self::GcCommandFailed { stderr, .. }
                if stderr.contains("no issue found matching")
                    || stderr.contains("not found")
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;
