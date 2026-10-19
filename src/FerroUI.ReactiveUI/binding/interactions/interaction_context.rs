//! `InteractionContext<TInput, TOutput>` (ReactiveUI.Binding, `Interactions/InteractionContext.cs`).

use super::{IInteractionContext, IOutputContext};
use std::cell::RefCell;
use std::fmt;

/// The context of one question asked through an interaction (`InteractionContext<TInput, TOutput>`):
/// the input, and the output a handler sets once.
pub struct InteractionContext<I, O> {
    /// `Input`.
    input: I,
    /// `_output`, with `_outputSet`: `Some` once a handler set it.
    output: RefCell<Option<O>>,
}

impl<I, O> InteractionContext<I, O> {
    /// A context carrying `input`, without an output (`internal InteractionContext(TInput)`).
    ///
    /// Upstream it is internal, and its tests reach it through `InternalsVisibleTo`; here it is
    /// public.
    pub fn new(input: I) -> Self {
        Self { input, output: RefCell::new(None) }
    }
}

impl<I, O> IInteractionContext<I, O> for InteractionContext<I, O> {
    fn input(&self) -> &I {
        &self.input
    }

    fn is_handled(&self) -> bool {
        self.output.borrow().is_some()
    }

    /// Sets the output and marks the interaction handled.
    ///
    /// # Panics
    /// When the output has already been set (`InvalidOperationException`).
    fn set_output(&self, output: O) {
        let mut slot = self.output.borrow_mut();
        if slot.is_some() {
            drop(slot);
            panic!("InvalidOperationException: Output has already been set.");
        }
        *slot = Some(output);
    }
}

impl<I, O: Clone> IOutputContext<I, O> for InteractionContext<I, O> {
    fn get_output(&self) -> O {
        match &*self.output.borrow() {
            Some(output) => output.clone(),
            None => panic!("InvalidOperationException: Output has not been set."),
        }
    }
}

impl<I: fmt::Debug, O: fmt::Debug> fmt::Debug for InteractionContext<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InteractionContext").field("input", &self.input).field("output", &self.output.borrow()).finish()
    }
}
