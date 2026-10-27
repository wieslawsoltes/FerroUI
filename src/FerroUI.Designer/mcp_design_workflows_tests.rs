use super::*;
use ferroui_controls::ContentControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui"><Label Name="Caption" Content="_Name"/><TextBox Name="Input" Text="{Binding Name}"/><Button Name="Accept" Content="Accept"/></StackPanel>"#;

fn fixture() -> (Rc<RefCell<DesignerSession>>, DesignerMcp) {
    let session = Rc::new(RefCell::new(
        DesignerSession::new(SOURCE, ContentControl::new()).unwrap(),
    ));
    let provider = DesignerMcp::new(&session);
    (session, provider)
}
fn permissions(write: bool) -> Permissions {
    Permissions {
        allow_property_writes: write,
        allow_input: false,
    }
}

#[test]
fn workflow_schemas_reject_mixed_actions_out_of_range_data_and_unchecked_models() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (_, provider) = fixture();
    for (name, args) in [
        (
            "designer_design_data_edit",
            json!({"expected_revision":0,"action":"enabled","enabled":true,"node":1}),
        ),
        (
            "designer_design_data_edit",
            json!({"expected_revision":0,"action":"sample_items","node":1,"count":201}),
        ),
        (
            "designer_design_data_edit",
            json!({"expected_revision":0,"action":"set_value","node":1,"property":"Text","value":4}),
        ),
        (
            "designer_focus_edit",
            json!({"expected_revision":0,"action":"order","scope":1,"nodes":[2,3]}),
        ),
        (
            "designer_focus_edit",
            json!({"expected_revision":0,"expected_model":"abc","action":"order","scope":1,"nodes":[]}),
        ),
        (
            "designer_layer",
            json!({"expected_revision":0,"action":"reset"}),
        ),
        (
            "designer_layer",
            json!({"expected_revision":0,"expected_layer_epoch":0,"action":"set_locked","nodes":[0],"locked":true}),
        ),
    ] {
        assert!(provider.validate(name, &args).is_err(), "{name} {args}");
    }
    provider.validate("designer_design_data_edit",&json!({"expected_revision":0,"action":"set_value","node":1,"property":"Text","value":null})).unwrap();
}

#[test]
fn workflow_reads_and_layer_edits_preserve_source_and_reject_stale_epochs() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let input = session
        .borrow()
        .document()
        .ast()
        .find_name("Input")
        .unwrap();
    let revision = session.borrow().document().revision();
    for name in [
        "designer_design_data",
        "designer_focus_order",
        "designer_layers",
    ] {
        provider.call(name, &json!({}), permissions(false)).unwrap();
    }
    let epoch = session.borrow().layer_view_epoch();
    let args = json!({"expected_revision":revision,"expected_layer_epoch":epoch,"action":"set_hidden","nodes":[input],"hidden":true});
    assert!(
        provider
            .call("designer_layer", &args, permissions(false))
            .is_err()
    );
    session.borrow_mut().set_read_only(true);
    provider
        .call("designer_layer", &args, permissions(true))
        .unwrap();
    assert!(session.borrow().layer_state(input).hidden);
    assert!(
        provider
            .call("designer_layer", &args, permissions(true))
            .is_err()
    );
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert_eq!(session.borrow().document().revision(), revision);
    assert!(!session.borrow().can_undo());
}

#[test]
fn design_data_tool_preserves_runtime_binding_and_uses_normal_undo_and_roles() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let input = session
        .borrow()
        .document()
        .ast()
        .find_name("Input")
        .unwrap();
    let revision = session.borrow().document().revision();
    let args = json!({"expected_revision":revision,"action":"set_value","node":input,"property":"Text","value":"Sample name"});
    session.borrow_mut().set_read_only(true);
    assert!(
        provider
            .call("designer_design_data_edit", &args, permissions(true))
            .is_err()
    );
    session.borrow_mut().set_read_only(false);
    provider
        .call("designer_design_data_edit", &args, permissions(true))
        .unwrap();
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(input)
            .unwrap()
            .attribute("Text"),
        Some("{Binding Name}")
    );
    let read = provider
        .call(
            "designer_design_data",
            &json!({"node":input}),
            permissions(false),
        )
        .unwrap();
    assert!(
        read["properties"]
            .as_array()
            .unwrap()
            .iter()
            .any(|property| property["name"] == "Text" && property["value"] == "Sample name")
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
}

#[test]
fn focus_tools_require_the_accepted_native_model_and_commit_label_target_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let label = session
        .borrow()
        .document()
        .ast()
        .find_name("Caption")
        .unwrap();
    let input = session
        .borrow()
        .document()
        .ast()
        .find_name("Input")
        .unwrap();
    let current = provider
        .call("designer_focus_order", &json!({}), permissions(false))
        .unwrap();
    let revision = session.borrow().document().revision();
    let mut args = json!({"expected_revision":revision,"expected_model":"obsolete","action":"label_target","label":label,"target":input});
    assert!(
        provider
            .call("designer_focus_edit", &args, permissions(true))
            .is_err()
    );
    args["expected_model"] = current["model"].clone();
    provider
        .call("designer_focus_edit", &args, permissions(true))
        .unwrap();
    let updated = provider
        .call("designer_focus_order", &json!({}), permissions(false))
        .unwrap();
    assert!(
        updated["catalog"]["labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value["node"] == label && value["target"] == input)
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
}
