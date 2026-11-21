use super::*;
use ferroui_base::Ref;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{TableView, Window};
use ferroui_markup_xaml::FromXamlObject;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/ColumnsWorkbench.xaml");

#[test]
fn studio_column_controls_reorder_the_native_table_and_reject_a_retired_click() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let table = session
        .borrow()
        .document()
        .ast()
        .find_name("Portfolio")
        .unwrap();
    let status = session
        .borrow()
        .document()
        .ast()
        .find_name("Status")
        .unwrap();
    session.borrow_mut().select([table]);
    workspace.refresh(true);
    let anchor = workspace.control().get_control::<Button>("EditColumns");
    assert!(anchor.is_visible());
    let flyout = anchor
        .flyout()
        .and_then(|flyout| flyout.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&anchor);
    let body =
        Ref::<Control>::from_xaml_object(&flyout.content().expect("column editor has opened"))
            .unwrap();
    let move_later = body
        .get_logical_descendants()
        .find(|control| {
            AutomationProperties::get_name(control).as_deref() == Some("Move column later")
        })
        .and_then(|control| control.cast::<Button>())
        .expect("Move column later button");
    assert!(move_later.is_enabled());
    let click = RoutedEventArgs::with_event(Button::click_event());
    move_later.raise_event(&click);
    let first_header = || {
        let view = session
            .borrow()
            .preview
            .visual(table)
            .unwrap()
            .cast::<TableView>()
            .unwrap();
        view.columns()
            .get(0)
            .header()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            .clone()
    };
    assert_eq!(first_header(), "Owner");
    let reordered = session.borrow().authored_source().to_owned();
    let revision = session.borrow().document().revision();
    move_later.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().authored_source(), reordered);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(first_header(), "Account");
    let grid = session.borrow().authored_columns(status).unwrap().unwrap();
    assert_eq!(grid.reason, None);
    assert_eq!(grid.columns.len(), 3);
    flyout.hide();
    window.close();
}

#[cfg(feature = "mcp")]
#[test]
fn studio_rsx_columns_share_preview_source_and_history_with_mcp() {
    use crate::mcp::DesignerMcp;
    use ferroui_mcp::{Permissions, UiToolProvider};
    use serde_json::json;

    const RUST: &str = include_str!("../../editors/vscode/test/fixtures/column_workbench.rs");
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let file = ferroui_designer_project::file_uri(std::env::temp_dir().join("column_workbench.rs")).unwrap();
    let workspace = DesignerWorkspace::with_uri(RUST, file).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let table = session.borrow().document().ast().find_name("Accounts").unwrap();
    session.borrow_mut().select([table]);
    workspace.refresh(true);
    let anchor = workspace.control().get_control::<Button>("EditColumns");
    assert!(anchor.is_visible());
    let flyout = anchor.flyout().and_then(|flyout| flyout.cast::<Flyout>()).unwrap();
    flyout.show_at(&anchor);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let move_later = body.get_logical_descendants().find(|control| {
        AutomationProperties::get_name(control).as_deref() == Some("Move column later")
    }).and_then(|control| control.cast::<Button>()).unwrap();
    assert!(move_later.is_enabled());
    move_later.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let provider = DesignerMcp::with_workspace(&workspace);
    let read = || provider.call("designer_columns", &json!({"node":table}), Permissions::default()).unwrap();
    let moved = read();
    assert_eq!(moved["columns"][0]["label"], "Region");
    assert!(session.borrow().authored_source().find("header(\"Region\")").unwrap()
        < session.borrow().authored_source().find("// Main account name").unwrap());
    let args = json!({"table":table,"expected_model":moved["model"],"expected_revision":moved["revision"],
        "column":moved["columns"][0]["node"],"action":"set","field":"Width","value":"132"});
    let write = Permissions { allow_property_writes: true, ..Default::default() };
    provider.call("designer_columns_edit", &args, write).unwrap();
    assert!(provider.call("designer_columns_edit", &args, write).is_err());
    let native = session.borrow().preview.visual(table).unwrap().cast::<TableView>().unwrap();
    assert_eq!(native.columns().get(0).width(), ferroui_controls::GridLength::from_pixels(132.0));
    session.borrow_mut().undo().unwrap();
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), RUST);
    assert_eq!(read()["columns"][0]["label"], "Account");
    flyout.hide();
    window.close();
}
