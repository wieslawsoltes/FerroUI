//! Diagnostics of the object model: what the developer tools and tests read
//! about the state of a property on an object.

mod ferro_object_extensions;
mod ferro_property_value;

pub use ferro_object_extensions::FerroObjectDiagnosticExtensions;
pub use ferro_property_value::FerroPropertyValue;

#[cfg(test)]
mod diagnostics_tests;
