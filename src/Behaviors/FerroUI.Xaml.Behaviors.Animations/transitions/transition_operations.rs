//! Ported from `Transitions/TransitionOperations.cs`.

use ferroui_base::animation::{Animatable, ITransition, TransitionBase, Transitions};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{FerroObjectExtensions, Ref, StyledElement};
use std::rc::Rc;

/// Provides reusable operations for a styled element's transition collection.
pub struct TransitionOperations;

impl TransitionOperations {
    /// Adds a transition to an element, creating its collection when necessary. Returns `true`
    /// when the transition was added; otherwise, `false`.
    pub fn add(target: Option<&StyledElement>, transition: Option<&Ref<TransitionBase>>) -> bool {
        let (Some(target), Some(transition)) = (target, transition) else {
            return false;
        };

        let transitions = match target.transitions() {
            Some(transitions) => transitions,
            None => {
                let transitions = Transitions::new();
                target.set_transitions(Some(transitions.clone()));
                transitions
            }
        };
        let transition: Rc<dyn ITransition> = transition.into();
        transitions.add(transition);
        true
    }

    /// Removes a transition from an element. Returns `true` when the transition was removed;
    /// otherwise, `false`.
    pub fn remove(target: Option<&StyledElement>, transition: Option<&Ref<TransitionBase>>) -> bool {
        let Some(transitions) = target.and_then(|target| target.transitions()) else {
            return false;
        };
        let Some(transition) = transition else {
            return false;
        };

        let transition: Rc<dyn ITransition> = transition.into();
        transitions.remove(&transition)
    }

    /// Clears all transitions from an element. Returns `true` when an existing collection was
    /// cleared; otherwise, `false`.
    pub fn clear(target: Option<&StyledElement>) -> bool {
        let Some(transitions) = target.and_then(|target| target.transitions()) else {
            return false;
        };

        transitions.clear();
        true
    }

    /// Replaces an element's transitions and returns the previous collection.
    pub fn replace(target: &StyledElement, transitions: Option<Transitions>) -> Option<Transitions> {
        let previous = target.transitions();
        target.set_transitions(transitions);
        previous
    }

    /// Observes replacements of an element's transition collection: `observer` is invoked with
    /// the current and subsequent transition collections. Returns a subscription that stops
    /// observation when disposed.
    ///
    /// The `ArgumentNullException` of the reference for a missing target or observer has no
    /// counterpart: both are required by the types of the parameters.
    pub fn observe(target: &StyledElement, observer: impl Fn(Option<Transitions>) + 'static) -> Rc<dyn IDisposable> {
        target
            .get_observable(Animatable::transitions_property())
            .subscribe(Rc::new(AnonymousObserver::new(observer)))
    }
}
