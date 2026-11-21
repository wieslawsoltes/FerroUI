use crate::{project_rust, RsxWrap};
use ferroui_designer_core::{Document, Edit, NodeId, TextPatch};

#[test]
fn an_xaml_value_edit_keeps_the_rust_collection_and_other_invocations() {
    let source = r###"fn other() { rsx! { Button { content("Elsewhere") } }; }
fn view() {
    rsx! { StackPanel {
        name("Dashboard"),
        children: [
            ::ferroui_view::rsx! { TextBlock { text(/* keep 🦀 */ "Daily results") } },
            (rsx! { Button { name("Accept"), content("Open") } }),
        ],
    }};
}"###
        .replace('\n', "\r\n");
    let caret = source.find("StackPanel").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    assert_eq!(projection.omitted_count, 0);
    assert_eq!(projection.nodes.len(), 3);
    assert_eq!(projection.blocks.len(), 4);
    assert!(projection.nodes[0].editable_children());
    assert!(!projection.can_replace_structure());
    let xaml = projection.xaml();
    let draft = xaml.replace("Content=\"Open\"", "Content=\"Inspect\"");
    let patches = projection.apply_xaml(&draft).unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("content(\"Open\")", "content(\"Inspect\")"),
    );
    assert!(projection
        .apply_xaml(&xaml.replace("<Button ", "<ToggleButton "))
        .is_err());
}

#[test]
fn adding_and_resetting_counterpart_properties_preserves_the_rust_collection() {
    let text = r#"fn other() { rsx! { Button { content("Unrelated") } }; }
fn view() { rsx! { StackPanel {
    children([
        rsx! { Button { width(/* wide variant 🧭 */ 80.0), content("Inspect") } },
    ]),
} }; }
"#
    .replace('\n', "\r\n");
    let caret = text.find("StackPanel").unwrap();
    let projection = project_rust(&text, caret).unwrap();
    let draft = projection
        .xaml()
        .replace("Width=\"80\"", "MinWidth=\"56\" MaxWidth=\"160\"");
    let patches = projection.apply_xaml(&draft).unwrap();
    assert_eq!(patches.len(), 2);
    let updated = TextPatch::apply_all(&text, patches).unwrap();
    assert_eq!(updated.lines().next(), text.lines().next());
    assert!(updated.contains("children(["));
    assert!(updated.contains("/* wide variant 🧭 */"));
    assert!(updated.contains("content(\"Inspect\")"));
    assert!(!updated.contains("width("));
    assert!(project_rust(&updated, caret)
        .unwrap()
        .matches_xaml(&draft)
        .unwrap());
}

#[test]
fn outline_move_keeps_an_opaque_array_sibling_and_moves_the_original_expression() {
    let source = r###"rsx! { StackPanel {
    children([
        rsx! { TextBlock { name("Heading"), text("Activity") } },
        loaded_row(/* dynamic 🦀 */ runtime()),
        rsx! { TextBlock { name("Month"), text("November") } },
        ::ferroui_view::rsx! { Button { name("Details"), width(80.0), /* keep */ content("Details") } }
    ]),
} }"###.replace('\n', "\r\n");
    let projection = project_rust(&source, 0).unwrap();
    assert_eq!(projection.omitted_count, 1);
    let original = Document::new(projection.xaml()).unwrap();
    let heading = original.ast().find_name("Heading").unwrap();
    let details = original.ast().find_name("Details").unwrap();
    let edits = [
        Edit::SetAttribute {
            node: details,
            name: "Width".into(),
            value: "120".into(),
        },
        Edit::MoveNode {
            node: details,
            parent: original.ast().root(),
            index: 1,
        },
    ];
    let mut draft = original.snapshot_without_history();
    draft
        .apply("Move details after heading", edits.to_vec())
        .unwrap();
    let plan = projection
        .plan_child_edit(&original, &draft, &edits, Some(heading))
        .unwrap();
    let edited = TextPatch::apply_all(&source, plan.patches).unwrap();
    assert_eq!(&edited[plan.controls[&details].range()], "Button");
    assert!(edited.contains("::ferroui_view::rsx! { Button { name(\"Details\"), width(120.0), /* keep */ content(\"Details\") } }"));
    assert!(edited.find("name(\"Details\")") < edited.find("loaded_row("));
    assert_eq!(
        edited
            .matches("loaded_row(/* dynamic 🦀 */ runtime())")
            .count(),
        1
    );
    assert_eq!(edited.matches('\n').count(), edited.matches("\r\n").count());
}

#[test]
fn reparenting_the_last_collection_member_keeps_its_scope_and_comments() {
    let source = r###"rsx! { StackPanel {
    StackPanel {
        name("Old"),
        children(/* list */ [
            ::ferroui_view::rsx! { Button { name("Open"), /* button */ content("Open") } }
        ]) /* keep next to the panel */,
    }
    StackPanel { name("New") }
} }"###;
    let projection = project_rust(source, 0).unwrap();
    let original = Document::new(projection.xaml()).unwrap();
    let button = original.ast().find_name("Open").unwrap();
    let new = original.ast().find_name("New").unwrap();
    let edited = reparent(source, button, new);
    let updated = project_rust(&edited, 0).unwrap();
    let xaml = Document::new(updated.xaml()).unwrap();
    assert_eq!(
        xaml.ast()
            .node(xaml.ast().find_name("Open").unwrap())
            .unwrap()
            .parent,
        xaml.ast().find_name("New")
    );
    assert!(!edited.contains("children("));
    assert!(edited.contains("{ ::ferroui_view::rsx! { Button"));
    for comment in ["/* list */", "/* button */", "/* keep next to the panel */"] {
        assert_eq!(edited.matches(comment).count(), 1);
    }
}

#[test]
fn removing_an_inline_subtree_keeps_opaque_members_and_unrelated_rust() {
    let text = r#"fn other() { rsx! { Button { content("Unrelated") } }; }
fn view() { rsx! { StackPanel {
    children: [
        rows(/* keep the computed entry 🧭 */ data()),
        rsx! { Border { name("Card"), TextBlock { text(model.title()) } } },
        rsx! { Button { name("Next"), content("Next") } },
    ],
} }; }
"#
    .replace('\n', "\r\n");
    let caret = text.find("StackPanel").unwrap();
    let projection = project_rust(&text, caret).unwrap();
    let current = Document::new(projection.xaml()).unwrap();
    let edits = [Edit::RemoveNode {
        node: current.ast().find_name("Card").unwrap(),
    }];
    let mut draft = current.snapshot_without_history();
    draft.apply("Remove card", edits.to_vec()).unwrap();
    let plan = projection
        .plan_child_edit(&current, &draft, &edits, None)
        .unwrap();
    let edited = TextPatch::apply_all(&text, plan.patches).unwrap();
    assert_eq!(
        edited,
        text.replace(
            "        rsx! { Border { name(\"Card\"), TextBlock { text(model.title()) } } },\r\n",
            ""
        ),
    );
    let updated = project_rust(&edited, caret).unwrap();
    assert_eq!(updated.omitted_count, 1);
    assert!(updated.nodes[0].editable_children());
}

#[test]
fn deleting_the_last_authored_collection_entries_clears_the_empty_assignment() {
    let source = r#"rsx! { StackPanel {
    children(/* keep the list note */ ::std::vec![
        rsx! { Button { name("First") } },
        rsx! { Button { name("Second") } },
    ]),
} }"#;
    let projection = project_rust(source, 0).unwrap();
    let current = Document::new(projection.xaml()).unwrap();
    let edits = ["First", "Second"].map(|name| Edit::RemoveNode {
        node: current.ast().find_name(name).unwrap(),
    });
    let mut draft = current.snapshot_without_history();
    draft.apply("Delete selection", edits.to_vec()).unwrap();
    let plan = projection
        .plan_child_edit(&current, &draft, &edits, None)
        .unwrap();
    assert_eq!(plan.patches.len(), 1);
    let edited = TextPatch::apply_all(source, plan.patches).unwrap();
    assert!(!edited.contains("children("));
    assert_eq!(edited.matches("/* keep the list note */").count(), 1);
    assert!(project_rust(&edited, 0).unwrap().nodes[0].editable_children());
}

#[test]
fn deleting_a_collection_member_preserves_the_surviving_state_targets() {
    let source = r#"rsx! { StackPanel {
        visual_states: [ferroui_view::visual_state_group("Review")
            .state(ferroui_view::visual_state("Ready")
                .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 120.0))],
        children: [rsx! { Border { name("Card") } }],
    } }"#;
    let projection = project_rust(source, 0).unwrap();
    assert_eq!(projection.omitted_count, 0);
    let current = Document::new(projection.xaml()).unwrap();
    let edits = [Edit::RemoveNode {
        node: current.ast().find_name("Card").unwrap(),
    }];
    let mut draft = current.snapshot_without_history();
    draft.apply("Remove card", edits.to_vec()).unwrap();
    assert!(projection
        .plan_child_edit(&current, &draft, &edits, None)
        .is_err());
}

#[test]
fn a_collection_member_name_rename_reaches_panel_styles_and_rejects_capture() {
    let source = r##"rsx! { StackPanel {
        styles: [ferroui_view::style("#Open")],
        children: [
            rsx! { Button { name("Open"), content("Inspect") } },
            rsx! { TextBlock { name("Heading"), text("Review") } },
        ],
    } }"##;
    let projection = project_rust(source, 0).unwrap();
    assert!(projection
        .edit_control_names(&[1], Some("Heading"))
        .is_err());
    let edits = projection
        .edit_control_names(&[1], Some("Details"))
        .unwrap();
    let edited = TextPatch::apply_all(source, edits).unwrap();
    assert_eq!(
        edited,
        source
            .replace("#Open", "#Details")
            .replace("name(\"Open\")", "name(\"Details\")"),
    );
}

#[test]
fn appending_to_a_vector_adds_a_separator_and_retains_the_last_comment() {
    let source = r###"rsx! { StackPanel {
        children(::std::vec![
            rsx! { TextBlock { text("Revenue") } } // previous result 🧪
        ]),
    } }"###;
    let projection = project_rust(source, 0).unwrap();
    let edit = projection
        .insert_child(0, None, "<Button Content=\"New\"/>")
        .unwrap();
    let edited = TextPatch::apply_all(source, vec![edit]).unwrap();
    assert!(edited.contains("} }, // previous result 🧪"));
    assert!(edited.contains("children(::std::vec!["));
    assert!(edited.contains("::ferroui_view::rsx! { Button"));
    let updated = project_rust(&edited, 0).unwrap();
    assert_eq!(updated.omitted_count, 0);
    assert_eq!(updated.nodes[0].child_indices, [1, 2]);
}

#[test]
fn wrapping_a_collection_run_keeps_original_rsx_expressions() {
    let source = r###"rsx! { StackPanel {
        children: [
            rsx! { TextBlock { name("Title"), text("Revenue") } },
            // Move with the authoring run 🧭
            rsx! { Button { name("Open"), content("Inspect") } },
        ],
    } }"###
        .replace('\n', "\r\n");
    let projection = project_rust(&source, 0).unwrap();
    let change = projection
        .wrap_children(RsxWrap {
            parent: 0,
            children: &[1, 2],
            fragment: "<StackPanel Spacing=\"9\"/>",
            guarded_properties: &[],
            attributes: &[],
        })
        .unwrap();
    let edited = TextPatch::apply_all(&source, change.patches).unwrap();
    let updated = project_rust(&edited, 0).unwrap();
    assert_eq!(updated.nodes[0].child_indices, [1]);
    assert_eq!(updated.nodes[1].child_indices, [2, 3]);
    assert_eq!(&edited[change.wrapper.range()], "StackPanel");
    assert_eq!(
        edited.matches("// Move with the authoring run 🧭").count(),
        1
    );
    assert!(edited.contains("rsx! { Button { name(\"Open\"), content(\"Inspect\") } }"));
    assert_eq!(edited.matches('\n').count(), edited.matches("\r\n").count());
}

#[test]
fn an_untyped_or_competing_collection_keeps_its_drop_target_read_only() {
    for fields in [
        "children([runtime()])",
        "children(make_children())",
        "children([rsx! { Button {} }]), child(rsx! { TextBlock {} })",
        "children([rsx! { Button {} }]), TextBlock {}",
    ] {
        let source = format!("rsx! {{ StackPanel {{ {fields} }} }}");
        let projection = project_rust(&source, 0).unwrap();
        assert!(!projection.nodes[0].editable_children());
        assert!(projection.insert_child(0, None, "<Button/>").is_err());
    }
}

fn reparent(source: &str, child: NodeId, parent: NodeId) -> String {
    let projection = project_rust(source, 0).unwrap();
    let original = Document::new(projection.xaml()).unwrap();
    let edit = Edit::MoveNode {
        node: child,
        parent,
        index: 0,
    };
    let mut draft = original.snapshot_without_history();
    draft.apply("Reparent control", vec![edit.clone()]).unwrap();
    let plan = projection
        .plan_child_edit(&original, &draft, &[edit], None)
        .unwrap();
    TextPatch::apply_all(source, plan.patches).unwrap()
}
