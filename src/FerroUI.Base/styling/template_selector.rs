use super::{Selector, SelectorMatch, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// The template combinator: matches a control in the template of a control
/// that matches the preceding selector.
pub(crate) struct TemplateSelector {
    parent: Selector,
    selector_string: OnceCell<String>,
}

impl TemplateSelector {
    pub fn new(parent: Option<Selector>) -> Self {
        match parent {
            Some(parent) => Self { parent, selector_string: OnceCell::new() },
            None => panic!("Template selector must be preceeded by a selector."),
        }
    }
}

impl SelectorNode for TemplateSelector {
    fn in_template(&self) -> bool {
        true
    }

    fn is_combinator(&self) -> bool {
        true
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string
            .get_or_init(|| format!("{} /template/ ", self.parent.to_string_with_next(owner, true)))
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        let templated_parent = control.templated_parent().and_then(|p| p.cast::<StyledElement>());

        match templated_parent {
            Some(templated_parent) => self.parent.match_(&templated_parent, parent, subscribe),
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
