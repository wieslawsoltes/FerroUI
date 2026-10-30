//! Actual downstream native scheduling, timeline and deferred control metadata.
use ferrodock_controls_deferred_content_control::{
    DeferredContentControl, DeferredContentPresentationBudgetMode, DeferredContentScheduling,
};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher, DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{AppBuilder, Application, Control, Grid, TextBlock, Window};
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
                assert_eq!(timeline.initial_delay().total_milliseconds(), 20.0);
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
