use super::support::start_themed_application;
use crate::MaterialSearchView;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::{Border, Button, Control, Window};
use std::cell::Cell;
use std::rc::Rc;

fn host() -> (Ref<Window>, Ref<MaterialSearchView>) {
    let view = MaterialSearchView::new();
    view.set_reduced_motion(true);
    view.set_expand_on_focus(false);
    let content = Border::new();
    content.set_height(180.0);
    view.set_content(Some(Control::boxed(&content)));
    view.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    window.update_layout();
    (window, view)
}

fn named<T: ObjectType>(view: &MaterialSearchView, name: &str) -> Ref<T> {
    view.get_visual_descendants()
        .find_map(|node| {
            (node.name().as_deref() == Some(name))
                .then(|| node.cast::<T>())
                .flatten()
        })
        .unwrap_or_else(|| panic!("missing search part {name}"))
}

#[test]
fn query_is_two_way_and_expansion_preserves_editor_and_selection() {
    let _app = start_themed_application();
    let (window, view) = host();
    let editor = view.editor().unwrap();
    view.set_query(Some("Material".to_string()));
    assert_eq!(Some("Material".to_string()), editor.text());
    editor.set_selection_start(2);
    editor.set_selection_end(5);
    view.expand();
    window.update_layout();
    assert_eq!(editor, view.editor().unwrap());
    assert_eq!(2, editor.selection_start());
    assert_eq!(5, editor.selection_end());
    editor.set_text(Some("Updated"));
    assert_eq!(Some("Updated".to_string()), view.query());
    view.collapse();
    assert_eq!(editor, view.editor().unwrap());
}

#[test]
fn docked_search_has_native_input_and_minimum_expanded_height() {
    let _app = start_themed_application();
    let (window, view) = host();
    assert_eq!(56.0, view.bounds().height);
    assert_eq!(360.0, view.min_width());
    assert_eq!(720.0, view.max_width());
    view.expand();
    window.update_layout();
    assert_eq!(240.0, view.bounds().height);
    view.collapse();
    window.update_layout();
    assert_eq!(56.0, view.bounds().height);
}

#[test]
fn actual_clear_and_back_buttons_keep_query_and_result_state_consistent() {
    let _app = start_themed_application();
    let (window, view) = host();
    view.set_query(Some("Keep me".to_string()));
    view.expand();
    window.update_layout();
    let clear: Ref<Button> = named(&view, "PART_ClearButton");
    clear.raise_event(
        &ferroui_base::interactivity::RoutedEventArgs::with_event_and_source(
            Button::click_event(),
            clear.to_ref(),
        ),
    );
    assert_eq!(None, view.query());
    assert!(view.is_expanded());
    view.set_query(Some("Remember".to_string()));
    let back: Ref<Button> = named(&view, "PART_BackButton");
    back.raise_event(
        &ferroui_base::interactivity::RoutedEventArgs::with_event_and_source(
            Button::click_event(),
            back.to_ref(),
        ),
    );
    assert!(!view.is_expanded());
    assert_eq!(Some("Remember".to_string()), view.query());
}

#[test]
fn submit_is_a_single_routed_action_and_does_not_clear_the_editor() {
    let _app = start_themed_application();
    let (_, view) = host();
    view.set_query(Some("forms".to_string()));
    let count = Rc::new(Cell::new(0));
    let callback = count.clone();
    view.query_submitted(move |_, _| callback.set(callback.get() + 1));
    view.submit_query();
    assert_eq!(1, count.get());
    assert_eq!(Some("forms".to_string()), view.query());
}

#[test]
fn native_docked_motion_animates_constraints_and_keeps_revealed_results_opaque() {
    let _app = start_themed_application();
    let (window, view) = host();
    view.set_reduced_motion(false);
    view.expand();
    view.advance_motion(0.1);
    window.update_layout();
    // At100ms the native SlowSpatial spring's interpolated screen cap already
    // exceeds240dp. Short content is fully revealed while the spring continues.
    assert_eq!(240.0, view.bounds().height);
    let results: Ref<ferroui_controls::ScrollViewer> = named(&view, "PART_Results");
    assert_eq!(1.0, results.opacity());
    view.set_reduced_motion(true);
    view.collapse();
    window.update_layout();
    assert_eq!(56.0, view.bounds().height);
}
