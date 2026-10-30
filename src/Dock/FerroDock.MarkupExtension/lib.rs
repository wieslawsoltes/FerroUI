//! Original URI loading and named-object extensions using native markup services.
mod load_extension;
mod reference_extension;
mod register_types;
pub use load_extension::LoadExtension;
pub use reference_extension::ReferenceExtension;
pub use register_types::{register_types, ASSEMBLY};
