//! The flex panel, its attached properties and the values they take.

mod flex;
mod flex_align_content;
mod flex_align_items;
mod flex_basis;
mod flex_basis_kind;
mod flex_direction;
mod flex_justify_content;
#[allow(clippy::module_inception)]
mod flex_panel;
mod flex_wrap;

pub use flex::Flex;
pub use flex_align_content::FlexAlignContent;
pub use flex_align_items::FlexAlignItems;
pub use flex_basis::FlexBasis;
pub use flex_basis_kind::FlexBasisKind;
pub use flex_direction::FlexDirection;
pub use flex_justify_content::FlexJustifyContent;
pub use flex_panel::FlexPanel;
pub use flex_wrap::FlexWrap;

#[cfg(test)]
mod flex_basis_tests;
#[cfg(test)]
mod flex_panel_tests;
