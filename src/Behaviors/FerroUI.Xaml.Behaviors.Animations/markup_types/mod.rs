//! Markup metadata of the types of this crate that are not classes of the object model, written
//! by hand: the static animation catalogs.

use ferroui_base::metadata::MarkupType;

mod composition;

/// The lists of the types declared in this module, by area.
const TYPE_LISTS: &[&[&MarkupType]] = &[composition::TYPES];

/// Adds the types declared in this module to the process-wide table of types with markup
/// metadata. Called once by `register_types()`.
pub(crate) fn register() {
    for types in TYPE_LISTS {
        MarkupType::register_all(types);
    }
}
