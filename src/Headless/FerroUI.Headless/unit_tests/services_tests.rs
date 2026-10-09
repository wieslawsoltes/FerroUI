//! Port of `ServicesTests.cs` of the upstream unit test project of the
//! headless platform.

use super::test_application::ferro_fact;
use ferroui_controls::Window;
use std::rc::Rc;

fn can_access_screens() {
    let window = Window::new();
    // `AssertHelper.NotNull(screens)`: the screens of a window are not optional.
    let screens = window.screens();

    let current_screen_from_window = screens.screen_from_window(&window);
    let current_screen_from_visual = screens.screen_from_visual(&window);

    match (current_screen_from_window, current_screen_from_visual) {
        (Some(from_window), Some(from_visual)) => assert!(Rc::ptr_eq(&from_window, &from_visual)),
        (None, None) => {}
        _ => panic!("the window and the visual are on different screens"),
    }
}
ferro_fact!(can_access_screens);
