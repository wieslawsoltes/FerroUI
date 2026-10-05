use super::InputElement;
use crate::Ref;

pub(crate) struct FocusHelpers;

impl FocusHelpers {
    pub fn get_input_element_children(parent: &InputElement) -> Vec<Ref<InputElement>> {
        match parent.visual_children_snapshot() {
            Some(children) => children.iter().filter_map(|child| child.cast::<InputElement>()).collect(),
            None => Vec::new(),
        }
    }

    pub fn can_have_focusable_children(parent: Option<&InputElement>) -> bool {
        let Some(parent) = parent else { return false };
        let Some(children) = parent.visual_children_snapshot() else { return false };

        for child in children.iter() {
            let Some(child) = child.downcast_ref::<InputElement>() else { continue };

            if Self::is_visible(child) && (child.focusable() || Self::can_have_focusable_children(Some(child))) {
                return true;
            }
        }

        false
    }

    pub fn get_focus_parent(input_element: Option<&InputElement>) -> Option<Ref<InputElement>> {
        let input_element = input_element?;
        let this: &crate::Visual = input_element;
        let is_root = input_element.visual_root().is_some_and(|root| std::ptr::eq::<crate::Visual>(&*root, this));

        if !is_root {
            return input_element.parent().and_then(|parent| parent.downcast::<InputElement>().ok());
        }

        None
    }

    pub fn is_potential_tab_stop(element: Option<&InputElement>) -> bool {
        element.is_some_and(InputElement::is_tab_stop)
    }

    pub fn is_visible(element: &InputElement) -> bool {
        element.is_effectively_visible()
    }

    pub fn is_focusable(element: Option<&InputElement>) -> bool {
        element.is_some_and(InputElement::focusable)
    }

    #[allow(dead_code)]
    pub fn can_have_children(element: Option<&InputElement>) -> bool {
        // We don't currently have a flag to indicate a visual can have
        // children, so we just return whether the element is a visual.
        element.is_some()
    }
}
