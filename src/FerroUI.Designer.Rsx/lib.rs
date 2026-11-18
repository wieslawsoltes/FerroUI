//! Portable XAML/RSX translation for the visual designer. Parsed Rust is data,
//! and the mapping is tied to the source snapshot used to construct it.

mod blocks;
mod catalog;
mod literal;
mod project;
mod tree;
mod xaml;

pub use project::{project_rust, RsxNode, RsxProjection};
pub use xaml::{apply_rsx, xaml_to_rsx};

#[cfg(test)]
mod tests;
