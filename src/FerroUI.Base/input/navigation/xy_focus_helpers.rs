use super::{XYFocus, XYFocusNavigationModes};
use crate::input::{InputElement, KeyDeviceType};
use crate::Ref;

pub struct XYFocusHelpers;

impl XYFocusHelpers {
    pub fn is_allowed_xy_navigation_mode(visual: &InputElement, key_device_type: Option<KeyDeviceType>) -> bool {
        let modes = XYFocus::get_navigation_modes(visual);

        match key_device_type {
            // programmatic input, allow any subtree except Disabled.
            None => modes != XYFocusNavigationModes::DISABLED,
            Some(KeyDeviceType::Keyboard) => modes.contains(XYFocusNavigationModes::KEYBOARD),
            Some(KeyDeviceType::Gamepad) => modes.contains(XYFocusNavigationModes::GAMEPAD),
            Some(KeyDeviceType::Remote) => modes.contains(XYFocusNavigationModes::REMOTE),
        }
    }

    pub fn find_xy_search_root(visual: &Ref<InputElement>, key_device_type: Option<KeyDeviceType>) -> Ref<InputElement> {
        let mut candidate = visual.clone();
        let mut candidate_parent = visual.find_ancestor_of_type::<InputElement>(false);

        while let Some(parent) = candidate_parent {
            if !Self::is_allowed_xy_navigation_mode(&parent, key_device_type) {
                break;
            }
            candidate_parent = parent.find_ancestor_of_type::<InputElement>(false);
            candidate = parent;
        }

        candidate
    }
}
