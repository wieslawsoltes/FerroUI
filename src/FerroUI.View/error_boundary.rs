//! Error boundaries: a component that shows a fallback instead of ending the
//! application when a run of a component below it panics
//! ([`error_boundary`]).
//!
//! A component mounted below a boundary runs its function under
//! `catch_unwind` (`mount.rs`). A run that panics renders nothing, the
//! runtime and the frames of the hooks are put back as they were before the
//! run, and the panic is reported to the nearest boundary above, which is
//! found through the contexts of the owners. The boundary does not change
//! its tree there: the report waits until the outermost run of a component in
//! progress has ended, so no tree is patched while it is being reconciled.
//! Then the boundary's state changes, the boundary runs again and shows its
//! fallback, and the child tree, the component that panicked included, is
//! taken out and its owner disposed.
//!
//! Only the function of a component is guarded: a panic in a handler, in an
//! effect started by `effect` or `use_effect`, or in the mounting of controls
//! propagates as before. Without a boundary above, a component runs as it did
//! before, and a panic propagates.

use crate::element::{rerender, with_key, Element, Key};
use crate::hooks::{use_hook, use_signal};
use crate::reactive::{batch, on_cleanup, provide_context, Owner, Signal};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// What a fallback is given: the message of the panic, and a way to mount
/// the child again.
#[derive(Clone)]
pub struct ErrorInfo {
    message: Rc<str>,
    reset: Rc<dyn Fn()>,
}

impl ErrorInfo {
    /// The message of the panic (its text, when the panic carried text).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Mounts the child of the boundary again, as new: its components start
    /// with fresh hooks. Nothing happens when the boundary was taken out.
    pub fn reset(&self) {
        (self.reset)();
    }
}

impl fmt::Debug for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorInfo").field("message", &self.message).finish()
    }
}

/// The boundary as the components below find it in the context.
#[derive(Clone, Copy)]
pub(crate) struct Boundary {
    failed: Signal<Option<Rc<str>>>,
}

impl Boundary {
    /// Reports a panic of a run below the boundary; the boundary shows its
    /// fallback once the outermost run in progress has ended.
    pub(crate) fn report(&self, message: String) {
        PENDING.with(|pending| pending.borrow_mut().push((*self, Rc::from(message))));
    }
}

ferroui_base::ferro_thread_local! {
    /// The boundaries alive on the thread: without any, a component does not
    /// look for one.
    static LIVE: Cell<u32> = Cell::new(0);
    /// The runs of components in progress, nested.
    static DEPTH: Cell<u32> = Cell::new(0);
    /// The panics reported and not yet given to their boundaries.
    static PENDING: RefCell<Vec<(Boundary, Rc<str>)>> = RefCell::new(Vec::new());
}

/// The nearest boundary above what is made now, if any.
pub(crate) fn nearest() -> Option<Boundary> {
    if LIVE.with(Cell::get) == 0 {
        return None;
    }
    Owner::current().and_then(|owner| owner.context::<Boundary>())
}

/// A run of a component in progress: the reports of panics wait until the
/// outermost one has ended.
pub(crate) struct RunGuard(());

impl RunGuard {
    pub(crate) fn enter() -> Self {
        DEPTH.with(|depth| depth.set(depth.get() + 1));
        RunGuard(())
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        let depth = DEPTH.with(|depth| {
            let value = depth.get().saturating_sub(1);
            depth.set(value);
            value
        });
        // A panic that nothing caught goes on unwinding; no code of the
        // application runs on its way.
        if depth == 0 && !std::thread::panicking() {
            deliver();
        }
    }
}

/// Gives the reported panics to their boundaries. The first panic a
/// boundary is given is the one it shows.
fn deliver() {
    loop {
        let pending = PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
        if pending.is_empty() {
            return;
        }
        for (boundary, message) in pending {
            if boundary.failed.is_alive() && boundary.failed.with_untracked(Option::is_none) {
                boundary.failed.set(Some(message));
            }
        }
    }
}

/// A component that mounts `child()`; when a run of a component below it
/// panics (its first run or a later one), the boundary takes the child tree
/// out (disposing its owner) and shows `fallback(info)` instead of ending the
/// application. `info.reset()` mounts the child again.
///
/// ```ignore
/// error_boundary(
///     || rsx! { Report {} },
///     |info| rsx! {
///         StackPanel {
///             TextBlock { text: format!("The report failed: {}", info.message()) }
///             Button { on_click: move |_| info.reset(), "Try again" }
///         }
///     },
/// )
/// ```
///
/// The nearest boundary catches: a boundary inside the child of another
/// catches the panics below it, and a panic of a fallback goes to the
/// boundary above. What is guarded is the function of each component below
/// (`rerender`, `#[component(rerender)]`), `child` itself included; the
/// handlers and effects of the controls are not. A caught panic is reported
/// as [`Diagnostic::ErrorCaught`](crate::Diagnostic::ErrorCaught). The panic
/// hook of the process still runs, so the message of the panic is printed as
/// usual.
pub fn error_boundary(
    child: impl Fn() -> Element + 'static,
    fallback: impl Fn(ErrorInfo) -> Element + 'static,
) -> Element {
    let child: Rc<dyn Fn() -> Element> = Rc::new(child);
    rerender(move || {
        let failed = use_signal(|| None::<Rc<str>>);
        let generation = use_signal(|| 0_i64);
        use_hook(|| {
            LIVE.with(|live| live.set(live.get() + 1));
            on_cleanup(|| LIVE.with(|live| live.set(live.get().saturating_sub(1))));
        });
        provide_context(Boundary { failed });
        match failed.get() {
            Some(message) => {
                let reset: Rc<dyn Fn()> = Rc::new(move || {
                    if failed.is_alive() {
                        batch(|| {
                            generation.update(|generation| *generation += 1);
                            failed.set(None);
                        });
                    }
                });
                fallback(ErrorInfo { message, reset })
            }
            None => {
                // The child is a component of its own, below the boundary:
                // its runs are guarded, and a reset mounts it anew.
                let child = child.clone();
                with_key(rerender(move || child()), Key::Int(generation.get()))
            }
        }
    })
}
