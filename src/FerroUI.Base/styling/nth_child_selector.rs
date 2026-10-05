use super::activators::NthChildActivator;
use super::{Selector, SelectorMatch, SelectorNode, Style, StyleBase};
use crate::logical_tree::IChildIndexProvider;
use crate::{StyledElement, TypeInfo};
use std::any::Any;

const NTH_CHILD_SELECTOR_NAME: &str = "nth-child";
const NTH_LAST_CHILD_SELECTOR_NAME: &str = "nth-last-child";

/// The `:nth-child()` pseudo-class matches elements based on their position
/// among a group of siblings.
///
/// Element indices are 1-based.
pub(crate) struct NthChildSelector {
    previous: Option<Selector>,
    reversed: bool,
    step: i32,
    offset: i32,
}

impl NthChildSelector {
    pub(crate) fn with_direction(previous: Option<Selector>, step: i32, offset: i32, reversed: bool) -> Self {
        Self { previous, reversed, step, offset }
    }

    /// Creates an instance of the nth-child selector.
    ///
    /// `step` is the position step and `offset` the initial index offset.
    pub fn new(previous: Option<Selector>, step: i32, offset: i32) -> Self {
        Self::with_direction(previous, step, offset, false)
    }

    #[allow(dead_code)]
    pub fn step(&self) -> i32 {
        self.step
    }

    #[allow(dead_code)]
    pub fn offset(&self) -> i32 {
        self.offset
    }

    pub(crate) fn evaluate_index(
        mut index: i32,
        child_index_provider: &dyn IChildIndexProvider,
        step: i32,
        offset: i32,
        reversed: bool,
    ) -> SelectorMatch {
        if index < 0 {
            return SelectorMatch::NEVER_THIS_INSTANCE;
        }

        if reversed {
            match child_index_provider.try_get_total_count() {
                Some(total_count) => index = total_count - index,
                None => return SelectorMatch::NEVER_THIS_INSTANCE,
            }
        } else {
            // nth child index is 1-based
            index += 1;
        }

        let n = step.signum();
        let diff = index - offset;
        let match_ = diff == 0 || (diff.signum() == n && diff % step == 0);

        if match_ {
            SelectorMatch::ALWAYS_THIS_INSTANCE
        } else {
            SelectorMatch::NEVER_THIS_INSTANCE
        }
    }
}

impl SelectorNode for NthChildSelector {
    fn in_template(&self) -> bool {
        self.previous.as_ref().is_some_and(Selector::in_template)
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        self.previous.as_ref().and_then(Selector::target_type)
    }

    fn evaluate(&self, control: &StyledElement, _parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        let control_parent = control.parent();
        let provider = control_parent.as_ref().and_then(|p| p.child_index_provider());

        match (control_parent, provider) {
            (Some(control_parent), Some(provider)) => {
                if subscribe {
                    SelectorMatch::sometimes(NthChildActivator::new(
                        control,
                        &control_parent,
                        self.step,
                        self.offset,
                        self.reversed,
                    ))
                } else {
                    Self::evaluate_index(
                        provider.get_child_index(control),
                        &*provider,
                        self.step,
                        self.offset,
                        self.reversed,
                    )
                }
            }
            _ => SelectorMatch::NEVER_THIS_INSTANCE,
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        let mut builder = String::with_capacity(NTH_LAST_CHILD_SELECTOR_NAME.len() + 8);

        if let Some(previous) = &self.previous {
            builder.push_str(&previous.to_string_with_next(owner, true));
        }
        builder.push(':');
        builder.push_str(if self.reversed { NTH_LAST_CHILD_SELECTOR_NAME } else { NTH_CHILD_SELECTOR_NAME });
        builder.push('(');

        let mut has_step = false;
        if self.step != 0 {
            has_step = true;
            builder.push_str(&self.step.to_string());
            builder.push('n');
        }

        if self.offset > 0 {
            if has_step {
                builder.push('+');
            }
            builder.push_str(&self.offset.to_string());
        } else if self.offset < 0 {
            builder.push('-');
            builder.push_str(&(-(self.offset as i64)).to_string());
        }

        builder.push(')');
        builder
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
