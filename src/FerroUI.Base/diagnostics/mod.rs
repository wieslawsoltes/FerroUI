//! Diagnostics of the object model: what the developer tools and tests read
//! about the state of a property on an object.

mod diagnostic_consts;
mod ferro_object_extensions;
mod ferro_property_value;
mod i_value_frame_diagnostic;
mod local_value_frame_diagnostic;
mod style_value_frame_diagnostic;
mod value_frame_diagnostic;
mod value_store_diagnostic;

pub use diagnostic_consts::{DiagnosticMeters, DiagnosticTags};
pub use ferro_object_extensions::FerroObjectDiagnosticExtensions;
pub use ferro_property_value::FerroPropertyValue;
pub use i_value_frame_diagnostic::{IValueFrameDiagnostic, IValueFrameDiagnosticFrameType, ValueEntryDiagnostic};
pub use value_store_diagnostic::ValueStoreDiagnostic;

pub(crate) use local_value_frame_diagnostic::LocalValueFrameDiagnostic;
pub(crate) use style_value_frame_diagnostic::StyleValueFrameDiagnostic;
pub(crate) use value_frame_diagnostic::ValueFrameDiagnostic;

#[cfg(test)]
mod diagnostics_tests;
