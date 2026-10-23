//! Warnings about mistakes the framework can see while it mounts and patches
//! element trees ([`Diagnostic`]). They are checked in debug builds (a panic
//! an error boundary caught is reported in every build); the handler set
//! with [`set_diagnostics_handler`] receives them, and without one they are
//! printed to the standard error, each once.

use crate::element::Key;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

/// A warning of the declarative API.
#[derive(Clone, Debug, PartialEq)]
pub enum Diagnostic {
    /// A list of elements (`for` in `rsx!`, a `Vec`) whose items have no
    /// keys: an item that moves or is removed makes the items after it patch
    /// one another's controls instead of keeping their own.
    MissingKeys {
        /// The number of items without a key.
        items: usize,
        /// What the first of them is (`Button`, a component).
        first: String,
    },
    /// Two siblings with the same key: the second does not keep its
    /// control.
    DuplicateKey {
        /// The key.
        key: Key,
    },
    /// A component that ran again more times than [`RUNAWAY_RUNS`] in one
    /// propagation of changes: a run that writes a signal it reads.
    RunawayComponent {
        /// The component.
        name: &'static str,
        /// Its runs in the propagation.
        runs: u32,
    },
    /// The source of an `rsx!` changed more than its literals: hot reload
    /// applies literals only, the rest needs a rebuild.
    NeedsRebuild {
        /// The file.
        file: String,
        /// The line of the `rsx!` in the build.
        line: u32,
    },
    /// Hot reload applied changed literals of a file.
    HotReloaded {
        /// The file.
        file: String,
        /// How many literals now differ from the build.
        literals: usize,
    },
    /// A run of a component below an error boundary panicked: the nearest
    /// boundary shows its fallback (see [`error_boundary`](fn@crate::error_boundary)).
    ErrorCaught {
        /// The component whose run panicked.
        component: &'static str,
        /// The message of the panic.
        message: String,
    },
}

/// The runs of a component in one propagation of changes beyond which it is
/// reported as running away.
pub const RUNAWAY_RUNS: u32 = 100;

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Diagnostic::MissingKeys { items, first } => write!(
                f,
                "a list of {items} elements ({first}, ...) has no keys: give each item a `key` so that it keeps its control when the list changes"
            ),
            Diagnostic::DuplicateKey { key } => {
                write!(f, "two siblings have the key {key:?}: keys identify siblings, so each must be unique among them")
            }
            Diagnostic::RunawayComponent { name, runs } => write!(
                f,
                "the component {name} ran {runs} times in one propagation of changes: does its function write a signal it reads?"
            ),
            Diagnostic::NeedsRebuild { file, line } => {
                write!(f, "{file}:{line}: the rsx! changed more than its literals; hot reload applies literals only, rebuild to see the change")
            }
            Diagnostic::HotReloaded { file, literals } => write!(f, "{file}: hot reload, {literals} literals differ from the build"),
            Diagnostic::ErrorCaught { component, message } => {
                write!(f, "the component {component} panicked ({message}); the nearest error boundary shows its fallback")
            }
        }
    }
}

type Handler = Rc<dyn Fn(&Diagnostic)>;

ferroui_base::ferro_thread_local! {
    static HANDLER: RefCell<Option<Handler>> = RefCell::new(None);
    static PRINTED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Gives the diagnostics of the thread to `handler` instead of printing
/// them (a test, a log, an inspector).
pub fn set_diagnostics_handler(handler: impl Fn(&Diagnostic) + 'static) {
    HANDLER.with(|slot| *slot.borrow_mut() = Some(Rc::new(handler)));
}

/// Prints the diagnostics again, each once.
pub fn clear_diagnostics_handler() {
    HANDLER.with(|slot| *slot.borrow_mut() = None);
}

/// Reports a diagnostic.
pub(crate) fn report(diagnostic: Diagnostic) {
    let handler = HANDLER.with(|slot| slot.borrow().clone());
    match handler {
        Some(handler) => handler(&diagnostic),
        None => {
            let text = diagnostic.to_string();
            let first = PRINTED.with(|printed| printed.borrow_mut().insert(text.clone()));
            if first {
                eprintln!("ferroui-view: {text}");
            }
        }
    }
}
