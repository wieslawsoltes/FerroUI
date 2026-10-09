//! Port of `Logging/LoggingTests.cs` (base unit tests): binding errors of a control in a window, so the tests live
//! with the controls.
//!
//! Upstream loads the window from markup. The trees are built in code here: the binding of the rectangle is the
//! string-path binding the markup declares (`$parent[Window].Background`, `$parent[Grid].Background`), with a type
//! resolver for the two class names where the markup loader has its own.

use crate::shapes::{Rectangle, Shape};
use crate::testing::{TestLogSink, TestServices, UnitTestApplication};
use crate::{Control, Grid, Panel, Window};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::parsers::TypeResolver;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::{FerroObject, Ref};
use std::cell::Cell;
use std::rc::Rc;

fn type_resolver() -> TypeResolver {
    Rc::new(|_, name| match name {
        "Window" => Some(CastTarget::Class(Window::TYPE)),
        "Grid" => Some(CastTarget::Class(Grid::TYPE)),
        _ => None,
    })
}

/// The window of the markup of the tests: a panel with a rectangle whose fill is bound to `path`. The panel and
/// the rectangle are handed out with it, where upstream finds them by name.
fn load(path: &str, panel_name: Option<&str>, rect_name: Option<&str>) -> (Ref<Window>, Ref<Panel>, Ref<Rectangle>) {
    let rect = Rectangle::new();
    rect.set_name(rect_name.map(str::to_string));
    rect.bind_binding(Shape::fill_property(), &ReflectionBinding::new(path).with_type_resolver(Some(type_resolver())));

    let panel = Panel::new();
    panel.set_name(panel_name.map(str::to_string));
    panel.children().add(&rect);

    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    (window, panel, rect)
}

#[test]
fn control_should_not_log_binding_errors_when_detached_from_visual_tree() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let (window, panel, rect) = load("$parent[Window].Background", Some("panel"), Some("rect"));
    let called_times = Rc::new(Cell::new(0));
    let counter = called_times.clone();
    let log_sink = TestLogSink::start(move |l, _, _, _, _| {
        if l >= LogEventLevel::Warning {
            counter.set(counter.get() + 1);
        }
    });
    window.apply_template();
    window.presenter().expect("the window has a presenter").apply_template();
    panel.children().remove(&rect);
    assert_eq!(0, called_times.get());

    log_sink.dispose();
}

#[test]
fn control_should_log_binding_errors_when_no_ancestor_with_such_name() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let called_times = Rc::new(Cell::new(0));
    let counter = called_times.clone();
    let log_sink = TestLogSink::start(move |l, _, s, _, _| {
        let is_rectangle =
            s.and_then(|s| s.downcast_ref::<FerroObject>()).is_some_and(|s| s.downcast_ref::<Rectangle>().is_some());
        if l >= LogEventLevel::Warning && is_rectangle {
            counter.set(counter.get() + 1);
        }
    });
    let (window, _panel, _rect) = load("$parent[Grid].Background", None, None);
    window.apply_template();
    window.presenter().expect("the window has a presenter").apply_template();
    assert_eq!(1, called_times.get());

    log_sink.dispose();
}
