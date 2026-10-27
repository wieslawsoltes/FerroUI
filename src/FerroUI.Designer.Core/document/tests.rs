use super::*;

fn set(node: NodeId, name: &str, value: &str) -> Edit {
    Edit::SetAttribute {
        node,
        name: name.into(),
        value: value.into(),
    }
}

#[test]
fn exhausted_revision_rejects_writes_and_history_without_any_mutation() {
    let mut document = Document::new("<Grid/>").unwrap();
    let root = document.ast().root();
    document.advance_revision_to(u64::MAX - 1).unwrap();
    document
        .apply("Last edit", vec![set(root, "Width", "1")])
        .unwrap();
    assert_eq!(document.revision(), u64::MAX);
    assert!(document.advance_revision_to(0).is_err());
    let before = document.source().to_owned();
    let next_id = document.next_id;
    let ast = document.current.ast.clone();

    assert!(document
        .apply("Overflow", vec![set(root, "Width", "2")])
        .is_err());
    assert!(document
        .apply(
            "Insert",
            vec![Edit::InsertChild {
                parent: root,
                index: 0,
                source: "<Button/>".into(),
            }]
        )
        .is_err());
    assert!(document
        .update_source_at(u64::MAX, "Source", "<Border/>")
        .is_err());
    assert!(document
        .try_update_source("Invalid source", "<Grid")
        .is_err());
    let rejected = document.update_source("Typing", "<Grid");
    assert!(rejected.change.is_empty());
    assert!(!rejected.valid);
    assert!(rejected.diagnostics[0]
        .message
        .contains("revision exhausted"));
    assert!(document.undo().is_err());
    assert_eq!(document.source(), before);
    assert_eq!(document.next_id, next_id);
    assert!(Arc::ptr_eq(&document.current.ast, &ast));
    assert_eq!(document.undo_label(), Some("Last edit"));
    assert!(!document.can_redo());
    assert!(document.is_valid());
    assert!(document.is_dirty());

    // Reads and idempotent writes remain usable even at the final revision.
    assert!(document
        .apply("Same", vec![set(root, "Width", "1")])
        .unwrap()
        .is_empty());
    assert!(document
        .update_source_at(u64::MAX, "Same source", before)
        .unwrap()
        .change
        .is_empty());
}

#[test]
fn exhausted_revision_does_not_consume_redo() {
    let mut document = Document::new("<Grid/>").unwrap();
    let root = document.ast().root();
    document
        .apply("Width", vec![set(root, "Width", "1")])
        .unwrap();
    document.advance_revision_to(u64::MAX - 1).unwrap();
    document.undo().unwrap();
    assert!(document.redo().is_err());
    assert_eq!(document.source(), "<Grid/>");
    assert_eq!(document.redo_label(), Some("Width"));
    assert!(!document.can_undo());
}

#[test]
fn repeated_source_preserves_syntax_cache_redo_and_invalid_diagnostics() {
    let mut document = Document::new("<Grid><Button/><Button/></Grid>").unwrap();
    let root = document.ast().root();
    document
        .apply("Width", vec![set(root, "Width", "12")])
        .unwrap();
    document.undo().unwrap();
    let ast = document.current.ast.clone();
    let source = document.source().to_owned();
    let update = document.update_source("Echo", source.clone());
    assert!(update.change.is_empty());
    assert!(document.can_redo());
    assert!(Arc::ptr_eq(&ast, &document.current.ast));
    let span = SourceSpan::new(0, source.len());
    assert!(document
        .apply_text_patches(
            document.revision(),
            "Same patch",
            vec![TextPatch {
                span,
                replacement: source.clone(),
            }]
        )
        .unwrap()
        .is_empty());
    assert!(document.can_redo());
    assert!(Arc::ptr_eq(&ast, &document.current.ast));

    document.update_source("Incomplete", "<Grid");
    let diagnostics = document.diagnostics().to_vec();
    let update = document.update_source("Same incomplete source", "<Grid");
    assert!(update.change.is_empty());
    assert!(!update.valid);
    assert_eq!(update.diagnostics, diagnostics);
    assert!(Arc::ptr_eq(&ast, &document.current.ast));
    assert!(document
        .apply("Empty transaction", vec![])
        .unwrap()
        .is_empty());
    document.update_source("Restore", source);
    assert!(document.is_valid());
    assert!(Arc::ptr_eq(&ast, &document.current.ast));
}

#[test]
fn moving_identical_siblings_commits_identity_even_when_source_is_unchanged() {
    let mut document = Document::new("<Grid><Button/><Button/></Grid>").unwrap();
    let root = document.ast().root();
    let children = document.ast().element_children(root);
    let source = document.source().to_owned();
    let change = document
        .apply(
            "Reorder",
            vec![Edit::MoveNode {
                node: children[0],
                parent: root,
                index: 1,
            }],
        )
        .unwrap();
    assert!(!change.is_empty());
    assert!(change.patches.is_empty());
    assert_eq!(document.source(), source);
    assert_eq!(
        document.ast().element_children(root),
        [children[1], children[0]]
    );
    document.undo().unwrap();
    assert_eq!(document.ast().element_children(root), children);
    document.redo().unwrap();
    assert_eq!(
        document.ast().element_children(root),
        [children[1], children[0]]
    );
}

#[test]
fn inherited_attribute_namespace_changes_are_reported_on_the_child() {
    let mut document =
        Document::new("<Grid xmlns:q='urn:first'><Button q:Flag='yes'/></Grid>").unwrap();
    let root = document.ast().root();
    let child = document.ast().element_children(root)[0];
    let change = document
        .apply("Remap namespace", vec![set(root, "xmlns:q", "urn:second")])
        .unwrap();
    assert_eq!(change.changed, [root, child]);
    assert!(change.inserted.is_empty());
    assert!(change.removed.is_empty());
    assert_eq!(
        document
            .ast()
            .node(child)
            .unwrap()
            .attribute_info("q:Flag")
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("urn:second")
    );
}

#[test]
fn mixed_transactions_batch_namespace_attributes_before_the_next_structural_edit() {
    let mut document = Document::new("<Grid><Button/></Grid>").unwrap();
    let root = document.ast().root();
    let child = document.ast().element_children(root)[0];
    document
        .apply(
            "Author",
            vec![
                Edit::InsertChild {
                    parent: root,
                    index: 1,
                    source: "<Border/>".into(),
                },
                set(child, "q:Flag", "value"),
                set(child, "xmlns:q", "urn:custom"),
                Edit::MoveNode {
                    node: child,
                    parent: root,
                    index: 1,
                },
            ],
        )
        .unwrap();
    assert_eq!(document.revision(), 1);
    assert_eq!(document.ast().element_children(root)[1], child);
    assert_eq!(
        document
            .ast()
            .node(child)
            .unwrap()
            .attribute_info("q:Flag")
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("urn:custom")
    );
    document.undo().unwrap();
    assert_eq!(document.source(), "<Grid><Button/></Grid>");
}

#[test]
fn reparenting_preserves_implicit_and_inherited_empty_default_namespaces() {
    for source in [
        "<Grid xmlns:q='urn:custom'><q:Wrapper><Button/></q:Wrapper><Panel xmlns='urn:target'/></Grid>",
        "<Grid xmlns='' xmlns:q='urn:custom'><q:Wrapper><Button/></q:Wrapper><Panel xmlns='urn:target'/></Grid>",
    ] {
        let mut document = Document::new(source).unwrap();
        let root = document.ast().root();
        let top = document.ast().element_children(root);
        let child = document.ast().element_children(top[0])[0];
        document.apply("Reparent", vec![Edit::MoveNode { node: top[0], parent: top[1], index: 0 }]).unwrap();
        assert_eq!(document.ast().node(top[0]).unwrap().namespace_uri.as_deref(), Some("urn:custom"));
        assert_eq!(document.ast().node(child).unwrap().namespace_uri, None);
        assert_eq!(document.ast().node(top[0]).unwrap().attributes.iter().filter(|attribute| attribute.name == "xmlns").count(), 1);
        assert_eq!(document.ast().node(child).unwrap().parent, Some(top[0]));
        document.undo().unwrap();
        assert_eq!(document.source(), source);
    }
}

#[test]
fn dropping_lexical_noops_does_not_hide_overlapping_or_invalid_patches() {
    let mut document = Document::new("<Button Text='🦀'/>").unwrap();
    let source = document.source().to_owned();
    assert!(document
        .apply_text_patches(
            0,
            "Overlap",
            vec![
                TextPatch {
                    span: SourceSpan::new(0, source.len()),
                    replacement: source.clone()
                },
                TextPatch {
                    span: SourceSpan::new(1, 2),
                    replacement: "B".into()
                },
            ]
        )
        .is_err());
    let offset = source.find('🦀').unwrap() + 1;
    assert!(document
        .apply_text_patches(
            0,
            "Invalid",
            vec![TextPatch {
                span: SourceSpan::new(offset, offset),
                replacement: String::new()
            },]
        )
        .is_err());
    assert_eq!(document.source(), source);
    assert_eq!(document.revision(), 0);
}

#[test]
fn child_fragments_cannot_escape_the_requested_parent() {
    let mut document = Document::new("<Grid><Panel/></Grid>").unwrap();
    let parent = document.ast().element_children(document.ast().root())[0];
    assert!(document
        .apply(
            "Escape",
            vec![Edit::InsertChild {
                parent,
                index: 0,
                source: "</Panel><Button/><Panel>".into(),
            }]
        )
        .is_err());
    assert_eq!(document.source(), "<Grid><Panel/></Grid>");
    assert_eq!(document.revision(), 0);
    assert!(!document.can_undo());
    assert!(document
        .apply(
            "Empty insert",
            vec![Edit::InsertChild {
                parent,
                index: 0,
                source: String::new(),
            }]
        )
        .unwrap()
        .is_empty());
}

#[test]
fn child_fragments_accept_multiple_nodes_trivia_and_inherited_namespaces() {
    let mut document = Document::new("<Grid xmlns:q='urn:custom'><Panel/></Grid>").unwrap();
    let parent = document.ast().element_children(document.ast().root())[0];
    let fragment = "<!--preserve--><q:Button/><![CDATA[text]]><q:Label/>";
    document
        .apply(
            "Insert children",
            vec![Edit::InsertChild {
                parent,
                index: 0,
                source: fragment.into(),
            }],
        )
        .unwrap();
    assert_eq!(document.ast().element_children(parent).len(), 2);
    assert!(document.source().contains(fragment));
    assert!(document
        .ast()
        .element_children(parent)
        .iter()
        .all(
            |id| document.ast().node(*id).unwrap().namespace_uri.as_deref() == Some("urn:custom")
        ));
}
