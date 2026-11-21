use crate::animation::i_page_transition::start_async;
use crate::animation::{IPageTransition, IProgressPageTransition, PageTransitionItem};
use crate::threading::CancellationToken;
use ferroui_system::task::{JoinError, JoinHandle};
use crate::{Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// Defines a composite page transition that can be used to combine
/// multiple transitions, which all run at the same time.
pub struct CompositePageTransition {
    page_transitions: RefCell<Vec<Rc<dyn IPageTransition>>>,
}

impl Default for CompositePageTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositePageTransition {
    pub fn new() -> Self {
        // The casts of the transition to `IPageTransition` (its `ValueTypeRegistration`), which
        // compiled markup needs before it sets a transition, are registered with the first one
        // made (#612): every transition of the type is made here.
        static CASTS: std::sync::Once = std::sync::Once::new();
        CASTS.call_once(crate::data::core::ValueTypes::register_used::<Self>);
        Self { page_transitions: RefCell::new(Vec::new()) }
    }

    /// The transitions to be executed.
    pub fn page_transitions(&self) -> Vec<Rc<dyn IPageTransition>> {
        self.page_transitions.borrow().clone()
    }

    pub fn set_page_transitions(&self, value: Vec<Rc<dyn IPageTransition>>) {
        *self.page_transitions.borrow_mut() = value;
    }

    /// Adds a transition.
    pub fn add(&self, transition: Rc<dyn IPageTransition>) {
        self.page_transitions.borrow_mut().push(transition);
    }
}

impl IPageTransition for CompositePageTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        let transition_tasks: Vec<JoinHandle<()>> = self
            .page_transitions()
            .iter()
            .map(|transition| transition.start(from, to, forward, cancellation_token.clone()))
            .collect();
        start_async(async move {
            let mut canceled = false;
            for task in transition_tasks {
                match task.await {
                    Ok(()) => {}
                    // A failed transition fails the composite with its failure (the payload of
                    // its panic, or the error it reported), without a panic (#567).
                    Err(error @ JoinError::Panicked(_)) => return Err(error),
                    Err(JoinError::Canceled) => canceled = true,
                }
            }
            if canceled {
                // The payload a `panic!` with the message had (#567).
                return Err(JoinError::Panicked(Box::new(crate::threading::Canceled.to_string())));
            }
            Ok(())
        })
    }

    fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
        Some(self)
    }

    fn as_composite_page_transition(&self) -> Option<&CompositePageTransition> {
        Some(self)
    }
}

impl IProgressPageTransition for CompositePageTransition {
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        for transition in self.page_transitions() {
            if let Some(progressive) = transition.as_progress_page_transition() {
                progressive.update(progress, from, to, forward, page_length, visible_items);
            }
        }
    }

    fn reset(&self, visual: &Ref<Visual>) {
        for transition in self.page_transitions() {
            if let Some(progressive) = transition.as_progress_page_transition() {
                progressive.reset(visual);
            }
        }
    }
}
