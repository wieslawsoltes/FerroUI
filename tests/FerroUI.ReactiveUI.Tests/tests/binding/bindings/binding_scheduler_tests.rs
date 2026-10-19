//! `BindingSchedulerTests` (`ReactiveUI.Binding.Tests/Bindings/BindingSchedulerTests.cs`).
//!
//! "Another thread" is a call while the stub invoker answers that the calling thread has no
//! access. The invokers are registered per thread, and each test runs on a thread of its own.

use crate::tests::binding::test_models::{ManualObservable, RecordingObserver, RecordingSynchronizationContext, StubViewThreadInvoker};
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

const WRITTEN: &str = "written";

/// An object no invoker claims (`UnclaimedTarget`).
fn unclaimed_target() -> Rc<dyn Any> {
    Rc::new(())
}

/// Registers `invoker` and reads the invokers again (`RegisterInvoker`).
fn register_invoker(invoker: &Rc<StubViewThreadInvoker>) {
    AppLocator::register_constant::<dyn IViewThreadInvoker>(invoker.clone());
    ViewThreadInvokers::refresh();
}

/// Delivers through `context` (`UseMainThread`).
fn use_main_thread(context: &Arc<RecordingSynchronizationContext>) {
    BindingSchedulers::use_synchronization_context(Some(context.clone() as Arc<dyn SynchronizationContext>));
}

fn same_source<T: 'static>(routed: &Rc<dyn IObservable<T>>, source: &Rc<ManualObservable<T>>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(routed), Rc::as_ptr(source))
}

#[test]
fn observe_on_view_thread_on_the_owning_thread_writes_inline() {
    let target: Rc<dyn Any> = Rc::new(());
    let invoker = StubViewThreadInvoker::new(vec![target.clone()]);
    invoker.set_has_access(true);
    let host = RecordingSynchronizationContext::new();
    let observer = RecordingObserver::<String>::new();
    register_invoker(&invoker);
    use_main_thread(&host);

    let source = ManualObservable::<String>::new();
    let subscription = BindingSchedulers::observe_on_view_thread(source.clone(), Some(target)).subscribe(observer.clone());

    source.on_next(WRITTEN.to_string());

    assert_eq!(observer.values().join(","), WRITTEN);
    assert_eq!(invoker.post_count(), 0);
    assert_eq!(host.post_count(), 0);
    subscription.dispose();
    BindingSchedulers::use_synchronization_context(None);
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_from_another_thread_posts_through_the_invoker() {
    let target: Rc<dyn Any> = Rc::new(());
    let invoker = StubViewThreadInvoker::new(vec![target.clone()]);
    let observer = RecordingObserver::<String>::new();
    register_invoker(&invoker);

    let source = ManualObservable::<String>::new();
    let subscription = BindingSchedulers::observe_on_view_thread(source.clone(), Some(target)).subscribe(observer.clone());

    source.on_next(WRITTEN.to_string());

    assert_eq!(observer.values().len(), 0);
    assert_eq!(invoker.post_count(), 1);

    invoker.run_posted();

    assert_eq!(observer.values().join(","), WRITTEN);
    subscription.dispose();
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_from_another_thread_when_the_host_sets_a_main_thread_delivers_through_it() {
    let target: Rc<dyn Any> = Rc::new(());
    let invoker = StubViewThreadInvoker::new(vec![target.clone()]);
    let host = RecordingSynchronizationContext::new();
    let observer = RecordingObserver::<String>::new();
    register_invoker(&invoker);
    use_main_thread(&host);

    let source = ManualObservable::<String>::new();
    let subscription = BindingSchedulers::observe_on_view_thread(source.clone(), Some(target)).subscribe(observer.clone());

    source.on_next(WRITTEN.to_string());

    assert_eq!(host.post_count(), 1);
    assert_eq!(invoker.post_count(), 0);
    assert_eq!(observer.values().join(","), WRITTEN);
    subscription.dispose();
    BindingSchedulers::use_synchronization_context(None);
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_when_nothing_claims_the_target_hands_back_the_source() {
    register_invoker(&StubViewThreadInvoker::unclaimed());
    use_main_thread(&RecordingSynchronizationContext::new());

    let source = ManualObservable::<String>::new();

    let routed = BindingSchedulers::observe_on_view_thread(source.clone(), Some(unclaimed_target()));
    assert!(same_source(&routed, &source));
    BindingSchedulers::use_synchronization_context(None);
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_with_no_target_hands_back_the_source() {
    let source = ManualObservable::<String>::new();

    let routed = BindingSchedulers::observe_on_view_thread(source.clone(), None);
    assert!(same_source(&routed, &source));
}

#[test]
fn observe_on_view_thread_with_two_targets_posts_each_through_its_own_invoker() {
    let first_target: Rc<dyn Any> = Rc::new(());
    let second_target: Rc<dyn Any> = Rc::new(());
    let first_invoker = StubViewThreadInvoker::new(vec![first_target.clone()]);
    let second_invoker = StubViewThreadInvoker::new(vec![second_target.clone()]);
    register_invoker(&first_invoker);
    register_invoker(&second_invoker);

    let first = ManualObservable::<String>::new();
    let second = ManualObservable::<String>::new();

    let first_subscription = BindingSchedulers::observe_on_view_thread(first.clone(), Some(first_target)).subscribe(RecordingObserver::<String>::new());
    let second_subscription = BindingSchedulers::observe_on_view_thread(second.clone(), Some(second_target)).subscribe(RecordingObserver::<String>::new());

    first.on_next(WRITTEN.to_string());

    assert_eq!(first_invoker.post_count(), 1);
    assert_eq!(second_invoker.post_count(), 0);
    second_subscription.dispose();
    first_subscription.dispose();
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_with_a_fallback_uses_it_when_nothing_registered_claims_the_target() {
    let fallback = StubViewThreadInvoker::unclaimed();
    let observer = RecordingObserver::<String>::new();
    register_invoker(&StubViewThreadInvoker::unclaimed());

    let source = ManualObservable::<String>::new();
    let subscription = BindingSchedulers::observe_on_view_thread_with_fallback(source.clone(), Some(unclaimed_target()), fallback.clone()).subscribe(observer.clone());

    source.on_next(WRITTEN.to_string());
    fallback.run_posted();

    assert_eq!(fallback.post_count(), 1);
    assert_eq!(observer.values().join(","), WRITTEN);
    subscription.dispose();
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_with_a_fallback_prefers_a_registered_invoker() {
    let target: Rc<dyn Any> = Rc::new(());
    let registered = StubViewThreadInvoker::new(vec![target.clone()]);
    let fallback = StubViewThreadInvoker::unclaimed();
    register_invoker(&registered);

    let source = ManualObservable::<String>::new();
    let subscription = BindingSchedulers::observe_on_view_thread_with_fallback(source.clone(), Some(target), fallback.clone()).subscribe(RecordingObserver::<String>::new());

    source.on_next(WRITTEN.to_string());

    assert_eq!(registered.post_count(), 1);
    assert_eq!(fallback.post_count(), 0);
    subscription.dispose();
    ViewThreadInvokers::refresh();
}

#[test]
fn observe_on_view_thread_with_a_fallback_and_no_target_hands_back_the_source() {
    let source = ManualObservable::<String>::new();

    let routed = BindingSchedulers::observe_on_view_thread_with_fallback(source.clone(), None, StubViewThreadInvoker::unclaimed());
    assert!(same_source(&routed, &source));
}

// Not ported: ObserveOnViewThread_WithNoSource_Throws - argument validation

// Not ported: ObserveOnViewThread_WithAFallbackButNoSource_Throws - argument validation

// Not ported: ObserveOnViewThread_WithNoFallback_Throws - argument validation

#[test]
fn for_target_with_no_target_claims_nothing() {
    ViewThreadInvokers::refresh();

    assert!(ViewThreadInvokers::for_target(None).is_none());
}

#[test]
fn use_synchronization_context_with_no_context_clears_the_main_thread() {
    BindingSchedulers::use_synchronization_context(Some(RecordingSynchronizationContext::new() as Arc<dyn SynchronizationContext>));
    BindingSchedulers::use_synchronization_context(None);

    assert!(BindingSchedulers::main_thread().is_none());
}
