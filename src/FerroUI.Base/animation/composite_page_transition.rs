use crate::animation::i_page_transition::start_async;
use crate::animation::{IPageTransition, IProgressPageTransition, PageTransitionItem};
use crate::threading::{CancellationToken, DispatcherTask};
use crate::{Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// Defines a composite page transition that can be used to combine
/// multiple transitions, which all run at the same time.
#[derive(Default)]
pub struct CompositePageTransition {
    page_transitions: RefCell<Vec<Rc<dyn IPageTransition>>>,
}

impl CompositePageTransition {
    pub fn new() -> Self {
        Self::default()
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
    ) -> DispatcherTask<()> {
        let transition_tasks: Vec<DispatcherTask<()>> = self
            .page_transitions()
            .iter()
            .map(|transition| transition.start(from, to, forward, cancellation_token.clone()))
            .collect();
        start_async(async move {
            let mut canceled = false;
            for task in transition_tasks {
                // A failed transition re-raises its failure here, which
                // fails the composite.
                canceled |= task.await.is_err();
            }
            if canceled {
                panic!("{}", crate::threading::OperationCanceledError);
            }
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
