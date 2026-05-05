//! Gas City event parsing.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::{BeadId, Error, EventSequence, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorEventKind {
    BeadCreated,
    BeadUpdated,
    BeadClosed,
    Other(String),
}

impl OrchestratorEventKind {
    pub fn from_event_type(event_type: impl Into<String>) -> Self {
        match event_type.into().as_str() {
            "bead.created" => Self::BeadCreated,
            "bead.updated" => Self::BeadUpdated,
            "bead.closed" => Self::BeadClosed,
            other_event_type => Self::Other(other_event_type.to_owned()),
        }
    }

    pub fn is_cascade_relevant(&self) -> bool {
        matches!(
            self,
            Self::BeadCreated | Self::BeadUpdated | Self::BeadClosed
        )
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::BeadCreated => "bead.created",
            Self::BeadUpdated => "bead.updated",
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
    payload: Option<EventPayload>,
}

impl EventLine {
    fn into_event(self) -> Result<Option<OrchestratorEvent>> {
        let kind = OrchestratorEventKind::from_event_type(self.event_type.as_str());
        if !kind.is_cascade_relevant() {
            return Ok(None);
        }
        if kind == OrchestratorEventKind::BeadUpdated && !self.has_cascade_marker() {
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

    fn has_cascade_marker(&self) -> bool {
        self.payload
            .as_ref()
            .and_then(|payload| payload.bead.as_ref())
            .is_some_and(EventBead::has_cascade_marker)
    }
}

#[derive(Debug, Deserialize)]
struct EventPayload {
    bead: Option<EventBead>,
}

#[derive(Debug, Deserialize)]
struct EventBead {
    labels: Option<Vec<String>>,
    metadata: Option<BTreeMap<String, Value>>,
}

impl EventBead {
    fn has_cascade_marker(&self) -> bool {
        self.has_cascade_chain_label() || self.has_cascade_metadata()
    }

    fn has_cascade_chain_label(&self) -> bool {
        self.labels
            .as_ref()
            .is_some_and(|labels| labels.iter().any(|label| label == "cascade-chain"))
    }

    fn has_cascade_metadata(&self) -> bool {
        self.metadata.as_ref().is_some_and(|metadata| {
            metadata.contains_key("cascade_position")
                || metadata.contains_key("cascade_next")
                || metadata.contains_key("cascade_final")
                || metadata.contains_key("cascade_id")
        })
    }
}
