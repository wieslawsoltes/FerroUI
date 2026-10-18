//! Markup metadata of the types of this crate, declared apart from the types themselves: the
//! declarative columns, the selection mode and the selection change (`columns.rs`) and the
//! converters (`converters.rs`) and the routed events of the grid (`events.rs`). The classes state their properties through their registered
//! properties, as every class of the framework.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupType;

mod columns;
mod converters;
mod events;

/// The lists of the types declared in this module.
const TYPE_LISTS: &[&[&MarkupType]] = &[columns::TYPES, converters::TYPES, events::TYPES];

/// Adds the types declared in this module to the process-wide table of types with markup
/// metadata. Called once by `register_types()`.
pub(crate) fn register() {
    for types in TYPE_LISTS {
        MarkupType::register_all(types);
    }
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    columns::register_value_types();
    converters::register_value_types();
}
