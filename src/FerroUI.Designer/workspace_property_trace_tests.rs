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

#[test]
fn property_trace_opens_the_included_stylesheet_at_its_effective_value() {
    use ferroui_designer_project::{file_uri, Solution};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../editors/vscode/test/fixtures").canonicalize().unwrap();
    let view = file_uri(fixtures.join("IncludedStylePalette.xaml")).unwrap();
    let style = file_uri(fixtures.join("Styling/Shared Styles.xaml")).unwrap();
    let source = include_str!("../../editors/vscode/test/fixtures/IncludedStylePalette.xaml");
    let workspace = DesignerWorkspace::with_uri(source, view).unwrap();
    workspace.set_solution(Solution::discover(&fixtures).unwrap()).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let node = session.borrow().document().ast().find_name("SharedStyleCard").unwrap();
    session.borrow_mut().select([node]);
    let context = TraceContext {
        workspace: Rc::downgrade(&workspace),
        snapshot: DesignTimeSnapshot::capture(&session.borrow()),
        target: TraceTarget::Authored(node),
        property: "Background".into(),
    };
    let body = context.content(&Flyout::new()).unwrap();
    let reveal = body.get_logical_descendants().find_map(|control| {
        let button = control.cast::<Button>()?;
        (AutomationProperties::get_name(&button).as_deref() == Some("Reveal Style Border.account in source"))
            .then_some(button)
    }).expect("the winning included setter has a source button");
    reveal.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let active = session.borrow();
    assert_eq!(active.document_uri(), style);
    let offset = active.authored_source().find("#FF476188").unwrap();
    let utf16 = super::super::source_tools::utf16_from_byte;
    assert_eq!(workspace.source.selection_start(), utf16(active.authored_source(), offset));
    assert_eq!(workspace.source.selection_end(), utf16(active.authored_source(), offset + 9));
    assert!(!active.can_undo());
    drop(active);
    window.close();
}
