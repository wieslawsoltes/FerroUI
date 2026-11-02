//! Actual downstream native scheduling, timeline and deferred control metadata.
use ferrodock_controls_deferred_content_control::{
    DeferredContentControl, DeferredContentPresentationBudgetMode, DeferredContentScheduling,
};
use ferrodock_ferro::controls::DelayedUniformTabPanel;
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::{
    ferro_model,
    threading::{CancellationToken, Dispatcher, DispatcherPriority, DispatcherTimer},
    BoxedValue, FerroObject, FerroProperty, Ref,
};
use ferroui_controls::{AppBuilder, Application, Border, Control, Grid, TextBlock, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-external-deferred",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Scene(Ref<Window>);
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
async fn delay() {
    let ready = Rc::new(Cell::new(false));
    let waker = Rc::new(RefCell::new(None::<std::task::Waker>));
    let done = ready.clone();
    let wake = waker.clone();
    DispatcherTimer::run_once(
        move || {
            done.set(true);
            if let Some(waker) = wake.borrow_mut().take() {
                waker.wake();
            }
        },
        Duration::from_millis(3),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
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
#[test]
fn external_compiled_deferred_scope_realizes_native_children_and_replaces_content_later() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch_task(
            || async {
                ferrodock_controls_deferred_content_control::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DeferredConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let grid = value.downcast_ref::<Ref<Grid>>().unwrap();
                let first = grid
                    .children()
                    .get(0)
                    .cast::<DeferredContentControl>()
                    .unwrap();
                let second = grid
                    .children()
                    .get(1)
                    .cast::<DeferredContentControl>()
                    .unwrap();
                let timeline = DeferredContentScheduling::get_timeline(grid).unwrap();
                assert_eq!(
                    timeline.budget_mode(),
                    DeferredContentPresentationBudgetMode::ItemCount
                );
                assert_eq!(timeline.max_presentations_per_pass(), 1);
                assert_eq!(timeline.initial_delay(), Duration::from_millis(20));
                assert!(Rc::ptr_eq(
                    &timeline,
                    &DeferredContentScheduling::get_timeline(&first).unwrap()
                ));
                assert_eq!(DeferredContentScheduling::get_order(&second), -10);
                let scene = Scene(Window::new());
                scene.0.set_width(640.0);
                scene.0.set_height(360.0);
                scene.0.set_content(Some(Control::boxed(grid.clone())));
                scene.0.show();
                scene.0.update_layout();
                assert!(first.presenter().unwrap().child().is_none());
                assert!(second.presenter().unwrap().child().is_none());
                let until = Instant::now() + Duration::from_secs(2);
                while (first.presenter().unwrap().child().is_none()
                    || second.presenter().unwrap().child().is_none())
                    && Instant::now() < until
                {
                    delay().await;
                    Dispatcher::ui_thread().run_jobs(None);
                    scene.0.update_layout();
                }
                let old = first.presenter().unwrap().child().unwrap();
                assert_eq!(
                    old.cast::<TextBlock>().unwrap().text().as_deref(),
                    Some("First")
                );
                assert_eq!(
                    second
                        .presenter()
                        .unwrap()
                        .child()
                        .unwrap()
                        .cast::<TextBlock>()
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some("Second")
                );
                let next = TextBlock::new();
                next.set_text(Some("Replacement"));
                first.set_content(Some(Control::boxed(next.clone())));
                scene.0.update_layout();
                assert_eq!(first.presenter().unwrap().child(), Some(old.clone()));
                let until = Instant::now() + Duration::from_secs(2);
                while first.presenter().unwrap().child() != Some(next.clone().upcast())
                    && Instant::now() < until
                {
                    delay().await;
                    Dispatcher::ui_thread().run_jobs(None);
                    scene.0.update_layout();
                }
                assert_eq!(first.presenter().unwrap().child(), Some(next.upcast()));
                assert!(old.visual_parent().is_none());
                first.clear_presentation();
                assert!(first.presenter().unwrap().child().is_none());
                assert!(first.content().is_some());
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

// --- the delays are durations (C5 T3 of #423) -----------------------------------------------
//
// Not from upstream (decision O2 of docs/idiomatic-rust-signatures.md): the delay of the
// deferred content scheduling, the intervals of its timeline and the expansion delay of the tab
// panel of the browser theme are `Duration` values. A time span literal sets them; a negative
// one is an error of the run-time loader and of the compiler, never clamped to zero; a binding
// that gives a negative span leaves the property at the value it has without the binding.

/// Runs `test` on the thread of the headless session of the tests of this module.
fn run_on_session(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_controls_deferred_content_control::register_types();
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

/// The documents of the delays, by the member they set; `{}` is the time span.
const DELAY_DOCUMENTS: &[(&str, &str)] = &[
    (
        "Delay",
        "<Border xmlns='https://github.com/ferroui' xmlns:dc='using:FerroDock.Controls.DeferredContentControl' dc:DeferredContentScheduling.Delay='{}'/>",
    ),
    (
        "ExpansionDelay",
        "<controls:DelayedUniformTabPanel xmlns='https://github.com/ferroui' xmlns:controls='using:FerroDock.Ferro.Controls' ExpansionDelay='{}'/>",
    ),
    (
        "InitialDelay",
        "<Grid xmlns='https://github.com/ferroui' xmlns:dc='using:FerroDock.Controls.DeferredContentControl'><dc:DeferredContentScheduling.Timeline><dc:DeferredContentPresentationTimeline InitialDelay='{}'/></dc:DeferredContentScheduling.Timeline></Grid>",
    ),
    (
        "FollowUpDelay",
        "<Grid xmlns='https://github.com/ferroui' xmlns:dc='using:FerroDock.Controls.DeferredContentControl'><dc:DeferredContentScheduling.Timeline><dc:DeferredContentPresentationTimeline FollowUpDelay='{}'/></dc:DeferredContentScheduling.Timeline></Grid>",
    ),
    (
        "RevealDuration",
        "<Grid xmlns='https://github.com/ferroui' xmlns:dc='using:FerroDock.Controls.DeferredContentControl'><dc:DeferredContentScheduling.Timeline><dc:DeferredContentPresentationTimeline RevealDuration='{}'/></dc:DeferredContentScheduling.Timeline></Grid>",
    ),
    (
        "MaxRealizationTimePerPass",
        "<Grid xmlns='https://github.com/ferroui' xmlns:dc='using:FerroDock.Controls.DeferredContentControl'><dc:DeferredContentScheduling.Timeline><dc:DeferredContentPresentationTimeline MaxRealizationTimePerPass='{}'/></dc:DeferredContentScheduling.Timeline></Grid>",
    ),
];

/// Loads `xaml` with the run-time loader: the root object, or the error in its debug form
/// (with the errors it wraps).
fn try_load(xaml: &str) -> Result<BoxedValue, String> {
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
        ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(xaml),
        Some(ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default()),
    )
    .map_err(|error| format!("{error:?}"))
}

#[test]
fn a_time_span_literal_sets_the_deferred_and_tab_panel_delays() {
    run_on_session(|| {
        let border = try_load(&DELAY_DOCUMENTS[0].1.replace("{}", "0:0:0.25")).unwrap();
        let border = border.downcast_ref::<Ref<Border>>().unwrap();
        assert_eq!(DeferredContentScheduling::get_delay(border), Duration::from_millis(250));

        let panel = try_load(&DELAY_DOCUMENTS[1].1.replace("{}", "0:0:0.5")).unwrap();
        let panel = panel.downcast_ref::<Ref<DelayedUniformTabPanel>>().unwrap();
        assert_eq!(panel.expansion_delay(), Duration::from_millis(500));

        let timeline_of = |index: usize, span: &str| {
            let grid = try_load(&DELAY_DOCUMENTS[index].1.replace("{}", span)).unwrap();
            let grid = grid.downcast_ref::<Ref<Grid>>().unwrap().clone();
            DeferredContentScheduling::get_timeline(&grid).unwrap()
        };
        assert_eq!(timeline_of(2, "0:0:0.02").initial_delay(), Duration::from_millis(20));
        assert_eq!(timeline_of(3, "0:0:0.003").follow_up_delay(), Duration::from_millis(3));
        assert_eq!(timeline_of(4, "0:0:0").reveal_duration(), Duration::ZERO);
        assert_eq!(timeline_of(5, "0.04").max_realization_time_per_pass(), Duration::from_millis(40));
    });
}

#[test]
fn a_negative_time_span_literal_for_a_deferred_or_tab_panel_delay_is_an_error() {
    run_on_session(|| {
        for (member, document) in DELAY_DOCUMENTS {
            // The document loads with a time span that is not negative.
            assert!(try_load(&document.replace("{}", "0:0:1")).is_ok(), "{member}");

            let error = match try_load(&document.replace("{}", "-0:0:1")) {
                Ok(_) => panic!("{member}: a negative time span is not a duration"),
                Err(error) => error,
            };
            assert!(error.contains("Duration"), "{member}: {error}");
            if *member == "Delay" || *member == "ExpansionDelay" {
                assert!(
                    error.contains("The TimeSpan -00:00:01 is negative and cannot be converted to a Duration."),
                    "{member}: {error}"
                );
            }

            // The compiler refuses the document, naming the member.
            let negative = document.replace("{}", "-0:0:1");
            let compiled = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
                &[("NegativeDelay.xaml", negative.as_str())],
                Some("ferres://ferrodock_markup_tests/"),
                &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
                &[],
            );
            let reason = compiled[0].source.clone().expect_err("the document is refused");
            assert!(
                reason.contains(&format!("{member}: the time span -00:00:01 is negative, and a `Duration` cannot be negative")),
                "{member}: {reason}"
            );
        }
    });
}

/// A view model with a time span.
struct Timing {
    span: Cell<SignedDuration>,
    property_changed: Event<str>,
}

impl Timing {
    fn new(span: SignedDuration) -> Rc<Self> {
        Model::new_model(Self { span: Cell::new(span), property_changed: Event::new() })
    }

    fn set_span(&self, value: SignedDuration) {
        self.span.set(value);
        self.property_changed.raise("Span");
    }
}

impl INotifyPropertyChanged for Timing {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Timing, |b| b
    .notify_property_changed()
    .property::<Value<SignedDuration>>("Span", |o| o.span.get(), |o, v| o.set_span(v)));

/// Binds `property` of `target` to a time span that starts negative: the property keeps
/// `default` (what `get` reads without the binding); a span that is not negative is set as the
/// duration of the same value; a negative one again returns the property to `default`.
fn a_negative_span_keeps_the_default(
    target: &FerroObject,
    property: &'static FerroProperty,
    get: impl Fn() -> Duration,
    default: Duration,
) {
    let name = property.name();
    assert_eq!(get(), default, "{name}: the default");
    let source = Timing::new(SignedDuration::from_seconds(-1.0));
    let binding = ReflectionBinding::new("Span");
    binding.set_source(Some(source.clone() as BoxedValue));
    target.bind_binding(property, &*binding);
    assert_eq!(get(), default, "{name}: a negative span is not set");

    source.set_span(SignedDuration::from_milliseconds(250.0));
    assert_eq!(get(), Duration::from_millis(250), "{name}: a span that is not negative is set");

    source.set_span(SignedDuration::from_seconds(-2.0));
    assert_eq!(get(), default, "{name}: a negative span returns the property to its default");
}

#[test]
fn a_negative_span_from_a_binding_leaves_the_deferred_and_tab_panel_delays_at_their_defaults() {
    run_on_session(|| {
        let border = Border::new();
        a_negative_span_keeps_the_default(
            &border,
            DeferredContentScheduling::delay_property().as_property(),
            || DeferredContentScheduling::get_delay(&border),
            Duration::ZERO,
        );

        let panel = DelayedUniformTabPanel::new();
        a_negative_span_keeps_the_default(
            &panel,
            DelayedUniformTabPanel::expansion_delay_property().as_property(),
            || panel.expansion_delay(),
            Duration::from_secs(1),
        );
    });
}
