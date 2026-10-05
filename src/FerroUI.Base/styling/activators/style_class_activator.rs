use super::{StyleActivator, StyleActivatorBase};
use crate::{StyledElement, WeakRef};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// An activator which is active when a set of classes match those on a
/// control.
pub(crate) struct StyleClassActivator {
    base: StyleActivatorBase,
    this: Weak<StyleClassActivator>,
    control: WeakRef<StyledElement>,
    match_: Rc<[String]>,
    listener: Cell<Option<u64>>,
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
            let weak = self.this.clone();
            let token = control.classes().add_listener(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.reevaluate_is_active();
                }
            }));
            self.listener.set(Some(token));
        }
    }

    fn deinitialize(&self) {
        if let Some(token) = self.listener.take() {
            if let Some(control) = self.control.upgrade() {
                control.classes().remove_listener(token);
            }
        }
    }
}
