//! The parts of ReactiveUI.Primitives that the `Reactive` flavour of ReactiveUI and of
//! ReactiveUI.Binding call (docs/porting/reactive.md, section 3.1), where `ferroui-reactive` has no
//! counterpart of the same behaviour.
//!
//! The others map to what exists: `ReturnSignal<T>` and `ImmutableReturnRxVoidSignal` are
//! `Observable::return_`, `FromAsyncSignal<T>` is `Observable::from_async`, `EmptyDisposable` and
//! `ActionDisposable` are `Disposable::empty` and `Disposable::create`, `OnceDisposable` is
//! `SingleAssignmentDisposable`, `SwapDisposable` and `MutableDisposable` are `SerialDisposable`,
//! and `MultipleDisposable` is `CompositeDisposable`.

pub(crate) mod broadcaster;
pub(crate) mod start_signal;
mod i_thread_affine_sequencer;
pub use i_thread_affine_sequencer::IThreadAffineSequencer;
