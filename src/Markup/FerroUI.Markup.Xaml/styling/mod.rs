//! Includes: styles and resources loaded from other markup documents.

mod merge_resource_include;
mod resource_include;
mod style_include;

pub use merge_resource_include::MergeResourceInclude;
pub use resource_include::ResourceInclude;
pub use style_include::StyleInclude;

#[cfg(test)]
mod includes_tests;
