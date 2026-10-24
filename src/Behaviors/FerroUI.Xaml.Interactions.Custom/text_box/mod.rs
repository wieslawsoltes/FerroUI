//! The behaviors of text boxes (`TextBox/`).

mod auto_select_behavior;
mod text_box_select_all_on_got_focus_behavior;
mod text_box_select_all_text_behavior;

pub use auto_select_behavior::AutoSelectBehavior;
pub use text_box_select_all_on_got_focus_behavior::TextBoxSelectAllOnGotFocusBehavior;
pub use text_box_select_all_text_behavior::TextBoxSelectAllTextBehavior;
