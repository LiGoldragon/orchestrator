use orchestrator::{
    AgentName, BeadId, CascadeAction, CascadeDispatchRecord, EventCursor, EventSequence,
    OrchestratorEvent, OrchestratorEventKind,
};

#[test]
fn cursor_persists_across_reopen() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should exist");
    let database_path = temporary_directory.path().join("orchestrator.redb");

    let cursor = EventCursor::open(&database_path).expect("cursor should open");
    assert_eq!(cursor.current().expect("cursor should read"), None);
    cursor
        .advance(EventSequence::new(42))
        .expect("cursor should advance");
    drop(cursor);

    let reopened_cursor = EventCursor::open(&database_path).expect("cursor should reopen");
    assert_eq!(
        reopened_cursor.current().expect("cursor should read"),
        Some(EventSequence::new(42))
    );
}

#[test]
fn dispatch_record_round_trips_through_rkyv_and_redb() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should exist");
    let database_path = temporary_directory.path().join("orchestrator.redb");
    let cursor = EventCursor::open(&database_path).expect("cursor should open");
    let event = OrchestratorEvent::from_parts(
        EventSequence::new(9),
        OrchestratorEventKind::BeadCreated,
        BeadId::new("cr-alpha").expect("bead id should be valid"),
    );
    let action = CascadeAction::StartChain {
        target_agent: AgentName::new("satya").expect("agent name should be valid"),
        bead_id: BeadId::new("cr-alpha").expect("bead id should be valid"),
    };
    let record = CascadeDispatchRecord::from_event_and_action(&event, &action);
    let bytes = record
        .archived_bytes()
        .expect("record should archive to bytes");
    let restored = CascadeDispatchRecord::from_archived_bytes(&bytes)
        .expect("record should restore from bytes");

    assert_eq!(record, restored);
    cursor
        .record_dispatch(&record)
        .expect("dispatch record should persist");
    assert_eq!(
        cursor
            .recorded_dispatch_count()
            .expect("dispatch count should read"),
        1
    );
}
