use super::XYFocusNavigationStrategy;
use crate::input::{InputElement, KeyDeviceType};
use crate::{Rect, Ref};

/// The options of a directional focus search.
pub(crate) struct XYFocusOptions {
    pub search_root: Option<Ref<InputElement>>,
    pub exclusion_rect: Rect,
    pub focus_hint_rectangle: Option<Rect>,
    pub focused_element_bounds: Option<Rect>,
    pub navigation_strategy_override: Option<XYFocusNavigationStrategy>,
    pub ignore_clipping: bool,
    pub ignore_cone: bool,
    pub key_device_type: Option<KeyDeviceType>,
    #[allow(dead_code)]
    pub consider_engagement: bool,
    pub update_manifold: bool,
    pub update_manifolds_from_focus_hint_rect: bool,
    pub ignore_occlusivity: bool,
}

impl Default for XYFocusOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl XYFocusOptions {
    pub fn new() -> Self {
        Self {
            search_root: None,
            exclusion_rect: Rect::default(),
            focus_hint_rectangle: None,
            focused_element_bounds: None,
            navigation_strategy_override: None,
            ignore_clipping: true,
            ignore_cone: false,
            key_device_type: None,
            consider_engagement: true,
            update_manifold: true,
            update_manifolds_from_focus_hint_rect: false,
            ignore_occlusivity: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
