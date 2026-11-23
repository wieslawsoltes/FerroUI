//! ferroui-controls-password-box
//!
//! `PasswordBox` of WinUI on the `TextBox` of FerroUI, whose masking, input and theme it uses:
//! `Password`, `PasswordChar`, `PasswordRevealMode` and `PasswordChanged`. An application that
//! does not use the control links none of this crate.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_password_box;

mod markup;
mod password_box;
mod register_types;
mod rust_paths;

pub use password_box::{PasswordBox, PasswordRevealMode};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
