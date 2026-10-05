//! Additional tests (not upstream) of the wiring the refresh container does
//! between its visualizer, the adapter and the scroll viewer in its content;
//! expectations derived from the reference source.

use super::{RefreshContainer, RefreshVisualizer, RefreshVisualizerState, ScrollablePullGestureRecognizer};
use crate::scroll_viewer_tests::create_template;
use crate::templates::FuncControlTemplate;
use crate::test_support::{test_scope, TestRoot};
use crate::testing::add_pull_to_refresh_themes;
use crate::text_box_tests_input::TouchTestHelper;
use crate::{Border, Control, Grid, PathIcon, ScrollViewer};
use ferroui_base::input::{InputElement, PullDirection};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::Styles;
use ferroui_base::{Point, Ref, Size, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Hosted {
    container: Ref<RefreshContainer>,
    scroll_viewer: Ref<ScrollViewer>,
    /// Keeps the tree rooted.
    _root: Ref<TestRoot>,
}

impl Hosted {
    fn visualizer(&self) -> Ref<RefreshVisualizer> {
        self.container.visualizer().expect("visualizer")
    }

    fn presenter(&self) -> Ref<Grid> {
        self.visualizer().get_visual_parent().and_then(|parent| parent.cast::<Grid>()).expect("visualizer presenter")
    }

    fn pull_recognizer(&self) -> Ref<ScrollablePullGestureRecognizer> {
        let recognizers: Vec<_> = self
            .scroll_viewer
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .filter_map(|recognizer| recognizer.cast::<ScrollablePullGestureRecognizer>())
            .collect();
        assert_eq!(recognizers.len(), 1);
        recognizers[0].clone()
    }
}

/// A refresh container around a scroll viewer, themed as in the reference theme and
/// laid out in a root.
fn host(configure: impl FnOnce(&Ref<RefreshContainer>)) -> Hosted {
    let content = Border::new();
    content.set_height(3000.0);

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    scroll_viewer.set_content(Some(Control::boxed(content)));

    let container = RefreshContainer::new();
    container.set_content(Some(Control::boxed(scroll_viewer.clone())));
    configure(&container);

    let root = TestRoot::new();
    let styles = Styles::new();
    add_pull_to_refresh_themes(&styles);
    root.styles().add(styles);
    root.set_child(container.clone());
    root.layout_manager().execute_initial_layout_pass();

    Hosted { container, scroll_viewer, _root: root }
}

#[test]
fn defaults() {
    let _scope = test_scope();
    let container = RefreshContainer::new();

    assert_eq!(container.pull_direction(), PullDirection::TopToBottom);
    assert!(!container.is_mouse_enabled());
    assert!(container.visualizer().is_none());
    assert!(container.refresh_info_provider_adapter().is_some());

    // Without a visualizer a refresh request does nothing.
    container.request_refresh();
}

#[test]
fn applying_the_template_creates_the_default_visualizer_in_the_presenter_part() {
    let _scope = test_scope();
    let hosted = host(|_| {});

    let visualizer = hosted.visualizer();
    let presenter = hosted.presenter();
    assert_eq!(presenter.name().as_deref(), Some("PART_RefreshVisualizerPresenter"));
    assert_eq!(presenter.children().count(), 1);
    assert_eq!(presenter.vertical_alignment(), VerticalAlignment::Top);
    assert_eq!(presenter.horizontal_alignment(), HorizontalAlignment::Stretch);

    assert_eq!(visualizer.pull_direction(), PullDirection::TopToBottom);
    assert_eq!(visualizer.height(), f64::from(RefreshContainer::DEFAULT_PULL_DIMENSION_SIZE));
    assert!(visualizer.width().is_nan());

    // The theme of the visualizer gives it the glyph as content.
    assert!(visualizer.content().as_ref().and_then(Control::from_boxed).is_some_and(|icon| icon.is::<PathIcon>()));
}

#[test]
fn the_adapter_connects_the_visualizer_to_the_scroll_viewer() {
    let _scope = test_scope();
    let hosted = host(|_| {});

    let visualizer = hosted.visualizer();
    let adapter = hosted.container.refresh_info_provider_adapter().expect("adapter");
    let provider = visualizer.refresh_info_provider().expect("refresh info provider");

    // The interaction source is the logical parent of the scrolled content.
    assert!(adapter
        .interaction_source()
        .is_some_and(|source| source == hosted.scroll_viewer.clone().upcast::<InputElement>()));
    let recognizer = hosted.pull_recognizer();
    assert_eq!(recognizer.pull_direction(), PullDirection::TopToBottom);
    assert!(!recognizer.is_mouse_enabled());

    assert_eq!(provider.pull_direction(), PullDirection::TopToBottom);
    assert_eq!(visualizer.bounds().height, 100.0);
    assert_eq!(provider.refresh_visualizer_size(), visualizer.bounds().size());

    hosted.container.set_is_mouse_enabled(true);
    assert!(recognizer.is_mouse_enabled());
}

#[test]
fn pull_direction_changes_reach_the_presenter_the_visualizer_and_the_adapter() {
    let _scope = test_scope();
    let hosted = host(|_| {});

    let visualizer = hosted.visualizer();
    let presenter = hosted.presenter();
    let provider = visualizer.refresh_info_provider().expect("refresh info provider");
    let recognizer = hosted.pull_recognizer();
    let size = f64::from(RefreshContainer::DEFAULT_PULL_DIMENSION_SIZE);

    hosted.container.set_pull_direction(PullDirection::BottomToTop);
    assert_eq!(presenter.vertical_alignment(), VerticalAlignment::Bottom);
    assert_eq!(presenter.horizontal_alignment(), HorizontalAlignment::Stretch);
    assert_eq!(visualizer.pull_direction(), PullDirection::BottomToTop);
    assert_eq!(visualizer.height(), size);
    assert!(visualizer.width().is_nan());
    assert_eq!(provider.pull_direction(), PullDirection::BottomToTop);
    assert_eq!(recognizer.pull_direction(), PullDirection::BottomToTop);

    hosted.container.set_pull_direction(PullDirection::LeftToRight);
    assert_eq!(presenter.vertical_alignment(), VerticalAlignment::Stretch);
    assert_eq!(presenter.horizontal_alignment(), HorizontalAlignment::Left);
    assert_eq!(visualizer.pull_direction(), PullDirection::LeftToRight);
    assert_eq!(visualizer.width(), size);
    assert!(visualizer.height().is_nan());
    assert_eq!(provider.pull_direction(), PullDirection::LeftToRight);
    assert_eq!(recognizer.pull_direction(), PullDirection::LeftToRight);
    // The size handed to the provider is the requested size of the visualizer.
    assert_eq!(provider.refresh_visualizer_size().width, size);
    assert!(provider.refresh_visualizer_size().height.is_nan());

    hosted.container.set_pull_direction(PullDirection::RightToLeft);
    assert_eq!(presenter.vertical_alignment(), VerticalAlignment::Stretch);
    assert_eq!(presenter.horizontal_alignment(), HorizontalAlignment::Right);
    assert_eq!(visualizer.pull_direction(), PullDirection::RightToLeft);
    assert_eq!(visualizer.width(), size);
    assert!(visualizer.height().is_nan());
    assert_eq!(provider.pull_direction(), PullDirection::RightToLeft);
    assert_eq!(recognizer.pull_direction(), PullDirection::RightToLeft);

    hosted.container.set_pull_direction(PullDirection::TopToBottom);
    assert_eq!(presenter.vertical_alignment(), VerticalAlignment::Top);
    assert_eq!(presenter.horizontal_alignment(), HorizontalAlignment::Stretch);
    assert_eq!(visualizer.pull_direction(), PullDirection::TopToBottom);

    assert_eq!(provider.pull_direction(), PullDirection::TopToBottom);
    assert_eq!(recognizer.pull_direction(), PullDirection::TopToBottom);
    assert_eq!(provider.refresh_visualizer_size().height, size);
}

#[test]
fn pull_direction_set_before_the_template_is_applied_is_used() {
    let _scope = test_scope();
    let hosted = host(|container| container.set_pull_direction(PullDirection::RightToLeft));

    let visualizer = hosted.visualizer();
    assert_eq!(hosted.presenter().horizontal_alignment(), HorizontalAlignment::Right);
    assert_eq!(visualizer.pull_direction(), PullDirection::RightToLeft);
    assert_eq!(visualizer.refresh_info_provider().expect("provider").pull_direction(), PullDirection::RightToLeft);
    assert_eq!(hosted.pull_recognizer().pull_direction(), PullDirection::RightToLeft);
}

#[test]
fn a_visualizer_set_before_the_template_is_applied_keeps_its_own_size_and_direction() {
    let _scope = test_scope();
    let custom = RefreshVisualizer::new();
    custom.set_height(40.0);
    let hosted = {
        let custom = custom.clone();
        host(move |container| container.set_visualizer(Some(custom)))
    };

    assert!(hosted.visualizer() == custom);
    assert_eq!(hosted.presenter().children().count(), 1);
    assert_eq!(custom.height(), 40.0);
    assert!(custom.refresh_info_provider().is_some());

    hosted.container.set_pull_direction(PullDirection::LeftToRight);
    assert_eq!(hosted.presenter().horizontal_alignment(), HorizontalAlignment::Left);
    assert_eq!(custom.pull_direction(), PullDirection::TopToBottom);
    assert_eq!(custom.height(), 40.0);
    // The adapter still follows the direction of the container.
    assert_eq!(custom.refresh_info_provider().expect("provider").pull_direction(), PullDirection::LeftToRight);
}

#[test]
fn replacing_the_visualizer_replaces_it_in_the_presenter_part_and_rewires_the_events() {
    let _scope = test_scope();
    let hosted = host(|_| {});
    let default_visualizer = hosted.visualizer();
    let provider = default_visualizer.refresh_info_provider().expect("refresh info provider");
    let presenter = hosted.presenter();

    let requested = Rc::new(Cell::new(0));
    {
        let requested = requested.clone();
        hosted.container.refresh_requested(move |_, _| requested.set(requested.get() + 1));
    }

    default_visualizer.request_refresh();
    assert_eq!(requested.get(), 1);

    let replacement = RefreshVisualizer::new();
    hosted.container.set_visualizer(Some(replacement.clone()));
    assert!(hosted.visualizer() == replacement);
    assert_eq!(presenter.children().count(), 1);
    assert!(replacement.get_visual_parent().is_some_and(|parent| parent == presenter.clone().upcast::<Visual>()));
    assert!(default_visualizer.get_visual_parent().is_none());

    // The provider adapted from the tree is handed to the new visualizer.
    assert!(replacement.refresh_info_provider().is_some_and(|current| current == provider));

    // The requests of the old visualizer are no longer forwarded.
    default_visualizer.request_refresh();
    assert_eq!(requested.get(), 1);
    replacement.request_refresh();
    assert_eq!(requested.get(), 2);

    hosted.container.set_visualizer(None);
    assert_eq!(presenter.children().count(), 0);
    hosted.container.request_refresh();
    assert_eq!(requested.get(), 2);
}

#[test]
fn request_refresh_raises_refresh_requested_and_waits_for_the_deferral() {
    let _scope = test_scope();
    let hosted = host(|_| {});
    let visualizer = hosted.visualizer();

    let deferral = Rc::new(RefCell::new(None));
    let token = {
        let deferral = deferral.clone();
        hosted.container.refresh_requested(move |sender, e| {
            assert!(sender.is::<RefreshContainer>());
            *deferral.borrow_mut() = Some(e.get_deferral());
        })
    };

    hosted.container.request_refresh();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);

    let deferral = deferral.borrow_mut().take().expect("deferral");
    deferral.complete();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);

    // Without a deferral taken the refresh completes at once.
    hosted.container.remove_handler(RefreshContainer::refresh_requested_event(), token);
    hosted.container.request_refresh();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
}

#[test]
fn pulling_the_scroll_viewer_past_the_execution_ratio_and_releasing_requests_a_refresh() {
    let _scope = test_scope();
    let hosted = host(|_| {});
    let visualizer = hosted.visualizer();
    let provider = visualizer.refresh_info_provider().expect("refresh info provider");

    let deferral = Rc::new(RefCell::new(None));
    {
        let deferral = deferral.clone();
        hosted.container.refresh_requested(move |_, e| {
            *deferral.borrow_mut() = Some(e.get_deferral());
        });
    }

    let touch = TouchTestHelper::new();
    let scroll_viewer = &hosted.scroll_viewer;

    touch.down(scroll_viewer, Point::new(10.0, 20.0));
    touch.move_(scroll_viewer, Point::new(10.0, 70.0));
    assert!(provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.5);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);

    touch.move_(scroll_viewer, Point::new(10.0, 115.0));
    assert_eq!(provider.interaction_ratio(), 0.95);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Pending);
    assert!(deferral.borrow().is_none());

    touch.up(scroll_viewer, Point::new(10.0, 115.0));
    assert!(!provider.is_interacting_for_refresh());
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);

    let deferral = deferral.borrow_mut().take().expect("deferral");
    deferral.complete();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    assert_eq!(provider.interaction_ratio(), 0.0);

    // A short pull does not request a refresh.
    touch.down(scroll_viewer, Point::new(10.0, 20.0));
    touch.move_(scroll_viewer, Point::new(10.0, 60.0));
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);
    touch.up(scroll_viewer, Point::new(10.0, 60.0));
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    assert_eq!(provider.refresh_visualizer_size(), Size::new(visualizer.bounds().width, 100.0));
}
