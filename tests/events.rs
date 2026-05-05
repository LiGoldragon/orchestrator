use orchestrator::{EventBatch, OrchestratorEventKind};

#[test]
fn parses_and_sorts_cascade_relevant_events() {
    let batch = EventBatch::from_json_lines(
        r#"
{"seq":12,"subject":"cr-second","type":"bead.closed"}
{"seq":10,"subject":"cr-first","type":"bead.created"}
{"seq":11,"subject":"cr-ignored","type":"session.created"}
{"seq":13,"type":"city.started"}
{"seq":14,"subject":"cr-third","type":"bead.updated","payload":{"bead":{"labels":["cascade-chain"],"metadata":{"cascade_position":"1"}}}}
{"seq":15,"subject":"cr-fourth","type":"bead.updated","payload":{"bead":{"labels":["gc:session"]}}}
"#,
    )
    .expect("events should parse");

    let events = batch.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].sequence().value(), 10);
    assert_eq!(events[0].bead_id().as_str(), "cr-first");
    assert_eq!(events[0].kind(), &OrchestratorEventKind::BeadCreated);
    assert_eq!(events[1].sequence().value(), 12);
    assert_eq!(events[1].kind(), &OrchestratorEventKind::BeadClosed);
    assert_eq!(events[2].sequence().value(), 14);
    assert_eq!(events[2].kind(), &OrchestratorEventKind::BeadUpdated);
}
