//! The platform feedback of the items controls.

use crate::platform::{FeedbackType, PlatformFeedback};
use crate::{ComboBox, ListBoxItem, TreeViewItem};
use ferroui_base::threading::Dispatcher;

#[test]
fn item_containers_and_combo_box_use_automatic_feedback_by_default() {
    let _scope = Dispatcher::unit_test_scope();

    assert_eq!(FeedbackType::Auto, PlatformFeedback::get_feedback_type(&ListBoxItem::new()));
    assert_eq!(FeedbackType::Auto, PlatformFeedback::get_feedback_type(&TreeViewItem::new()));
    assert_eq!(FeedbackType::Auto, PlatformFeedback::get_feedback_type(&ComboBox::new()));
}
