use super::{RefreshInfoProvider, RefreshVisualizer, RefreshVisualizerOrientation, RefreshVisualizerState};
use crate::test_support::{test_scope, TestRoot};
use crate::testing::refresh_visualizer_template;
use crate::{Border, Control, Grid};
use ferroui_base::input::{PullDirection, PullGestureEndedEventArgs, PullGestureEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{FerroObject, FerroObjectExtensions, Matrix, Ref, Size, Vector, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn interaction_ratio_is_one_while_refreshing_and_zero_after_completion() {
    let _scope = test_scope();
    let provider = RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 100.0)), None);

    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));

    let saw_during_refresh = Rc::new(Cell::new(false));

    {
        let provider = provider.clone();
        let saw_during_refresh = saw_during_refresh.clone();
        visualizer.refresh_requested(move |_, _| {
            assert_eq!(provider.interaction_ratio(), 1.0);
            saw_during_refresh.set(true);
        });
    }

    visualizer.request_refresh();

    assert!(saw_during_refresh.get(), "RefreshRequested event should have been raised");

    assert_eq!(provider.interaction_ratio(), 0.0);
}

// --- Additional tests (not upstream); expectations derived from the reference source. ---

fn provider() -> Ref<RefreshInfoProvider> {
    RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 100.0)), None)
}

fn pull(provider: &RefreshInfoProvider, distance: f64) {
    provider.interacting_state_entered(&PullGestureEventArgs::new(
        0,
        Vector::new(0.0, distance),
        PullDirection::TopToBottom,
    ));
}

fn release(provider: &RefreshInfoProvider) {
    provider.interacting_state_exited(&PullGestureEndedEventArgs::new(0, PullDirection::TopToBottom));
}

/// Records every value of the state property of the visualizer, starting with the
/// current one.
fn record_states(
    visualizer: &Ref<RefreshVisualizer>,
) -> (Rc<RefCell<Vec<RefreshVisualizerState>>>, Rc<dyn IDisposable>) {
    let states = Rc::new(RefCell::new(Vec::new()));
    let object: &FerroObject = visualizer;
    let subscription = {
        let states = states.clone();
        FerroObjectExtensions::get_observable(object, RefreshVisualizer::refresh_visualizer_state_property())
            .subscribe_fn(move |state| states.borrow_mut().push(state))
    };
    (states, subscription)
}

#[test]
fn state_follows_the_pull_from_idle_to_refreshing_and_back_when_the_deferral_completes() {
    let _scope = test_scope();
    let provider = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);

    let (states, _subscription) = record_states(&visualizer);

    let deferral = Rc::new(RefCell::new(None));
    {
        let deferral = deferral.clone();
        visualizer.refresh_requested(move |_, e| {
            *deferral.borrow_mut() = Some(e.get_deferral());
        });
    }
    let started = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    {
        let started = started.clone();
        provider.refresh_started(move |_, _| started.set(started.get() + 1));
        let completed = completed.clone();
        provider.refresh_completed(move |_, _| completed.set(completed.get() + 1));
    }

    // Below the execution ratio (0.8): interacting.
    pull(&provider, 50.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);

    // Past the execution ratio: a release would start the refresh.
    pull(&provider, 90.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Pending);

    // Back below it.
    pull(&provider, 80.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);

    pull(&provider, 95.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Pending);
    assert!(deferral.borrow().is_none());

    // Releasing while pending requests the refresh; the handler holds a deferral.
    release(&provider);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);
    assert_eq!(started.get(), 1);
    assert_eq!(completed.get(), 0);
    assert!(!provider.is_interacting_for_refresh());

    // The end of an interaction does not interrupt a refresh, and neither does a
    // new pull.
    pull(&provider, 30.0);
    release(&provider);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);

    let deferral = deferral.borrow_mut().take().expect("deferral");
    deferral.complete();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    assert_eq!(completed.get(), 1);
    assert_eq!(provider.interaction_ratio(), 0.0);

    // Requesting the refresh sets the interaction ratio to one before the state
    // changes, which shows as a peek between Pending and Refreshing.
    assert_eq!(
        *states.borrow(),
        [
            RefreshVisualizerState::Idle,
            RefreshVisualizerState::Interacting,
            RefreshVisualizerState::Pending,
            RefreshVisualizerState::Interacting,
            RefreshVisualizerState::Pending,
            RefreshVisualizerState::Peeking,
            RefreshVisualizerState::Refreshing,
            RefreshVisualizerState::Idle,
        ]
    );
}

#[test]
fn release_before_the_execution_ratio_returns_to_idle_without_a_refresh() {
    let _scope = test_scope();
    let provider = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));

    let requested = Rc::new(Cell::new(0));
    {
        let requested = requested.clone();
        visualizer.refresh_requested(move |_, _| requested.set(requested.get() + 1));
    }

    pull(&provider, 80.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);

    release(&provider);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    assert_eq!(requested.get(), 0);

    // Pulling back to the start while interacting is idle too.
    pull(&provider, 40.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);
    pull(&provider, 0.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    release(&provider);
    assert_eq!(requested.get(), 0);
}

#[test]
fn a_pull_that_starts_past_the_execution_ratio_is_pending_at_once() {
    let _scope = test_scope();
    let provider = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));

    pull(&provider, 100.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Pending);
}

#[test]
fn interaction_ratio_without_interaction_is_a_peek() {
    let _scope = test_scope();
    let provider = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));

    provider.set_interaction_ratio(0.3);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Peeking);

    provider.set_interaction_ratio(0.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
}

#[test]
fn request_refresh_without_a_deferral_completes_at_once() {
    let _scope = test_scope();
    let provider = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(provider.clone()));

    let (states, _subscription) = record_states(&visualizer);
    let log = Rc::new(RefCell::new(Vec::new()));
    {
        let log = log.clone();
        provider.refresh_started(move |_, _| log.borrow_mut().push("started"));
    }
    {
        let log = log.clone();
        let handler_visualizer = visualizer.clone();
        visualizer.refresh_requested(move |_, _| {
            assert_eq!(handler_visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);
            log.borrow_mut().push("requested");
        });
    }
    {
        let log = log.clone();
        provider.refresh_completed(move |_, _| log.borrow_mut().push("completed"));
    }

    visualizer.request_refresh();

    assert_eq!(*log.borrow(), ["started", "requested", "completed"]);
    assert_eq!(
        *states.borrow(),
        [
            RefreshVisualizerState::Idle,
            RefreshVisualizerState::Peeking,
            RefreshVisualizerState::Refreshing,
            RefreshVisualizerState::Idle,
        ]
    );
}

#[test]
fn refresh_completes_when_every_deferral_is_completed() {
    let _scope = test_scope();
    let visualizer = RefreshVisualizer::new();

    let deferrals = Rc::new(RefCell::new(Vec::new()));
    for _ in 0..2 {
        let deferrals = deferrals.clone();
        visualizer.refresh_requested(move |_, e| deferrals.borrow_mut().push(e.get_deferral()));
    }

    // Without a refresh info provider the request still runs.
    visualizer.request_refresh();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);

    let deferrals = std::mem::take(&mut *deferrals.borrow_mut());
    assert_eq!(deferrals.len(), 2);
    // Both handlers got the same deferral object.
    assert!(Rc::ptr_eq(&deferrals[0], &deferrals[1]));

    deferrals[0].complete();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);

    deferrals[1].complete();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
}

#[test]
fn replacing_the_refresh_info_provider_stops_following_the_old_one() {
    let _scope = test_scope();
    let first = provider();
    let second = provider();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_refresh_info_provider(Some(first.clone()));
    visualizer.set_refresh_info_provider(Some(second.clone()));

    pull(&first, 50.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);

    pull(&second, 50.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);

    visualizer.set_refresh_info_provider(None);
    pull(&second, 95.0);
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Interacting);
}

#[test]
fn content_is_hosted_centered_in_the_root_part() {
    let _scope = test_scope();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_template(Some(refresh_visualizer_template()));
    let first = Border::new();
    visualizer.set_content(Some(Control::boxed(first.clone())));

    let root = TestRoot::with_child(visualizer.clone());
    root.layout_manager().execute_initial_layout_pass();

    assert!(!visualizer.clip_to_bounds());
    let part_root = first.get_visual_parent().and_then(|parent| parent.cast::<Grid>()).expect("PART_Root");
    assert_eq!(part_root.name().as_deref(), Some("PART_Root"));
    assert_eq!(part_root.children().count(), 1);
    assert_eq!(first.vertical_alignment(), VerticalAlignment::Center);
    assert_eq!(first.horizontal_alignment(), HorizontalAlignment::Center);

    // New content replaces the old one in the root part.
    let second = Border::new();
    visualizer.set_content(Some(Control::boxed(second.clone())));
    assert!(first.get_visual_parent().is_none());
    assert_eq!(part_root.children().count(), 1);
    assert!(second.get_visual_parent().is_some_and(|parent| parent == part_root.clone().upcast::<Visual>()));
    assert_eq!(second.vertical_alignment(), VerticalAlignment::Center);
    assert_eq!(second.horizontal_alignment(), HorizontalAlignment::Center);

    visualizer.set_content(None);
    assert_eq!(part_root.children().count(), 0);
}

#[test]
fn render_transform_moves_the_visualizer_out_of_view_against_the_pull_direction() {
    let _scope = test_scope();
    let visualizer = RefreshVisualizer::new();
    visualizer.set_template(Some(refresh_visualizer_template()));
    visualizer.set_width(60.0);
    visualizer.set_height(40.0);
    visualizer.set_horizontal_alignment(HorizontalAlignment::Left);
    visualizer.set_vertical_alignment(VerticalAlignment::Top);

    let root = TestRoot::with_child(visualizer.clone());
    root.layout_manager().execute_initial_layout_pass();

    let translation = |visualizer: &RefreshVisualizer| visualizer.render_transform().expect("render transform").value();

    assert_eq!(visualizer.bounds().size(), Size::new(60.0, 40.0));
    assert_eq!(translation(&visualizer), Matrix::create_translation(0.0, -40.0));

    visualizer.set_pull_direction(PullDirection::BottomToTop);
    assert_eq!(translation(&visualizer), Matrix::create_translation(0.0, 40.0));

    visualizer.set_pull_direction(PullDirection::LeftToRight);
    assert_eq!(translation(&visualizer), Matrix::create_translation(-60.0, 0.0));

    visualizer.set_pull_direction(PullDirection::RightToLeft);
    assert_eq!(translation(&visualizer), Matrix::create_translation(60.0, 0.0));
}

#[test]
fn orientation_is_a_direct_property_that_raises_changes() {
    let _scope = test_scope();
    let visualizer = RefreshVisualizer::new();
    assert_eq!(visualizer.orientation(), RefreshVisualizerOrientation::Auto);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let object: &FerroObject = &visualizer;
    let _subscription = {
        let seen = seen.clone();
        FerroObjectExtensions::get_observable(object, RefreshVisualizer::orientation_property())
            .subscribe_fn(move |orientation| seen.borrow_mut().push(orientation))
    };

    visualizer.set_orientation(RefreshVisualizerOrientation::Rotate90DegreesCounterclockwise);
    visualizer.set_direct_value(RefreshVisualizer::orientation_property(), RefreshVisualizerOrientation::Normal);

    assert_eq!(visualizer.orientation(), RefreshVisualizerOrientation::Normal);
    assert_eq!(
        *seen.borrow(),
        [
            RefreshVisualizerOrientation::Auto,
            RefreshVisualizerOrientation::Rotate90DegreesCounterclockwise,
            RefreshVisualizerOrientation::Normal,
        ]
    );
}

/// Not from upstream: over a compositing renderer, the loaded content of
/// the visualizer gets the implicit animations of its composition visual,
/// and a refresh starts the endless rotation of the content on the server.
#[test]
fn content_animations_run_on_the_compositor() {
    use crate::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
    use crate::Window;
    use ferroui_base::rendering::composition::server::{
        IAnimatedServerObject, ServerCompositionVisual, ServerObjectAnimations,
    };
    use ferroui_base::rendering::composition::ElementComposition;

    let services = CompositorTestServices::start(TestServices::styled_window());
    let visualizer = RefreshVisualizer::new();
    visualizer.set_template(Some(refresh_visualizer_template()));
    let content = Border::new();
    content.set_width(20.0);
    content.set_height(20.0);
    visualizer.set_content(Some(Control::boxed(content.clone())));
    let host = Border::new();
    host.set_child(Some(visualizer.clone().upcast()));

    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    window.set_content(Some(Control::boxed(host.clone())));
    window.show();
    services.run_jobs();

    let provider = RefreshInfoProvider::new(
        PullDirection::TopToBottom,
        Some(Size::new(100.0, 100.0)),
        ElementComposition::get_element_visual(&host),
    );
    visualizer.set_refresh_info_provider(Some(provider.clone()));
    services.run_jobs();

    let content_visual = ElementComposition::get_element_visual(&content).expect("the content is composited");
    let implicit = content_visual.implicit_animations().expect("the loaded content has implicit animations");
    assert_eq!(implicit.keys(), ["RotationAngle", "Offset", "Scale", "Opacity"]);

    // A deferral keeps the visualizer refreshing.
    let deferral = Rc::new(RefCell::new(None));
    {
        let deferral = deferral.clone();
        visualizer.refresh_requested(move |_, e| *deferral.borrow_mut() = Some(e.get_deferral()));
    }
    visualizer.request_refresh();
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Refreshing);
    assert!(content_visual.object().pending_animations().count() >= 1);
    services.run_jobs();

    let server = services
        .compositor()
        .server()
        .get::<ServerCompositionVisual>(content_visual.server())
        .expect("the server visual exists");
    let animations: Option<Rc<ServerObjectAnimations>> = server.server_object().animations();
    assert!(animations.is_some());
    // The rotation runs forever: the server clock keeps ticking.
    assert!(services.compositor().server().animations().need_next_tick());

    if let Some(deferral) = deferral.borrow_mut().take() {
        deferral.complete();
    }
    assert_eq!(visualizer.refresh_visualizer_state(), RefreshVisualizerState::Idle);
    window.close();
}
