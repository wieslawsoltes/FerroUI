//! `DisposableMixinsTests` (`ReactiveUI.Tests/Utilities/DisposableMixinsTests.cs`).
//!
//! Upstream's `MultipleDisposable` of ReactiveUI.Primitives is `CompositeDisposable` in the
//! `Reactive` flavour, and `DisposeWith` is `DisposableExt::dispose_with`.

use ferroui_reactive::disposables::{CompositeDisposable, Disposable, DisposableExt};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn dispose_with_adds_to_composite() {
    let disposable1 = Disposable::create(|| {});
    let disposable2 = Disposable::create(|| {});
    let composite_disposable = CompositeDisposable::new();

    const EXPECTED_COUNT: usize = 2;
    let _ = disposable1.dispose_with(&composite_disposable);
    let _ = disposable2.dispose_with(&composite_disposable);

    assert_eq!(composite_disposable.count(), EXPECTED_COUNT);
}

#[test]
fn dispose_with_disposes_when_composite_disposed() {
    let disposed = Rc::new(Cell::new(false));
    let flag = disposed.clone();
    let disposable = Disposable::create(move || flag.set(true));
    let composite_disposable = CompositeDisposable::new();

    let _ = disposable.dispose_with(&composite_disposable);
    composite_disposable.dispose();

    assert!(disposed.get());
}

#[test]
fn dispose_with_returns_original_disposable() {
    let disposable = Disposable::create(|| {});
    let composite_disposable = CompositeDisposable::new();

    let result = disposable.clone().dispose_with(&composite_disposable);

    assert!(Rc::ptr_eq(&result, &disposable));
}
