//! Original panel tests and native lifecycle/transition boundary checks.
use super::*;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::{AppBuilder, Application, Border, Control, Dock, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::time::{Duration, Instant};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-delayed-uniform-tabs",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn run_task<F, Future>(test: F)
where
    F: FnOnce() -> Future + Send + 'static,
    Future: std::future::Future<Output = ()> + 'static,
{
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch_task(
            move || async move {
                crate::register_types();
                test().await;
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
async fn dispatcher_delay(duration: Duration) {
    let ready = Rc::new(Cell::new(false));
    let waker = Rc::new(RefCell::new(None::<std::task::Waker>));
    let completed = ready.clone();
    let wake = waker.clone();
    DispatcherTimer::run_once(
        move || {
            completed.set(true);
            if let Some(waker) = wake.borrow_mut().take() {
                waker.wake();
            }
        },
        duration,
        ferroui_base::threading::DispatcherPriority::BACKGROUND,
    );
    std::future::poll_fn(move |cx| {
        if ready.get() {
            std::task::Poll::Ready(())
        } else {
            *waker.borrow_mut() = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    })
    .await;
}
fn panel(count: usize) -> Ref<DelayedUniformTabPanel> {
    let p = DelayedUniformTabPanel::new();
    p.set_item_spacing(4.0);
    for _ in 0..count {
        p.children().add(Border::new());
    }
    p
}
fn layout(p: &DelayedUniformTabPanel, width: f64) {
    p.measure(Size::new(width, 32.0));
    p.arrange(Rect::new(0.0, 0.0, width, 32.0));
}
fn widths(p: &DelayedUniformTabPanel, expected: f64) {
    for child in p.children().snapshot().iter() {
        if child.is_visible() {
            assert!(
                (child.bounds().width - expected).abs() <= 1.1,
                "{} != {expected}",
                child.bounds().width
            );
        }
    }
}
async fn wait(p: &DelayedUniformTabPanel, expected: f64, width: f64) -> Duration {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(800) {
        Dispatcher::ui_thread().run_jobs(None);
        layout(p, width);
        if (p.children().get(0).bounds().width - expected).abs() <= 1.1 {
            return start.elapsed();
        }
        dispatcher_delay(Duration::from_millis(5)).await;
    }
    widths(p, expected);
    start.elapsed()
}
#[test]
fn original_maximum_immediate_shrink_and_viewport_only_expansion() {
    run(|| {
        let p = panel(3);
        assert_eq!(p.max_tab_width(), 220.0);
        assert_eq!(p.min_tab_width(), 80.0);
        assert_eq!(p.expansion_delay(), TimeSpan::from_seconds(1.0));
        assert_eq!(p.resize_animation_duration(), TimeSpan::ZERO);
        assert!(p
            .get_value(DelayedUniformTabPanel::animated_tab_width_property())
            .is_nan());
        layout(&p, 1000.0);
        widths(&p, 220.0);
        layout(&p, 500.0);
        widths(&p, 164.0);
        p.set_expansion_delay(TimeSpan::from_seconds(10.0));
        layout(&p, 1000.0);
        widths(&p, 220.0);
    });
}
#[test]
fn original_close_expands_only_after_the_configured_dispatcher_delay() {
    run_task(|| async {
        let p = panel(4);
        p.set_expansion_delay(TimeSpan::from_milliseconds(40.0));
        layout(&p, 500.0);
        widths(&p, 122.0);
        p.children().remove_at(3);
        layout(&p, 500.0);
        widths(&p, 122.0);
        assert!(wait(&p, 164.0, 500.0).await >= Duration::from_millis(20));
        assert!(!p.expansion_timer.is_enabled());
    });
}
#[test]
fn original_repeated_closes_restart_the_delay_and_defer_viewport_expansion() {
    run_task(|| async {
        let p = panel(6);
        p.set_expansion_delay(TimeSpan::from_milliseconds(80.0));
        layout(&p, 1000.0);
        let compact = p.children().get(0).bounds().width;
        p.children().remove_at(5);
        layout(&p, 1000.0);
        widths(&p, compact);
        std::thread::sleep(Duration::from_millis(25));
        p.children().remove_at(4);
        layout(&p, 1000.0);
        widths(&p, compact);
        assert!(wait(&p, 220.0, 1000.0).await >= Duration::from_millis(60));
        let p = panel(4);
        p.set_expansion_delay(TimeSpan::from_milliseconds(50.0));
        layout(&p, 500.0);
        p.children().remove_at(3);
        layout(&p, 500.0);
        layout(&p, 600.0);
        widths(&p, 122.0);
        assert!(wait(&p, 197.0, 600.0).await >= Duration::from_millis(30));
    });
}
#[test]
fn source_delay_change_restarts_pending_timer_and_signed_nonpositive_delay_is_immediate() {
    run_task(|| async {
        let p = panel(4);
        p.set_expansion_delay(TimeSpan::from_milliseconds(100.0));
        layout(&p, 500.0);
        p.children().remove_at(3);
        layout(&p, 500.0);
        p.set_expansion_delay(TimeSpan::from_milliseconds(40.0));
        widths(&p, 122.0);
        assert!(wait(&p, 164.0, 500.0).await >= Duration::from_millis(20));
        for delay in [TimeSpan::ZERO, TimeSpan::from_ticks(-1)] {
            let p = panel(4);
            p.set_expansion_delay(delay);
            layout(&p, 500.0);
            p.children().remove_at(3);
            layout(&p, 500.0);
            widths(&p, 164.0);
            assert!(!p.expansion_timer.is_enabled());
        }
    });
}
#[test]
fn source_hidden_children_minimum_width_negative_spacing_and_empty_reset() {
    run(|| {
        let p = panel(3);
        p.set_expansion_delay(TimeSpan::ZERO);
        p.set_item_spacing(-6.0);
        p.children().get(1).set_is_visible(false);
        layout(&p, 300.0);
        widths(&p, 150.0);
        assert_eq!(p.children().get(1).bounds(), Rect::default());
        p.children().get(0).set_min_width(180.0);
        p.invalidate_measure();
        layout(&p, 300.0);
        assert_eq!(p.current_tab_width.get(), 180.0);
        p.children().clear();
        layout(&p, 300.0);
        assert_eq!(p.desired_size(), Size::default());
        assert!(p.current_tab_width.get().is_nan());
        p.children().add(Border::new());
        layout(&p, 300.0);
        widths(&p, 220.0);
    });
}
#[test]
fn source_observers_prioritize_named_dock_viewport_and_release_on_detachment() {
    run(|| {
        let dock = DockPanel::new();
        let fixed = Border::new();
        fixed.set_width(40.0);
        DockPanel::set_dock(&fixed, Dock::Left);
        dock.children().add(fixed.clone());
        let part = Panel::new();
        part.set_name(Some("PART_Panel".into()));
        let scroll = ScrollViewer::new();
        let p = panel(3);
        scroll.set_content(Some(Control::boxed(p.clone())));
        part.children().add(scroll);
        dock.children().add(part);
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(80.0);
        window.set_content(Some(Control::boxed(dock.clone())));
        window.show();
        for _ in 0..3 {
            Dispatcher::ui_thread().run_jobs(None);
            window.update_layout();
        }
        assert_eq!(p.layout_subscriptions.borrow().len(), 5);
        assert_eq!(p.resolve_viewport_width(Size::new(20.0, 32.0)), 560.0);
        fixed.set_width(100.0);
        for _ in 0..3 {
            Dispatcher::ui_thread().run_jobs(None);
            window.update_layout();
        }
        assert_eq!(p.resolve_viewport_width(Size::new(20.0, 32.0)), 500.0);
        window.set_content(None);
        assert!(p.layout_subscriptions.borrow().is_empty());
        assert!(!p.expansion_timer.is_enabled());
        window.close();
    });
}
#[test]
fn source_duration_installs_actual_cubic_double_transition_and_clears_it() {
    run(|| {
        let p = panel(1);
        assert!(p.transitions().is_none());
        p.set_resize_animation_duration(TimeSpan::from_milliseconds(180.0));
        let transitions = p.transitions().unwrap();
        assert_eq!(transitions.count(), 1);
        let transition = transitions.get(0);
        assert_eq!(
            transition.property(),
            DelayedUniformTabPanel::animated_tab_width_property().as_property()
        );
        let object = transition
            .as_object()
            .unwrap()
            .cast::<DoubleTransition>()
            .unwrap();
        assert_eq!(object.duration(), TimeSpan::from_milliseconds(180.0));
        assert_eq!(object.easing().ease(0.5), 0.875);
        p.set_resize_animation_duration(TimeSpan::from_ticks(-1));
        assert!(p.transitions().is_none());
    });
}
#[test]
fn source_numeric_boundaries_keep_nan_signed_zero_and_ties_to_even() {
    run(|| {
        assert!(max(0.0, f64::NAN).is_nan());
        assert!(min(f64::NAN, 1.0).is_nan());
        assert_eq!(max(-0.0, 0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(min(-0.0, 0.0).to_bits(), (-0.0f64).to_bits());
        let p = panel(3);
        assert_eq!(p.round_to_device_pixels(0.5), 0.0);
        assert_eq!(p.round_to_device_pixels(1.5), 2.0);
        p.set_min_tab_width(300.0);
        assert_eq!(p.compute_target_tab_width(3, Size::new(10.0, 32.0)), 300.0);
        p.set_max_tab_width(f64::NAN);
        assert!(p
            .compute_target_tab_width(3, Size::new(500.0, 32.0))
            .is_nan());
    });
}
#[test]
fn actual_original_dotnet_layout_corpus_matches_native_desired_extent_and_every_child() {
    run(|| {
        let mut count = 0;
        for line in include_str!("../../../../tests/Dock/data/delayed-tab-layout.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
        {
            let fields: Vec<_> = line.split('\t').collect();
            let number = |index: usize| fields[index].parse::<f64>().unwrap();
            let p = panel(fields[0].parse().unwrap());
            p.set_expansion_delay(TimeSpan::ZERO);
            p.set_max_tab_width(number(2));
            p.set_min_tab_width(number(3));
            p.set_item_spacing(number(4));
            let hidden = fields[5].parse::<i32>().unwrap();
            for (index, child) in p.children().snapshot().iter().enumerate() {
                child.set_height(32.0);
                child.set_is_visible(index as i32 != hidden);
                if index == 0 {
                    child.set_min_width(number(6));
                }
            }
            let available = number(1);
            p.measure(Size::new(available, 32.0));
            let width = if available.is_finite() {
                available
            } else {
                p.desired_size().width
            };
            p.arrange(Rect::new(0.0, 0.0, width, 32.0));
            assert_eq!(p.desired_size().width, number(7), "{line}");
            let expected: Vec<_> = fields[8]
                .split(';')
                .filter(|text| !text.is_empty() && *text != "-")
                .collect();
            assert_eq!(p.children().count(), expected.len());
            for (index, pair) in expected.into_iter().enumerate() {
                let (x, width) = pair.split_once(',').unwrap();
                let bounds = p.children().get(index).bounds();
                assert_eq!(
                    bounds.x,
                    x.parse::<f64>().unwrap(),
                    "{line}: child {index} x"
                );
                assert_eq!(
                    bounds.width,
                    width.parse::<f64>().unwrap(),
                    "{line}: child {index} width"
                );
            }
            count += 1;
        }
        assert_eq!(count, 24);
    });
}
#[repr(C)]
struct MutatingChild {
    base: Control,
    action: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferroui_base::ferro_class!(MutatingChild:Control);
ferroui_base::ferro_class_info!(MutatingChild {});
ferroui_base::ferro_impl_classes!(MutatingChild:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for MutatingChild {
    fn measure_override(this: &Self, _size: Size) -> Size {
        let action = this.action.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
        Size::default()
    }
}
#[test]
fn source_measure_reentry_rejects_child_collection_mutation_after_the_callback() {
    run(|| {
        let p = panel(0);
        let parent = p.clone();
        let child = ferroui_base::instantiate(MutatingChild {
            base: Control::construct(),
            action: RefCell::new(Some(Box::new(move || parent.children().add(Border::new())))),
        });
        p.children().add(child);
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| layout(&p, 500.0)))
            .unwrap_err();
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap();
        assert_eq!(
            message,
            "Collection was modified; enumeration operation may not execute."
        );
        assert_eq!(p.children().count(), 2);
    });
}

#[repr(C)]
struct DerivedPanel {
    base: DelayedUniformTabPanel,
}
ferroui_base::ferro_class!(DerivedPanel:DelayedUniformTabPanel);
ferroui_base::ferro_class_info!(DerivedPanel {});
ferroui_base::ferro_impl_classes!(DerivedPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
#[test]
fn source_constructor_timer_subscription_is_inherited_by_native_derived_panels() {
    run_task(|| async {
        let p = ferroui_base::instantiate(DerivedPanel {
            base: DelayedUniformTabPanel::construct(),
        });
        p.set_item_spacing(4.0);
        p.set_expansion_delay(TimeSpan::from_milliseconds(30.0));
        for _ in 0..4 {
            p.children().add(Border::new());
        }
        layout(&p, 500.0);
        p.children().remove_at(3);
        layout(&p, 500.0);
        widths(&p, 122.0);
        assert!(wait(&p, 164.0, 500.0).await >= Duration::from_millis(10));
    });
}
