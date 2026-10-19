//! `ReactiveUI.Binding/View`: the location of the view of a view model ([`DefaultViewLocator`],
//! [`ViewLocator`], [`ViewMappingBuilder`]) and the options of the registration of a view
//! ([`ViewContract`], [`SingleInstanceView`], [`ExcludeFromViewRegistration`], gathered by
//! [`ViewRegistrations`] where upstream's source generator reads them as attributes).
//!
//! Not ported: the lookup of a view by the name of the type of its view model (reflection).

mod default_view_locator;
mod exclude_from_view_registration_attribute;
mod single_instance_view_attribute;
mod view_contract_attribute;
mod view_locator;
mod view_locator_not_found_exception;
mod view_mapping_builder;
mod view_mapping_key;
mod register_view_for;
mod view_registrations;

pub use default_view_locator::{DefaultViewLocator, GeneratedViewDispatch};
pub use exclude_from_view_registration_attribute::ExcludeFromViewRegistration;
pub use single_instance_view_attribute::SingleInstanceView;
pub use view_contract_attribute::ViewContract;
pub use view_locator::ViewLocator;
pub use view_locator_not_found_exception::ViewLocatorNotFoundException;
pub use view_mapping_builder::ViewMappingBuilder;
pub use register_view_for::RegisterViewForExt;
pub use view_registrations::{ViewRegistrationOptions, ViewRegistrations};
pub(crate) use default_view_locator::record_view_model_type;
