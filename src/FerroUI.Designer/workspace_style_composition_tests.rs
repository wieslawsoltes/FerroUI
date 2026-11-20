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
