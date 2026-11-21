use super::*;
use crate::mcp::{DesignerMcp, RSX_STYLES, RSX_STYLE_EDIT};
use ferroui_base::{input::{InputElement, Key, KeyEventArgs}, interactivity::RoutedEventArgs, Ref};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{TextBox, Window};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/rust_style_workbench.rs");

#[test]
fn rust_style_transactions_match_studio_controls_mcp_and_native_font_size() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let uri =
        ferroui_designer_project::file_uri(std::env::temp_dir().join("rust_style_workbench.rs"))
            .unwrap();
    let workspace = DesignerWorkspace::with_uri(SOURCE, uri).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let font_size = || {
        let session = session.borrow();
        let node = session.document().ast().find_name("Review").unwrap();
        session
            .preview
            .visual(node)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .font_size()
    };
    assert_eq!(font_size(), 24.0);
    let anchor = workspace.control().get_control::<Button>("EditRustStyles");
    assert!(anchor.is_visible());
    let flyout = anchor
        .flyout()
        .and_then(|flyout| flyout.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&anchor);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let control = |name: &str| {
        body.get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(name))
            .unwrap()
    };
    let input = control("FontSize").cast::<TextBox>().unwrap();
    input.set_text(Some("30.5"));
    let apply = control("Apply FontSize").cast::<Button>().unwrap();
    apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(font_size(), 30.5);
    assert_eq!(
        session.borrow().authored_source(),
        SOURCE.replace("24.0f64", "30.5f64")
    );

    let provider = DesignerMcp::with_workspace(&workspace);
    let read = |args| {
        provider
            .call(RSX_STYLES, &args, Permissions::default())
            .unwrap()
    };
    let catalog = read(json!({}));
    let node = catalog["definitions"][0]["node"].as_u64().unwrap();
    let style = read(json!({"node":node}));
    let target = style["setters"][0]["node"].as_u64().unwrap();
    let args = json!({"action":"setter","style":node,"setter":target,"value":"36",
        "expected_revision":session.borrow().document().revision()});
    let writes = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    provider.call(RSX_STYLE_EDIT, &args, writes).unwrap();
    assert_eq!(font_size(), 36.0);
    assert!(provider.call(RSX_STYLE_EDIT, &args, writes).is_err());
    input.set_text(Some("40"));
    apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(font_size(), 36.0);
    session.borrow_mut().undo().unwrap();
    assert_eq!(font_size(), 30.5);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(font_size(), 24.0);
    flyout.hide();
    window.close();
}

#[test]
fn rust_style_color_reaches_the_native_preview_and_keeps_source_navigation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = SOURCE.replace(".set(ferroui_controls::TextBlock::foreground_property(), \"#1C3554\")",
        ".set_resource(ferroui_controls::TextBlock::foreground_property(), \"ReviewInk\")");
    let uri = ferroui_designer_project::file_uri(std::env::temp_dir().join("rust_style_resources.rs")).unwrap();
    let workspace = DesignerWorkspace::with_uri(&source, uri).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let trace = || {
        let node = session.borrow().document().ast().find_name("Review").unwrap();
        provider.call("designer_property_trace", &json!({"node":node,"property":"TextElement.Foreground"}), Permissions::default()).unwrap()
    };
    let initial = trace();
    assert_eq!(initial["effective"], "#FF224E7C");
    let winner = initial["contributors"].as_array().unwrap().iter()
        .find(|entry| entry["kind"] == "style" && entry["state"] == "effective").unwrap();
    assert_eq!(winner["source"]["text"], "\"ReviewInk\"");

    let anchor = workspace.control().get_control::<Button>("EditRustStyles");
    let flyout = anchor.flyout().and_then(|flyout| flyout.cast::<Flyout>()).unwrap();
    flyout.show_at(&anchor);
    let menu = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let color = menu.get_logical_descendants().find_map(|control| {
        let field = control.cast::<TextBox>()?;
        (AutomationProperties::get_name(&field).as_deref() == Some("ReviewInk color resource value")).then_some(field)
    }).unwrap();
    color.set_text(Some("#FF17674D"));
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    color.raise_event(&enter);
    assert_eq!(trace()["effective"], "#FF17674D");
    assert_eq!(session.borrow().authored_source(), source.replace("34, /* palette */ 78, 124", "23, /* palette */ 103, 77"));
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert_eq!(trace()["effective"], "#FF224E7C");
    flyout.hide();
    window.close();
}

#[test]
fn rust_palette_layers_compare_and_edit_the_dark_color_in_studio() {
    use crate::{PreviewEnvironment, PreviewTheme};
    use ferroui_controls::ComboBox;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = SOURCE.replace(
        ".resource(\"ReviewInk\",",
        ".theme_resource(ferroui_base::styling::ThemeVariant::default(), \"ReviewInk\",",
    ).replace(".set(ferroui_controls::TextBlock::foreground_property(), \"#1C3554\")",
        ".set_resource(ferroui_controls::TextBlock::foreground_property(), \"ReviewInk\")");
    let uri = ferroui_designer_project::file_uri(std::env::temp_dir().join("rust_palette.rs")).unwrap();
    let workspace = DesignerWorkspace::with_uri(&source, uri).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();

    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let read = |tool, args| provider.call(tool, &args, Permissions::default()).unwrap();
    let node = read(RSX_STYLES, json!({}))["definitions"][0]["node"].as_u64().unwrap();
    let edit = json!({"action":"add_color_resource", "style":node, "layer":"dark",
        "key":"ReviewInk", "value":"#FFB3D8EF", "expected_revision":session.borrow().document().revision()});
    provider.call(RSX_STYLE_EDIT, &edit, Permissions { allow_property_writes: true, ..Default::default() }).unwrap();
    let authored = session.borrow().authored_source().to_owned();
    let report = read(RSX_STYLES, json!({"node":node}));
    let comparison = &report["resource_comparison"][0];
    assert_eq!(comparison["light"]["value"], "#FF224E7C");
    assert_eq!(comparison["dark"]["value"], "#FFB3D8EF");
    assert_eq!(comparison["dark"], comparison["high_contrast"]);
    workspace.set_preview_environment(PreviewEnvironment { theme: PreviewTheme::Dark, ..workspace.preview_environment() }).unwrap();
    let ink = || {
        let node = session.borrow().document().ast().find_name("Review").unwrap();
        read("designer_property_trace", json!({"node":node,"property":"TextElement.Foreground"}))["effective"].clone()
    };
    assert_eq!(ink(), "#FFB3D8EF");

    let anchor = workspace.control().get_control::<Button>("EditRustStyles");
    let flyout = anchor.flyout().and_then(|flyout| flyout.cast::<Flyout>()).unwrap();
    flyout.show_at(&anchor);
    let menu = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let control = |name: &str| menu.get_logical_descendants()
        .find(|control| AutomationProperties::get_name(control).as_deref() == Some(name)).unwrap();
    let toggle = control("Compare Rust theme colors").cast::<Button>().unwrap();
    let click = RoutedEventArgs::with_event(Button::click_event());
    toggle.raise_event(&click);
    assert!(menu.get_logical_descendants().any(|control| AutomationProperties::get_name(&control)
        .is_some_and(|name| name.starts_with("Open High contrast declaration for ReviewInk") && name.contains("Dark"))));
    control("Rust color layer").cast::<ComboBox>().unwrap().set_selected_index(3);
    window.update_layout();
    let field = control("ReviewInk Dark color resource value").cast::<TextBox>().unwrap();
    assert!(field.is_effectively_visible());
    field.set_text(Some("#FFE3C792"));
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    field.raise_event(&enter);
    assert_eq!(ink(), "#FFE3C792");
    assert_ne!(session.borrow().authored_source(), authored);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), authored);
    assert_eq!(ink(), "#FFB3D8EF");
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert_eq!(ink(), "#FF224E7C");
    flyout.hide();
    window.close();
}
