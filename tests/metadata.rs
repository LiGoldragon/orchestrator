use orchestrator::CascadeBead;

#[test]
fn parses_cascade_metadata_from_bead_show_json() {
    let bead = CascadeBead::from_show_json(
        r#"
        [
          {
            "id": "cr-alpha",
            "status": "open",
            "labels": ["cascade-chain"],
            "metadata": {
              "gc.routed_to": "satya",
              "cascade_position": "1",
              "cascade_next": "cr-beta",
              "cascade_id": "round-one"
            }
          }
        ]
        "#,
    )
    .expect("bead should parse");

    assert!(bead.is_dispatchable());
    assert_eq!(bead.position().expect("position should parse"), Some(1));
    assert_eq!(
        bead.routed_to()
            .expect("target should parse")
            .expect("target should exist")
            .as_str(),
        "satya"
    );
    assert_eq!(
        bead.cascade_next()
            .expect("next bead should parse")
            .expect("next bead should exist")
            .as_str(),
        "cr-beta"
    );
    assert_eq!(
        bead.cascade_id()
            .expect("cascade id should parse")
            .expect("cascade id should exist")
            .as_str(),
        "round-one"
    );
}

#[test]
fn order_tracking_label_blocks_dispatch() {
    let bead = CascadeBead::from_show_json(
        r#"
        [
          {
            "id": "cr-tracking",
            "labels": ["cascade-chain", "order-tracking"],
            "metadata": {
              "gc.routed_to": "satya",
              "cascade_position": "1"
            }
          }
        ]
        "#,
    )
    .expect("bead should parse");

    assert!(!bead.is_dispatchable());
}
