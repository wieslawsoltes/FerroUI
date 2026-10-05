use super::{Selector, SelectorMatch, SelectorMatchResult, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// The child combinator: matches a control whose logical parent matches the
/// preceding selector.
pub(crate) struct ChildSelector {
    parent: Selector,
    selector_string: OnceCell<String>,
}

impl ChildSelector {
    pub fn new(parent: Option<Selector>) -> Self {
        match parent {
            Some(parent) => Self { parent, selector_string: OnceCell::new() },
            None => panic!("Child selector must be preceeded by a selector."),
        }
    }
}

impl SelectorNode for ChildSelector {
    fn in_template(&self) -> bool {
        self.parent.in_template()
    }

    fn is_combinator(&self) -> bool {
        true
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string
            .get_or_init(|| format!("{} > ", self.parent.to_string_with_next(owner, true)))
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        match control.parent() {
            Some(control_parent) => {
                let parent_match = self.parent.match_(&control_parent, parent, subscribe);

                if parent_match.result() == SelectorMatchResult::Sometimes {
                    parent_match
                } else if parent_match.is_match() {
                    SelectorMatch::ALWAYS_THIS_INSTANCE
                } else {
                    SelectorMatch::NEVER_THIS_INSTANCE
                }
            }
            None => SelectorMatch::NEVER_THIS_INSTANCE,
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        Some(&self.parent)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
