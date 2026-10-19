//! Not a port (the reference has no tests): the main window built in the unit-test application,
//! with ReactiveUI built as the entry point builds it: the bindings its code-behind makes when it
//! is activated, and the view the items of its list are shown with.

use super::support::{catalogue_package, run_jobs, start_application, RecordingLauncher};
use crate::{MainWindow, NugetDetailsView, NugetDetailsViewModel};
use ferroui_base::input::{InputElement, TextInputEventArgs};
use ferroui_reactiveui::{IViewFor, IViewLocator, ViewLocator};
use ferroui_reactiveui_controls::view_control;
use std::any::Any;
use std::rc::Rc;

/// Types `text` into `text_box` (a text input event, as the keyboard raises it).
fn type_text(text_box: &ferroui_controls::TextBox, text: &str) {
    let mut args = TextInputEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_event()));
    args.text = Some(text.to_owned());
    text_box.raise_event(&args);
}

#[test]
fn typing_into_the_search_box_updates_the_view_model() {
    let _app = start_application();
    let window = MainWindow::new();
    window.show();
    run_jobs();

    let view_model = window.view_model().expect("the window has its view model");
    let text_box = window.search_text_box();
    assert_eq!(view_model.search_term(), None);

    // Bind(ViewModel, vm => vm.SearchTerm, view => view.searchTextBox.Text): from the view ...
    type_text(&text_box, "rx");
    run_jobs();
    assert_eq!(text_box.text().as_deref(), Some("rx"));
    assert_eq!(view_model.search_term().as_deref(), Some("rx"));

    // ... and from the view model.
    view_model.set_search_term(Some("splat".to_owned()));
    run_jobs();
    assert_eq!(text_box.text().as_deref(), Some("splat"));

    window.close();
}

#[test]
fn the_list_is_hidden_until_there_are_results() {
    let _app = start_application();
    let window = MainWindow::new();
    window.show();
    run_jobs();

    // OneWayBind(ViewModel, vm => vm.IsAvailable, view => view.searchResultsListBox.Visibility).
    let view_model = window.view_model().expect("the window has its view model");
    assert!(!view_model.is_available());
    assert!(!window.search_results_list_box().is_visible());

    window.close();
}

#[test]
fn the_view_of_a_package_is_located_for_its_view_model() {
    let _app = start_application();
    let launcher = RecordingLauncher::new();
    let view_model: Rc<dyn Any> = NugetDetailsViewModel::new(catalogue_package("DynamicData"), RecordingLauncher::source(&launcher));

    // What the view host of an item of the list does with its view model: the views that
    // `ViewRegistrar` registers are in the service locator, which the hosts also ask.
    let view = ViewLocator::current().resolve_view_unsafe(Some(view_model.clone()), None).expect("a view is registered for the view model");
    view.set_view_model(Some(view_model.clone()));

    let control = view_control(&*view).expect("the view is a control");
    let details = control.cast::<NugetDetailsView>().expect("the view is a NugetDetailsView");
    let shown = details.view_model().expect("the control has the view model");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&shown), Rc::as_ptr(&view_model)));
}
