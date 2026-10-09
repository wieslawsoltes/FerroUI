//! Port of `LeakTests.cs` of the upstream unit test project of the headless
//! platform.
//!
//! The static of the original is a value of the session thread, where the
//! font manager lives. A weak reference is dead as soon as the last
//! reference is dropped, so the forced collections of the original have no
//! counterpart. The fifty rows of the theory are fifty dispatches of one
//! test per test assembly, in their order.

use crate::HeadlessWindowExtensions;
use ferroui_base::input::{MouseButton, RawInputModifiers};
use ferroui_base::media::FontManager;
use ferroui_base::{BoxedValue, Point};
use ferroui_controls::{Button, Control, Window};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

fn test_data() -> impl Iterator<Item = String> {
    (0..50).map(|i| i.to_string())
}

thread_local! {
    static PREVIOUS_FONT_MANAGER: RefCell<Option<Weak<FontManager>>> = const { RefCell::new(None) };
}

fn previous_font_manager_should_be_collected(data: String) {
    // Arrange
    let font_manager = Rc::downgrade(&FontManager::current());
    let button = Button::new();
    let content: BoxedValue = Rc::new(data);
    button.set_content(Some(content));
    let window = Window::new();
    window.set_content(Some(Control::boxed(button.clone())));

    // Act, just some interaction, to make sure the FontManager is actually used
    window.show();
    button.focus();
    window.mouse_down(Point::new(1.0, 1.0), MouseButton::Left, RawInputModifiers::NONE);
    window.close();

    // Assert

    // Either previous font manager is collected (IsAlive == false), or it is the same as current (shared isolation mode).
    let previous_font_manager = PREVIOUS_FONT_MANAGER.with(|previous| previous.borrow().clone());
    if let Some(previous_font_manager) = previous_font_manager {
        if !Weak::ptr_eq(&previous_font_manager, &font_manager) {
            assert!(previous_font_manager.upgrade().is_none());
        }
    }

    PREVIOUS_FONT_MANAGER.with(|previous| *previous.borrow_mut() = Some(font_manager));
}

mod previous_font_manager_should_be_collected {
    use super::test_data;
    use crate::unit_tests::test_application::{run, PER_ASSEMBLY, PER_TEST};

    #[test]
    fn per_test() {
        for data in test_data() {
            run(&PER_TEST, move || super::previous_font_manager_should_be_collected(data));
        }
    }

    #[test]
    fn per_assembly() {
        for data in test_data() {
            run(&PER_ASSEMBLY, move || super::previous_font_manager_should_be_collected(data));
        }
    }
}
