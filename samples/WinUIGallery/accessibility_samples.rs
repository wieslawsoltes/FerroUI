//! The code behind of the pinned AccessibilityKeyboard page in Rust: the handlers of
//! `AccessibilityKeyboardPage.xaml.cs`, which color the rectangle of the accelerators example.
//! The tab order, arrow keys and access keys of the page are the markup's.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::Ref;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, Control};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "AccessibilityKeyboard" {
        wire_keyboard(root);
    }
}

/// The buttons of `MakeRedButton_Click`, `MakeBlueButton_Click` and `MakeChartreuseButton_Click`
/// with the color each sets.
pub(crate) const ACCELERATOR_COLORS: [(&str, Color); 3] = [
    ("MakeRedButton", Colors::RED),
    ("MakeBlueButton", Colors::BLUE),
    ("MakeChartreuseButton", Colors::CHARTREUSE),
];

fn wire_keyboard(root: &Ref<Control>) {
    let rectangle = root.get_control::<Rectangle>("ColorRectangle");
    for (name, color) in ACCELERATOR_COLORS {
        let weak_rectangle = rectangle.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let Some(rectangle) = weak_rectangle.upgrade() {
                rectangle.set_fill(Some(Rc::new(ImmutableSolidColorBrush::new(color)) as Rc<dyn IBrush>));
            }
        });
    }
}
