use super::*;
use crate::mcp::DesignerMcp;
use ferroui_base::{interactivity::RoutedEventArgs, Ref};
use ferroui_controls::{
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
    ComboBox, Control, TextBox, Window,
};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:DataType="Button" x:CompileBindings="False">
    <!-- Keep the original selection and markup -->
    <Button Name="SaveButton" Content="Save" />
</StackPanel>"#;

#[test]
fn studio_picker_and_mcp_apply_the_same_command_edit_and_restore_in_one_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(720.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let node = session
        .borrow()
        .document()
        .ast()
        .find_name("SaveButton")
        .unwrap();
    session.borrow_mut().select([node]);
    workspace.refresh(true);
    let anchor = workspace
        .control()
        .get_control::<Button>("EditCommandBinding");
    assert!(anchor.is_visible());
    let flyout = anchor
        .flyout()
        .and_then(|value| value.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&anchor);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().expect("Command binding flyout"))
        .unwrap();
    let control = |label| {
        body.get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(label))
            .unwrap_or_else(|| {
                panic!(
                    "Missing {label}; preview: {:?}",
                    session.borrow().preview_status()
                )
            })
    };
    let select = |label, prefix: &str| {
        let combo = control(label).cast::<ComboBox>().unwrap();
        let index = (0..combo.items().count())
            .find(|index| {
                combo
                    .items()
                    .get_at(*index)
                    .as_ref()
                    .and_then(|label| label.downcast_ref::<String>())
                    .is_some_and(|label| label.starts_with(prefix))
            })
            .unwrap();
        combo.set_selected_index(index as i32);
    };
    select("Command", "Command ·");
    select("CommandParameter", "Content ·");
    let apply = control("Apply Command binding").cast::<Button>().unwrap();
    apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let edited = session.borrow().authored_source().to_owned();
    assert!(
        edited.contains(r#"Command="{Binding Command}""#),
        "{edited}"
    );
    assert!(
        edited.contains(r#"CommandParameter="{Binding Content}""#),
        "{edited}"
    );
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);

    let provider = DesignerMcp::with_workspace(&workspace);
    let revision = session.borrow().document().revision();
    let reported = provider
        .call(
            "designer_command_bindings",
            &json!({
                "node": node, "expected_revision": revision,
            }),
            Permissions::default(),
        )
        .unwrap();
    assert!(reported["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|member| { member["path"] == "Command" && member["command"] == true }));
    let mut edit = json!({
        "node": node, "expected_revision": revision, "expected_uri": reported["uri"],
        "expected_version": reported["version"], "expected_scope_epoch": reported["scopeEpoch"].as_u64().unwrap() + 1,
        "change": {"property": "Command", "command": {"kind": "bind", "path": "Command"},
            "parameter": {"kind": "bind", "path": "Content"}},
    });
    let permissions = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    assert!(provider
        .call("designer_command_binding", &edit, permissions)
        .unwrap_err()
        .contains("scope changed"));
    edit["expected_scope_epoch"] = reported["scopeEpoch"].clone();
    provider
        .call("designer_command_binding", &edit, permissions)
        .unwrap();
    assert_eq!(session.borrow().authored_source(), edited);
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    window.close();
}

#[test]
fn studio_rust_command_flyout_and_mcp_share_a_single_undo_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = "fn view() { rsx! { Button { name: \"Approve\",\r\n    command: /* 🧭 */ previous(),\r\n    command_parameter(current()),\r\n    {dynamic_label()}\r\n} } }\r\nfn another() { rsx! { TextBlock { text: \"Account\" } } }\r\n";
    let path = std::env::temp_dir().join("ferroui-studio-rust-command-binding.rs");
    let uri = ferroui_designer_project::file_uri(&path).unwrap();
    let workspace = DesignerWorkspace::with_uri(source, uri).unwrap();
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(720.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let node = session.borrow().document().ast().root();
    session.borrow_mut().select([node]);
    workspace.refresh(true);

    let anchor = workspace
        .control()
        .get_control::<Button>("EditCommandBinding");
    assert!(anchor.is_visible());
    let flyout = anchor
        .flyout()
        .and_then(|value| value.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&anchor);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let control = |label: &str| {
        body.get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(label))
            .unwrap_or_else(|| panic!("Missing {label}"))
    };
    let command = control("Command Rust expression")
        .cast::<TextBox>()
        .unwrap();
    let parameter = control("CommandParameter Rust expression")
        .cast::<TextBox>()
        .unwrap();
    assert_eq!(command.text().as_deref(), Some("previous()"));
    assert_eq!(parameter.text().as_deref(), Some("current()"));
    command.set_text(Some("approve.clone()"));
    parameter.set_text(Some("account.clone()"));
    control("Apply Command binding")
        .cast::<Button>()
        .unwrap()
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let edited = source
        .replace("previous()", "approve.clone()")
        .replace("current()", "account.clone()");
    assert_eq!(session.borrow().authored_source(), edited);
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), source);

    let provider = DesignerMcp::with_workspace(&workspace);
    let report = provider
        .call(
            "designer_rsx_commands",
            &json!({"node":node}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(report["slots"][0]["binding"]["authored"], "previous()");
    assert_eq!(report["slots"][0]["parameter"]["authored"], "current()");
    let mut edit = json!({
        "node":node,"expected_revision":report["revision"],"expected_model":report["model"],
        "change":{"property":"Command","command":{"kind":"set","expression":"approve.clone()"},
            "parameter":{"kind":"set","expression":"account.clone()"}}
    });
    assert!(provider
        .call("designer_rsx_command_edit", &edit, Permissions::default())
        .is_err());
    let permitted = Permissions {
        allow_property_writes: true,
        ..Permissions::default()
    };
    provider
        .call("designer_rsx_command_edit", &edit, permitted)
        .unwrap();
    assert_eq!(session.borrow().authored_source(), edited);
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    edit["expected_revision"] = json!(session.borrow().document().revision());
    assert!(provider
        .call("designer_rsx_command_edit", &edit, permitted)
        .unwrap_err()
        .contains("model changed"));
    assert!(!session.borrow().can_undo());
    window.close();
}
