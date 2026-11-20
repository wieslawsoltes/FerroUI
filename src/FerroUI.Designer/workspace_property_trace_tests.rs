use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::{
    TextBox, Window,
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
};
use ferroui_markup_xaml::FromXamlObject;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/PropertyTraceWorkbench.xaml");

#[test]
fn property_trace_flyout_reveals_the_exact_winning_setter_without_editing() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let session = workspace.session();
    let node = session
        .borrow()
        .document()
        .ast()
        .find_name("StyleCard")
        .unwrap();
    session.borrow_mut().select([node]);
    workspace
        .control()
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("Opacity"));
    workspace.refresh(true);
    window.update_layout();
    let button = workspace
        .control()
        .get_visual_descendants()
        .find_map(|visual| {
            let button = visual.cast::<Button>()?;
            (AutomationProperties::get_name(&button).as_deref() == Some("Trace Opacity value"))
                .then_some(button)
        })
        .expect("the property row offers a trace");
    let flyout = button
        .flyout()
        .and_then(|flyout| flyout.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&button);
    window.update_layout();
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let reveal = body
        .get_logical_descendants()
        .find_map(|control| {
            let button = control.cast::<Button>()?;
            (AutomationProperties::get_name(&button).as_deref()
                == Some("Reveal Style Border.priority in source"))
            .then_some(button)
        })
        .expect("the native winner has a source button");
    reveal.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let style = SOURCE.find("Border.priority").unwrap();
    let offset = style + SOURCE[style..].find("0.72").unwrap();
    assert_eq!(
        workspace.source.selection_start(),
        super::super::source_tools::utf16_from_byte(SOURCE, offset)
    );
    assert_eq!(
        workspace.source.selection_end(),
        super::super::source_tools::utf16_from_byte(SOURCE, offset + 4)
    );
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().can_undo());
    window.close();
}
