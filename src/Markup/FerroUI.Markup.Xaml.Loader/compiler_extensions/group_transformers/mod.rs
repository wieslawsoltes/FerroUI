//! Port of `CompilerExtensions/GroupTransformers`: transformers that see every document of a
//! compilation. Append one `mod`/`pub use` pair per ported file.

mod i_xaml_ast_group_transformer;

pub use i_xaml_ast_group_transformer::*;

mod xaml_include_group_transformer;
mod xaml_merge_resource_group_transformer;

pub use xaml_include_group_transformer::*;
pub use xaml_merge_resource_group_transformer::*;
