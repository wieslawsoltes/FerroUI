//! The behavior, the trigger and the actions of the properties of the view model (the data
//! context) of a control (`ViewModel/`).
//!
//! Deviation (DEVIATIONS.md, Behaviors): the reference finds the properties of the view model with
//! reflection; the port finds the plain properties the view model declares in its markup
//! metadata (`ferro_model!`, `ferro_markup_type!`), and observes the property changes of a view
//! model whose model type declares them.

mod increment_view_model_property_action;
mod set_view_model_property_action;
mod set_view_model_property_on_load_behavior;
mod toggle_view_model_boolean_action;
mod view_model_property_changed_trigger;

pub use increment_view_model_property_action::IncrementViewModelPropertyAction;
pub use set_view_model_property_action::SetViewModelPropertyAction;
pub use set_view_model_property_on_load_behavior::SetViewModelPropertyOnLoadBehavior;
pub use toggle_view_model_boolean_action::ToggleViewModelBooleanAction;
pub use view_model_property_changed_trigger::ViewModelPropertyChangedTrigger;
