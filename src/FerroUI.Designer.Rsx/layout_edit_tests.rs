use crate::project_rust;
use ferroui_designer_core::{Document, Edit, TextPatch};

#[test]
fn mixed_child_placement_moves_original_tokens_at_an_outline_boundary() {
    let source = r##"fn first() { rsx! { Button { content: "Another view" } }; }
fn view() {
    rsx! { StackPanel {
        TextBlock { name: "Anchor", text: "Start 🧭" }
        for item in items() { TextBlock { text: item.title } }
        TextBlock { name: "Divider", text: model() }
        { more_rows() }
        Button { name: "Moved", width: 96.0, /* keep 🦀 */ on_click: save() }
        if loading() { TextBlock { text: "Loading" } }
        Border { background: "#1a2436" }
    } };
}
"##
    .replace('\n', "\r\n");
    let offset = source.find("StackPanel").unwrap();
    let rsx = project_rust(&source, offset).unwrap();
    let original = Document::new(rsx.xaml()).unwrap();
    let root = original.ast().root();
    let anchor = original.ast().find_name("Anchor").unwrap();
    let moved = original.ast().find_name("Moved").unwrap();
    let border = *original.ast().element_children(root).last().unwrap();
    let edits = [
        Edit::SetAttribute {
            node: moved,
            name: "Width".into(),
            value: "128".into(),
        },
        Edit::MoveNode {
            node: moved,
            parent: root,
            index: 1,
        },
        Edit::MoveNode {
            node: border,
            parent: root,
            index: 2,
        },
    ];
    let mut draft = original.snapshot_without_history();
    draft
        .apply("Move selected controls", edits.to_vec())
        .unwrap();
    let plan = rsx
        .plan_child_edit(&original, &draft, &edits, Some(anchor))
        .unwrap();
    let result = TextPatch::apply_all(&source, plan.patches).unwrap();
    assert_eq!(&result[plan.controls[&moved].range()], "Button");
    assert_eq!(&result[plan.controls[&border].range()], "Border");
    let start = result.find("name: \"Anchor\"").unwrap();
    let button = result.find("name: \"Moved\"").unwrap();
    let border = result.find("Border {").unwrap();
    let loop_start = result.find("for item in items()").unwrap();
    let computed = result.find("{ more_rows() }").unwrap();
    let condition = result.find("if loading()").unwrap();
    assert!(start < button && button < border && border < loop_start);
    assert!(loop_start < computed && computed < condition);
    assert!(result.contains("width: 128.0, /* keep 🦀 */ on_click: save()"));
    assert!(result.contains("text: model()"));
    assert_eq!(result.lines().next(), source.lines().next());
    assert_eq!(result.matches('\n').count(), result.matches("\r\n").count());
    assert!(result.ends_with("    } };\r\n}\r\n"));
}

#[test]
fn placement_keeps_native_programs_and_rejects_computed_content_slots() {
    let source = r#"rsx! { StackPanel {
        visual_states: [ferroui_view::visual_state_group("Look")
            .state(ferroui_view::visual_state("Ready")
                 .set(ferroui_base::Visual::opacity_property(), 0.6))],
        Border { name: "Computed", { body() } }
        if visible() { Button {} }
        Border { name: "Empty" }
    } }"#
        .replace('\n', "\r\n");
    let rsx = project_rust(&source, 0).unwrap();
    assert!(rsx.has_projected_states());
    let original = Document::new(rsx.xaml()).unwrap();
    let insert = |parent_name| {
        let parent = original.ast().find_name(parent_name).unwrap();
        let edits = [Edit::InsertChild {
            parent,
            index: 0,
            source: r#"<TextBlock Name="NewRow" Text="New row"/>"#.into(),
        }];
        let mut draft = original.snapshot_without_history();
        draft.apply("Insert", edits.to_vec()).unwrap();
        (
            rsx.plan_child_edit(&original, &draft, &edits, None),
            draft.ast().find_name("NewRow").unwrap(),
        )
    };
    assert!(insert("Computed").0.is_err());
    let (plan, inserted) = insert("Empty");
    let plan = plan.unwrap();
    let result = TextPatch::apply_all(&source, plan.patches).unwrap();
    assert!(result.contains("if visible() { Button {} }"));
    assert!(result.contains("{ body() }"));
    assert!(result.contains(".set(ferroui_base::Visual::opacity_property(), 0.6)"));
    assert_eq!(result.matches('\n').count(), result.matches("\r\n").count());
    assert!(project_rust(&result, 0).unwrap().has_projected_states());
    assert_eq!(&result[plan.controls[&inserted].range()], "TextBlock");
}
