//! Port of `Actions/FocusNextElementAction.cs`.

use super::control_of;
use crate::part_a::FocusNavigationHelper;
use ferroui_base::input::{KeyModifiers, NavigationDirection, NavigationMethod};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, Visual};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};

/// Moves the focus to the next element of the tab order of the top level of the sender, with
/// the tab navigation rules of the framework (the keyboard navigation modes of the containers),
/// without raising key events.
#[repr(C)]
pub struct FocusNextElementAction {
    base: StyledElementAction,
}

ferro_class!(FocusNextElementAction: StyledElementAction);
ferro_class_info!(FocusNextElementAction { new: FocusNextElementAction::new });
ferroui_base::ferro_impl_classes!(FocusNextElementAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for FocusNextElementAction {
    fn execute(_this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let source = control_of(sender)?;

        let source_visual: &Visual = &source;
        let top_level = TopLevel::get_top_level(Some(source_visual));
        let current = top_level
            .as_ref()
            .and_then(|top_level| top_level.focus_manager().get_focused_element())
            .unwrap_or_else(|| source.clone().upcast());
        if let Some(top_level) = top_level {
            if let Some(next) =
                FocusNavigationHelper::find_adjacent(&top_level, Some(&*current), NavigationDirection::Next, false)
            {
                next.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
            }
        }

        None
    }
}

impl FocusNextElementAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
