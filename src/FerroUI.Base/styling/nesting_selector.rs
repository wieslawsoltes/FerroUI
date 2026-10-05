use super::{ContainerQuery, ControlTheme, Selector, SelectorMatch, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;

/// The `^` nesting style selector: stands for the selector of the parent
/// style.
pub(crate) struct NestingSelector;

fn match_theme(theme: &ControlTheme, control: &StyledElement) -> SelectorMatch {
    match theme.target_type() {
        None => panic!("ControlTheme has no TargetType."),
        Some(target_type) => {
            if target_type.is_assignable_from(control.style_key()) {
                SelectorMatch::ALWAYS_THIS_TYPE
            } else {
                SelectorMatch::NEVER_THIS_TYPE
            }
        }
    }
}

impl SelectorNode for NestingSelector {
    fn in_template(&self) -> bool {
        false
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        match owner.and_then(|o| o.parent()) {
            Some(parent) => parent.to_display_string(),
            None => "^".to_string(),
        }
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        if let Some(parent) = parent {
            if let Some(s) = parent.downcast_ref::<Style>() {
                if let Some(selector) = s.selector() {
                    let style_parent = s.parent();
                    return selector.match_(control, style_parent.as_deref(), subscribe);
                }
            } else if let Some(theme) = parent.downcast_ref::<ControlTheme>() {
                return match_theme(theme, control);
            } else if let Some(query) = parent.downcast_ref::<ContainerQuery>() {
                let query_parent = query.parent();
                if let Some(query_theme) = query_parent.as_deref().and_then(|p| p.downcast_ref::<ControlTheme>()) {
                    return match_theme(query_theme, control);
                }
            }
        }

        panic!("Nesting selector was specified but cannot determine parent selector.");
    }

    fn move_previous(&self) -> Option<&Selector> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
