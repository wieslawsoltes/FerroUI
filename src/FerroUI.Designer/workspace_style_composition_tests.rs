use super::*;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ComboBox, Window};
use ferroui_markup_xaml::FromXamlObject;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/StylePalette.xaml");

#[test]
fn studio_palette_edits_the_dark_declaration_and_retains_exact_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let session = workspace.session();
    assert!(
        session.borrow().can_manipulate(),
        "{:?}",
        session.borrow().preview_status()
    );
    let owner = session.borrow().document().ast().root();
    workspace.inspect_component(owner).unwrap();
    window.update_layout();
    let button = workspace
        .control()
        .get_logical_descendants()
        .find_map(|control| {
            let button = control.cast::<Button>()?;
            (button.name().as_deref() == Some("DefinitionPalette")).then_some(button)
        })
        .unwrap();
    let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.show_at(&button);
    window.update_layout();
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let dark = body
        .get_logical_descendants()
        .find_map(|control| {
            let button = control.cast::<Button>()?;
            (button.name().as_deref() == Some("PalettePreviewDark")).then_some(button)
        })
        .unwrap();
    dark.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        workspace.preview_environment().theme,
        crate::PreviewTheme::Dark
    );
    assert!(!session.borrow().can_undo());
    let layer = body
        .get_logical_descendants()
        .find_map(|control| {
            let select = control.cast::<ComboBox>()?;
            (select.name().as_deref() == Some("PaletteLayer")).then_some(select)
        })
        .unwrap();
    layer.set_selected_index(3);
    let color = body
        .get_logical_descendants()
        .find_map(|control| {
            let field = control.cast::<TextBox>()?;
            (AutomationProperties::get_name(&field).as_deref() == Some("ActionSurface color value"))
                .then_some(field)
        })
        .unwrap();
    assert_eq!(color.text().as_deref(), Some("#FF537BE1"));
    color.set_text(Some("#FF40AB94"));
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    color.raise_event(&enter);
    let updated = session.borrow().document().source().to_owned();
    let status = body
        .get_logical_descendants()
        .find_map(|control| {
            let field = control.cast::<ferroui_controls::TextBlock>()?;
            (field.name().as_deref() == Some("PaletteStatus")).then_some(field)
        })
        .and_then(|field| field.text());
    assert!(
        updated.contains("#FF40AB94"),
        "palette commit status: {status:?}; preview: {:?}",
        session.borrow().preview_status()
    );
    assert_eq!(updated, SOURCE.replace("#FF537BE1", "#FF40AB94"));
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().can_undo());
    color.set_text(Some("#FF885522"));
    color.raise_event(&enter);
    assert_eq!(
        session.borrow().document().source(),
        SOURCE,
        "the retired flyout cannot edit"
    );
    flyout.hide();
    window.close();
}

#[cfg(feature = "mcp")]
#[test]
fn studio_edits_included_styles_and_palette_colors_from_the_consumer() {
    use crate::mcp::{DesignerMcp, Permissions};
    use ferroui_designer_project::{file_uri, Solution};
    use ferroui_mcp::UiToolProvider;
    use serde_json::{json, Value};
    use std::{fs, path::PathBuf};

    struct Files(PathBuf);
    impl Drop for Files {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    const VIEW: &str = include_str!("../../editors/vscode/test/fixtures/IncludedStylePalette.xaml");
    const STYLE: &str =
        include_str!("../../editors/vscode/test/fixtures/Styling/Shared Styles.xaml");
    const PALETTE: &str =
        include_str!("../../editors/vscode/test/fixtures/Styling/Shared Palette.xaml");
    const DARK: &str = include_str!("../../editors/vscode/test/fixtures/Styling/Colors/Dark.xaml");
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files =
        Files(std::env::temp_dir().join(format!("ferroui-style-sources-{}", std::process::id())));
    fs::create_dir_all(files.0.join("Styling/Colors")).unwrap();
    for (path, source) in [
        ("View.xaml", VIEW),
        ("Styling/Shared Styles.xaml", STYLE),
        ("Styling/Shared Palette.xaml", PALETTE),
        ("Styling/Colors/Dark.xaml", DARK),
    ] {
        fs::write(files.0.join(path), source).unwrap();
    }
    let root = files.0.canonicalize().unwrap();
    let uri = file_uri(root.join("View.xaml")).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, uri.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&root).unwrap())
        .unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let mcp = DesignerMcp::with_workspace(&workspace);
    let button = |name| {
        workspace
            .control()
            .get_logical_descendants()
            .find_map(|control| {
                let button = control.cast::<Button>()?;
                (button.name().as_deref() == Some(name)).then_some(button)
            })
            .unwrap_or_else(|| panic!("No Studio button named {name}"))
    };
    let call = |name: &str, args: Value| {
        mcp.call(
            name,
            &args,
            Permissions {
                allow_property_writes: true,
                ..Default::default()
            },
        )
    };
    let revision = || session.borrow().document().revision();
    let owner = session.borrow().document().ast().root();
    workspace.inspect_component(owner).unwrap();
    let catalog = call("designer_style_catalog", json!({"target":owner})).unwrap();
    let choice = catalog["included_sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["label"] == "Border.account")
        .unwrap();
    let opened = call(
        "designer_open_style_source",
        json!({"source":choice["source"],"expected_revision":revision()}),
    )
    .unwrap();
    assert_eq!(
        session.borrow().document_uri(),
        file_uri(root.join("Styling/Shared Styles.xaml")).unwrap()
    );
    let style = &opened["editor"]["node"];
    call("designer_style_edit", json!({"expected_revision":revision(),"action":"setter","owner":style,"property":"Background","value":"#FF3C705A"})).unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        STYLE.replace("#FF476188", "#FF3C705A")
    );
    assert!(call("designer_style_source", json!({"source":choice["source"]})).is_err());
    call("designer_undo", json!({"expected_revision":revision()})).unwrap();
    assert_eq!(session.borrow().authored_source(), STYLE);
    let back = button("DefinitionStyleBack");
    back.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(session.borrow().document_uri(), uri);
    assert_eq!(session.borrow().authored_source(), VIEW);

    let report = call("designer_palette", json!({"owner":owner})).unwrap();
    let row = report["comparison"]
        .as_array()
        .unwrap()
        .iter()
        .find(|color| color["key"] == "AccountAccent")
        .unwrap();
    assert_eq!(row["light"]["value"], "#FF3468AC");
    assert_eq!(row["dark"]["value"], "#FF8E4AA3");
    let opened = call(
        "designer_open_style_source",
        json!({"source":row["dark"]["source"],"expected_revision":revision()}),
    )
    .unwrap();
    assert_eq!(
        session.borrow().document_uri(),
        file_uri(root.join("Styling/Colors/Dark.xaml")).unwrap()
    );
    assert_eq!(opened["editor"]["layer"], "shared");
    let palette = button("DefinitionPalette");
    let flyout = palette.flyout().unwrap().cast::<Flyout>().unwrap();
    window.update_layout();
    flyout.show_at(&palette);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let editor = body
        .get_logical_descendants()
        .find_map(|control| {
            let field = control.cast::<TextBox>()?;
            (AutomationProperties::get_name(&field).as_deref() == Some("AccountAccent color value"))
                .then_some(field)
        })
        .unwrap();
    editor.set_text(Some("#FF347A71"));
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    editor.raise_event(&enter);
    assert_eq!(
        session.borrow().authored_source(),
        DARK.replace("#FF8E4AA3", "#FF347A71")
    );
    flyout.hide();
    let back = button("DefinitionStyleBack");
    back.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let report = call("designer_palette", json!({"owner":owner})).unwrap();
    let row = report["comparison"]
        .as_array()
        .unwrap()
        .iter()
        .find(|color| color["key"] == "AccountAccent")
        .unwrap();
    assert_eq!(row["dark"]["value"], "#FF347A71");
    assert_eq!(row["light"]["value"], "#FF3468AC");
    assert_eq!(session.borrow().authored_source(), VIEW);
    assert!(!session.borrow().can_undo());
    window.close();
}
