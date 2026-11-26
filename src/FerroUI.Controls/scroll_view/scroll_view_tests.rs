//! Tests of WinUI's `ScrollView` and `ScrollPresenter` (#1268).

use super::*;
use crate::primitives::{IScrollController, ScrollBar, ScrollPresenter};
use crate::scroll_viewer_tests::{create_scroll_bar_template, create_template};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, Grid, ScrollViewer};
use ferroui_base::animation::easings::{IEasing, LinearEasing};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::Orientation;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::animations::KeyFrame;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_base::Upcast;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn presenter_engine_template(presenter: &Ref<ScrollPresenter>) {
    presenter
        .engine()
        .unwrap()
        .set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
            create_template,
        )));
}

/// The template of the view: the presenter and the two bars, as WinUI's `ScrollView.xaml`.
fn view_template(_view: &Ref<ScrollView>, scope: &NameScopeRef) -> Ref<Control> {
    let grid = Grid::new();
    let presenter = ScrollPresenter::new();
    presenter.set_name(Some("PART_ScrollPresenter".to_string()));
    presenter_engine_template(&presenter);
    for (property, source) in [
        (
            ScrollPresenter::content_property().as_property(),
            ScrollView::content_property().as_property(),
        ),
        (
            ScrollPresenter::content_orientation_property().as_property(),
            ScrollView::content_orientation_property().as_property(),
        ),
        (
            ScrollPresenter::zoom_mode_property().as_property(),
            ScrollView::zoom_mode_property().as_property(),
        ),
        (
            ScrollPresenter::horizontal_scroll_mode_property().as_property(),
            ScrollView::horizontal_scroll_mode_property().as_property(),
        ),
        (
            ScrollPresenter::vertical_scroll_mode_property().as_property(),
            ScrollView::vertical_scroll_mode_property().as_property(),
        ),
        (
            ScrollPresenter::min_zoom_factor_property().as_property(),
            ScrollView::min_zoom_factor_property().as_property(),
        ),
        (
            ScrollPresenter::max_zoom_factor_property().as_property(),
            ScrollView::max_zoom_factor_property().as_property(),
        ),
    ] {
        presenter.bind_binding(property, &TemplateBinding::new(source));
    }
    grid.children()
        .add(presenter.register_in_name_scope(&**scope));
    for (name, orientation) in [
        ("PART_HorizontalScrollBar", Orientation::Horizontal),
        ("PART_VerticalScrollBar", Orientation::Vertical),
    ] {
        let bar = ScrollBar::new();
        bar.set_name(Some(name.to_string()));
        bar.set_orientation(orientation);
        bar.set_template(Some(FuncControlTemplate::for_type::<ScrollBar>(
            create_scroll_bar_template,
        )));
        grid.children().add(bar.register_in_name_scope(&**scope));
    }
    grid.upcast()
}

/// A 100 x 100 view over a 200 x 300 border.
fn scene(
    orientation: ScrollingContentOrientation,
) -> (Ref<TestRoot>, Ref<ScrollView>, Ref<ScrollPresenter>) {
    let view = ScrollView::new();
    view.set_template(Some(FuncControlTemplate::for_type::<ScrollView>(
        view_template,
    )));
    view.set_width(100.0);
    view.set_height(100.0);
    view.set_content_orientation(orientation);
    let content = Border::new();
    // A content without a width is as wide as the viewport where the orientation constrains it.
    if orientation != ScrollingContentOrientation::Vertical {
        content.set_width(200.0);
    }
    content.set_height(300.0);
    view.set_content(Some(content.upcast()));
    let root = TestRoot::with_child(&view);
    root.layout_manager().execute_initial_layout_pass();
    root.layout_manager().execute_layout_pass();
    let presenter = view
        .scroll_presenter()
        .expect("the template has a presenter");
    (root, view, presenter)
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::NORMAL));
}

#[test]
fn defaults_are_winui_s() {
    let _scope = test_scope();
    let view = ScrollView::new();
    assert_eq!(
        view.content_orientation(),
        ScrollingContentOrientation::Vertical
    );
    assert_eq!(view.horizontal_scroll_mode(), ScrollingScrollMode::Auto);
    assert_eq!(view.zoom_mode(), ScrollingZoomMode::Disabled);
    assert_eq!(
        view.horizontal_scroll_bar_visibility(),
        ScrollingScrollBarVisibility::Auto
    );
    assert_eq!(view.min_zoom_factor(), 0.1);
    assert_eq!(view.max_zoom_factor(), 10.0);
    assert_eq!(view.ignored_input_kinds(), ScrollingInputKinds::NONE);
    // Without a template there is no presenter: the requests return -1.
    assert_eq!(view.scroll_to(1.0, 1.0), -1);
}

#[test]
fn key_frames_ease_each_segment_from_the_start_value() {
    let linear: Rc<dyn IEasing> = Rc::new(LinearEasing::new());
    let frames = vec![
        KeyFrame {
            normalized_progress_key: 0.5,
            value: 10.0f32,
            expression: None,
            easing_function: linear.clone(),
        },
        KeyFrame {
            normalized_progress_key: 1.0,
            value: 0.0f32,
            expression: None,
            easing_function: linear,
        },
    ];
    let lerp = |a: f32, b: f32, t: f64| a + (b - a) * t as f32;
    assert_eq!(
        crate::primitives::scroll_presenter_sample(&frames, 0.0, 0.25, lerp),
        5.0
    );
    assert_eq!(
        crate::primitives::scroll_presenter_sample(&frames, 0.0, 0.5, lerp),
        10.0
    );
    assert_eq!(
        crate::primitives::scroll_presenter_sample(&frames, 0.0, 0.75, lerp),
        5.0
    );
    assert_eq!(
        crate::primitives::scroll_presenter_sample(&frames, 0.0, 2.0, lerp),
        0.0
    );
}

#[test]
fn the_content_orientation_constrains_the_content() {
    let _scope = test_scope();
    let (_root, view, _) = scene(ScrollingContentOrientation::None);
    assert_eq!((view.extent_width(), view.extent_height()), (200.0, 300.0));
    assert_eq!(
        (view.scrollable_width(), view.scrollable_height()),
        (100.0, 200.0)
    );
    let (_root, view, _) = scene(ScrollingContentOrientation::Vertical);
    // The 200-pixel wide border is arranged in the viewport's width: nothing scrolls horizontally.
    assert_eq!(view.scrollable_width(), 0.0);
    assert_eq!(view.scrollable_height(), 200.0);
    assert_eq!(
        view.computed_horizontal_scroll_mode(),
        ScrollingScrollMode::Disabled
    );
    assert_eq!(
        view.computed_vertical_scroll_mode(),
        ScrollingScrollMode::Enabled
    );
    assert!(!view.computed_horizontal_scroll_bar_visibility());
    assert!(view.computed_vertical_scroll_bar_visibility());
}

#[test]
fn scroll_to_out_of_a_window_lands_at_once_and_completes_later() {
    let _scope = test_scope();
    let (root, view, _) = scene(ScrollingContentOrientation::None);
    let completed = Rc::new(RefCell::new(Vec::new()));
    let starting = Rc::new(Cell::new(0));
    let recorded = completed.clone();
    view.scroll_completed(move |_, args| recorded.borrow_mut().push(args.correlation_id()))
        .detach();
    let count = starting.clone();
    view.scroll_starting(move |_, _| count.set(count.get() + 1))
        .detach();
    let views = Rc::new(Cell::new(0));
    let counted = views.clone();
    view.view_changed(move |_| counted.set(counted.get() + 1))
        .detach();
    let id = view.scroll_to(50.0, 500.0);
    root.layout_manager().execute_layout_pass();
    // Kept within the scrollable range.
    assert_eq!(
        (view.horizontal_offset(), view.vertical_offset()),
        (50.0, 200.0)
    );
    assert_eq!(starting.get(), 1);
    assert!(views.get() >= 1);
    assert!(
        completed.borrow().is_empty(),
        "a change completes after the request returns"
    );
    run_jobs();
    assert_eq!(*completed.borrow(), vec![id]);
    // ScrollBy adds to the offsets.
    let next = view.scroll_by_with(
        -20.0,
        -50.0,
        ScrollingScrollOptions::new(ScrollingAnimationMode::Disabled),
    );
    assert!(next > id);
    root.layout_manager().execute_layout_pass();
    assert_eq!(
        (view.horizontal_offset(), view.vertical_offset()),
        (30.0, 150.0)
    );
}

#[test]
fn a_velocity_without_decay_moves_until_the_end_of_the_content() {
    let _scope = test_scope();
    let (root, view, presenter) = scene(ScrollingContentOrientation::None);
    let states = Rc::new(RefCell::new(Vec::new()));
    let recorded = states.clone();
    view.state_changed(move |view| recorded.borrow_mut().push(view.state()))
        .detach();
    // Out of a window the change ends at once: drive it by hand, as the animation frames do.
    presenter.set_manual_frames_for_test();
    let id = presenter
        .add_scroll_velocity_for_test(Vector2::new(0.0, 30.0), Some(Vector2::new(0.0, 0.0)));
    assert!(id > 0);
    assert_eq!(presenter.state(), ScrollingInteractionState::Inertia);
    presenter.advance(0.0);
    presenter.advance(1.0);
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.vertical_offset(), 30.0);
    presenter.advance(2.0);
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.vertical_offset(), 60.0);
    presenter.advance(100.0);
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.vertical_offset(), 200.0);
    assert_eq!(presenter.state(), ScrollingInteractionState::Idle);
    assert_eq!(
        *states.borrow(),
        vec![
            ScrollingInteractionState::Inertia,
            ScrollingInteractionState::Idle
        ]
    );
}

#[test]
fn an_animated_scroll_follows_the_key_frames_a_handler_sets() {
    let _scope = test_scope();
    let (root, _view, presenter) = scene(ScrollingContentOrientation::None);
    presenter.set_manual_frames_for_test();
    let linear: Rc<dyn IEasing> = Rc::new(LinearEasing::new());
    let frames = vec![
        KeyFrame {
            normalized_progress_key: 0.5,
            value: Vector3::new(0.0, 150.0, 0.0),
            expression: None,
            easing_function: linear.clone(),
        },
        KeyFrame {
            normalized_progress_key: 1.0,
            value: Vector3::new(0.0, 100.0, 0.0),
            expression: None,
            easing_function: linear,
        },
    ];
    let id = presenter.start_scroll_animation_for_test(frames, 1.0);
    presenter.advance(10.0);
    presenter.advance(10.25);
    root.layout_manager().execute_layout_pass();
    assert_eq!(presenter.vertical_offset(), 75.0);
    presenter.advance(10.5);
    root.layout_manager().execute_layout_pass();
    assert_eq!(presenter.vertical_offset(), 150.0);
    let completed = Rc::new(Cell::new(0));
    let recorded = completed.clone();
    presenter
        .scroll_completed(move |_, args| recorded.set(args.correlation_id()))
        .detach();
    presenter.advance(11.0);
    root.layout_manager().execute_layout_pass();
    assert_eq!(presenter.vertical_offset(), 100.0);
    assert_eq!(completed.get(), id);
}

#[test]
fn zoom_to_keeps_the_center_point_in_place() {
    let _scope = test_scope();
    let (root, view, _) = scene(ScrollingContentOrientation::None);
    view.zoom_to_with(
        2.0,
        Some(Vector2::new(0.0, 0.0)),
        ScrollingZoomOptions::new(ScrollingAnimationMode::Disabled),
    );
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.zoom_factor(), 2.0);
    assert_eq!((view.extent_width(), view.extent_height()), (400.0, 600.0));
    // The view's center by default; kept within MaxZoomFactor.
    view.set_max_zoom_factor(3.0);
    view.zoom_to_with(
        8.0,
        None,
        ScrollingZoomOptions::new(ScrollingAnimationMode::Disabled),
    );
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.zoom_factor(), 3.0);
    assert_eq!(
        (view.horizontal_offset(), view.vertical_offset()),
        (25.0, 25.0)
    );
}

#[test]
fn the_bars_drive_the_presenter_through_their_controllers() {
    let _scope = test_scope();
    let (root, view, presenter) = scene(ScrollingContentOrientation::None);
    let vertical = view
        .scroll_bar_controller(false)
        .expect("the vertical bar's controller");
    let bar = vertical.scroll_bar().clone();
    assert_eq!(
        (bar.minimum(), bar.maximum(), bar.viewport_size()),
        (0.0, 200.0, 100.0)
    );
    assert!(vertical.can_scroll());
    // A drag of the thumb is a scroll to its value, without animation.
    bar.set_range_value(120.0);
    bar.on_scroll(crate::primitives::ScrollEventType::ThumbTrack);
    root.layout_manager().execute_layout_pass();
    assert_eq!(presenter.vertical_offset(), 120.0);
    // The presenter's view is the bar's value.
    view.scroll_to_with(
        0.0,
        40.0,
        ScrollingScrollOptions::new(ScrollingAnimationMode::Disabled),
    );
    root.layout_manager().execute_layout_pass();
    assert_eq!(bar.value(), 40.0);
    // A hidden bar does not show.
    view.set_vertical_scroll_bar_visibility(ScrollingScrollBarVisibility::Hidden);
    assert!(!view.computed_vertical_scroll_bar_visibility());
    view.set_vertical_scroll_bar_visibility(ScrollingScrollBarVisibility::Visible);
    assert!(view.computed_vertical_scroll_bar_visibility());
}
