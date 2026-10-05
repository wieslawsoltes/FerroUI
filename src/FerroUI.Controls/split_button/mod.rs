//! Buttons with a primary part and a secondary part that opens a flyout.

#[allow(clippy::module_inception)]
mod split_button;
mod toggle_split_button;

pub use split_button::{SplitButton, SplitButtonImpl, SplitButtonImplExt, SplitButtonVTable};
pub use toggle_split_button::{
    ToggleSplitButton, ToggleSplitButtonImpl, ToggleSplitButtonImplExt, ToggleSplitButtonVTable,
};

#[cfg(test)]
mod split_button_tests;
