#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::services::SystemInputMethodGuard;
use ferroui_base::{data::BindingPriority, input::InputMethod};
use ferroui_controls::TextBox;
#[test]
fn guards_stack_without_overwriting_local_values_or_later_changes() {
    headless::run(|| {
        let field = TextBox::new();
        let property = InputMethod::is_input_method_enabled_property();
        assert!(InputMethod::get_is_input_method_enabled(&field));
        let first = SystemInputMethodGuard::new(&field);
        let second = SystemInputMethodGuard::new(&field);
        assert!(!InputMethod::get_is_input_method_enabled(&field));
        InputMethod::set_is_input_method_enabled(&field, false);
        drop(first);
        assert!(!InputMethod::get_is_input_method_enabled(&field));
        InputMethod::set_is_input_method_enabled(&field, true);
        assert!(!InputMethod::get_is_input_method_enabled(&field));
        drop(second);
        assert!(InputMethod::get_is_input_method_enabled(&field));
        field.clear_value(property);
        assert!(!field.is_set(property));
        let guard = SystemInputMethodGuard::new(&field);
        drop(guard);
        assert!(
            !field.is_set(property),
            "default must not become a local override"
        );
    });
}
#[test]
fn releasing_a_guard_reveals_an_existing_style_value_and_other_field_is_untouched() {
    headless::run(|| {
        let field = TextBox::new();
        let other = TextBox::new();
        let property = InputMethod::is_input_method_enabled_property();
        let style = field
            .set_value_with_priority(property, false, BindingPriority::Style)
            .unwrap();
        let guard = SystemInputMethodGuard::new(&field);
        assert!(InputMethod::get_is_input_method_enabled(&other));
        drop(guard);
        assert!(!InputMethod::get_is_input_method_enabled(&field));
        style.dispose();
        assert!(InputMethod::get_is_input_method_enabled(&field));
    });
}

#[test]
fn an_existing_binding_keeps_receiving_updates_under_the_guard() {
    headless::run(|| {
        use ferroui_base::reactive::{IObserver, LightweightSubject};
        use std::rc::Rc;
        let field = TextBox::new();
        let source = Rc::new(LightweightSubject::<bool>::new());
        let binding = field.bind(
            InputMethod::is_input_method_enabled_property(),
            source.clone(),
            BindingPriority::LocalValue,
        );
        source.on_next(true);
        let guard = SystemInputMethodGuard::new(&field);
        source.on_next(false);
        source.on_next(true);
        assert!(!InputMethod::get_is_input_method_enabled(&field));
        drop(guard);
        assert!(InputMethod::get_is_input_method_enabled(&field));
        source.on_next(false);
        assert!(
            !InputMethod::get_is_input_method_enabled(&field),
            "the original subscription remains live"
        );
        binding.dispose();
        assert!(InputMethod::get_is_input_method_enabled(&field));
    });
}
