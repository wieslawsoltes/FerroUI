//! The tests of the sample: the generated locators show the pages, the metrics, the reports and the
//! views of ReactiveUI.

use crate::view_models::reactive::{ContextHost, HomeViewModel, ReactiveDemoViewModel, SettingsViewModel as ReactiveSettingsViewModel};
use crate::view_models::{DashboardViewModel, MainWindowViewModel, MetricViewModel, ReportsViewModel, SettingsViewModel, TestViewModel};
use crate::{ReactiveViewLocator, ViewLocator};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, TextBlock};
use ferroui_reactiveui::IViewLocator;
use ferroui_reactiveui_controls::view_control;
use std::any::Any;
use std::rc::Rc;

fn class_name(control: &Ref<Control>) -> &'static str {
    control.get_type().name()
}

fn build(locator: &dyn IDataTemplate, view_model: BoxedValue) -> Ref<Control> {
    locator.build(&Some(view_model)).expect("a view")
}

#[test]
fn the_window_locator_shows_every_page() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let locator = ViewLocator;
    let pages: [(BoxedValue, &str); 5] = [
        (Rc::new(DashboardViewModel::new()), "DashboardView"),
        (Rc::new(TestViewModel), "TestView"),
        (Rc::new(SettingsViewModel::new()), "SettingsView"),
        (Rc::new(ReportsViewModel::new()), "ReportsView"),
        (Rc::new(ReactiveDemoViewModel::new()), "ReactiveDemoView"),
    ];
    for (view_model, view) in pages {
        assert!(locator.match_(Some(&view_model)), "{view}");
        assert_eq!(class_name(&build(&locator, view_model)), view);
    }
    let metric: BoxedValue = Rc::new(MetricViewModel::new("Active Users", "people", 1280.0, ""));
    assert_eq!(class_name(&build(&locator, metric)), "MetricView");
}

#[test]
fn the_window_has_no_view_of_its_own() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let locator = ViewLocator;
    let window: BoxedValue = Rc::new(MainWindowViewModel::new());
    assert!(!locator.match_(Some(&window)));
    let missing = build(&locator, window).cast::<TextBlock>().expect("the text of the missing view");
    assert_eq!(missing.text().as_deref(), Some("Not Found: static_view_locator_demo::views::main_window_view::MainWindowView"));
}

#[test]
fn a_view_shows_its_view_model() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let view = build(&ViewLocator, Rc::new(DashboardViewModel::new())).cast::<ContentControl>().expect("a user control");
    assert!(view.content().is_some(), "the content is built from the data context");
}

#[test]
fn the_reactive_locator_shows_views_of_reactive_ui_and_the_context_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let locator = ReactiveViewLocator;

    let home: Rc<dyn Any> = Rc::new(HomeViewModel);
    let view = locator.resolve_view(Some(home.clone()), None).expect("a view of ReactiveUI");
    let control = view_control(&*view).expect("the control of the view");
    assert_eq!(class_name(&control), "HomeView");
    assert!(view.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &home)));

    let settings = Rc::new(ReactiveSettingsViewModel);
    let host: BoxedValue = Rc::new(ContextHost::new(settings));
    assert!(locator.match_(Some(&host)));
    let fallback = build(&locator, host);
    assert_eq!(class_name(&fallback), "ContentControl");
}
