use crate::{project_rust, RsxCommandEdit, RsxCommandSelection::*, RsxProjection};
use ferroui_designer_core::{Document, TextPatch};

fn command(value: &str) -> crate::RsxCommandSelection {
    Set {
        expression: value.into(),
    }
}

fn plan(view: &RsxProjection<'_>, change: RsxCommandEdit) -> Result<Vec<TextPatch>, String> {
    let document = Document::new(view.xaml()).unwrap();
    view.plan_command(&document, document.ast().root(), &change)
}

#[test]
fn maps_assignment_and_builder_call_without_executing_runtime_values() {
    let source = concat!(
        "fn another() -> Element { rsx! { Button { content: \"Before 🧭\" } } }\r\n",
        "fn approval() -> Element {\r\n",
        "    rsx! { Button {\r\n",
        "        command: make(/* selected account 🪴 */ account),\r\n",
        "        command_parameter(/* type hint */ selected_id.clone(),),\r\n",
        "        {dynamic_label()}\r\n",
        "    } }\r\n",
        "}\r\n",
    );
    let caret = source.find("command:").unwrap();
    let view = project_rust(source, caret).unwrap();
    let document = Document::new(view.xaml()).unwrap();
    let slots = view.command_slots(&document, document.ast().root());
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert!(slot.binding.type_name.contains("ICommand"));
    assert_eq!(
        &source[slot.binding.expression.unwrap().range()],
        "make(/* selected account 🪴 */ account)"
    );
    assert!(slot.parameter.as_ref().unwrap().editable);
    assert!(!view.xaml().contains("command_parameter"));
    let patches = plan(
        &view,
        RsxCommandEdit {
            property: "Command".into(),
            command: command("approve.clone()"),
            parameter: command("request.id()"),
        },
    )
    .unwrap();
    let updated = TextPatch::apply_all(source, patches).unwrap();
    assert_eq!(
        updated,
        source
            .replace(
                "make(/* selected account 🪴 */ account)",
                "/* selected account 🪴 */ approve.clone()"
            )
            .replace("selected_id.clone()", "request.id()")
    );
    let view = project_rust(&updated, caret).unwrap();
    let cleared = TextPatch::apply_all(
        &updated,
        plan(
            &view,
            RsxCommandEdit {
                property: "Command".into(),
                command: Clear,
                parameter: Clear,
            },
        )
        .unwrap(),
    )
    .unwrap();
    for preserved in [
        "/* selected account 🪴 */",
        "/* type hint */",
        "{dynamic_label()}",
        "Before 🧭",
    ] {
        assert!(cleared.contains(preserved));
    }
    assert!(!cleared.contains("command:"));
    assert!(!cleared.contains("command_parameter("));
    assert_eq!(
        cleared.matches("\r\n").count(),
        cleared.matches('\n').count()
    );
}

#[test]
fn inserts_into_inherited_button_and_updates_rust_shorthand() {
    let source = "rsx! { CheckBox{command, command_parameter: \"Account\"} }";
    let view = project_rust(source, 0).unwrap();
    assert!(view.xaml().contains("CommandParameter=\"Account\""));
    let patch = plan(
        &view,
        RsxCommandEdit {
            property: "Command".into(),
            command: command("other.clone()"),
            parameter: Keep,
        },
    )
    .unwrap();
    assert_eq!(
        TextPatch::apply_all(source, patch).unwrap(),
        "rsx! { CheckBox{command: other.clone(), command_parameter: \"Account\"} }"
    );

    let source = "rsx! { Button{command} }";
    let view = project_rust(source, 0).unwrap();
    let patch = plan(
        &view,
        RsxCommandEdit {
            property: "Command".into(),
            command: Clear,
            parameter: command("ticket // current item"),
        },
    )
    .unwrap();
    assert_eq!(
        TextPatch::apply_all(source, patch).unwrap(),
        "rsx! { Button{ command_parameter: ticket // current item\n, } }"
    );

    let source = "rsx! { Button { content: \"Approve\" } }";
    let view = project_rust(source, 0).unwrap();
    let patch = plan(
        &view,
        RsxCommandEdit {
            property: "Command".into(),
            command: command("self.save.clone()"),
            parameter: command("ticket"),
        },
    )
    .unwrap();
    assert_eq!(patch.len(), 1);
    assert!(TextPatch::apply_all(source, patch)
        .unwrap()
        .contains("command: self.save.clone(), command_parameter: ticket,"));
}

#[test]
fn rejects_invalid_or_ambiguous_command_assignments() {
    for fields in [
        "command: save(), command(other())",
        "command()",
        "command(save(), other())",
        "Button.command: save()",
        "prop(Button::command_property(), save())",
        "with(apply_runtime_bindings)",
    ] {
        let source = format!("rsx! {{ Button {{ {fields} }} }}");
        let view = project_rust(&source, 0).unwrap();
        let document = Document::new(view.xaml()).unwrap();
        assert!(
            !view.command_slots(&document, document.ast().root())[0]
                .binding
                .editable,
            "{fields}"
        );
        assert!(
            plan(
                &view,
                RsxCommandEdit {
                    property: "Command".into(),
                    command: command("replacement"),
                    parameter: Keep,
                }
            )
            .is_err(),
            "{fields}"
        );
    }
    let view = project_rust("rsx! { Button {} }", 0).unwrap();
    for input in ["", "replacement(), content: \"changed\"", "{ missing"] {
        assert!(
            plan(
                &view,
                RsxCommandEdit {
                    property: "Command".into(),
                    command: command(input),
                    parameter: Keep,
                }
            )
            .is_err(),
            "{input}"
        );
    }
    assert!(plan(
        &view,
        RsxCommandEdit {
            property: "Command".into(),
            command: Clear,
            parameter: Clear,
        }
    )
    .unwrap()
    .is_empty());
}
