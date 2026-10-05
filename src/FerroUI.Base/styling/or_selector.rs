use super::activators::OrActivatorBuilder;
use super::{Selector, SelectorMatch, SelectorMatchResult, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// The OR style selector.
pub(crate) struct OrSelector {
    selectors: Vec<Selector>,
    selector_string: OnceCell<String>,
    target_type: OnceCell<Option<&'static TypeInfo>>,
}

impl OrSelector {
    /// Creates the selector from the selectors to OR.
    pub fn new(selectors: Vec<Selector>) -> Self {
        if selectors.len() <= 1 {
            panic!("Need more than one selector to OR.");
        }
        Self { selectors, selector_string: OnceCell::new(), target_type: OnceCell::new() }
    }

    fn evaluate_target_type(&self) -> Option<&'static TypeInfo> {
        let mut result: Option<&'static TypeInfo> = None;

        for selector in &self.selectors {
            let selector_type = selector.target_type()?;

            match result {
                None => result = Some(selector_type),
                Some(_) => {
                    while let Some(current) = result {
                        if current.is_assignable_from(selector_type) {
                            break;
                        }
                        result = current.base_type();
                    }
                }
            }
        }

        result
    }
}

impl SelectorNode for OrSelector {
    fn in_template(&self) -> bool {
        false
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        *self.target_type.get_or_init(|| self.evaluate_target_type())
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.to_string_with_next(owner, false)
    }

    fn to_string_with_next(&self, owner: Option<&Style>, has_next: bool) -> String {
        self.selector_string
            .get_or_init(|| {
                let joined = self
                    .selectors
                    .iter()
                    .map(|x| x.to_string_with_next(owner, true))
                    .collect::<Vec<_>>()
                    .join(", ");
                if has_next {
                    format!("({joined})")
                } else {
                    joined
                }
            })
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        let mut activators = OrActivatorBuilder::new();
        let mut never_this_instance = false;

        for selector in &self.selectors {
            let match_ = selector.match_(control, parent, subscribe);

            match match_.result() {
                SelectorMatchResult::AlwaysThisType | SelectorMatchResult::AlwaysThisInstance => return match_,
                SelectorMatchResult::NeverThisInstance => never_this_instance = true,
                SelectorMatchResult::Sometimes => activators.add(match_.into_activator()),
                SelectorMatchResult::NeverThisType => {}
            }
        }

        if activators.count() > 0 {
            SelectorMatch::sometimes(activators.get())
        } else if never_this_instance {
            SelectorMatch::NEVER_THIS_INSTANCE
        } else {
            SelectorMatch::NEVER_THIS_TYPE
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        None
    }

    fn validate_nesting_selector(&self, in_control_theme: bool, template_count: i32) {
        for selector in &self.selectors {
            selector.0.validate_nesting_selector(in_control_theme, template_count);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
