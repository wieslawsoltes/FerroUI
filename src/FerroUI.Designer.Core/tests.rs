use crate::*;

const SAMPLE: &str = "<?xml version='1.0'?>\n<!-- keep this -->\n<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Grid.Resources><SolidColorBrush x:Key='Accent'>#ff0088</SolidColorBrush></Grid.Resources>\n  <Button x:Name='first' Content='{Binding Name, Converter={StaticResource ValueConverter}}' Width = '120'/>\n  <TextBlock x:Name='second'><![CDATA[2 < 3 & café]]></TextBlock>\n</Grid>\n";
fn named(doc: &Document, name: &str) -> NodeId {
    doc.ast().find_name(name).unwrap()
}
fn set(node: NodeId, name: &str, value: &str) -> Edit {
    Edit::SetAttribute {
        node,
        name: name.into(),
        value: value.into(),
    }
}

#[test]
fn round_trip_keeps_every_source_byte() {
    let doc = Document::new(SAMPLE).unwrap();
    assert_eq!(doc.source(), SAMPLE);
    assert_eq!(doc.valid_source(), SAMPLE);
    assert!(doc.ast().nodes().any(|n| n.kind == NodeKind::Comment));
    assert!(doc
        .ast()
        .nodes()
        .any(|n| n.kind == NodeKind::CData && n.text.as_deref() == Some("2 < 3 & café")));
    assert!(doc
        .ast()
        .nodes()
        .any(|n| n.kind == NodeKind::ProcessingInstruction));
    assert!(doc
        .ast()
        .elements()
        .any(|n| n.kind == NodeKind::PropertyElement && n.name == "Grid.Resources"));
    for node in doc.ast().nodes() {
        assert!(doc.source().is_char_boundary(node.span.start));
        assert!(doc.source().is_char_boundary(node.span.end));
    }
}

#[test]
fn attribute_edit_preserves_quotes_spacing_namespaces_and_identity() {
    let mut doc = Document::new(SAMPLE).unwrap();
    let first = named(&doc, "first");
    let second = named(&doc, "second");
    let change = doc
        .apply("Resize", vec![set(first, "Width", "250")])
        .unwrap();
    assert_eq!(
        doc.source(),
        SAMPLE.replace("Width = '120'", "Width = '250'")
    );
    assert_eq!(named(&doc, "first"), first);
    assert_eq!(named(&doc, "second"), second);
    assert_eq!(change.changed, vec![first]);
    assert!(change.inserted.is_empty());
    assert!(change.removed.is_empty());
    let patch = &change.patches[0];
    let mut patched = SAMPLE.to_string();
    patched.replace_range(patch.span.range(), &patch.replacement);
    assert_eq!(patched, doc.source());
}

#[test]
fn xml_escaping_round_trips_unicode_quotes_and_whitespace() {
    let mut doc = Document::new("<Button Text='initial'/>").unwrap();
    let root = doc.ast().root();
    let value = "Zażółć 🦀 < & > ' \"\n\t\r";
    doc.apply("Edit text", vec![set(root, "Text", value)])
        .unwrap();
    assert_eq!(doc.ast().node(root).unwrap().attribute("Text"), Some(value));
    assert!(doc.source().contains("&apos;"));
    assert!(doc.source().contains("&#10;"));
}

#[test]
fn equivalent_value_is_noop_even_with_noncanonical_entity_spelling() {
    let mut doc = Document::new("<TextBlock Text='&#x41; &amp; B'/>").unwrap();
    let root = doc.ast().root();
    let before = doc.source().to_owned();
    let change = doc
        .apply("Same value", vec![set(root, "Text", "A & B")])
        .unwrap();
    assert!(change.is_empty());
    assert_eq!(doc.source(), before);
    assert!(!doc.can_undo());
}

#[test]
fn properties_and_attached_properties_add_remove_without_reserializing() {
    let mut doc = Document::new("<Grid>\n  <Button Content=\"Hello\"/>\n</Grid>").unwrap();
    let id = doc.ast().element_children(doc.ast().root())[0];
    doc.apply(
        "Place",
        vec![set(id, "Grid.Row", "2"), set(id, "Margin", "8,4")],
    )
    .unwrap();
    assert!(doc
        .source()
        .contains("Content=\"Hello\" Grid.Row=\"2\" Margin=\"8,4\""));
    doc.apply(
        "Clear",
        vec![Edit::RemoveAttribute {
            node: id,
            name: "Grid.Row".into(),
        }],
    )
    .unwrap();
    assert_eq!(doc.ast().node(id).unwrap().attribute("Grid.Row"), None);
    assert_eq!(doc.ast().node(id).unwrap().attribute("Margin"), Some("8,4"));
}

#[test]
fn failed_batch_rolls_back_source_identity_revision_and_history() {
    let mut doc = Document::new(SAMPLE).unwrap();
    let id = named(&doc, "first");
    let old = doc.ast().clone();
    let result = doc.apply(
        "Atomic",
        vec![set(id, "Width", "999"), set(999_999, "Width", "1")],
    );
    assert!(matches!(result, Err(DesignerError::UnknownNode(999_999))));
    assert_eq!(doc.source(), SAMPLE);
    assert_eq!(doc.ast(), &old);
    assert_eq!(doc.revision(), 0);
    assert!(!doc.can_undo());
}

#[test]
fn stale_revision_cannot_edit_or_replace_source() {
    let mut doc = Document::new("<Grid/>").unwrap();
    let root = doc.ast().root();
    doc.apply_at(0, "First", vec![set(root, "Width", "100")])
        .unwrap();
    assert!(matches!(
        doc.apply_at(0, "Stale", vec![set(root, "Width", "200")]),
        Err(DesignerError::RevisionConflict {
            expected: 0,
            actual: 1
        })
    ));
    assert!(doc.update_source_at(0, "Stale text", "<Border/>").is_err());
    assert_eq!(
        doc.ast().node(root).unwrap().attribute("Width"),
        Some("100")
    );
}

#[test]
fn invalid_source_is_retained_without_discarding_valid_tree() {
    let mut doc = Document::new(SAMPLE).unwrap();
    let first = named(&doc, "first");
    let old = doc.ast().clone();
    let update = doc.update_source("Typing", "<Grid><Button");
    assert!(!update.valid);
    assert!(!update.diagnostics.is_empty());
    assert_eq!(doc.source(), "<Grid><Button");
    assert_eq!(doc.valid_source(), SAMPLE);
    assert_eq!(doc.ast(), &old);
    assert_eq!(
        doc.apply("Visual edit", vec![set(first, "Width", "8")]),
        Err(DesignerError::InvalidDocument)
    );
    let update = doc.update_source("Repair", SAMPLE.replace("Width = '120'", "Width = '180'"));
    assert!(update.valid);
    assert_eq!(named(&doc, "first"), first);
    assert!(doc.diagnostics().is_empty());
}

#[test]
fn invalid_buffer_can_be_replaced_transactionally() {
    let mut doc = Document::new("<Grid/>").unwrap();
    doc.update_source("Typing", "<Grid");
    doc.apply(
        "Replace",
        vec![Edit::ReplaceSource {
            source: "<Grid Width='100'/>".into(),
        }],
    )
    .unwrap();
    assert!(doc.is_valid());
}

#[test]
fn undo_redo_restores_exact_sources_ids_and_invalid_states() {
    let mut doc = Document::new(SAMPLE).unwrap();
    let first = named(&doc, "first");
    doc.apply("Resize", vec![set(first, "Width", "160")])
        .unwrap();
    let edited = doc.source().to_owned();
    doc.update_source("Typing", "<Grid");
    assert_eq!(doc.revision(), 2);
    doc.undo().unwrap();
    assert_eq!(doc.source(), edited);
    assert!(doc.is_valid());
    doc.undo().unwrap();
    assert_eq!(doc.source(), SAMPLE);
    assert_eq!(named(&doc, "first"), first);
    assert_eq!(doc.revision(), 4);
    doc.redo().unwrap();
    assert_eq!(doc.source(), edited);
    doc.redo().unwrap();
    assert_eq!(doc.source(), "<Grid");
    assert!(!doc.is_valid());
    assert_eq!(named(&doc, "first"), first);
}

#[test]
fn editing_after_undo_discards_redo_and_never_reuses_published_ids() {
    let mut doc = Document::new("<Grid/>").unwrap();
    let root = doc.ast().root();
    doc.apply(
        "Insert",
        vec![Edit::InsertChild {
            parent: root,
            index: 0,
            source: "<Button/>".into(),
        }],
    )
    .unwrap();
    let old = doc.ast().element_children(root)[0];
    doc.undo().unwrap();
    doc.apply(
        "Insert another",
        vec![Edit::InsertChild {
            parent: root,
            index: 0,
            source: "<TextBlock/>".into(),
        }],
    )
    .unwrap();
    assert!(!doc.can_redo());
    assert!(doc.ast().element_children(root)[0] > old);
}

#[test]
fn history_limit_and_dirty_tracking_are_independent() {
    let mut doc = Document::new("<Grid/>").unwrap();
    let root = doc.ast().root();
    doc.set_history_limit(1);
    doc.apply("One", vec![set(root, "Width", "1")]).unwrap();
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
    doc.apply("Two", vec![set(root, "Width", "2")]).unwrap();
    doc.undo().unwrap();
    assert!(!doc.is_dirty());
    assert!(!doc.can_undo());
}

#[test]
fn insert_child_expands_self_closing_parent_and_inherits_namespaces() {
    let mut doc = Document::new(
        "<Grid xmlns='urn:ui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' />",
    )
    .unwrap();
    let root = doc.ast().root();
    doc.apply(
        "Add",
        vec![Edit::InsertChild {
            parent: root,
            index: 0,
            source: "<Button x:Name='new'/>".into(),
        }],
    )
    .unwrap();
    let child = named(&doc, "new");
    assert_eq!(doc.ast().node(child).unwrap().parent, Some(root));
    assert_eq!(
        doc.ast().node(child).unwrap().namespace_uri.as_deref(),
        Some("urn:ui")
    );
    assert!(doc.source().ends_with("</Grid>"));
}

#[test]
fn child_indices_ignore_comments_and_whitespace() {
    let mut doc =
        Document::new("<Grid>\n<!-- first --><Button/>\n<!-- last --><TextBlock/>\n</Grid>")
            .unwrap();
    let root = doc.ast().root();
    doc.apply(
        "Insert",
        vec![Edit::InsertChild {
            parent: root,
            index: 1,
            source: "<Border/>".into(),
        }],
    )
    .unwrap();
    let names: Vec<_> = doc
        .ast()
        .element_children(root)
        .iter()
        .map(|id| doc.ast().node(*id).unwrap().name.clone())
        .collect();
    assert_eq!(names, ["Button", "Border", "TextBlock"]);
    assert!(doc.source().contains("<!-- last -->"));
}

#[test]
fn named_node_reconciles_across_source_reordering_and_reparenting() {
    let mut doc=Document::new("<Grid xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Button x:Name='a'/><TextBlock x:Name='b'/></Grid>").unwrap();
    let a = named(&doc, "a");
    let b = named(&doc, "b");
    doc.update_source("Source","<Grid xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border><TextBlock x:Name='b'/></Border><Button x:Name='a' Width='12'/></Grid>");
    assert_eq!(named(&doc, "a"), a);
    assert_eq!(named(&doc, "b"), b);
}

#[test]
fn unnamed_siblings_keep_identity_when_new_child_is_inserted() {
    let mut doc = Document::new("<Grid><Button Width='1'/><Button Width='2'/></Grid>").unwrap();
    let root = doc.ast().root();
    let old = doc.ast().element_children(root);
    doc.apply(
        "Insert",
        vec![Edit::InsertChild {
            parent: root,
            index: 0,
            source: "<Button Width='3'/>".into(),
        }],
    )
    .unwrap();
    assert_eq!(&doc.ast().element_children(root)[1..], old.as_slice());
}

#[test]
fn move_duplicate_unnamed_nodes_keeps_exact_identities() {
    let mut doc = Document::new("<Grid><Button/><Button/><Button/></Grid>").unwrap();
    let root = doc.ast().root();
    let old = doc.ast().element_children(root);
    doc.apply(
        "Move",
        vec![Edit::MoveNode {
            node: old[0],
            parent: root,
            index: 2,
        }],
    )
    .unwrap();
    assert_eq!(
        doc.ast().element_children(root),
        vec![old[1], old[2], old[0]]
    );
    doc.undo().unwrap();
    assert_eq!(doc.ast().element_children(root), old);
}

#[test]
fn move_backward_and_reparent_preserve_subtree_ids() {
    let mut doc =
        Document::new("<Grid><Canvas><Border><Button/></Border></Canvas><StackPanel/></Grid>")
            .unwrap();
    let root = doc.ast().root();
    let top = doc.ast().element_children(root);
    let border = doc.ast().element_children(top[0])[0];
    let button = doc.ast().element_children(border)[0];
    doc.apply(
        "Reparent",
        vec![Edit::MoveNode {
            node: border,
            parent: top[1],
            index: 0,
        }],
    )
    .unwrap();
    assert_eq!(doc.ast().node(border).unwrap().parent, Some(top[1]));
    assert_eq!(doc.ast().node(button).unwrap().parent, Some(border));
    doc.apply(
        "Reorder",
        vec![Edit::MoveNode {
            node: top[1],
            parent: root,
            index: 0,
        }],
    )
    .unwrap();
    assert_eq!(doc.ast().element_children(root), vec![top[1], top[0]]);
}

#[test]
fn moving_across_namespace_scopes_preserves_element_meaning() {
    let mut doc=Document::new("<Grid xmlns='urn:one' xmlns:q='urn:custom'><Canvas><q:Thing><Button/></q:Thing></Canvas><Panel xmlns='urn:two' xmlns:q='urn:other'/></Grid>").unwrap();
    let root = doc.ast().root();
    let top = doc.ast().element_children(root);
    let thing = doc.ast().element_children(top[0])[0];
    let button = doc.ast().element_children(thing)[0];
    doc.apply(
        "Reparent",
        vec![Edit::MoveNode {
            node: thing,
            parent: top[1],
            index: 0,
        }],
    )
    .unwrap();
    assert_eq!(
        doc.ast().node(thing).unwrap().namespace_uri.as_deref(),
        Some("urn:custom")
    );
    assert_eq!(
        doc.ast().node(button).unwrap().namespace_uri.as_deref(),
        Some("urn:one")
    );
}

#[test]
fn moving_into_default_namespace_keeps_unqualified_node_unqualified() {
    let mut doc = Document::new("<Grid><Button/><Panel xmlns='urn:other'/></Grid>").unwrap();
    let root = doc.ast().root();
    let top = doc.ast().element_children(root);
    doc.apply(
        "Reparent",
        vec![Edit::MoveNode {
            node: top[0],
            parent: top[1],
            index: 0,
        }],
    )
    .unwrap();
    assert_eq!(doc.ast().node(top[0]).unwrap().namespace_uri, None);
}

#[test]
fn cycles_root_deletion_and_invalid_child_indices_are_rejected() {
    let mut doc = Document::new("<Grid><Canvas><Button/></Canvas></Grid>").unwrap();
    let root = doc.ast().root();
    let canvas = doc.ast().element_children(root)[0];
    let button = doc.ast().element_children(canvas)[0];
    let before = doc.source().to_owned();
    assert!(doc
        .apply(
            "Cycle",
            vec![Edit::MoveNode {
                node: canvas,
                parent: button,
                index: 0
            }]
        )
        .is_err());
    assert!(doc
        .apply("Delete root", vec![Edit::RemoveNode { node: root }])
        .is_err());
    assert!(doc
        .apply(
            "Index",
            vec![Edit::InsertChild {
                parent: canvas,
                index: 5,
                source: "<Border/>".into()
            }]
        )
        .is_err());
    assert_eq!(doc.source(), before);
}

#[test]
fn invalid_insert_and_namespace_removal_are_atomic() {
    let mut doc = Document::new("<Grid xmlns:q='urn:custom'><q:Thing/></Grid>").unwrap();
    let root = doc.ast().root();
    let before = doc.source().to_owned();
    assert!(doc
        .apply(
            "Bad insert",
            vec![Edit::InsertChild {
                parent: root,
                index: 0,
                source: "<Button".into()
            }]
        )
        .is_err());
    assert!(doc
        .apply(
            "Unbind prefix",
            vec![Edit::RemoveAttribute {
                node: root,
                name: "xmlns:q".into()
            }]
        )
        .is_err());
    assert_eq!(doc.source(), before);
    assert_eq!(doc.revision(), 0);
}

#[test]
fn source_patch_rejects_mid_codepoint_and_overlap() {
    let mut doc = Document::new("<TextBlock Text='🦀'/>").unwrap();
    let offset = doc.source().find('🦀').unwrap();
    let before = doc.source().to_owned();
    assert!(matches!(
        doc.apply_text_patches(
            0,
            "Bad",
            vec![TextPatch {
                span: SourceSpan::new(offset + 1, offset + 2),
                replacement: "x".into()
            }]
        ),
        Err(DesignerError::InvalidRange(_))
    ));
    assert!(doc
        .apply_text_patches(
            0,
            "Overlap",
            vec![
                TextPatch {
                    span: SourceSpan::new(offset, offset + 4),
                    replacement: "x".into()
                },
                TextPatch {
                    span: SourceSpan::new(offset, offset + 4),
                    replacement: "y".into()
                }
            ]
        )
        .is_err());
    assert_eq!(doc.source(), before);
}

#[test]
fn simultaneous_text_patches_validate_only_the_completed_candidate() {
    let mut doc = Document::new("<Grid><Button/></Grid>").unwrap();
    doc.apply_text_patches(
        0,
        "Rename root",
        vec![
            TextPatch {
                span: SourceSpan::new(1, 5),
                replacement: "Canvas".into(),
            },
            TextPatch {
                span: SourceSpan::new(17, 21),
                replacement: "Canvas".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(doc.source(), "<Canvas><Button/></Canvas>");
}

#[test]
fn minimal_patch_has_utf8_boundaries_for_multibyte_prefix_and_suffix() {
    let mut doc = Document::new("<TextBlock Text='ż🦀ć'/>").unwrap();
    let before = doc.source().to_owned();
    let id = doc.ast().root();
    let change = doc.apply("Unicode", vec![set(id, "Text", "ź🐙ć")]).unwrap();
    let patch = &change.patches[0];
    assert!(before.is_char_boundary(patch.span.start));
    assert!(before.is_char_boundary(patch.span.end));
    let mut after = before;
    after.replace_range(patch.span.range(), &patch.replacement);
    assert_eq!(after, doc.source());
}

#[test]
fn xml_attribute_injection_and_dtd_entities_are_rejected() {
    let mut doc = Document::new("<Grid/>").unwrap();
    let root = doc.ast().root();
    assert!(doc
        .apply("Inject", vec![set(root, "Width='4' Height", "9")])
        .is_err());
    assert!(Document::new("<!DOCTYPE Grid [<!ENTITY x 'expanded'>]><Grid>&x;</Grid>").is_err());
}

#[test]
fn markup_parser_handles_nested_extensions_quotes_and_literal_escape() {
    let value = MarkupValue::parse(
        "{Binding Path=Name, Converter={StaticResource ValueConverter}, StringFormat='A, B = {0}'}",
    )
    .unwrap();
    let MarkupValue::Extension(binding) = value else {
        panic!("expected extension")
    };
    assert_eq!(binding.name, "Binding");
    assert_eq!(binding.arguments.len(), 3);
    assert!(
        matches!(&binding.arguments[1].value,MarkupValue::Extension(ext) if ext.name=="StaticResource")
    );
    assert_eq!(
        binding.arguments[2].value,
        MarkupValue::Literal("A, B = {0}".into())
    );
    assert_eq!(
        MarkupValue::parse("{}{not an extension}"),
        Ok(MarkupValue::Literal("{not an extension}".into()))
    );
}

#[test]
fn malformed_markup_extensions_report_errors_without_rewriting_xaml() {
    for value in [
        "{Binding",
        "{Binding Path={StaticResource key}",
        "{Binding Path='oops}",
        "{Binding Path=A, Path=B}",
        "{,}",
    ] {
        assert!(MarkupValue::parse(value).is_err(), "{value}");
    }
    let doc = Document::new("<Button Content='{UnknownExtension Path=Name}'/>").unwrap();
    assert_eq!(
        doc.source(),
        "<Button Content='{UnknownExtension Path=Name}'/>"
    );
}

#[test]
fn lexical_attribute_values_normalize_xml_whitespace_but_character_references_do_not() {
    let doc = Document::new("<TextBlock Text='a\r\nb\tc&#10;d'/>").unwrap();
    assert_eq!(
        doc.ast().node(doc.ast().root()).unwrap().attribute("Text"),
        Some("a b c\nd")
    );
}

#[test]
fn source_lookup_returns_smallest_node_and_element() {
    let doc = Document::new("<Grid><!--note--><TextBlock>héllo</TextBlock></Grid>").unwrap();
    let at = doc.source().find("héllo").unwrap();
    assert_eq!(
        doc.ast().node(doc.ast().node_at(at).unwrap()).unwrap().kind,
        NodeKind::Text
    );
    assert_eq!(
        doc.ast()
            .node(doc.ast().element_at(at).unwrap())
            .unwrap()
            .name,
        "TextBlock"
    );
}

#[test]
fn namespace_prefix_renaming_preserves_semantic_identity() {
    let mut doc=Document::new("<a:Grid xmlns:a='urn:ui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><a:Button x:Name='action'/></a:Grid>").unwrap();
    let root = doc.ast().root();
    let action = named(&doc, "action");
    let update=doc.update_source("Rename namespace","<u:Grid xmlns:u='urn:ui' xmlns:n='http://schemas.microsoft.com/winfx/2006/xaml'><u:Button n:Name='action'/></u:Grid>");
    assert!(update.valid);
    assert_eq!(doc.ast().root(), root);
    assert_eq!(named(&doc, "action"), action);
}

#[test]
fn namespace_scope_is_restored_after_self_closing_and_nested_elements() {
    let doc=Document::new("<Grid xmlns='urn:one'><Panel xmlns='urn:two'><Button/></Panel><Button/><Panel xmlns='urn:three'/><Button/></Grid>").unwrap();
    let children = doc.ast().element_children(doc.ast().root());
    assert_eq!(
        doc.ast()
            .node(children[1])
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("urn:one")
    );
    assert_eq!(
        doc.ast()
            .node(children[3])
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("urn:one")
    );
}

#[test]
fn large_document_edits_preserve_ids_and_have_bounded_patch_output() {
    let mut source = String::from("<Canvas>");
    for index in 0..10_000 {
        source.push_str(&format!("<Button Width='{index}'/>"));
    }
    source.push_str("</Canvas>");
    let mut doc = Document::new(source).unwrap();
    let root = doc.ast().root();
    let children = doc.ast().element_children(root);
    let id = children[5000];
    let change = doc
        .apply("Resize one", vec![set(id, "Width", "42")])
        .unwrap();
    assert_eq!(doc.ast().element_children(root), children);
    assert_eq!(change.changed, vec![id]);
    assert_eq!(change.patches.len(), 1);
    assert!(change.patches[0].replacement.len() < 8);
    let update = doc.update_source(
        "Source edit",
        doc.source().replace("Width='9999'", "Width='10000'"),
    );
    assert!(update.valid);
    assert_eq!(doc.ast().element_children(root), children);
}

#[test]
fn large_multiselection_tracks_mixed_patch_deltas_and_preserves_syntax_identities() {
    let mut source = String::from("<Canvas><!-- 🦀 layout -->\n");
    for index in 0..4096 {
        source.push_str(&format!(
            "  <Button Width='{index}' Height='100' Tag='{{Binding Résumé}}'/>\n"
        ));
    }
    source.push_str("</Canvas>");
    let mut doc = Document::new(source.clone()).unwrap();
    let root = doc.ast().root();
    let children = doc.ast().element_children(root);
    let syntax_ids: Vec<_> = doc.ast().nodes().map(|node| node.id).collect();
    let mut edits = Vec::new();
    for (index, &id) in children.iter().enumerate() {
        edits.push(set(
            id,
            "Width",
            if index % 2 == 0 { "7" } else { "123456.5" },
        ));
        if index % 3 == 0 {
            edits.push(Edit::RemoveAttribute {
                node: id,
                name: "Height".into(),
            });
        } else {
            edits.push(set(id, "Opacity", "0.75"));
        }
    }
    let change = doc.apply("Resize multiselection", edits).unwrap();
    assert_eq!(doc.revision(), 1);
    assert!(change.inserted.is_empty() && change.removed.is_empty());
    assert_eq!(
        doc.ast().nodes().map(|node| node.id).collect::<Vec<_>>(),
        syntax_ids
    );
    assert_eq!(doc.ast().element_children(root), children);
    assert!(doc.source().contains("<!-- 🦀 layout -->"));
    for (index, &id) in children.iter().enumerate() {
        let node = doc.ast().node(id).unwrap();
        assert_eq!(
            node.attribute("Width"),
            Some(if index % 2 == 0 { "7" } else { "123456.5" })
        );
        assert_eq!(node.attribute("Height"), (index % 3 != 0).then_some("100"));
        assert_eq!(
            node.attribute("Opacity"),
            (index % 3 != 0).then_some("0.75")
        );
        assert_eq!(node.attribute("Tag"), Some("{Binding Résumé}"));
        assert_eq!(node.attribute_info("Width").unwrap().quote, '\'');
    }
    doc.undo().unwrap();
    assert_eq!(doc.source(), source);
    doc.redo().unwrap();
    assert_eq!(
        doc.ast().nodes().map(|node| node.id).collect::<Vec<_>>(),
        syntax_ids
    );
}

#[test]
fn source_patch_pins_respect_empty_insertions_and_adjacent_replacement_boundaries() {
    let mut doc =
        Document::new("<Canvas><Button/><Button/><Button/><Button/><Button/></Canvas>").unwrap();
    let root = doc.ast().root();
    let original = doc.ast().element_children(root);
    let spans: Vec<_> = original
        .iter()
        .map(|id| doc.ast().node(*id).unwrap().span)
        .collect();
    // Deliberately unordered. An insertion and a replacement may share their start, while
    // the preceding deletion ends at exactly that boundary. Unnamed duplicate siblings must
    // still keep the identities of every surviving original node.
    doc.apply_text_patches(
        0,
        "Patch sibling boundaries",
        vec![
            TextPatch {
                span: SourceSpan::new(spans[4].end, spans[4].end),
                replacement: "<Button/>".into(),
            },
            TextPatch {
                span: spans[2],
                replacement: "<Border/>".into(),
            },
            TextPatch {
                span: SourceSpan::new(spans[0].start, spans[0].start),
                replacement: "<Button/>".into(),
            },
            TextPatch {
                span: spans[1],
                replacement: String::new(),
            },
            TextPatch {
                span: SourceSpan::new(spans[2].start, spans[2].start),
                replacement: "<Button/>".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(
        doc.source(),
        "<Canvas><Button/><Button/><Button/><Border/><Button/><Button/><Button/></Canvas>"
    );
    let actual = doc.ast().element_children(root);
    assert_eq!(actual[1], original[0]);
    assert_eq!(actual[4], original[3]);
    assert_eq!(actual[5], original[4]);
    assert_eq!(doc.ast().node(actual[3]).unwrap().name, "Border");
    doc.undo().unwrap();
    assert_eq!(doc.ast().element_children(root), original);
    doc.redo().unwrap();
    assert_eq!(doc.ast().element_children(root), actual);
}

#[test]
fn namespace_and_prefixed_attribute_can_be_added_in_one_atomic_batch() {
    let mut doc = Document::new("<Button/>").unwrap();
    let root = doc.ast().root();
    doc.apply(
        "Name",
        vec![
            set(root, "x:Name", "action"),
            set(
                root,
                "xmlns:x",
                "http://schemas.microsoft.com/winfx/2006/xaml",
            ),
        ],
    )
    .unwrap();
    assert_eq!(named(&doc, "action"), root);
    assert_eq!(doc.revision(), 1);
}

#[test]
fn property_batch_coalesces_repeated_assignments_and_keeps_one_history_item() {
    let mut doc = Document::new("<Grid Width='8'/>").unwrap();
    let root = doc.ast().root();
    doc.apply(
        "Batch",
        vec![
            set(root, "Width", "9"),
            set(root, "Width", "10"),
            set(root, "Height", "5"),
            Edit::RemoveAttribute {
                node: root,
                name: "Height".into(),
            },
            set(root, "Tag", "first"),
            set(root, "Tag", "last"),
        ],
    )
    .unwrap();
    assert_eq!(doc.source(), "<Grid Width='10' Tag='last'/>");
    assert_eq!(doc.revision(), 1);
    doc.undo().unwrap();
    assert_eq!(doc.source(), "<Grid Width='8'/>");
    assert!(!doc.can_undo());
}

#[test]
fn restored_document_revision_rebases_without_losing_history_or_identity() {
    let mut doc = Document::new("<Button/>").unwrap();
    let id = doc.ast().root();
    doc.apply(
        "Width",
        vec![Edit::SetAttribute {
            node: id,
            name: "Width".into(),
            value: "10".into(),
        }],
    )
    .unwrap();
    doc.advance_revision_to(100).unwrap();
    assert_eq!(doc.revision(), 100);
    assert!(doc.is_dirty());
    doc.advance_revision_to(5).unwrap();
    assert_eq!(doc.revision(), 100);
    doc.undo().unwrap();
    assert_eq!(doc.source(), "<Button/>");
    assert_eq!(doc.ast().root(), id);
    assert_eq!(doc.revision(), 101);
    assert!(!doc.is_dirty());
    assert!(doc.advance_revision_to(u64::MAX).is_err());
    assert_eq!(doc.revision(), 101);
}

#[test]
fn name_scopes_distinguish_template_declarations_contents_and_siblings() {
    let document = Document::new("<Grid Name='Root'><ControlTemplate Name='Outer'><Grid Name='Content'><DataTemplate Name='Inner'><Button Name='Nested'/><!--inside--></DataTemplate></Grid></ControlTemplate><DataTemplate Name='Sibling'><Button Name='Other'/></DataTemplate><Button Name='Outside'/></Grid>").unwrap();
    let ast = document.ast();
    let id = |name| ast.find_name(name).unwrap();
    assert_eq!(ast.name_scope(id("Root")), id("Root"));
    assert_eq!(ast.name_scope(id("Outer")), id("Root"));
    assert_eq!(ast.name_scope(id("Content")), id("Outer"));
    assert_eq!(ast.name_scope(id("Inner")), id("Outer"));
    assert_eq!(ast.name_scope(id("Nested")), id("Inner"));
    assert_eq!(ast.name_scope(id("Sibling")), id("Root"));
    assert_eq!(ast.name_scope(id("Other")), id("Sibling"));
    assert_eq!(ast.name_scope(id("Outside")), id("Root"));
    let comment = ast
        .nodes()
        .find(|node| node.kind == crate::NodeKind::Comment)
        .unwrap();
    assert_eq!(ast.name_scope(comment.id), id("Inner"));
    assert_eq!(ast.name_scope(u64::MAX), id("Root"));
}
