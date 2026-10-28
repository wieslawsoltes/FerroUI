use super::*;
use crate::{PreviewStatus, PropertyValueEdit, resolve_property_editor_metadata};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control};
use ferroui_designer_tools::ComponentAction;

fn ready(session: &DesignerSession) {
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { revision } if *revision == session.document().revision()),
        "{:?}",
        session.preview_status()
    );
}

#[test]
fn native_auto_literal_session_and_component_edits_serialize_float_sentinels_and_undo_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" /></Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let target = session.document().ast().find_name("Target").unwrap();
    for component in [false, true] {
        ready(&session);
        if component {
            session
                .edit_component(ComponentAction::SetProperty {
                    node: target,
                    property: "Width".into(),
                    value: Some("Auto".into()),
                })
                .unwrap();
        } else {
            session.set_property(&[target], "Width", "Auto").unwrap();
        }
        assert_eq!(
            session
                .document()
                .ast()
                .node(target)
                .unwrap()
                .attribute("Width"),
            Some("NaN")
        );
        ready(&session);
        assert!(
            session
                .preview
                .visual(target)
                .unwrap()
                .cast::<Control>()
                .unwrap()
                .width()
                .is_nan()
        );
        session.undo().unwrap();
        assert_eq!(session.document().source(), source);
        assert!(!session.can_undo());
        ready(&session);
        assert_eq!(
            session
                .preview
                .visual(target)
                .unwrap()
                .cast::<Control>()
                .unwrap()
                .width(),
            100.0
        );
    }
}

#[test]
fn native_auto_literal_theme_setter_uses_resolved_target_type_and_native_validation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:q="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:ui="using:FerroUI.Controls"><StackPanel.Resources><ControlTheme q:Key="Action" TargetType="{q:Type ui:Button}"><Setter Property="Width" Value="100" /></ControlTheme></StackPanel.Resources><Button Name="Target" Theme="{StaticResource Action}" /></StackPanel>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let target = session.document().ast().find_name("Target").unwrap();
    let owner = session
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTheme")
        .unwrap()
        .id;
    let revision = session.document().revision();
    ready(&session);
    session
        .edit_component(ComponentAction::SetSetter {
            owner,
            property: "Width".into(),
            value: Some("Auto".into()),
        })
        .unwrap();
    assert_eq!(session.document().revision(), revision + 1);
    assert_eq!(
        session
            .document()
            .ast()
            .elements()
            .find(|node| node.local_name() == "Setter")
            .unwrap()
            .attribute("Value"),
        Some("NaN")
    );
    ready(&session);
    assert!(
        session
            .preview
            .visual(target)
            .unwrap()
            .cast::<Control>()
            .unwrap()
            .width()
            .is_nan()
    );
    session.undo().unwrap();
    ready(&session);
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
}

#[test]
fn native_auto_literal_preserves_text_grid_lengths_and_unknown_types_and_rejects_invalid_floats() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui"><Grid.ColumnDefinitions><ColumnDefinition Width="100" /></Grid.ColumnDefinitions><TextBlock Name="Target" Text="Initial" /></Grid>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let column = session
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "ColumnDefinition")
        .unwrap()
        .id;
    let target = session.document().ast().find_name("Target").unwrap();
    for (node, property) in [(column, "Width"), (target, "Text")] {
        session.set_property(&[node], property, "Auto").unwrap();
        assert_eq!(
            session
                .document()
                .ast()
                .node(node)
                .unwrap()
                .attribute(property),
            Some("Auto")
        );
        ready(&session);
        session.undo().unwrap();
        assert_eq!(session.document().source(), source);
    }
    // Component commands bypass portable number descriptors, so native range validation
    // must still reject NaN without changing source or history.
    assert!(
        session
            .edit_component(ComponentAction::SetProperty {
                node: target,
                property: "MinWidth".into(),
                value: Some("Auto".into()),
            })
            .is_err()
    );
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());

    let unknown = r#"<app:Textual xmlns:app="urn:unregistered" Width="Initial" />"#;
    let mut session = DesignerSession::new(unknown, ContentControl::new()).unwrap();
    let target = session.document().ast().root();
    session.set_property(&[target], "Width", "Auto").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("Auto")
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), unknown);

    let captured = Rc::new(std::cell::RefCell::new(None));
    let context = PropertyEditorContext {
        property: "Width".into(),
        targets: vec![],
        value: None,
        mixed: false,
        runtime_type: Some("alloc::string::String".into()),
        apply: {
            let captured = captured.clone();
            Rc::new(move |value| *captured.borrow_mut() = value)
        },
    };
    resolve_property_editor_metadata(&context, None)
        .apply(&context, PropertyValueEdit::Literal("Auto".into()))
        .unwrap();
    assert_eq!(captured.borrow().as_deref(), Some("Auto"));
}

#[cfg(feature = "mcp")]
#[test]
fn native_auto_literal_mcp_property_writes_render_and_undo_as_one_transaction() {
    use ferroui_mcp::UiToolProvider;
    use serde_json::json;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Button xmlns="https://github.com/ferroui" Width="100" />"#;
    let session = Rc::new(std::cell::RefCell::new(
        DesignerSession::new(source, ContentControl::new()).unwrap(),
    ));
    let provider = crate::mcp::DesignerMcp::new(&session);
    let target = session.borrow().document().ast().root();
    provider.call("designer_property", &json!({"action":"set", "expected_revision":0, "nodes":[target], "property":"Width", "value":"Auto"}), crate::mcp::Permissions { allow_property_writes:true, allow_input:false }).unwrap();
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("NaN")
    );
    ready(&session.borrow());
    assert!(
        session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<Control>()
            .unwrap()
            .width()
            .is_nan()
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), source);
    assert!(!session.borrow().can_undo());
    ready(&session.borrow());
}
