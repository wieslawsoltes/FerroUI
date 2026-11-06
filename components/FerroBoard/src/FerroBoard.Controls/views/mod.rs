// Internal source views composed by the native OnScreenKeyboard owner.
#![allow(dead_code, unused_imports)]
mod key_view;
pub(crate) use key_view::*;
mod key_automation_peer;
mod keys_view;
pub(crate) use keys_view::*;
mod smartbar_view;
pub(crate) mod view_helpers;
pub(crate) use smartbar_view::{EmojiSearchState, SmartbarView, TextKeyDataFactory};
pub use smartbar_view::{SmartbarAction, SmartbarExecute, SmartbarIsActive};
mod padded_grid;
mod panel_views;
pub(crate) use padded_grid::PaddedGrid;
pub(crate) use panel_views::*;
mod emoji_panel_view;
pub(crate) use emoji_panel_view::*;

#[cfg(test)]
mod native_lifetime_tests;
