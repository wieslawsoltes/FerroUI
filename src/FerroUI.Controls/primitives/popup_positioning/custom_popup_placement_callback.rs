use super::CustomPopupPlacement;
use std::rc::Rc;

/// Represents a method that provides custom positioning for a popup
/// control.
pub type CustomPopupPlacementCallback = Rc<dyn Fn(&mut CustomPopupPlacement)>;
