use super::{ScrollViewerIRefreshInfoProviderAdapter, ScrollablePullGestureRecognizer};
use crate::scroll_viewer_tests::create_template;
use crate::templates::FuncControlTemplate;
use crate::test_support::{test_scope, TestRoot};
use crate::text_box_tests_input::TouchTestHelper;
use crate::{Border, Control, ScrollViewer};
use ferroui_base::input::{InputElement, PullDirection};
use ferroui_base::{Point, Ref, Size, Vector};

fn pull_recognizer_count(element: &InputElement) -> usize {
    element
        .gesture_recognizers()
        .to_vec()
        .into_iter()
        .filter(|recognizer| recognizer.is::<ScrollablePullGestureRecognizer>())
        .count()
}

// Repro for the "gesture recognizer leaked across adapt calls" bug.
//
// `adapt` cleans up the pointer handlers of the previous scroll viewer and the pull
// event handlers of the previous refresh info provider, but never removed the
// previously created scrollable pull gesture recognizer from the gesture recognizers
// of the interaction source. Each subsequent `adapt` instantiates and adds a new
// recognizer, so the input element ends up holding N recognizers after N calls. They
// all listen for the same pointer events and raise duplicate pull gesture and pull
// gesture ended pairs, which (combined with the `entered` desync fix in the refresh
// info provider) corrupts the visualizer state.
#[test]
fn adapt_called_twice_does_not_leak_pull_gesture_recognizer() {
    let _scope = test_scope();
    let sv = ScrollViewer::new();
    sv.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    sv.set_content(Some(Control::boxed(Border::new())));

    // Wrap in a test root and execute the layout pass so Loaded fires and the visual
    // tree under the scroll content presenter is fully wired up (otherwise the adapter
    // never reaches `make_interaction_source`).
    let root = TestRoot::with_child(sv.clone());
    root.layout_manager().execute_initial_layout_pass();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);

    adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));
    let interaction_source = adapter.interaction_source();
    let interaction_source = interaction_source.expect("interaction source");
    let after_first = pull_recognizer_count(&interaction_source);
    assert_eq!(after_first, 1);

    adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));
    let interaction_source_after = adapter.interaction_source();
    let interaction_source_after = interaction_source_after.expect("interaction source");
    let after_second = pull_recognizer_count(&interaction_source_after);
    assert_eq!(after_second, 1);
}

// --- Additional tests (not upstream); expectations derived from the reference source. ---

/// A scroll viewer whose content is taller than the viewport, laid out in a root.
fn scrolling_viewer() -> (Ref<ScrollViewer>, Ref<TestRoot>) {
    let content = Border::new();
    content.set_width(100.0);
    content.set_height(3000.0);

    let sv = ScrollViewer::new();
    sv.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    sv.set_content(Some(Control::boxed(content)));

    let root = TestRoot::with_child(sv.clone());
    root.layout_manager().execute_initial_layout_pass();
    (sv, root)
}

#[test]
fn adapt_uses_the_logical_parent_of_the_content_as_interaction_source() {
    let _scope = test_scope();
    let (sv, _root) = scrolling_viewer();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::LeftToRight, true);
    let provider = adapter.adapt(&sv, Some(Size::new(40.0, 60.0)));

    assert_eq!(provider.pull_direction(), PullDirection::LeftToRight);
    assert_eq!(provider.refresh_visualizer_size(), Size::new(40.0, 60.0));
    assert!(adapter.interaction_source().is_some_and(|source| source == sv.clone().upcast::<InputElement>()));

    let recognizer = sv
        .gesture_recognizers()
        .to_vec()
        .into_iter()
        .find_map(|recognizer| recognizer.cast::<ScrollablePullGestureRecognizer>())
        .expect("pull recognizer");
    assert_eq!(recognizer.pull_direction(), PullDirection::LeftToRight);
    assert!(recognizer.is_mouse_enabled());

    adapter.update_pull_direction(PullDirection::BottomToTop);
    adapter.update_is_mouse_enabled(false);
    adapter.update_visualizer_size(Some(Size::new(10.0, 20.0)));
    assert_eq!(provider.pull_direction(), PullDirection::BottomToTop);
    assert_eq!(recognizer.pull_direction(), PullDirection::BottomToTop);
    assert!(!recognizer.is_mouse_enabled());
    assert_eq!(provider.refresh_visualizer_size(), Size::new(10.0, 20.0));

    adapter.update_visualizer_size(None);
    assert_eq!(provider.refresh_visualizer_size(), Size::default());
}

#[test]
fn adapt_before_the_content_is_attached_waits_for_loaded() {
    let _scope = test_scope();
    let sv = ScrollViewer::new();
    sv.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    sv.set_content(Some(Control::boxed(Border::new())));

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    adapter.adapt(&sv, None);

    // The content has no visual parent yet: no interaction source.
    assert!(adapter.interaction_source().is_none());
    assert_eq!(pull_recognizer_count(&sv), 0);
}

#[test]
#[should_panic(expected = "Adaptee's content property cannot be null.")]
fn adapt_requires_content() {
    let _scope = test_scope();
    let sv = ScrollViewer::new();
    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    adapter.adapt(&sv, None);
}

#[test]
fn adapt_from_tree_finds_a_nested_scroll_viewer() {
    let _scope = test_scope();
    let content = Border::new();
    let sv = ScrollViewer::new();
    sv.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    sv.set_content(Some(Control::boxed(content)));

    let inner = Border::new();
    inner.set_child(sv.clone());
    let outer = Border::new();
    outer.set_child(inner);

    let root = TestRoot::with_child(outer.clone());
    root.layout_manager().execute_initial_layout_pass();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    assert!(adapter.adapt_from_tree(&outer, None).is_some());
    assert!(adapter.interaction_source().is_some_and(|source| source == sv.clone().upcast::<InputElement>()));

    // The scroll viewer itself is adapted directly.
    let direct = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    assert!(direct.adapt_from_tree(&sv, None).is_some());

    // A tree without a scroll viewer adapts nothing.
    let none = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    assert!(none.adapt_from_tree(&Border::new(), None).is_none());
}

#[test]
fn pull_gesture_updates_is_interacting_for_refresh_and_interaction_ratio() {
    let _scope = test_scope();
    let (sv, _root) = scrolling_viewer();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    let provider = adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));

    let touch = TouchTestHelper::new();

    touch.down(&sv, Point::new(10.0, 20.0));
    assert!(!provider.peeking_mode());
    assert!(!provider.is_interacting_for_refresh());

    touch.move_(&sv, Point::new(10.0, 70.0));
    assert!(provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.5);

    touch.move_(&sv, Point::new(10.0, 110.0));
    assert!(provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.9);

    // Pulls longer than the visualizer are capped.
    touch.move_(&sv, Point::new(10.0, 400.0));
    assert_eq!(provider.interaction_ratio(), 1.0);

    touch.up(&sv, Point::new(10.0, 400.0));
    assert!(!provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.0);
}

#[test]
fn scrolling_away_from_the_edge_clears_is_interacting_for_refresh() {
    let _scope = test_scope();
    let (sv, root) = scrolling_viewer();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    let provider = adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));

    let touch = TouchTestHelper::new();
    touch.down(&sv, Point::new(10.0, 20.0));
    touch.move_(&sv, Point::new(10.0, 70.0));
    assert!(provider.is_interacting_for_refresh());

    // A scroll change within the threshold keeps the interaction.
    sv.set_offset(Vector::new(0.0, 0.5));
    root.layout_manager().execute_layout_pass();
    assert!(provider.is_interacting_for_refresh());

    sv.set_offset(Vector::new(0.0, 200.0));
    root.layout_manager().execute_layout_pass();
    assert_eq!(sv.offset(), Vector::new(0.0, 200.0));
    assert!(!provider.is_interacting_for_refresh());

    touch.cancel();
}

#[test]
fn pointer_pressed_away_from_the_edge_enters_peeking_mode() {
    let _scope = test_scope();
    let (sv, root) = scrolling_viewer();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    let provider = adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));

    sv.set_offset(Vector::new(0.0, 200.0));
    root.layout_manager().execute_layout_pass();

    let touch = TouchTestHelper::new();
    touch.down(&sv, Point::new(10.0, 20.0));
    assert!(provider.peeking_mode());

    // No pull starts while the scroll viewer is not at the edge.
    touch.move_(&sv, Point::new(10.0, 70.0));
    assert!(!provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.0);
    touch.up(&sv, Point::new(10.0, 70.0));

    sv.set_offset(Vector::new(0.0, 0.0));
    root.layout_manager().execute_layout_pass();
    touch.down(&sv, Point::new(10.0, 20.0));
    assert!(!provider.peeking_mode());
    touch.up(&sv, Point::new(10.0, 20.0));
}

#[test]
fn pointer_released_on_the_scroll_viewer_clears_is_interacting_for_refresh() {
    let _scope = test_scope();
    let (sv, _root) = scrolling_viewer();

    let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(PullDirection::TopToBottom, false);
    let provider = adapter.adapt(&sv, Some(Size::new(100.0, 100.0)));

    provider.set_is_interacting_for_refresh(true);
    assert!(provider.is_interacting_for_refresh());

    // A press and release without a pull: the release reaches the scroll viewer.
    let touch = TouchTestHelper::new();
    touch.down(&sv, Point::new(10.0, 20.0));
    touch.up(&sv, Point::new(10.0, 20.0));
    assert!(!provider.is_interacting_for_refresh());
}
