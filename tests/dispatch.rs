use orchestrator::{
    AgentName, BeadId, CascadeAction, CascadeBead, CascadeDecision, EventSequence,
    OrchestratorEvent, OrchestratorEventKind,
};

struct CascadeFixture;

impl CascadeFixture {
    fn event(kind: OrchestratorEventKind) -> OrchestratorEvent {
        OrchestratorEvent::from_parts(
            EventSequence::new(7),
            kind,
            BeadId::new("cr-alpha").expect("fixture bead id should be valid"),
        )
    }

    fn bead(bead_id: &str, labels: &[&str], metadata: &[(&str, &str)]) -> CascadeBead {
        CascadeBead::new(
            BeadId::new(bead_id).expect("fixture bead id should be valid"),
            labels.iter().map(|label| (*label).to_owned()),
            metadata
                .iter()
                .map(|(field, value)| ((*field).to_owned(), (*value).to_owned())),
            Some("open".to_owned()),
        )
    }
}

#[test]
fn created_position_one_starts_chain() {
    let bead = CascadeFixture::bead(
        "cr-alpha",
        &["cascade-chain"],
        &[("gc.routed_to", "satya"), ("cascade_position", "1")],
    );

    let decision = CascadeDecision::from_event_and_beads(
        &CascadeFixture::event(OrchestratorEventKind::BeadCreated),
        &bead,
        None,
    )
    .expect("decision should succeed");

    assert_eq!(
        decision.action(),
        &CascadeAction::StartChain {
            target_agent: AgentName::new("satya").expect("agent name should be valid"),
            bead_id: BeadId::new("cr-alpha").expect("bead id should be valid"),
        }
    );
}

#[test]
fn closed_bead_with_next_advances_chain() {
    let bead = CascadeFixture::bead(
        "cr-alpha",
        &["cascade-chain"],
        &[("cascade_next", "cr-beta")],
    );
    let next_bead = CascadeFixture::bead(
        "cr-beta",
        &["cascade-chain"],
        &[("gc.routed_to", "viveka"), ("cascade_position", "2")],
    );

    let decision = CascadeDecision::from_event_and_beads(
        &CascadeFixture::event(OrchestratorEventKind::BeadClosed),
        &bead,
        Some(&next_bead),
    )
    .expect("decision should succeed");

    assert_eq!(
        decision.action(),
        &CascadeAction::AdvanceChain {
            target_agent: AgentName::new("viveka").expect("agent name should be valid"),
            bead_id: BeadId::new("cr-beta").expect("bead id should be valid"),
        }
    );
}

#[test]
fn closed_final_bead_signals_complete() {
    let bead = CascadeFixture::bead(
        "cr-final",
        &["cascade-chain"],
        &[("cascade_final", "true"), ("cascade_id", "round-one")],
    );
    let event = OrchestratorEvent::from_parts(
        EventSequence::new(8),
        OrchestratorEventKind::BeadClosed,
        BeadId::new("cr-final").expect("bead id should be valid"),
    );

    let decision = CascadeDecision::from_event_and_beads(&event, &bead, None)
        .expect("decision should succeed");

    assert_eq!(decision.action().action_name(), "signal-complete");
}

#[test]
fn order_tracking_bead_is_skipped() {
    let bead = CascadeFixture::bead(
        "cr-tracking",
        &["cascade-chain", "order-tracking"],
        &[("gc.routed_to", "satya"), ("cascade_position", "1")],
    );

    let decision = CascadeDecision::from_event_and_beads(
        &CascadeFixture::event(OrchestratorEventKind::BeadCreated),
        &bead,
        None,
    )
    .expect("decision should succeed");

    assert_eq!(decision.action().action_name(), "skip");
}
