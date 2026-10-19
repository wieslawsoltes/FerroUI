//! The models of the example (namespace `ReactiveUIExample.Models`): one module per reference file,
//! and the feature of the DynamicData page, an addition of the port.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod machine_snapshot;
mod showcase_feature;
mod showcase_json_context;
mod showcase_state;

pub use machine_snapshot::MachineSnapshot;
pub use showcase_feature::ShowcaseFeature;
pub use showcase_json_context::ShowcaseJsonContext;
pub use showcase_state::ShowcaseState;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<MachineSnapshot as MarkupTyped>::MARKUP, <ShowcaseFeature as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<ShowcaseFeature>();
}
