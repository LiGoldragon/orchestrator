//! Cascade dispatch decisions and side effects.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::{
    AgentName, BeadId, CascadeBead, CascadeId, Error, EventCursor, GcClient, OrchestratorEvent,
    OrchestratorEventKind, Result,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeAction {
    Skip {
        reason: String,
    },
    StartChain {
        target_agent: AgentName,
        bead_id: BeadId,
    },
    AdvanceChain {
        target_agent: AgentName,
        bead_id: BeadId,
    },
    SignalComplete {
        cascade_id: CascadeId,
        final_bead_id: BeadId,
    },
}

impl CascadeAction {
    pub fn execute(&self, gc_client: &GcClient) -> Result<()> {
        match self {
            Self::Skip { reason } => {
                eprintln!("orchestrator: skip: {reason}");
                Ok(())
            }
            Self::StartChain {
                target_agent,
                bead_id,
            } => gc_client.sling(target_agent, bead_id),
            Self::AdvanceChain {
                target_agent,
                bead_id,
            } => gc_client.sling(target_agent, bead_id),
            Self::SignalComplete {
                cascade_id,
                final_bead_id,
            } => gc_client.mail_cascade_complete(cascade_id, final_bead_id),
        }
    }

    pub fn action_name(&self) -> &'static str {
        match self {
            Self::Skip { .. } => "skip",
            Self::StartChain { .. } => "start-chain",
            Self::AdvanceChain { .. } => "advance-chain",
            Self::SignalComplete { .. } => "signal-complete",
        }
    }

    pub fn target_agent(&self) -> Option<&AgentName> {
        match self {
            Self::StartChain { target_agent, .. } | Self::AdvanceChain { target_agent, .. } => {
                Some(target_agent)
            }
            Self::Skip { .. } | Self::SignalComplete { .. } => None,
        }
    }

    pub fn target_bead_id(&self) -> Option<&BeadId> {
        match self {
            Self::StartChain { bead_id, .. } | Self::AdvanceChain { bead_id, .. } => Some(bead_id),
            Self::SignalComplete { final_bead_id, .. } => Some(final_bead_id),
            Self::Skip { .. } => None,
        }
    }

    pub fn cascade_id(&self) -> Option<&CascadeId> {
        match self {
            Self::SignalComplete { cascade_id, .. } => Some(cascade_id),
            Self::Skip { .. } | Self::StartChain { .. } | Self::AdvanceChain { .. } => None,
        }
    }

    pub fn recorded_duplicate_reason(&self) -> Option<String> {
        match self {
            Self::StartChain { bead_id, .. } => Some(format!(
                "start-chain for bead {bead_id} is already recorded"
            )),
            Self::AdvanceChain { bead_id, .. } => Some(format!(
                "advance-chain for bead {bead_id} is already recorded"
            )),
            Self::SignalComplete {
                cascade_id,
                final_bead_id,
            } => Some(format!(
                "completion for cascade {cascade_id} final bead {final_bead_id} is already recorded"
            )),
            Self::Skip { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CascadeDecision {
    action: CascadeAction,
}

impl CascadeDecision {
    pub fn from_event_and_beads(
        event: &OrchestratorEvent,
        bead: &CascadeBead,
        next_bead: Option<&CascadeBead>,
    ) -> Result<Self> {
        if !bead.is_dispatchable() {
            return Ok(Self::skip(format!(
                "bead {} is not a dispatchable cascade-chain bead",
                bead.bead_id()
            )));
        }

        match event.kind() {
            OrchestratorEventKind::BeadCreated | OrchestratorEventKind::BeadUpdated => {
                Self::from_start_candidate_bead(bead)
            }
            OrchestratorEventKind::BeadClosed => Self::from_closed_bead(bead, next_bead),
            OrchestratorEventKind::Other(_) => Ok(Self::skip("event kind is not cascade-relevant")),
        }
    }

    pub fn action(&self) -> &CascadeAction {
        &self.action
    }

    fn from_start_candidate_bead(bead: &CascadeBead) -> Result<Self> {
        if bead.position()? == Some(1) {
            Ok(Self {
                action: CascadeAction::StartChain {
                    target_agent: bead.required_routed_to()?,
                    bead_id: bead.bead_id().clone(),
                },
            })
        } else {
            Ok(Self::skip("bead is not cascade position 1"))
        }
    }

    fn from_closed_bead(bead: &CascadeBead, next_bead: Option<&CascadeBead>) -> Result<Self> {
        if let Some(next_bead_id) = bead.cascade_next()? {
            let next_bead = next_bead.ok_or_else(|| Error::MissingNextBead {
                bead_id: bead.bead_id().to_string(),
                next_bead_id: next_bead_id.to_string(),
            })?;
            return Ok(Self {
                action: CascadeAction::AdvanceChain {
                    target_agent: next_bead.required_routed_to()?,
                    bead_id: next_bead_id,
                },
            });
        }

        if bead.is_final() {
            return Ok(Self {
                action: CascadeAction::SignalComplete {
                    cascade_id: bead.cascade_id_or_bead_id()?,
                    final_bead_id: bead.bead_id().clone(),
                },
            });
        }

        Ok(Self::skip(
            "closed cascade bead has no next bead and is not final",
        ))
    }

    fn skip(reason: impl Into<String>) -> Self {
        Self {
            action: CascadeAction::Skip {
                reason: reason.into(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CascadeDispatcher {
    gc_client: GcClient,
}

impl CascadeDispatcher {
    pub fn new(gc_client: GcClient) -> Self {
        Self { gc_client }
    }

    pub fn dispatch(
        &self,
        event: &OrchestratorEvent,
        dispatch_history: &EventCursor,
    ) -> Result<CascadeDispatchRecord> {
        let bead = match self.gc_client.bead(event.bead_id()) {
            Ok(bead) => bead,
            Err(Error::MissingBead { bead_id }) => {
                let action = CascadeAction::Skip {
                    reason: format!("event subject {bead_id} is not in the city bead store"),
                };
                action.execute(&self.gc_client)?;
                return Ok(CascadeDispatchRecord::from_event_and_action(event, &action));
            }
            Err(error) => return Err(error),
        };
        let next_bead = match event.kind() {
            OrchestratorEventKind::BeadClosed => self.next_bead_for(&bead)?,
            OrchestratorEventKind::BeadCreated
            | OrchestratorEventKind::BeadUpdated
            | OrchestratorEventKind::Other(_) => None,
        };
        let decision = CascadeDecision::from_event_and_beads(event, &bead, next_bead.as_ref())?;
        if let Some(duplicate_reason) = decision.action().recorded_duplicate_reason() {
            if dispatch_history.has_recorded_action(decision.action())? {
                let action = CascadeAction::Skip {
                    reason: duplicate_reason,
                };
                action.execute(&self.gc_client)?;
                return Ok(CascadeDispatchRecord::from_event_and_action(event, &action));
            }
        }
        decision.action().execute(&self.gc_client)?;
        Ok(CascadeDispatchRecord::from_event_and_action(
            event,
            decision.action(),
        ))
    }

    fn next_bead_for(&self, bead: &CascadeBead) -> Result<Option<CascadeBead>> {
        bead.cascade_next()?
            .map(|next_bead_id| self.gc_client.bead(&next_bead_id))
            .transpose()
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CascadeDispatchRecord {
    event_sequence: u64,
    event_type: String,
    source_bead_id: String,
    action: String,
    target_agent: Option<String>,
    target_bead_id: Option<String>,
    cascade_id: Option<String>,
}

impl CascadeDispatchRecord {
    pub fn from_event_and_action(event: &OrchestratorEvent, action: &CascadeAction) -> Self {
        Self {
            event_sequence: event.sequence().value(),
            event_type: event.kind().as_str().to_owned(),
            source_bead_id: event.bead_id().to_string(),
            action: action.action_name().to_owned(),
            target_agent: action.target_agent().map(ToString::to_string),
            target_bead_id: action.target_bead_id().map(ToString::to_string),
            cascade_id: action.cascade_id().map(ToString::to_string),
        }
    }

    pub fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn describes_action(&self, action: &CascadeAction) -> bool {
        self.action == action.action_name()
            && self.target_agent.as_deref() == action.target_agent().map(AgentName::as_str)
            && self.target_bead_id.as_deref() == action.target_bead_id().map(BeadId::as_str)
            && self.cascade_id.as_deref() == action.cascade_id().map(CascadeId::as_str)
    }

    pub fn archived_bytes(&self) -> Result<Vec<u8>> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(Error::archive)
    }

    pub fn from_archived_bytes(bytes: &[u8]) -> Result<Self> {
        rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes).map_err(Error::archive)
    }
}
