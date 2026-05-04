//! Gas City event parsing.

use serde::Deserialize;

use crate::{BeadId, Error, EventSequence, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorEventKind {
    BeadCreated,
    BeadClosed,
    Other(String),
}

impl OrchestratorEventKind {
    pub fn from_event_type(event_type: impl Into<String>) -> Self {
        match event_type.into().as_str() {
            "bead.created" => Self::BeadCreated,
            "bead.closed" => Self::BeadClosed,
            other_event_type => Self::Other(other_event_type.to_owned()),
        }
    }

    pub fn is_cascade_relevant(&self) -> bool {
        matches!(self, Self::BeadCreated | Self::BeadClosed)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::BeadCreated => "bead.created",
            Self::BeadClosed => "bead.closed",
            Self::Other(event_type) => event_type.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorEvent {
    sequence: EventSequence,
    kind: OrchestratorEventKind,
    bead_id: BeadId,
}

impl OrchestratorEvent {
    pub fn from_parts(
        sequence: EventSequence,
        kind: OrchestratorEventKind,
        bead_id: BeadId,
    ) -> Self {
        Self {
            sequence,
            kind,
            bead_id,
        }
    }

    pub fn from_json_line(line: &str) -> Result<Self> {
        let event_line: EventLine = serde_json::from_str(line)?;
        event_line.into_required_event()
    }

    pub fn sequence(&self) -> EventSequence {
        self.sequence
    }

    pub fn kind(&self) -> &OrchestratorEventKind {
        &self.kind
    }

    pub fn bead_id(&self) -> &BeadId {
        &self.bead_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBatch {
    events: Vec<OrchestratorEvent>,
}

impl EventBatch {
    pub fn from_json_lines(lines: &str) -> Result<Self> {
        let mut events = Vec::new();
        for line in lines.lines().filter(|line| !line.trim().is_empty()) {
            let event_line: EventLine = serde_json::from_str(line)?;
            if let Some(event) = event_line.into_event()? {
                events.push(event);
            }
        }
        events.sort_by_key(OrchestratorEvent::sequence);
        Ok(Self { events })
    }

    pub fn events(&self) -> &[OrchestratorEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<OrchestratorEvent> {
        self.events
    }
}

#[derive(Debug, Deserialize)]
struct EventLine {
    #[serde(rename = "seq")]
    sequence: u64,
    subject: Option<String>,
    #[serde(rename = "type")]
    event_type: String,
}

impl EventLine {
    fn into_event(self) -> Result<Option<OrchestratorEvent>> {
        let kind = OrchestratorEventKind::from_event_type(self.event_type);
        if !kind.is_cascade_relevant() {
            return Ok(None);
        }

        let Some(subject) = self.subject else {
            return Ok(None);
        };

        Ok(Some(OrchestratorEvent {
            sequence: EventSequence::new(self.sequence),
            kind,
            bead_id: BeadId::new(subject)?,
        }))
    }

    fn into_required_event(self) -> Result<OrchestratorEvent> {
        self.into_event()?.ok_or_else(|| Error::InvalidMetadata {
            bead_id: "gc event".to_owned(),
            field: "subject",
            value: "missing or non-bead event".to_owned(),
        })
    }
}
