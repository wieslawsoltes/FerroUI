use super::*;
use crate::mcp::{DesignerMcp, Permissions};
use crate::PreviewTheme;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ComboBox, Window};
use ferroui_designer_project::{file_uri, Solution};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::UiToolProvider;
use serde_json::{json, Value};
use std::path::Path;

struct ThemeRestore(Option<ThemeVariant>);

impl Drop for ThemeRestore {
    fn drop(&mut self) {
        ThemeVariant::set_high_contrast_variant(self.0.take());
    }
}

#[test]
fn studio_high_contrast_palette_previews_native_fallback_and_undoes_local_override() {
    const SOURCE: &str =
        include_str!("../../editors/vscode/test/fixtures/HighContrastPalette.xaml");
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_themes_winui::register_types();
    let _restore_theme = ThemeRestore(ThemeVariant::high_contrast_variant());
    ThemeVariant::set_high_contrast_variant(
        Some(ferroui_themes_winui::WinUITheme::high_contrast()),
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../editors/vscode/test/fixtures/HighContrastPalette.xaml")
        .canonicalize()
        .unwrap();
    let workspace = DesignerWorkspace::with_uri(SOURCE, file_uri(&path).unwrap()).unwrap();
    workspace
        .set_solution(Solution::discover(path.parent().unwrap()).unwrap())
        .unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let session = workspace.session();
    let owner = session.borrow().document().ast().root();
    workspace.inspect_component(owner).unwrap();
    window.update_layout();
    let open = workspace
        .control()
        .get_logical_descendants()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| button.name().as_deref() == Some("DefinitionPalette"))
        .unwrap();
    let flyout = open.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.show_at(&open);
    window.update_layout();
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let click = |name| {
        let button = body
            .get_logical_descendants()
            .filter_map(|control| control.cast::<Button>())
            .find(|button| button.name().as_deref() == Some(name))
            .unwrap_or_else(|| panic!("Studio palette has no {name} button"));
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    };
    click("PalettePreviewHighContrast");
    assert_eq!(
        workspace.preview_environment().theme,
        PreviewTheme::HighContrast
    );
    click("PaletteCompare");
    let mcp = DesignerMcp::with_workspace(&workspace);
    let call = |tool, args| mcp.call(tool, &args, Permissions::default()).unwrap();
    let read = || call("designer_palette", json!({"owner":owner}));
    let effective = |node| {
        call(
            "designer_property_trace",
            json!({"node":node,"property":"Background"}),
        )
    };
    let accent = session
        .borrow()
        .document()
        .ast()
        .find_name("AccentCard")
        .unwrap();
    let palette = read();
    assert_eq!(palette["contrastPreview"], true);
    let resolved = |key: &str| -> &Value {
        palette["comparison"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("Missing {key} in the accepted palette {palette}"))
    };
    let surface = &resolved("PanelSurface")["high_contrast"];
    assert_eq!(surface["value"], "#FF0A0B0D");
    assert!(surface["source"]["uri"]
        .as_str()
        .unwrap()
        .ends_with("/High%20Contrast.xaml"));
    assert_eq!(
        resolved("PanelAccent")["high_contrast"],
        resolved("PanelAccent")["dark"]
    );
    assert_eq!(effective(owner)["effective"], "#FF0A0B0D");
    assert_eq!(effective(accent)["effective"], "#FF24334B");
    assert!(!session.borrow().can_undo());

    let layers = body
        .get_logical_descendants()
        .filter_map(|control| control.cast::<ComboBox>())
        .find(|select| select.name().as_deref() == Some("PaletteLayer"))
        .unwrap();
    let contrast = ferroui_designer_tools::component_palette::PaletteLayer::HighContrast;
    let index = ferroui_designer_tools::component_palette::PaletteLayer::ALL
        .iter()
        .position(|layer| *layer == contrast)
        .unwrap();
    layers.set_selected_index(index as i32);
    for (name, value) in [
        ("PaletteResourceKey", "PanelSurface"),
        ("PaletteNewColor", "#FF10141A"),
    ] {
        let field = body
            .get_logical_descendants()
            .filter_map(|control| control.cast::<TextBox>())
            .find(|field| field.name().as_deref() == Some(name))
            .unwrap();
        field.set_text(Some(value));
    }
    click("PaletteAddColor");
    assert!(session
        .borrow()
        .authored_source()
        .contains(":WinUITheme.HighContrast}"));
    assert_eq!(effective(owner)["effective"], "#FF10141A");
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(effective(owner)["effective"], "#FF0A0B0D");
    assert!(!session.borrow().can_undo());
    let appearance = call("designer_preview_appearance", json!({}));
    assert_eq!(appearance["environment"]["theme"], "high_contrast");
    let change = json!({
        "theme":"dark","expected_uri":appearance["uri"],"expected_revision":appearance["revision"]
    });
    let write = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    assert!(mcp
        .call(
            "designer_preview_set_theme",
            &change,
            Permissions::default()
        )
        .is_err());
    let mut retired = change.clone();
    retired["expected_uri"] = json!("memory://retired.xaml");
    assert!(mcp
        .call("designer_preview_set_theme", &retired, write)
        .is_err());
    let updated = mcp
        .call("designer_preview_set_theme", &change, write)
        .unwrap();
    assert_eq!(updated["environment"]["theme"], "dark");
    assert_eq!(updated["revision"], appearance["revision"]);
    assert_eq!(effective(owner)["effective"], "#FF171C28");
    assert!(!session.borrow().can_undo());
    flyout.hide();
    window.close();
}
