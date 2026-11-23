use super::{StyleActivator, StyleActivatorBase};
use crate::controls::IClassesChangedListener;
use crate::{StyledElement, WeakRef};
use std::cell::Cell;
use std::num::NonZeroU64;
use std::rc::{Rc, Weak};

/// An activator which is active when a set of classes match those on a
/// control.
pub(crate) struct StyleClassActivator {
    base: StyleActivatorBase,
    this: Weak<StyleClassActivator>,
    control: WeakRef<StyledElement>,
    match_: Rc<[String]>,
    /// The token the activator listens to the classes of the control with. A token is never
    /// zero, so that none is no wider than one.
    listener: Cell<Option<NonZeroU64>>,
}

impl StyleClassActivator {
    pub fn new(control: &StyledElement, match_: Rc<[String]>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            base: StyleActivatorBase::new(),
            this: this.clone(),
            control: control.to_ref().downgrade(),
            match_,
            listener: Cell::new(None),
        })
    }

    pub fn are_classes_matching(classes: &[String], to_match: &[String]) -> bool {
        let mut remaining_matches = to_match.len();
        let classes_count = classes.len();

        // Early bail out - we can't match if the control does not have enough
        // classes.
        if classes_count < remaining_matches {
            return false;
        }

        for c in classes {
            if to_match.contains(c) {
                remaining_matches -= 1;

                // Already matched so we can skip checking other classes.
                if remaining_matches == 0 {
                    break;
                }
            }
        }

        remaining_matches == 0
    }
}

impl IClassesChangedListener for StyleClassActivator {
    fn changed(&self) {
        self.reevaluate_is_active();
    }
}

impl StyleActivator for StyleClassActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        match self.control.upgrade() {
            Some(control) => Self::are_classes_matching(&control.classes().snapshot(), &self.match_),
            None => false,
        }
    }

    fn initialize(&self) {
        if let Some(control) = self.control.upgrade() {
            // The activator is the listener (docs/porting/desktop-performance.md, section 9.66):
            // a control has one for every style with a class selector that matches its type,
            // and a closure for each was a block of its own. The classes keep the activator
            // alive until it stops listening, and the activator does not keep the control.
            if let Some(this) = self.this.upgrade() {
                let token = control.classes().add_changed_listener(this);
                self.listener.set(NonZeroU64::new(token));
            }
        }
    }

    fn deinitialize(&self) {
        if let Some(token) = self.listener.take() {
            if let Some(control) = self.control.upgrade() {
                control.classes().remove_listener(token.get());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{IStyleActivator, IStyleActivatorSink};
    use std::cell::RefCell;
    use std::mem::size_of;

    struct Sink {
        values: RefCell<Vec<bool>>,
    }

    impl IStyleActivatorSink for Sink {
        fn on_next(&self, value: bool) {
            self.values.borrow_mut().push(value);
        }
    }

    /// The activator is what the classes of its control tell: one listener and no block
    /// besides the activator while it is subscribed, and none after.
    #[test]
    fn the_activator_listens_to_the_classes_itself_while_it_is_subscribed() {
        let control = StyledElement::new();
        let activator = StyleClassActivator::new(&control, Rc::from(vec![String::from("foo")]));
        assert_eq!(control.classes().listener_count(), 0);
        assert_eq!(Rc::strong_count(&activator), 1);

        let sink = Rc::new(Sink { values: RefCell::new(Vec::new()) });
        let weak: Weak<dyn IStyleActivatorSink> = Rc::downgrade(&sink) as Weak<dyn IStyleActivatorSink>;
        activator.subscribe(weak.clone());
        assert!(activator.is_subscribed());
        assert!(!activator.get_is_active());
        assert_eq!(control.classes().listener_count(), 1);
        assert_eq!(Rc::strong_count(&activator), 2, "the classes hold the activator that listens");

        control.classes().add("foo");
        control.classes().add("bar");
        control.classes().remove("foo");
        assert_eq!(*sink.values.borrow(), [true, false]);

        activator.unsubscribe(&weak);
        assert!(!activator.is_subscribed());
        assert_eq!(control.classes().listener_count(), 0);
        assert_eq!(Rc::strong_count(&activator), 1);
        control.classes().add("foo");
        assert_eq!(*sink.values.borrow(), [true, false], "an activator that stopped listening is not told");

        // Subscribed again, it listens again, and a control that goes away lets go of it.
        activator.subscribe(weak);
        assert_eq!(Rc::strong_count(&activator), 2);
        drop(control);
        assert_eq!(Rc::strong_count(&activator), 1);
        assert!(!activator.get_is_active());
    }

    /// What an activator of a class selector keeps, one for every such style that matches the
    /// type of a control: no word for a sink or a token that is not there.
    #[test]
    fn an_activator_keeps_no_word_for_what_is_absent() {
        assert_eq!(size_of::<Cell<Option<NonZeroU64>>>(), size_of::<u64>());
        assert_eq!(size_of::<StyleActivatorBase>(), 3 * size_of::<usize>());
        assert_eq!(
            size_of::<StyleClassActivator>(),
            size_of::<StyleActivatorBase>()
                + size_of::<Weak<StyleClassActivator>>()
                + size_of::<WeakRef<StyledElement>>()
                + size_of::<Rc<[String]>>()
                + size_of::<u64>(),
        );
    }
}
