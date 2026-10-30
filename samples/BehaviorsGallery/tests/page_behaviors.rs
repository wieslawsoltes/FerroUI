//! The behaviors of some ported pages, run in a window of the unit test application: a click on a
//! button runs the actions of its trigger, which change the view model or an element of the page.

use super::support::start_application;
use crate::pages::PageInfo;
use crate::view_models::MainWindowViewModel;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::IBrush;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, Control, Window};
use std::rc::Rc;

/// The page with the header `header`, created with the view model of the main window as its data
/// context (the context the pages inherit) and shown in a window.
fn show(header: &str, view_model: &Rc<MainWindowViewModel>) -> (Ref<Window>, Ref<Control>) {
    let page: &'static PageInfo =
        PageInfo::find(header).unwrap_or_else(|| panic!("the page {header}"));
    assert!(page.is_ported(), "{header} is ported");
    let window = Window::new();
    window.set_width(1000.0);
    window.set_height(700.0);
    window.set_data_context(Some(view_model.clone() as BoxedValue));
    let view = page.create_view();
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    (window, view)
}

/// Raises the click of the button named `name` of `view`.
fn click(view: &Control, name: &str) {
    let button = view
        .find_control::<Button>(name)
        .unwrap_or_else(|| panic!("the button {name}"));
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn call_method_action_calls_the_methods_of_the_view_model() {
    let _app = start_application();
    let view_model = MainWindowViewModel::new();
    let (window, view) = show("CallMethodAction", &view_model);

    click(&view, "IncrementButton");
    click(&view, "IncrementButton");
    assert_eq!(view_model.count(), 2);
    click(&view, "DecrementButton");
    assert_eq!(view_model.count(), 1);
    click(&view, "ResetButton");
    assert_eq!(view_model.count(), 0);
    window.close();
}

#[test]
fn invoke_command_action_runs_the_commands_of_the_view_model() {
    let _app = start_application();
    let view_model = MainWindowViewModel::new();
    let (window, view) = show("InvokeCommandAction", &view_model);

    click(&view, "MoveRightButton");
    assert_eq!(view_model.position(), 105.0);
    click(&view, "MoveLeftButton");
    click(&view, "MoveLeftButton");
    assert_eq!(view_model.position(), 95.0);
    click(&view, "ResetMoveButton");
    assert_eq!(view_model.position(), 100.0);
    window.close();
}

#[test]
fn change_property_action_changes_the_fill_of_the_rectangle() {
    let _app = start_application();
    let view_model = MainWindowViewModel::new();
    let (window, view) = show("ChangePropertyAction", &view_model);
    let rectangle = view
        .find_control::<Rectangle>("ChangePropertyRectangle")
        .expect("the rectangle");
    let color = |fill: Option<Rc<dyn IBrush>>| {
        fill.and_then(|fill| fill.as_solid_color_brush().map(|brush| brush.color()))
    };

    click(&view, "YellowButton");
    assert_eq!(
        color(rectangle.fill()),
        Some(ferroui_base::media::Colors::YELLOW)
    );
    click(&view, "PinkButton");
    assert_eq!(
        color(rectangle.fill()),
        Some(ferroui_base::media::Colors::PINK)
    );
    window.close();
}
