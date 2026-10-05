use super::activators::NotActivator;
use super::{Selector, SelectorMatch, SelectorMatchResult, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// The `:not()` style selector.
pub(crate) struct NotSelector {
    previous: Option<Selector>,
    argument: Selector,
    selector_string: OnceCell<String>,
}

impl NotSelector {
    /// Creates the selector from the previous selector and the selector to
    /// be not-ed.
    pub fn new(previous: Option<Selector>, argument: Selector) -> Self {
        Self { previous, argument, selector_string: OnceCell::new() }
    }
}

impl SelectorNode for NotSelector {
    fn in_template(&self) -> bool {
        self.argument.in_template()
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        self.previous.as_ref().and_then(Selector::target_type)
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string
            .get_or_init(|| {
                let previous = self.previous.as_ref().map(|p| p.to_string_with_next(owner, true)).unwrap_or_default();
                format!("{previous}:not({})", self.argument)
            })
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        let inner_result = self.argument.match_(control, parent, subscribe);

        match inner_result.result() {
            SelectorMatchResult::AlwaysThisInstance => SelectorMatch::NEVER_THIS_INSTANCE,
            SelectorMatchResult::AlwaysThisType => SelectorMatch::NEVER_THIS_TYPE,
            SelectorMatchResult::NeverThisInstance => SelectorMatch::ALWAYS_THIS_INSTANCE,
            SelectorMatchResult::NeverThisType => SelectorMatch::ALWAYS_THIS_TYPE,
            SelectorMatchResult::Sometimes => SelectorMatch::sometimes(NotActivator::new(
                inner_result.into_activator().expect("a sometimes match has an activator"),
            )),
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
