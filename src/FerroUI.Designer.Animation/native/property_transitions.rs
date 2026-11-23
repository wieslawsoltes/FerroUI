//! The registered native classes are the authority for transition value types.
use crate::property_transitions::PROPERTY_TRANSITION_TYPES;
use ferroui_base::animation::TransitionBase;
use ferroui_base::{FerroProperty, TypeInfo};
use std::any::TypeId;
use std::sync::OnceLock;

/// The native classes that can animate this property, in default preference order.
pub fn native_property_transition_types(
    property: &FerroProperty,
) -> impl Iterator<Item = &'static str> {
    static TYPES: OnceLock<Vec<(&'static str, TypeId)>> = OnceLock::new();
    let is_writable_styled = !property.is_direct() && !property.is_read_only();
    let value_type = property.property_type();
    TYPES
        .get_or_init(|| {
            PROPERTY_TRANSITION_TYPES
                .iter()
                .filter_map(|&name| {
                    // Only instantiate FerroUI's own known classes for metadata.
                    // A query never invokes a project's transition constructor.
                    let type_ = TypeInfo::find("FerroUI.Animation", name)?;
                    let transition = type_.create_instance()?.cast::<TransitionBase>()?;
                    Some((name, transition.value_type().0))
                })
                .collect()
        })
        .iter()
        .filter(move |(_, type_)| is_writable_styled && *type_ == value_type)
        .map(|(name, _)| *name)
}
