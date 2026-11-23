use super::*;
use crate::project_rust;
use ferroui_designer_animation::{BehaviorAction, BehaviorSource};
use ferroui_designer_core::{Document, Edit, TextPatch};

const SOURCE: &str = r###"use ferroui_view::behavior::{event as when, change_property, add_class};
fn title() { rsx! { TextBlock { text: "Keep this view 🌿" } } }
fn page() { rsx! {
    StackPanel {
        Border { name: "Mark" }
        Button {
            name: "Approve",
            behaviors: [
                other_behavior(),
                when("Click", None, [
                    change_property(Some("Mark"), "Opacity", /* keep 🌿 */ r#"0.5"#),
                    add_class(Some("Mark"), "approved"),
                ]),
            ],
        }
    }
} }
"###;

#[test]
fn maps_native_behavior_and_actions_to_their_imported_source() {
    let source = SOURCE.replace('\n', "\r\n");
    let projection = project_rust(&source, source.rfind("when(").unwrap()).unwrap();
    let (markup, map) = projection.mapped_markup();
    let document = Document::new(markup).unwrap();
    let button = document.ast().find_name("Approve").unwrap();
    let entries = BehaviorSource::read(&document, button).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].actions.len(), 2);
    assert!(
        matches!(&entries[0].actions[0].value, BehaviorAction::ChangeProperty { target: Some(value), .. }
        if value == "{Binding #Mark}")
    );
    let action = document.ast().node(entries[0].actions[0].node).unwrap();
    let span = map.behavior_source(action.span.start).unwrap();
    assert_eq!(
        &source[span.range()],
        "change_property(Some(\"Mark\"), \"Opacity\", /* keep 🌿 */ r#\"0.5\"#)"
    );
    assert_eq!(projection.omitted_count, 1);
    assert!(projection.can_attach_behavior(2));
}

#[test]
fn edits_a_literal_and_reorders_actions_without_replacing_the_behavior() {
    let source = SOURCE.replace('\n', "\r\n");
    let caret = source.find("fn page").unwrap() + "fn page() { rsx! ".len();
    let projection = project_rust(&source, caret).unwrap();
    let patches = projection
        .edit_behavior(
            2,
            RsxBehaviorEdit::SetAction {
                behavior: 0,
                action: 0,
                value: BehaviorAction::ChangeProperty {
                    target: Some("Mark".into()),
                    property: "Opacity".into(),
                    value: "0.67".into(),
                },
            },
        )
        .unwrap();
    assert_eq!(patches.len(), 1);
    let edited = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(edited, source.replace("r#\"0.5\"#", "\"0.67\""));
    let projection = project_rust(&edited, caret).unwrap();
    let patches = projection
        .edit_behavior(
            2,
            RsxBehaviorEdit::MoveAction {
                behavior: 0,
                action: 0,
                index: 1,
            },
        )
        .unwrap();
    let moved = TextPatch::apply_all(&edited, patches).unwrap();
    assert!(moved.find("add_class(Some").unwrap() < moved.find("change_property(Some").unwrap());
    assert!(moved.contains("/* keep 🌿 */"));
    assert!(moved.contains("other_behavior()"));
    assert_eq!(
        moved.split("fn page").next(),
        source.split("fn page").next()
    );
    assert_eq!(
        moved.matches("\r\n").count(),
        source.matches("\r\n").count()
    );
}

#[test]
fn renames_the_control_and_its_behavior_references_in_one_edit() {
    let projection = project_rust(SOURCE, SOURCE.find("when(").unwrap()).unwrap();
    let patches = projection
        .edit_control_names(&[1], Some("ReadyCard"))
        .unwrap();
    let updated = TextPatch::apply_all(SOURCE, patches).unwrap();
    assert!(updated.contains("name: \"ReadyCard\""));
    assert_eq!(updated.matches("Some(\"ReadyCard\")").count(), 2);
    assert!(projection.edit_control_names(&[1], None).is_err());
}

#[test]
fn literal_behavior_counterparts_patch_only_authored_values() {
    let source = SOURCE
        .replace("                other_behavior(),\n", "")
        .replace('\n', "\r\n");
    let caret = source.find("when(").unwrap();
    let projection = project_rust(&source, caret).unwrap();
    let xaml = projection.xaml();
    assert!(projection.matches_xaml(&xaml).unwrap());
    assert!(projection.apply_xaml(&xaml).unwrap().is_empty());
    let draft = xaml.replace("Value=\"0.5\"", "Value=\"0.67\"");
    let patched = TextPatch::apply_all(&source, projection.apply_xaml(&draft).unwrap()).unwrap();
    assert_eq!(patched, source.replace("r#\"0.5\"#", "\"0.67\""));
    assert!(projection
        .apply_xaml(&xaml.replace("{Binding #Mark}", "{Binding Unrelated}"))
        .is_err());

    let authored = xaml.replace("Value=\"0.5\"", "Value='0.5'").replace(
        "<ChangePropertyAction",
        "<!-- preserve 🌿 -->\n<ChangePropertyAction",
    );
    let authored = authored.replace('\n', "\r\n");
    let xml = Document::new(authored.as_str()).unwrap();
    let rsx = crate::xaml_to_rsx(&xml).unwrap();
    assert!(rsx.contains("Some(\"Mark\")"));
    assert!(crate::apply_rsx(&xml, &rsx).unwrap().is_empty());
    let draft = rsx.replace("\"0.5\"", "\"0.67\"");
    let patched = TextPatch::apply_all(&authored, crate::apply_rsx(&xml, &draft).unwrap()).unwrap();
    assert_eq!(patched, authored.replace("Value='0.5'", "Value='0.67'"));
    assert!(crate::apply_rsx(&xml, &rsx.replace("Some(\"Mark\")", "Some(\"Unknown\")")).is_err());
}

#[test]
fn layout_move_keeps_a_behavior_on_its_authored_control() {
    let projection = project_rust(SOURCE, SOURCE.find("when(").unwrap()).unwrap();
    let current = Document::new(projection.xaml()).unwrap();
    let approve = current.ast().find_name("Approve").unwrap();
    let mark = current.ast().find_name("Mark").unwrap();
    let root = current.ast().root();
    let edits = vec![Edit::MoveNode {
        node: approve,
        parent: root,
        index: 0,
    }];
    let mut draft = Document::new(current.source()).unwrap();
    draft.apply("Move approval", edits.clone()).unwrap();
    let plan = projection
        .plan_child_edit(&current, &draft, &edits, None)
        .unwrap();
    assert!(
        plan.authored_source.find("name: \"Approve\"").unwrap()
            < plan.authored_source.find("name: \"Mark\"").unwrap()
    );
    assert!(plan
        .authored_source
        .contains("change_property(Some(\"Mark\"), \"Opacity\", /* keep 🌿 */ r#\"0.5\"#)"));
    let edits = vec![Edit::RemoveNode { node: mark }];
    let mut draft = Document::new(current.source()).unwrap();
    draft
        .apply("Remove behavior target", edits.clone())
        .unwrap();
    assert!(projection
        .plan_child_edit(&current, &draft, &edits, None)
        .is_err());
}
