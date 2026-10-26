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
