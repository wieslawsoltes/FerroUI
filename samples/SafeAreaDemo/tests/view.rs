//! The main view in the main window of the headless shell (`sample_testing::Shell`), and the
//! view model driven by an insets manager and an input pane of the test (the bases the platforms
//! build theirs on) in place of the ones of a mobile platform, which a window of the shell does
//! not have.

use crate::view_models::MainViewModel;
use crate::views::{MainView, MainWindow};
use crate::{smoke, App, ViewLocator, SAMPLE};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Rect, Ref, Thickness};
use ferroui_controls::platform::{
    IInputPane, IInsetsManager, InputPaneBase, InputPaneState, InputPaneStateEventArgs, InsetsManagerBase,
    SafeAreaChangedArgs,
};
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
use ferroui_controls::{Application, TextBlock, TopLevel};
use sample_testing::{assert_accepted, Shell};
use std::cell::RefCell;
use std::rc::Rc;

/// The size of the window of the test.
const SIZE: (f64, f64) = (800.0, 600.0);

/// The binding errors the sample is accepted to report. The transition of the marker button
/// binds its duration and its easing to the properties of the view model, which are null until
/// the input pane changes its state: the null converts to neither, as in the reference, whose
/// bindings report the same on a desktop.
const ACCEPTED: &[(&str, &str, usize)] = &[
    (
        "MainWindow",
        "ThicknessTransition.Duration <- InputPaneDuration: Could not convert '(null)' (null) to 'ferroui_base::animation::time_span::SignedDuration'.",
        1,
    ),
    (
        "MainWindow",
        "ThicknessTransition.Easing <- InputPaneEasing: Could not convert '(null)' (null) to 'ferroui_base::animation::easings::easing::Easing'.",
        1,
    ),
];

fn start() -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let window = MainWindow::new();
    window.show();
    shell.settle();
    (shell, window)
}

fn main_view(window: &MainWindow) -> Ref<MainView> {
    window.get_visual_descendants().find_map(|visual| visual.cast::<MainView>()).expect("the main view of the window")
}

/// The names of the properties a view model reports changed.
fn changes(view_model: &Rc<MainViewModel>) -> Rc<RefCell<Vec<String>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
    seen
}

#[test]
fn the_main_window_shows_the_main_view_with_its_view_model() {
    let (shell, window) = start();

    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<App>().is_some(), "the application is the application of the sample");
    assert_eq!(application.data_templates().count(), 1, "App.xaml declares one data template: the view locator");

    let view = main_view(&window);
    let view_model = smoke::main_view_model(&view).expect("the view sets its view model when it is loaded");
    // A window of the shell has neither an insets manager nor an input pane.
    assert!(TopLevel::get_top_level(Some(&view)).and_then(|top_level| top_level.insets_manager()).is_none());
    assert_eq!(view_model.safe_area_padding(), Thickness::default());
    assert_eq!(view_model.input_pane_state(), InputPaneState::Closed);
    // `TopLevel.AutoSafeAreaPadding` of the view is true by default, and the view model takes it.
    assert!(view_model.auto_safe_area_padding());
    // `Bounds="{Binding CanvasSize, Mode=OneWayToSource}"` of the grid of the view.
    assert_eq!((view_model.canvas_size().width, view_model.canvas_size().height), SIZE, "the bounds of the grid");

    assert!(shell.frames(0) > 0, "the compositor drew the window");
    let frame = shell.last_frame(0);
    assert!(frame.colors((0.0, 0.0, SIZE.0, SIZE.1)) > 2, "the view draws its borders, labels and options");

    let reports: Vec<(String, String, usize)> = shell
        .take_binding_reports()
        .into_iter()
        .map(|(report, count)| ("MainWindow".to_string(), report.line(), count))
        .collect();
    assert_accepted(&reports, ACCEPTED);

    window.close();
    drop(shell);
}

#[test]
fn automatic_paddings_follow_the_attached_property_of_the_view() {
    let (shell, window) = start();
    let view = main_view(&window);
    let view_model = smoke::main_view_model(&view).expect("the view model");

    view_model.set_auto_safe_area_padding(false);
    assert!(!TopLevel::get_auto_safe_area_padding(&view), "the two-way binding sets the attached property");
    TopLevel::set_auto_safe_area_padding(&view, true);
    assert!(view_model.auto_safe_area_padding(), "the two-way binding sets the view model");

    window.close();
    drop(shell);
}

#[test]
fn the_buttons_switch_the_theme_variant_of_the_application() {
    let (shell, window) = start();
    let view_model = smoke::main_view_model(&main_view(&window)).expect("the view model");
    let application = Application::current().expect("the application");

    view_model.set_dark();
    assert_eq!(application.requested_theme_variant(), Some(ThemeVariant::dark()));
    view_model.set_light();
    assert_eq!(application.requested_theme_variant(), Some(ThemeVariant::light()));
    view_model.set_default();
    assert_eq!(application.requested_theme_variant(), Some(ThemeVariant::default()));

    window.close();
    drop(shell);
}

#[test]
fn the_view_model_follows_the_insets_manager() {
    let (shell, window) = start();
    let view = main_view(&window);
    TopLevel::set_auto_safe_area_padding(&view, false);

    let insets = Rc::new(InsetsManagerBase::new());
    insets.set_display_edge_to_edge_preference(true);
    insets.set_is_system_bar_visible(Some(true));
    let padding = Thickness::new(0.0, 47.0, 0.0, 34.0);
    insets.set_safe_area_padding(padding);

    let view_model = MainViewModel::new();
    view_model.initialize(&view, Some(insets.clone() as Rc<dyn IInsetsManager>), None);
    assert!(view_model.display_edge_to_edge(), "the preference of the insets manager");
    assert!(!view_model.hide_system_bars(), "the system bars are visible");
    assert_eq!(view_model.safe_area_padding(), padding);
    assert_eq!(view_model.view_padding(), padding);

    view_model.set_use_safe_area(false);
    assert_eq!(view_model.view_padding(), Thickness::default(), "without the safe area the view is not padded");
    view_model.set_use_safe_area(true);

    view_model.set_hide_system_bars(true);
    assert_eq!(insets.is_system_bar_visible(), Some(false));
    view_model.set_display_edge_to_edge(false);
    assert!(!insets.display_edge_to_edge_preference());

    let seen = changes(&view_model);
    insets.on_safe_area_changed(SafeAreaChangedArgs::new(padding));
    assert_eq!(*seen.borrow(), ["SafeAreaPadding", "ViewPadding", "InputPaneMarkerMargin"]);

    // With automatic paddings the top-level pads the content, and the view does not.
    view_model.set_auto_safe_area_padding(true);
    assert_eq!(view_model.safe_area_padding(), Thickness::default());

    // A second initialization removes the handler of the first.
    view_model.initialize(&view, None, None);
    seen.borrow_mut().clear();
    insets.on_safe_area_changed(SafeAreaChangedArgs::new(padding));
    assert!(!seen.borrow().iter().any(|name| name == "SafeAreaPadding"), "the handler was removed: {:?}", seen.borrow());

    window.close();
    drop(shell);
}

#[test]
fn the_marker_follows_the_input_pane() {
    let (shell, window) = start();
    let view = main_view(&window);
    let input_pane = Rc::new(InputPaneBase::new());
    let view_model = MainViewModel::new();
    view_model.set_canvas_size(Rect::new(0.0, 0.0, 400.0, 800.0));
    view_model.initialize(&view, None, Some(input_pane.clone() as Rc<dyn IInputPane>));
    assert_eq!(view_model.input_pane_marker_margin(), Thickness::default());
    assert!(view_model.input_pane_duration().is_none());

    let seen = changes(&view_model);
    let occluded = Rect::new(0.0, 500.0, 400.0, 300.0);
    input_pane.set_occluded_rect(occluded);
    input_pane.set_state(InputPaneState::Open);
    input_pane.on_state_changed(InputPaneStateEventArgs::new(InputPaneState::Open, None, occluded));

    assert_eq!(view_model.input_pane_state(), InputPaneState::Open);
    assert_eq!(view_model.input_pane_rect(), occluded);
    assert_eq!(view_model.input_pane_marker_margin(), Thickness::new(0.0, 0.0, 0.0, 300.0));
    assert_eq!(view_model.input_pane_duration().map(|duration| duration.ticks()), Some(0));
    assert!(view_model.input_pane_easing().is_some(), "a linear easing when the platform states none");
    assert_eq!(
        *seen.borrow(),
        ["InputPaneState", "InputPaneRect", "InputPaneEasing", "InputPaneDuration", "InputPaneMarkerMargin"]
    );

    window.close();
    drop(shell);
}

#[test]
fn the_view_locator_builds_the_view_of_a_view_model() {
    let (shell, window) = start();
    let locator = ViewLocator::new();

    let view_model: BoxedValue = MainViewModel::new();
    assert!(locator.match_(Some(&view_model)), "a view model of the sample");
    let built = locator.build(&Some(view_model)).expect("a control");
    assert!(built.cast::<MainView>().is_some(), "SafeAreaDemo.ViewModels.MainViewModel builds SafeAreaDemo.Views.MainView");

    let text: BoxedValue = Rc::new(String::from("text"));
    assert!(!locator.match_(Some(&text)), "not a view model");
    assert!(!locator.match_(None));
    let built = locator.build(&Some(text)).expect("a control");
    let text_block = built.cast::<TextBlock>().expect("a text block when there is no view");
    assert!(text_block.text().is_some_and(|name| !name.is_empty()), "the text block names the type it looked for");
    assert!(locator.build(&None).is_none());

    window.close();
    drop(shell);
}
