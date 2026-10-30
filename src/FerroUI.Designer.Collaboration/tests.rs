use super::*;
const SOURCE: &str =
    "<Grid><!-- preserve me --><Button Name='a' Width='10'/><Button Name='b'/></Grid>";
fn replica(actor: &str) -> DocumentReplica {
    DocumentReplica::new(actor, "design-session", SOURCE).unwrap()
}
fn named(replica: &DocumentReplica, name: &str) -> SharedNodeId {
    replica
        .shared_id(replica.document().ast().find_name(name).unwrap())
        .unwrap()
}
fn message(operations: Vec<Operation>) -> SyncMessage {
    SyncMessage {
        protocol: PROTOCOL_VERSION,
        document_id: "design-session".into(),
        baseline_hash: replica("helper").baseline_hash().into(),
        operations,
    }
}
#[test]
fn concurrent_property_writes_converge_in_every_delivery_order() {
    let mut a = replica("a");
    let mut b = replica("b");
    let a_node = named(&a, "a");
    let b_node = named(&b, "a");
    let left = a.set_attribute(a_node, "Width", "100").unwrap();
    let right = b.set_attribute(b_node, "Width", "200").unwrap();
    for operations in [
        vec![left.clone(), right.clone()],
        vec![right.clone(), left.clone()],
    ] {
        let mut peer = replica("peer");
        for op in operations {
            peer.merge(message(vec![op])).unwrap();
        }
        assert!(peer.source().contains("Width='200'"));
        assert!(peer.source().contains("<!-- preserve me -->"));
        assert!(peer.conflicts().is_empty());
    }
    a.merge(message(vec![right])).unwrap();
    b.merge(message(vec![left])).unwrap();
    assert_eq!(a.source(), b.source());
}
#[test]
fn out_of_order_insert_dependencies_retry_and_duplicate_delivery_is_idempotent() {
    let mut author = replica("author");
    let insert = author
        .submit(
            "Insert",
            vec![SharedEdit::InsertChild {
                parent: author.root(),
                index: 2,
                source: "<TextBlock Name='new'/>".into(),
            }],
        )
        .unwrap();
    let inserted = named(&author, "new");
    let write = author
        .set_attribute(inserted, "Text", "Zażółć & <ok>")
        .unwrap();
    let mut remote = replica("remote");
    let report = remote.merge(message(vec![write.clone()])).unwrap();
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(remote.version().get("author"), Some(&0));
    remote.merge(message(vec![insert.clone()])).unwrap();
    assert_eq!(remote.source(), author.source());
    assert!(remote.conflicts().is_empty());
    assert_eq!(remote.version()["author"], 2);
    let report = remote.merge(message(vec![insert, write])).unwrap();
    assert_eq!(report.received, 0);
    assert_eq!(remote.source(), author.source());
}
#[test]
fn structural_cycles_are_resolved_deterministically_without_invalid_xml() {
    let mut a = replica("a");
    let mut b = replica("b");
    let one = named(&a, "a");
    let two = named(&a, "b");
    let first = a
        .submit(
            "Move a under b",
            vec![SharedEdit::MoveNode {
                node: one.clone(),
                parent: two.clone(),
                index: 0,
            }],
        )
        .unwrap();
    let second = b
        .submit(
            "Move b under a",
            vec![SharedEdit::MoveNode {
                node: two,
                parent: one,
                index: 0,
            }],
        )
        .unwrap();
    a.merge(message(vec![second])).unwrap();
    b.merge(message(vec![first])).unwrap();
    assert_eq!(a.source(), b.source());
    assert_eq!(a.conflicts().len(), 1);
    assert_eq!(a.conflicts(), b.conflicts());
}
#[test]
fn remote_transaction_is_atomic_when_a_later_edit_conflicts() {
    let mut remote = replica("remote");
    let node = named(&remote, "a");
    let operation = Operation {
        id: OperationId {
            lamport: 1,
            actor: "writer".into(),
            sequence: 1,
        },
        document_id: "design-session".into(),
        label: "atomic".into(),
        edits: vec![
            SharedEdit::SetAttribute {
                node,
                name: "Width".into(),
                value: "900".into(),
            },
            SharedEdit::RemoveNode {
                node: SharedNodeId::Baseline(999_999),
            },
        ],
    };
    remote.merge(message(vec![operation])).unwrap();
    assert_eq!(remote.source(), SOURCE);
    assert_eq!(remote.conflicts().len(), 1);
}

#[test]
fn failed_local_projection_does_not_publish_partial_source_identities_or_counters() {
    let mut local = replica("local");
    let before = local.snapshot();
    assert!(local
        .submit(
            "Rejected insertion",
            vec![
                SharedEdit::InsertChild {
                    parent: local.root(),
                    index: 0,
                    source: "<TextBlock Name='temporary'/>".into()
                },
                SharedEdit::SetAttribute {
                    node: named(&local, "a"),
                    name: "Width".into(),
                    value: "900".into()
                },
                SharedEdit::RemoveNode {
                    node: SharedNodeId::Baseline(999_999)
                },
            ]
        )
        .is_err());
    assert_eq!(local.source(), SOURCE);
    assert_eq!(local.snapshot(), before);
    assert!(local.version().is_empty());
    assert!(local.document().ast().find_name("temporary").is_none());
    let valid = local
        .set_attribute(named(&local, "a"), "Width", "22")
        .unwrap();
    assert_eq!(valid.id.sequence, 1);
    assert_eq!(valid.id.lamport, 1);
    local.undo_property(&valid.id).unwrap();
    assert_eq!(local.source(), SOURCE);
}
#[test]
fn equivocation_rejects_the_entire_batch() {
    let mut author = replica("author");
    let node = named(&author, "a");
    let op = author.set_attribute(node, "Width", "50").unwrap();
    let mut remote = replica("remote");
    remote.merge(message(vec![op.clone()])).unwrap();
    let before = remote.source().to_owned();
    let mut invalid = op.clone();
    invalid.label = "different".into();
    let next = author
        .set_attribute(named(&author, "b"), "Height", "90")
        .unwrap();
    assert!(remote.merge(message(vec![next, invalid])).is_err());
    assert_eq!(remote.source(), before);
    assert_eq!(remote.operations().count(), 1);
}
#[test]
fn snapshots_resume_counters_and_reject_different_baselines() {
    let mut original = replica("author");
    original
        .set_attribute(named(&original, "a"), "Width", "42")
        .unwrap();
    let mut resumed = DocumentReplica::from_snapshot("author", original.snapshot()).unwrap();
    let next = resumed
        .set_attribute(named(&resumed, "a"), "Height", "20")
        .unwrap();
    assert_eq!(next.id.sequence, 2);
    assert_eq!(next.id.lamport, 2);
    let mut different = DocumentReplica::new("other", "design-session", "<Grid/>").unwrap();
    assert!(different.merge_snapshot(original.snapshot()).is_err());
}
#[test]
fn source_compare_and_set_reports_concurrent_edits() {
    let mut a = replica("a");
    let mut b = replica("b");
    let property = a.set_attribute(named(&a, "a"), "Width", "99").unwrap();
    let replacement = b
        .submit(
            "Source edit",
            vec![SharedEdit::ReplaceSource {
                expected_source: SOURCE.into(),
                source: SOURCE.replace("Button", "TextBlock"),
            }],
        )
        .unwrap();
    a.merge(message(vec![replacement])).unwrap();
    b.merge(message(vec![property])).unwrap();
    assert_eq!(a.source(), b.source());
    assert_eq!(a.conflicts().len(), 1);
    assert!(a.source().contains("Width='99'"));
}
#[test]
fn undo_is_a_compensating_write_and_does_not_clobber_remote_changes() {
    let mut a = replica("a");
    let op = a.set_attribute(named(&a, "a"), "Width", "22").unwrap();
    let undo = a.undo_property(&op.id).unwrap();
    assert_eq!(undo.id.sequence, 2);
    assert_eq!(a.source(), SOURCE);
    let op = a.set_attribute(named(&a, "a"), "Width", "22").unwrap();
    let mut b = DocumentReplica::from_snapshot("b", a.snapshot()).unwrap();
    let remote = b.set_attribute(named(&b, "a"), "Width", "22").unwrap();
    a.merge(message(vec![remote])).unwrap();
    assert!(a.undo_property(&op.id).is_err());
}
#[test]
fn presence_uses_monotonic_receive_time_and_ignores_old_packets() {
    let mut replica = replica("local");
    let presence = Presence {
        actor: "peer".into(),
        sequence: 2,
        display_name: "Peer".into(),
        selection: vec![replica.root()],
        cursor: Some(Cursor { x: 10.0, y: 20.0 }),
    };
    assert!(replica.receive_presence(presence.clone(), 100).unwrap());
    let mut old = presence;
    old.sequence = 1;
    assert!(!replica.receive_presence(old, 200).unwrap());
    assert_eq!(replica.presence(250, 200).len(), 1);
    assert!(replica.presence(300, 200).is_empty());
}
#[test]
fn all_permutations_of_concurrent_inserts_converge() {
    let mut operations = vec![];
    for actor in ["a", "b", "c"] {
        let mut replica = replica(actor);
        operations.push(
            replica
                .submit(
                    "Insert",
                    vec![SharedEdit::InsertChild {
                        parent: replica.root(),
                        index: 0,
                        source: format!("<TextBlock Name='{actor}new'/>"),
                    }],
                )
                .unwrap(),
        );
    }
    let mut expected = None;
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut peer = replica("peer");
        for index in order {
            peer.merge(message(vec![operations[index].clone()]))
                .unwrap();
        }
        if let Some(expected) = &expected {
            assert_eq!(peer.source(), expected);
        } else {
            expected = Some(peer.source().to_owned());
        }
    }
}
#[test]
fn baseline_hash_and_actor_causality_are_checked_before_merging() {
    let mut remote = replica("remote");
    let mut other = DocumentReplica::new("other", "design-session", "<Grid/>").unwrap();
    let foreign = other.set_attribute(other.root(), "Width", "20").unwrap();
    assert!(remote.merge(other.message(vec![foreign])).is_err());
    let mut author = replica("author");
    let first = author
        .set_attribute(named(&author, "a"), "Width", "1")
        .unwrap();
    let mut second = author
        .set_attribute(named(&author, "a"), "Width", "2")
        .unwrap();
    second.id.lamport = 1;
    assert!(remote.merge(message(vec![first, second])).is_err());
    assert_eq!(remote.source(), SOURCE);
}
#[test]
fn host_document_ids_can_differ_without_misdirecting_shared_commands() {
    let mut host = ferroui_designer_core::Document::new("<Grid/>").unwrap();
    host.apply(
        "Open",
        vec![ferroui_designer_core::Edit::ReplaceSource {
            source: SOURCE.into(),
        }],
    )
    .unwrap();
    let node = host.ast().find_name("a").unwrap();
    let mut collaboration = replica("local");
    let operation = collaboration
        .submit_document_edits(
            &host,
            "Edit",
            vec![ferroui_designer_core::Edit::SetAttribute {
                node,
                name: "Width".into(),
                value: "40".into(),
            }],
        )
        .unwrap();
    let mut remote = replica("remote");
    remote
        .merge(collaboration.message(vec![operation]))
        .unwrap();
    assert_eq!(remote.source(), collaboration.source());
    assert!(remote.source().contains("Width='40'"));
}
#[test]
fn namespace_and_prefixed_attribute_are_one_atomic_syntax_boundary() {
    let mut host = ferroui_designer_core::Document::new("<Button/>").unwrap();
    let root = host.ast().root();
    let mut replica = DocumentReplica::new("author", "namespaces", host.source()).unwrap();
    let edits = vec![
        ferroui_designer_core::Edit::SetAttribute {
            node: root,
            name: "x:Name".into(),
            value: "action".into(),
        },
        ferroui_designer_core::Edit::SetAttribute {
            node: root,
            name: "xmlns:x".into(),
            value: "http://schemas.microsoft.com/winfx/2006/xaml".into(),
        },
    ];
    replica
        .submit_document_edits(&host, "Name control", edits.clone())
        .unwrap();
    host.apply("Name control", edits).unwrap();
    assert_eq!(replica.source(), host.source());
}

#[test]
fn mixed_structural_transactions_batch_namespace_attributes_like_the_host_document() {
    use ferroui_designer_core::{Document, Edit};
    let mut host = Document::new("<Panel><Button Name='a'/><Button Name='b'/></Panel>").unwrap();
    let root = host.ast().root();
    let a = host.ast().find_name("a").unwrap();
    let mut author = DocumentReplica::new("author", "mixed-namespace", host.source()).unwrap();
    let baseline = author.snapshot();
    let edits = vec![
        Edit::InsertChild {
            parent: root,
            index: 0,
            source: "<TextBlock/>".into(),
        },
        Edit::SetAttribute {
            node: a,
            name: "q:Flag".into(),
            value: "on".into(),
        },
        Edit::SetAttribute {
            node: a,
            name: "xmlns:q".into(),
            value: "urn:custom".into(),
        },
        Edit::MoveNode {
            node: a,
            parent: root,
            index: 0,
        },
    ];
    let operation = author
        .submit_document_edits(&host, "Insert, configure and move", edits.clone())
        .unwrap();
    host.apply("Insert, configure and move", edits).unwrap();
    assert_eq!(author.source(), host.source());
    let mut remote = DocumentReplica::from_snapshot("remote", baseline).unwrap();
    remote.merge(author.message(vec![operation])).unwrap();
    assert_eq!(remote.source(), host.source());
    assert!(remote.conflicts().is_empty());
}

#[test]
fn undo_replay_preserves_unseen_concurrent_same_value_writer() {
    let mut a = replica("a");
    let mut b = replica("b");
    let original = a.set_attribute(named(&a, "a"), "Width", "22").unwrap();
    // B has not observed A, and deliberately chooses the same value. Equality
    // of the source text cannot substitute for writer identity in an undo CAS.
    let concurrent = b.set_attribute(named(&b, "a"), "Width", "22").unwrap();
    let undo = a.undo_property(&original.id).unwrap();
    assert_eq!(a.source(), SOURCE);
    a.merge(message(vec![concurrent.clone()])).unwrap();
    assert!(a.source().contains("Width='22'"));
    assert_eq!(a.conflicts().len(), 1);
    assert_eq!(a.conflicts()[0].operation, undo.id);
    for order in [
        vec![undo.clone(), original.clone(), concurrent.clone()],
        vec![concurrent.clone(), undo.clone(), original.clone()],
        vec![original, concurrent, undo],
    ] {
        let mut peer = replica("peer");
        for operation in order {
            peer.merge(message(vec![operation])).unwrap();
        }
        assert_eq!(peer.source(), a.source());
        assert_eq!(peer.conflicts(), a.conflicts());
    }
}

#[test]
fn conditional_property_undo_allows_unrelated_concurrent_edit() {
    let mut a = replica("a");
    let mut b = replica("b");
    let original = a.set_attribute(named(&a, "a"), "Width", "22").unwrap();
    let unrelated = b.set_attribute(named(&b, "a"), "Height", "30").unwrap();
    let undo = a.undo_property(&original.id).unwrap();
    a.merge(message(vec![unrelated])).unwrap();
    assert!(a.conflicts().is_empty());
    assert!(a.source().contains("Width='10'"));
    assert!(a.source().contains("Height='30'"));
    assert!(
        matches!(&undo.edits[0],SharedEdit::UndoAttribute{expected_writer,..} if expected_writer==&original.id)
    );
}

#[test]
fn failed_source_cas_does_not_claim_property_ownership() {
    let mut a = replica("a");
    let mut b = replica("b");
    let original = a.set_attribute(named(&a, "a"), "Width", "22").unwrap();
    let source = b
        .submit(
            "Concurrent source",
            vec![SharedEdit::ReplaceSource {
                expected_source: SOURCE.into(),
                source: SOURCE.replace("Button", "TextBlock"),
            }],
        )
        .unwrap();
    a.merge(message(vec![source])).unwrap();
    assert_eq!(a.conflicts().len(), 1);
    a.undo_property(&original.id).unwrap();
    assert_eq!(a.source(), SOURCE);
}

#[test]
fn namespace_property_replacements_receive_shared_identities() {
    let mut a = DocumentReplica::new("a", "namespaces", "<Grid><Button/></Grid>").unwrap();
    let mut b = DocumentReplica::new("b", "namespaces", "<Grid><Button/></Grid>").unwrap();
    let old_root = a.root();
    let change = a
        .set_attribute(old_root.clone(), "xmlns", "urn:designer-ui")
        .unwrap();
    let new_root = a.root();
    assert_ne!(old_root, new_root);
    let property = a.set_attribute(new_root.clone(), "Width", "120").unwrap();
    b.merge(a.message(vec![property, change])).unwrap();
    assert_eq!(b.source(), a.source());
    assert_eq!(b.root(), new_root);
    assert!(b.conflicts().is_empty());
}

#[test]
fn cached_vectors_track_gaps_replays_duplicates_and_snapshot_restore() {
    let mut author = replica("author");
    let mut operations = Vec::new();
    for width in 1..=6 {
        operations.push(
            author
                .set_attribute(named(&author, "a"), "Width", width.to_string())
                .unwrap(),
        );
    }
    let mut peer = replica("peer");
    for (index, contiguous) in [(5, 0), (2, 0), (0, 1), (1, 3), (4, 3), (3, 6)] {
        peer.merge(message(vec![operations[index].clone()]))
            .unwrap();
        assert_eq!(peer.version()["author"], contiguous);
        assert_eq!(
            peer.has_sequence_gaps(),
            contiguous != peer.operations().count() as u64
        );
        assert_eq!(
            peer.pending_operation_count(&VersionVector::from([("author".into(), 2)]), "author"),
            peer.operations()
                .filter(|operation| operation.id.sequence > 2)
                .count(),
        );
    }
    peer.merge(message(operations)).unwrap();
    let restored = DocumentReplica::from_snapshot("peer", peer.snapshot()).unwrap();
    assert_eq!(restored.version(), peer.version());
    assert!(!restored.has_sequence_gaps());
    assert_eq!(restored.source(), author.source());
}

#[test]
fn actor_timestamps_must_leave_room_for_missing_sequences() {
    let mut author = replica("author");
    let mut first = author
        .set_attribute(named(&author, "a"), "Width", "20")
        .unwrap();
    first.id.lamport = 100;
    let mut third = first.clone();
    third.id.sequence = 3;
    third.id.lamport = 101;
    for operations in [
        vec![first.clone(), third.clone()],
        vec![third.clone(), first.clone()],
    ] {
        let mut peer = replica("peer");
        assert!(peer.merge(message(operations)).is_err());
        assert_eq!(peer.source(), SOURCE);
        assert!(peer.version().is_empty());
    }
    for (accepted, rejected) in [(first.clone(), third.clone()), (third, first)] {
        let mut peer = replica("peer");
        peer.merge(message(vec![accepted])).unwrap();
        let before = peer.snapshot();
        assert!(peer.merge(message(vec![rejected])).is_err());
        assert_eq!(peer.snapshot(), before);
    }
}

#[test]
fn resumed_writer_waits_for_missing_local_history() {
    let mut author = replica("author");
    let first = author
        .set_attribute(named(&author, "a"), "Width", "20")
        .unwrap();
    let second = author
        .set_attribute(named(&author, "a"), "Width", "30")
        .unwrap();
    let mut resumed = replica("author");
    resumed.merge(message(vec![second])).unwrap();
    assert!(resumed
        .set_attribute(named(&resumed, "a"), "Height", "40")
        .is_err());
    resumed.merge(message(vec![first])).unwrap();
    let next = resumed
        .set_attribute(named(&resumed, "a"), "Height", "40")
        .unwrap();
    assert_eq!(next.id.sequence, 3);
}

#[test]
fn bounded_sync_pages_measure_escaped_bytes_and_converge() {
    let mut author = replica("author");
    for index in 0..12 {
        author
            .set_attribute(
                named(&author, "a"),
                "Text",
                format!("{index}: {}", "\"\\\n𝄞".repeat(35)),
            )
            .unwrap();
    }
    let mut peer = replica("peer");
    let mut pages = 0;
    loop {
        let page = author
            .changes_since_bounded(&peer.version(), None, 1600)
            .unwrap();
        assert!(page.to_json().unwrap().len() <= 1600);
        if page.operations.is_empty() {
            break;
        }
        let report = peer.merge(page).unwrap();
        assert!(
            report.received > 0,
            "each page must advance its receiving vector"
        );
        pages += 1;
        assert!(pages <= 12);
    }
    assert!(pages > 1);
    assert_eq!(peer.version(), author.version());
    assert_eq!(peer.source(), author.source());
    assert!(author
        .changes_since_bounded(&VersionVector::new(), None, 10)
        .is_err());
    assert!(author
        .changes_since_bounded(&VersionVector::new(), None, 300)
        .is_err());
}

#[test]
fn bounded_pages_filter_writers_and_do_not_stall_behind_sequence_holes() {
    let mut a = replica("a");
    let first = a.set_attribute(named(&a, "a"), "Width", "20").unwrap();
    let second = a.set_attribute(named(&a, "a"), "Width", "30").unwrap();
    let mut b = replica("b");
    let other = b.set_attribute(named(&b, "b"), "Height", "50").unwrap();
    let mut host = replica("host");
    host.merge(message(vec![second, other.clone()])).unwrap();
    let page = host
        .changes_since_bounded(&VersionVector::new(), None, 1000)
        .unwrap();
    assert_eq!(page.operations, vec![other]);
    host.merge(message(vec![first])).unwrap();
    let page = host
        .changes_since_bounded(&VersionVector::new(), Some("a"), 1000)
        .unwrap();
    assert_eq!(page.operations.len(), 2);
    assert!(page
        .operations
        .iter()
        .all(|operation| operation.id.actor == "a"));
}

#[test]
fn retained_checkpoints_can_span_multiple_wire_batches() {
    let mut snapshot = replica("author").snapshot();
    let payload = "x".repeat(4 * 1024 * 1024);
    for sequence in 1..=9 {
        snapshot.operations.push(Operation {
            id: OperationId {
                actor: "author".into(),
                lamport: sequence,
                sequence,
            },
            document_id: snapshot.document_id.clone(),
            label: "Missing-node conflict remains durable".into(),
            edits: vec![SharedEdit::SetAttribute {
                node: SharedNodeId::Baseline(999_999),
                name: "Text".into(),
                value: payload.clone(),
            }],
        });
    }
    let restored = DocumentReplica::from_snapshot("reader", snapshot).unwrap();
    assert_eq!(restored.operations().count(), 9);
    assert_eq!(restored.conflicts().len(), 9);
    assert!(restored
        .changes_since(&VersionVector::new())
        .to_json()
        .is_err());
    let mut peer = replica("peer");
    let page = restored
        .changes_since_bounded(&peer.version(), None, MAX_MESSAGE_BYTES)
        .unwrap();
    assert!(page.operations.len() < 9);
    peer.merge(page).unwrap();
    peer.merge(
        restored
            .changes_since_bounded(&peer.version(), None, MAX_MESSAGE_BYTES)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(peer.version(), restored.version());
    assert_eq!(peer.conflicts(), restored.conflicts());
    let mut snapshot_peer = replica("snapshot-peer");
    snapshot_peer.merge_snapshot(restored.snapshot()).unwrap();
    assert_eq!(snapshot_peer.version(), restored.version());
    let mut another = restored.snapshot();
    for operation in &mut another.operations {
        operation.id.actor = "another".into();
    }
    let previous = snapshot_peer.version();
    assert!(
        snapshot_peer.merge_snapshot(another).is_err(),
        "two individually valid checkpoints must still obey the combined retained-log limit"
    );
    assert_eq!(snapshot_peer.version(), previous);
    assert_eq!(snapshot_peer.operations().count(), 9);
}

#[test]
fn presence_validates_node_identities_and_bounds_retained_actors() {
    let mut peer = replica("peer");
    let mut presence = Presence {
        actor: "writer".into(),
        sequence: 1,
        display_name: "Writer".into(),
        selection: vec![SharedNodeId::Baseline(0)],
        cursor: None,
    };
    assert!(peer.receive_presence(presence.clone(), 0).is_err());
    presence.selection.clear();
    for index in 0..MAX_PRESENCE_ACTORS {
        presence.actor = format!("writer-{index}");
        peer.receive_presence(presence.clone(), 0).unwrap();
    }
    presence.sequence += 1;
    peer.receive_presence(presence.clone(), 1).unwrap();
    presence.actor = "one-too-many".into();
    assert!(peer.receive_presence(presence.clone(), 1).is_err());
    peer.prune_presence(100, 10);
    assert!(peer.receive_presence(presence, 100).unwrap());
}
