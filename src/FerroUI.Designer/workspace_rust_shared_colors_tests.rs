use super::*;
use crate::mcp::{DesignerMcp, RSX_STYLE_EDIT, RSX_STYLES};
use ferroui_base::{Ref, interactivity::RoutedEventArgs};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ComboBox, TextBox, Window};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/rust_shared_colors.rs");

#[test]
fn shared_rust_color_commits_keep_the_selected_view_and_one_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let uri =
        ferroui_designer_project::file_uri(std::env::temp_dir().join("rust_shared_colors.rs"))
            .unwrap();
    let workspace = DesignerWorkspace::with_uri(SOURCE, uri).unwrap();
    let session = workspace.session();
    let offset = SOURCE.find("    rsx! {\n        Border").unwrap() + 4;
    session.borrow_mut().select_rust_invocation(offset).unwrap();
    workspace.refresh(true);
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let provider = DesignerMcp::with_workspace(&workspace);
    let read = |tool, args| provider.call(tool, &args, Permissions::default()).unwrap();
    let color = || {
        let node = session
            .borrow()
            .document()
            .ast()
            .find_name("SharedStatus")
            .unwrap();
        read(
            "designer_property_trace",
            json!({"node":node,"property":"TextElement.Foreground"}),
        )["effective"]
            .clone()
    };
    assert_eq!(color(), "#FF254A70");
    let anchor = workspace.control().get_control::<Button>("EditRustStyles");
    let styles = anchor
        .flyout()
        .and_then(|flyout| flyout.cast::<Flyout>())
        .unwrap();
    styles.show_at(&anchor);
    let menu = Ref::<Control>::from_xaml_object(&styles.content().unwrap()).unwrap();
    let find = |menu: &Control, name: &str| {
        menu.get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(name))
            .unwrap()
    };
    let edit = find(&menu, "Change color from const DEFAULT_INK")
        .cast::<Button>()
        .unwrap();
    let choice = edit
        .flyout()
        .and_then(|flyout| flyout.cast::<Flyout>())
        .unwrap();
    choice.show_at(&edit);
    let form = Ref::<Control>::from_xaml_object(&choice.content().unwrap()).unwrap();
    find(&form, "Rust color edit scope")
        .cast::<ComboBox>()
        .unwrap()
        .set_selected_index(1);
    find(&form, "Color value")
        .cast::<TextBox>()
        .unwrap()
        .set_text(Some("#FF187257"));
    find(&form, "Apply Color value")
        .cast::<Button>()
        .unwrap()
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(color(), "#FF187257");
    assert_eq!(
        session.borrow().authored_source(),
        SOURCE.replace(
            "37u8, /* brand 🧭 */ 74, 112",
            "24u8, /* brand 🧭 */ 114, 87"
        )
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(color(), "#FF254A70");

    let style = read(RSX_STYLES, json!({}))["definitions"][0]["node"].clone();
    let entry = read(RSX_STYLES, json!({"node":style}))["color_resources"][0]["node"].clone();
    let revision = session.borrow().document().revision();
    let edit = json!({"action":"color_resource","style":style,"resource":entry,
        "mode":"local","value":"#FF465A9F","expected_revision":revision});
    provider
        .call(
            RSX_STYLE_EDIT,
            &edit,
            Permissions {
                allow_property_writes: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        session
            .borrow()
            .authored_source()
            .contains("Rgb::from_rgb(37u8, /* brand 🧭 */ 74, 112)")
    );
    assert_eq!(color(), "#FF465A9F");
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(color(), "#FF254A70");
    styles.hide();
    window.close();
}
