//! Popup positioning: the positioner contract, its parameters and the
//! managed positioner for platforms on which popups can be arbitrarily
//! positioned.

mod custom_popup_placement;
mod custom_popup_placement_callback;
mod i_popup_positioner;
mod managed_popup_positioner;
mod managed_popup_positioner_popup_impl_helper;
mod popup_position_request;

pub use custom_popup_placement::CustomPopupPlacement;
pub use custom_popup_placement_callback::CustomPopupPlacementCallback;
pub use i_popup_positioner::{
    IPopupPositioner, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment, PopupPositionerParameters,
};
pub use managed_popup_positioner::{
    IManagedPopupPositionerPopup, ManagedPopupPositioner, ManagedPopupPositionerScreenInfo,
};
pub use managed_popup_positioner_popup_impl_helper::{ManagedPopupPositionerPopupImplHelper, MoveResizeDelegate};
pub use popup_position_request::PopupPositionRequest;

#[cfg(test)]
mod managed_popup_positioner_tests;
