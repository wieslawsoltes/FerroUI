//! Pinned-framework mapping reproducer: native Tab and TryMoveFocus have different
//! disabled-candidate rules. The adapter uses native Tab with explicit root containment.
#[path = "../tests/support/headless_session.rs"]
mod headless;
use ferroboard_controls::input::TextBoxInputTarget;
use ferroboard_core::editor::EnterAction;
use ferroui_base::{
    input::{FindNextElementOptions, KeyboardNavigationHandler, NavigationDirection},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBox, Window};
fn main() {
    headless::run(|| {
        let first = TextBox::new();
        let disabled = TextBox::new();
        disabled.set_is_enabled(false);
        let third = TextBox::new();
        let panel = StackPanel::new();
        for field in [&first, &disabled, &third] {
            panel.children().add(field.clone());
        }
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(first.focus());
        let moved = window.focus_manager().try_move_focus(
            NavigationDirection::Next,
            Some(&FindNextElementOptions {
                search_root: Some(panel.upcast()),
                focused_element: Some(first.clone().upcast()),
                ..Default::default()
            }),
        );
        assert!(
            !moved,
            "Re-audit the selected mapping if this framework prototype is corrected"
        );
        assert!(first.is_focused());
        let candidate =
            KeyboardNavigationHandler::get_next(&first.clone().upcast(), NavigationDirection::Next)
                .unwrap();
        assert_eq!(
            candidate,
            third.clone().upcast::<ferroui_base::input::InputElement>()
        );
        let target = TextBoxInputTarget::new(first, None).unwrap();
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(third.is_focused());
        target.detach();
        window.close();
        println!(
            "Pinned TryMoveFocus stops at disabled candidate; native Tab and the bounded adapter skip it."
        );
    });
}
