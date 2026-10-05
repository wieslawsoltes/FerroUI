//! Port of the `TypeSystem/` directory: the type system abstraction used by the compiler.

mod type_system;
mod type_system_helpers;
mod xaml_locals_pool;
mod xaml_type_well_known_types;

pub use type_system::*;
pub use type_system_helpers::*;
pub use xaml_locals_pool::*;
pub use xaml_type_well_known_types::*;
