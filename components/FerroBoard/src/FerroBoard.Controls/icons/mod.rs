mod icon_data;
mod icon_factory;
pub use icon_factory::*;

mod metadata;
pub(crate) use metadata::register as register_markup_types;
