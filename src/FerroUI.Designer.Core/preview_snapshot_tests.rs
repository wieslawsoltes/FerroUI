use crate::{Document, Edit};
use std::sync::Arc;

#[test]
fn preview_snapshot_shares_exact_source_identity_without_authoring_history() {
    let mut document = Document::new("<Button Content='Original'/>").unwrap();
    let root = document.ast().root();
    document
        .apply(
            "Author",
            vec![Edit::SetAttribute {
                node: root,
                name: "Content".into(),
                value: "Authored".into(),
            }],
        )
        .unwrap();
    let mut snapshot = document.snapshot_without_history();
    assert!(Arc::ptr_eq(
        &document.ast_snapshot(),
        &snapshot.ast_snapshot()
    ));
    assert_eq!(snapshot.revision(), document.revision());
    assert_eq!(snapshot.source(), document.source());
    assert!(snapshot.is_dirty());
    assert!(!snapshot.can_undo());
    assert!(!snapshot.can_redo());
    assert!(document.can_undo());
    snapshot
        .apply(
            "Independent",
            vec![Edit::SetAttribute {
                node: root,
                name: "Content".into(),
                value: "Preview".into(),
            }],
        )
        .unwrap();
    assert!(snapshot.can_undo());
    assert_eq!(
        document.ast().node(root).unwrap().attribute("Content"),
        Some("Authored")
    );
    document.undo().unwrap();
    assert_eq!(document.source(), "<Button Content='Original'/>");
    assert_eq!(
        snapshot.ast().node(root).unwrap().attribute("Content"),
        Some("Preview")
    );
}

#[test]
fn invalid_preview_snapshot_keeps_draft_diagnostics_and_last_valid_identity() {
    let mut document = Document::new("<Grid/>").unwrap();
    document.update_source("Draft", "<Grid>");
    let snapshot = document.snapshot_without_history();
    assert!(!snapshot.is_valid());
    assert_eq!(snapshot.source(), "<Grid>");
    assert_eq!(snapshot.valid_source(), "<Grid/>");
    assert_eq!(snapshot.diagnostics(), document.diagnostics());
    assert_eq!(snapshot.revision(), document.revision());
    assert!(Arc::ptr_eq(
        &snapshot.ast_snapshot(),
        &document.ast_snapshot()
    ));
    assert!(!snapshot.can_undo());
    assert!(document.can_undo());
}

#[test]
fn preview_snapshot_preserves_saved_state_and_history_policy() {
    let mut document = Document::new("<Grid/>").unwrap();
    document.set_history_limit(0);
    let mut snapshot = document.snapshot_without_history();
    assert!(!snapshot.is_dirty());
    snapshot
        .apply(
            "Local",
            vec![Edit::SetAttribute {
                node: snapshot.ast().root(),
                name: "Width".into(),
                value: "100".into(),
            }],
        )
        .unwrap();
    assert!(snapshot.is_dirty());
    assert!(!snapshot.can_undo());
    assert!(!document.is_dirty());
}
