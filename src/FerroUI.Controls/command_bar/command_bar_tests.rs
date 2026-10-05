//! Port of the reference `CommandBarTests`: the test classes of the buttons,
//! the separator, the enumerations, the defaults, the property round trips,
//! the open state and the collections, one module for each class. The
//! classes of the label positions, the overflow button, the item widths and
//! the separators in overflow are in `command_bar_tests_overflow.rs`, the
//! class of the keyboard in `command_bar_tests_keyboard.rs`.

use super::ICommandBarElement;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::ICommand;
use ferroui_base::media::IBrush;
use ferroui_base::{AnyValue, BoxedValue, ObjectType};
use std::rc::Rc;

/// Whether the element is of exactly class `T`.
pub(super) fn is_type<T: ObjectType>(element: &Rc<dyn ICommandBarElement>) -> bool {
    element.as_object().is_some_and(|object| std::ptr::eq(object.get_type(), T::TYPE))
}

/// Whether two untyped values are the same object.
pub(super) fn same_value(a: &Option<BoxedValue>, b: &BoxedValue) -> bool {
    a.as_ref().is_some_and(|a| Rc::ptr_eq(a, b))
}

pub(super) fn same_brush(a: &Option<Rc<dyn IBrush>>, b: &Rc<dyn IBrush>) -> bool {
    a.as_ref().is_some_and(|a| std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)))
}

fn same_command(a: &Option<Rc<dyn ICommand>>, b: &Option<Rc<dyn ICommand>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        _ => false,
    }
}

/// Whether the untyped conversions view the object in `value` as the
/// command bar element `element` (the class states the interface).
fn casts_to_element(value: &BoxedValue, element: &Rc<dyn ICommandBarElement>) -> bool {
    let cast = ValueTypes::try_cast(value, ValueType::of::<Rc<dyn ICommandBarElement>>());
    cast.is_some_and(|cast| {
        let cast: &dyn AnyValue = &*cast;
        cast.downcast_ref::<Rc<dyn ICommandBarElement>>().is_some_and(|cast| cast == element)
    })
}

fn int_of(value: &BoxedValue) -> Option<i32> {
    let value: &dyn AnyValue = &**value;
    value.downcast_ref::<i32>().copied()
}

mod command_bar_button_tests {
    use super::super::{CommandBarButton, CommandBarDefaultLabelPosition, ICommandBarElement};
    use super::{casts_to_element, is_type, same_brush, same_command, same_value};
    use crate::primitives::TemplatedControl;
    use crate::test_command::TestCommand;
    use crate::test_support::{boxed_str, string_of, test_scope};
    use crate::{Control, PathIcon};
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    #[test]
    fn label_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarButton::new().label().is_none());
    }

    #[test]
    fn label_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_label(Some("Save"));
        assert_eq!(Some("Save".to_string()), btn.label());
    }

    #[test]
    fn icon_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarButton::new().icon().is_none());
    }

    #[test]
    fn icon_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        let icon: BoxedValue = Rc::new("icon".to_string());
        btn.set_icon(Some(icon.clone()));
        assert!(same_value(&btn.icon(), &icon));
    }

    #[test]
    fn is_compact_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarButton::new().is_compact());
    }

    #[test]
    fn is_compact_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_is_compact(true);
        assert!(btn.is_compact());
    }

    #[test]
    fn dynamic_overflow_order_default_is_zero() {
        let _scope = test_scope();
        assert_eq!(0, CommandBarButton::new().dynamic_overflow_order());
    }

    #[test]
    fn dynamic_overflow_order_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_dynamic_overflow_order(3);
        assert_eq!(3, btn.dynamic_overflow_order());
    }

    #[test]
    fn label_position_default_is_bottom() {
        let _scope = test_scope();
        assert_eq!(CommandBarDefaultLabelPosition::Bottom, CommandBarButton::new().label_position());
    }

    #[test]
    fn label_position_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_label_position(CommandBarDefaultLabelPosition::Right);
        assert_eq!(CommandBarDefaultLabelPosition::Right, btn.label_position());
    }

    #[test]
    fn is_in_overflow_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarButton::new().is_in_overflow());
    }

    #[test]
    fn is_in_overflow_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_is_in_overflow(true);
        assert!(btn.is_in_overflow());
    }

    #[test]
    fn implements_i_command_bar_element() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        let elem: Rc<dyn ICommandBarElement> = btn.as_command_bar_element();
        assert!(is_type::<CommandBarButton>(&elem));
        // The untyped conversion the class states.
        let boxed: BoxedValue = Rc::new(btn.clone());
        assert!(casts_to_element(&boxed, &elem));
    }

    #[test]
    fn i_command_bar_element_is_compact_read_write() {
        let _scope = test_scope();
        let elem: Rc<dyn ICommandBarElement> = CommandBarButton::new().as_command_bar_element();
        elem.set_is_compact(true);
        assert!(elem.is_compact());
    }

    #[test]
    fn command_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarButton::new().command().is_none());
    }

    #[test]
    fn command_parameter_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarButton::new().command_parameter().is_none());
    }

    #[test]
    fn command_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        let cmd = TestCommand::with_can_execute_and_execute(|_| true, |_| {});
        btn.set_command(cmd.as_command());
        assert!(same_command(&btn.command(), &cmd.as_command()));
    }

    #[test]
    fn command_parameter_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarButton::new();
        btn.set_command_parameter(boxed_str("param"));
        assert_eq!(Some("param".to_string()), btn.command_parameter().as_ref().and_then(string_of));
    }

    #[test]
    fn foreground_does_not_set_or_overwrite_icon_element_foreground() {
        let _scope = test_scope();
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();
        let green: Rc<dyn IBrush> = Brushes::green();

        let icon = PathIcon::new();
        let btn = CommandBarButton::new();
        btn.set_icon(Some(Control::boxed(icon.clone())));
        btn.set_foreground(Some(red.clone()));

        assert!(!icon.is_set(TemplatedControl::foreground_property().as_property()));

        btn.set_foreground(Some(blue));

        assert!(!icon.is_set(TemplatedControl::foreground_property().as_property()));

        icon.set_foreground(Some(green.clone()));
        btn.set_foreground(Some(red));

        assert!(same_brush(&icon.foreground(), &green));
    }
}

mod command_bar_toggle_button_tests {
    use super::super::{CommandBarDefaultLabelPosition, CommandBarToggleButton, ICommandBarElement};
    use super::{casts_to_element, int_of, is_type, same_brush, same_command};
    use crate::primitives::TemplatedControl;
    use crate::test_command::TestCommand;
    use crate::test_support::test_scope;
    use crate::{Control, PathIcon};
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    #[test]
    fn label_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarToggleButton::new().label().is_none());
    }

    #[test]
    fn label_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        btn.set_label(Some("Bold"));
        assert_eq!(Some("Bold".to_string()), btn.label());
    }

    #[test]
    fn icon_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarToggleButton::new().icon().is_none());
    }

    #[test]
    fn is_compact_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarToggleButton::new().is_compact());
    }

    #[test]
    fn is_compact_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        btn.set_is_compact(true);
        assert!(btn.is_compact());
    }

    #[test]
    fn dynamic_overflow_order_default_is_zero() {
        let _scope = test_scope();
        assert_eq!(0, CommandBarToggleButton::new().dynamic_overflow_order());
    }

    #[test]
    fn dynamic_overflow_order_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        btn.set_dynamic_overflow_order(5);
        assert_eq!(5, btn.dynamic_overflow_order());
    }

    #[test]
    fn label_position_default_is_bottom() {
        let _scope = test_scope();
        assert_eq!(CommandBarDefaultLabelPosition::Bottom, CommandBarToggleButton::new().label_position());
    }

    #[test]
    fn label_position_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        btn.set_label_position(CommandBarDefaultLabelPosition::Collapsed);
        assert_eq!(CommandBarDefaultLabelPosition::Collapsed, btn.label_position());
    }

    #[test]
    fn is_in_overflow_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarToggleButton::new().is_in_overflow());
    }

    #[test]
    fn implements_i_command_bar_element() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        let elem: Rc<dyn ICommandBarElement> = btn.as_command_bar_element();
        assert!(is_type::<CommandBarToggleButton>(&elem));
        // The untyped conversion the class states.
        let boxed: BoxedValue = Rc::new(btn.clone());
        assert!(casts_to_element(&boxed, &elem));
    }

    #[test]
    fn i_command_bar_element_is_compact_read_write() {
        let _scope = test_scope();
        let elem: Rc<dyn ICommandBarElement> = CommandBarToggleButton::new().as_command_bar_element();
        elem.set_is_compact(true);
        assert!(elem.is_compact());
    }

    #[test]
    fn command_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBarToggleButton::new().command().is_none());
    }

    #[test]
    fn command_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        let cmd = TestCommand::with_can_execute_and_execute(|_| true, |_| {});
        btn.set_command(cmd.as_command());
        assert!(same_command(&btn.command(), &cmd.as_command()));
    }

    #[test]
    fn command_parameter_round_trip() {
        let _scope = test_scope();
        let btn = CommandBarToggleButton::new();
        let parameter: BoxedValue = Rc::new(42_i32);
        btn.set_command_parameter(Some(parameter));
        assert_eq!(Some(42), btn.command_parameter().as_ref().and_then(int_of));
    }

    #[test]
    fn foreground_does_not_set_or_overwrite_icon_element_foreground() {
        let _scope = test_scope();
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();
        let green: Rc<dyn IBrush> = Brushes::green();

        let icon = PathIcon::new();
        let btn = CommandBarToggleButton::new();
        btn.set_icon(Some(Control::boxed(icon.clone())));
        btn.set_foreground(Some(red.clone()));

        assert!(!icon.is_set(TemplatedControl::foreground_property().as_property()));

        btn.set_foreground(Some(blue));

        assert!(!icon.is_set(TemplatedControl::foreground_property().as_property()));

        icon.set_foreground(Some(green.clone()));
        btn.set_foreground(Some(red));

        assert!(same_brush(&icon.foreground(), &green));
    }
}

mod command_bar_separator_tests {
    use super::super::{CommandBarSeparator, ICommandBarElement};
    use super::{casts_to_element, is_type};
    use crate::test_support::test_scope;
    use crate::Separator;
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    #[test]
    fn is_compact_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarSeparator::new().is_compact());
    }

    #[test]
    fn is_compact_round_trip() {
        let _scope = test_scope();
        let sep = CommandBarSeparator::new();
        sep.set_is_compact(true);
        assert!(sep.is_compact());
    }

    #[test]
    fn is_in_overflow_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBarSeparator::new().is_in_overflow());
    }

    #[test]
    fn is_in_overflow_round_trip() {
        let _scope = test_scope();
        let sep = CommandBarSeparator::new();
        sep.set_is_in_overflow(true);
        assert!(sep.is_in_overflow());
    }

    #[test]
    fn implements_i_command_bar_element() {
        let _scope = test_scope();
        let sep = CommandBarSeparator::new();
        let elem: Rc<dyn ICommandBarElement> = sep.as_command_bar_element();
        assert!(is_type::<CommandBarSeparator>(&elem));
        // The untyped conversion the class states.
        let boxed: BoxedValue = Rc::new(sep.clone());
        assert!(casts_to_element(&boxed, &elem));
    }

    #[test]
    fn derives_from_separator() {
        let _scope = test_scope();
        assert!(CommandBarSeparator::new().is::<Separator>());
    }

    #[test]
    fn i_command_bar_element_is_compact_read_write() {
        let _scope = test_scope();
        let elem: Rc<dyn ICommandBarElement> = CommandBarSeparator::new().as_command_bar_element();
        elem.set_is_compact(true);
        assert!(elem.is_compact());
    }
}

mod command_bar_enum_tests {
    use super::super::{CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility};

    #[test]
    fn label_position_bottom_is_zero() {
        assert_eq!(0, CommandBarDefaultLabelPosition::Bottom as i32);
    }

    #[test]
    fn label_position_right_is_one() {
        assert_eq!(1, CommandBarDefaultLabelPosition::Right as i32);
    }

    #[test]
    fn label_position_collapsed_is_two() {
        assert_eq!(2, CommandBarDefaultLabelPosition::Collapsed as i32);
    }

    #[test]
    fn overflow_button_visibility_auto_is_zero() {
        assert_eq!(0, CommandBarOverflowButtonVisibility::Auto as i32);
    }

    #[test]
    fn overflow_button_visibility_visible_is_one() {
        assert_eq!(1, CommandBarOverflowButtonVisibility::Visible as i32);
    }

    #[test]
    fn overflow_button_visibility_collapsed_is_two() {
        assert_eq!(2, CommandBarOverflowButtonVisibility::Collapsed as i32);
    }
}

mod command_bar_defaults_tests {
    use super::super::{
        CommandBar, CommandBarButton, CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility,
        CommandBarToggleButton,
    };
    use super::same_brush;
    use crate::presenters::ContentPresenter;
    use crate::primitives::TemplatedControl;
    use crate::test_support::{test_scope, TestRoot};
    use crate::testing::{
        command_bar_button_theme, command_bar_theme, command_bar_toggle_button_theme, create_test_theme, TestServices,
        UnitTestApplication,
    };
    use crate::{Control, PathIcon};
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::styling::ControlTheme;
    use ferroui_base::{Ref, StaticType};
    use std::rc::Rc;

    #[test]
    fn default_label_position_is_bottom() {
        let _scope = test_scope();
        assert_eq!(CommandBarDefaultLabelPosition::Bottom, CommandBar::new().default_label_position());
    }

    #[test]
    fn overflow_button_visibility_default_is_auto() {
        let _scope = test_scope();
        assert_eq!(CommandBarOverflowButtonVisibility::Auto, CommandBar::new().overflow_button_visibility());
    }

    #[test]
    fn is_open_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBar::new().is_open());
    }

    #[test]
    fn is_sticky_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBar::new().is_sticky());
    }

    #[test]
    fn is_dynamic_overflow_enabled_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBar::new().is_dynamic_overflow_enabled());
    }

    #[test]
    fn has_secondary_commands_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBar::new().has_secondary_commands());
    }

    #[test]
    fn is_overflow_button_visible_default_is_false() {
        let _scope = test_scope();
        assert!(!CommandBar::new().is_overflow_button_visible());
    }

    #[test]
    fn content_default_is_null() {
        let _scope = test_scope();
        assert!(CommandBar::new().content().is_none());
    }

    #[test]
    fn foreground_is_inherited_by_command_bar_button_path_icon_through_theme_template() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let _text = TextTestScope::new();
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        let icon = PathIcon::new();
        let btn = CommandBarButton::new();
        btn.set_icon(Some(Control::boxed(icon.clone())));
        let command_bar = CommandBar::new();
        command_bar.set_foreground(Some(red.clone()));
        command_bar.primary_commands().add(btn.as_command_bar_element());

        let presented =
            apply_simple_theme_and_get_presented_path_icon(&command_bar, &btn.clone().upcast(), command_bar_button_theme());
        assert!(presented.ptr_eq(&icon));
        assert!(same_brush(&icon.foreground(), &red));

        command_bar.set_foreground(Some(blue.clone()));

        assert!(same_brush(&icon.foreground(), &blue));
    }

    #[test]
    fn foreground_is_inherited_by_command_bar_toggle_button_path_icon_through_theme_template() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let _text = TextTestScope::new();
        let red: Rc<dyn IBrush> = Brushes::red();
        let blue: Rc<dyn IBrush> = Brushes::blue();

        let icon = PathIcon::new();
        let btn = CommandBarToggleButton::new();
        btn.set_icon(Some(Control::boxed(icon.clone())));
        let command_bar = CommandBar::new();
        command_bar.set_foreground(Some(red.clone()));
        command_bar.primary_commands().add(btn.as_command_bar_element());

        let presented = apply_simple_theme_and_get_presented_path_icon(
            &command_bar,
            &btn.clone().upcast(),
            command_bar_toggle_button_theme(),
        );
        assert!(presented.ptr_eq(&icon));
        assert!(same_brush(&icon.foreground(), &red));

        command_bar.set_foreground(Some(blue.clone()));

        assert!(same_brush(&icon.foreground(), &blue));
    }

    #[test]
    fn primary_commands_not_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let _ = cb.primary_commands();
        assert!(cb.get_value(CommandBar::primary_commands_property()).is_some());
    }

    #[test]
    fn secondary_commands_not_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let _ = cb.secondary_commands();
        assert!(cb.get_value(CommandBar::secondary_commands_property()).is_some());
    }

    #[test]
    fn visible_primary_commands_not_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        // The same collection every time.
        assert!(cb.visible_primary_commands() == cb.visible_primary_commands());
    }

    #[test]
    fn overflow_items_not_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        // The same collection every time.
        assert!(cb.overflow_items() == cb.overflow_items());
    }

    #[test]
    fn primary_commands_starts_empty() {
        let _scope = test_scope();
        assert!(CommandBar::new().primary_commands().is_empty());
    }

    #[test]
    fn secondary_commands_starts_empty() {
        let _scope = test_scope();
        assert!(CommandBar::new().secondary_commands().is_empty());
    }

    #[test]
    fn primary_commands_returns_new_list_when_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.clear_value(CommandBar::primary_commands_property());
        let commands = cb.primary_commands();
        assert!(commands.is_empty());
        assert!(cb.get_value(CommandBar::primary_commands_property()) == Some(commands));
    }

    #[test]
    fn secondary_commands_returns_new_list_when_null() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.clear_value(CommandBar::secondary_commands_property());
        let commands = cb.secondary_commands();
        assert!(commands.is_empty());
        assert!(cb.get_value(CommandBar::secondary_commands_property()) == Some(commands));
    }

    #[test]
    fn visible_primary_commands_starts_empty() {
        let _scope = test_scope();
        assert!(CommandBar::new().visible_primary_commands().is_empty());
    }

    #[test]
    fn overflow_items_starts_empty() {
        let _scope = test_scope();
        assert!(CommandBar::new().overflow_items().is_empty());
    }

    #[test]
    fn item_width_bottom_default_is_70() {
        let _scope = test_scope();
        assert_eq!(70.0, CommandBar::new().item_width_bottom());
    }

    #[test]
    fn item_width_right_default_is_102() {
        let _scope = test_scope();
        assert_eq!(102.0, CommandBar::new().item_width_right());
    }

    #[test]
    fn item_width_collapsed_default_is_42() {
        let _scope = test_scope();
        assert_eq!(42.0, CommandBar::new().item_width_collapsed());
    }

    /// Gives the command bar and the command the control themes the test
    /// theme has for them (the reference takes them from the resources of
    /// the simple theme), lays the bar out in a root styled with the theme
    /// and returns the icon the icon presenter of the command presents.
    fn apply_simple_theme_and_get_presented_path_icon(
        command_bar: &Ref<CommandBar>,
        command: &Ref<TemplatedControl>,
        command_theme: Ref<ControlTheme>,
    ) -> Ref<PathIcon> {
        command_bar.set_theme(command_bar_theme());
        command.set_theme(command_theme);

        let root = TestRoot::new();
        root.set_width(500.0);
        root.set_height(200.0);
        root.styles().add(create_test_theme());
        root.set_child(command_bar.clone());

        root.apply_styling();
        command_bar.apply_styling();
        command_bar.apply_template();
        root.layout_manager().execute_initial_layout_pass();

        command.apply_styling();
        command.apply_template();

        let presenters: Vec<Ref<ContentPresenter>> = command
            .get_template_descendants()
            .into_iter()
            .filter_map(|descendant| descendant.cast::<ContentPresenter>())
            .filter(|presenter| presenter.name().as_deref() == Some("PART_IconPresenter"))
            .collect();
        assert_eq!(1, presenters.len());
        let presenter = &presenters[0];

        presenter.apply_styling();
        presenter.update_child();

        let child = presenter.child().expect("the icon presenter has a child");
        assert!(std::ptr::eq(child.get_type(), <PathIcon as StaticType>::TYPE));
        let path_icon = child.cast::<PathIcon>().unwrap();
        path_icon.apply_styling();

        path_icon
    }
}

mod command_bar_property_round_trip_tests {
    use super::super::{CommandBar, CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility};
    use super::same_value;
    use crate::test_support::test_scope;
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    #[test]
    fn content_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let content: BoxedValue = Rc::new("content".to_string());
        cb.set_content(Some(content.clone()));
        assert!(same_value(&cb.content(), &content));
    }

    #[test]
    fn default_label_position_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(CommandBarDefaultLabelPosition::Right);
        assert_eq!(CommandBarDefaultLabelPosition::Right, cb.default_label_position());
    }

    #[test]
    fn is_open_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_open(true);
        assert!(cb.is_open());
        cb.set_is_open(false);
        assert!(!cb.is_open());
    }

    #[test]
    fn is_sticky_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_sticky(true);
        assert!(cb.is_sticky());
    }

    #[test]
    fn is_dynamic_overflow_enabled_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_dynamic_overflow_enabled(true);
        assert!(cb.is_dynamic_overflow_enabled());
    }

    #[test]
    fn overflow_button_visibility_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_overflow_button_visibility(CommandBarOverflowButtonVisibility::Visible);
        assert_eq!(CommandBarOverflowButtonVisibility::Visible, cb.overflow_button_visibility());
    }

    #[test]
    fn item_width_bottom_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_item_width_bottom(80.0);
        assert_eq!(80.0, cb.item_width_bottom());
    }

    #[test]
    fn item_width_right_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_item_width_right(120.0);
        assert_eq!(120.0, cb.item_width_right());
    }

    #[test]
    fn item_width_collapsed_round_trip() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_item_width_collapsed(50.0);
        assert_eq!(50.0, cb.item_width_collapsed());
    }
}

mod command_bar_is_open_tests {
    use super::super::CommandBar;
    use crate::test_support::test_scope;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[test]
    fn opening_fired_when_is_open_becomes_true() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let fired = Rc::new(Cell::new(false));
        cb.opening({
            let fired = fired.clone();
            move |_, _| fired.set(true)
        });
        cb.set_is_open(true);
        assert!(fired.get());
    }

    #[test]
    fn opened_fired_when_is_open_becomes_true() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let fired = Rc::new(Cell::new(false));
        cb.opened({
            let fired = fired.clone();
            move |_, _| fired.set(true)
        });
        cb.set_is_open(true);
        assert!(fired.get());
    }

    #[test]
    fn closing_fired_when_is_open_becomes_false() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_open(true);
        let fired = Rc::new(Cell::new(false));
        cb.closing({
            let fired = fired.clone();
            move |_, _| fired.set(true)
        });
        cb.set_is_open(false);
        assert!(fired.get());
    }

    #[test]
    fn closed_fired_when_is_open_becomes_false() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_open(true);
        let fired = Rc::new(Cell::new(false));
        cb.closed({
            let fired = fired.clone();
            move |_, _| fired.set(true)
        });
        cb.set_is_open(false);
        assert!(fired.get());
    }

    #[test]
    fn opening_not_fired_when_already_open() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_is_open(true);
        let count = Rc::new(Cell::new(0));
        cb.opening({
            let count = count.clone();
            move |_, _| count.set(count.get() + 1)
        });
        cb.set_is_open(true);
        assert_eq!(0, count.get());
    }

    #[test]
    fn closing_not_fired_when_already_closed() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let count = Rc::new(Cell::new(0));
        cb.closing({
            let count = count.clone();
            move |_, _| count.set(count.get() + 1)
        });
        cb.set_is_open(false);
        assert_eq!(0, count.get());
    }

    #[test]
    fn events_fired_in_order_open_then_close() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let events = Rc::new(RefCell::new(Vec::new()));
        let record = |name: &'static str| {
            let events = events.clone();
            move |_: &ferroui_base::interactivity::Interactive, _: &ferroui_base::interactivity::RoutedEventArgs| {
                events.borrow_mut().push(name)
            }
        };
        cb.opening(record("Opening"));
        cb.opened(record("Opened"));
        cb.closing(record("Closing"));
        cb.closed(record("Closed"));

        cb.set_is_open(true);
        cb.set_is_open(false);

        assert_eq!(vec!["Opening", "Opened", "Closing", "Closed"], *events.borrow());
    }
}

mod command_bar_collection_tests {
    use super::super::{CommandBar, CommandBarButton, CommandBarSeparator, CommandBarToggleButton, ICommandBarElement};
    use crate::test_support::test_scope;
    use ferroui_base::collections::NotifyCollectionChangedEventArgs;
    use ferroui_base::Ref;
    use std::cell::Cell;
    use std::rc::Rc;

    fn button(label: &str) -> Ref<CommandBarButton> {
        let button = CommandBarButton::new();
        button.set_label(Some(label));
        button
    }

    #[test]
    fn primary_commands_added_appear_in_visible_primary_when_dynamic_overflow_disabled() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button("Save");
        cb.primary_commands().add(btn.as_command_bar_element());
        assert!(cb.visible_primary_commands().contains(&btn.as_command_bar_element()));
    }

    #[test]
    fn primary_commands_default_collection_does_not_duplicate_visible_primary_notifications() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let notifications = Rc::new(Cell::new(0));

        cb.visible_primary_commands().add_collection_changed(Rc::new({
            let notifications = notifications.clone();
            move |_: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>| {
                notifications.set(notifications.get() + 1)
            }
        }));

        cb.primary_commands().add(button("Save").as_command_bar_element());

        assert_eq!(2, notifications.get());
    }

    #[test]
    fn primary_commands_removed_disappears_from_visible_primary() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button("Save");
        cb.primary_commands().add(btn.as_command_bar_element());
        cb.primary_commands().remove(&btn.as_command_bar_element());
        assert!(!cb.visible_primary_commands().contains(&btn.as_command_bar_element()));
    }

    #[test]
    fn secondary_commands_added_appear_in_overflow_items() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button("Settings");
        cb.secondary_commands().add(btn.as_command_bar_element());
        assert!(cb.overflow_items().contains(&btn.as_command_bar_element()));
    }

    #[test]
    fn secondary_commands_default_collection_does_not_duplicate_overflow_notifications() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let notifications = Rc::new(Cell::new(0));

        cb.overflow_items().add_collection_changed(Rc::new({
            let notifications = notifications.clone();
            move |_: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>| {
                notifications.set(notifications.get() + 1)
            }
        }));

        cb.secondary_commands().add(button("Settings").as_command_bar_element());

        assert_eq!(2, notifications.get());
    }

    #[test]
    fn secondary_commands_removed_disappears_from_overflow_items() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button("Settings");
        cb.secondary_commands().add(btn.as_command_bar_element());
        cb.secondary_commands().remove(&btn.as_command_bar_element());
        assert!(!cb.overflow_items().contains(&btn.as_command_bar_element()));
    }

    #[test]
    fn has_secondary_commands_true_when_secondary_added() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.secondary_commands().add(button("Options").as_command_bar_element());
        assert!(cb.has_secondary_commands());
    }

    #[test]
    fn has_secondary_commands_false_after_secondary_cleared() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button("Options");
        cb.secondary_commands().add(btn.as_command_bar_element());
        cb.secondary_commands().remove(&btn.as_command_bar_element());
        assert!(!cb.has_secondary_commands());
    }

    #[test]
    fn overflow_items_count_matches_secondary_command_count() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.secondary_commands().add(CommandBarButton::new().as_command_bar_element());
        cb.secondary_commands().add(CommandBarButton::new().as_command_bar_element());
        assert_eq!(2, cb.overflow_items().count());
    }

    #[test]
    fn visible_primary_commands_count_matches_primary_when_dynamic_overflow_disabled() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.primary_commands().add(CommandBarButton::new().as_command_bar_element());
        cb.primary_commands().add(CommandBarButton::new().as_command_bar_element());
        assert_eq!(2, cb.visible_primary_commands().count());
    }

    #[test]
    fn multiple_primary_commands_all_visible_in_order() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn1 = button("A");
        let btn2 = button("B");
        let btn3 = button("C");
        cb.primary_commands().add(btn1.as_command_bar_element());
        cb.primary_commands().add(btn2.as_command_bar_element());
        cb.primary_commands().add(btn3.as_command_bar_element());
        assert_eq!(
            vec![btn1.as_command_bar_element(), btn2.as_command_bar_element(), btn3.as_command_bar_element()],
            cb.visible_primary_commands().to_vec()
        );
    }

    #[test]
    fn command_bar_separator_can_be_added_to_primary_commands() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let sep = CommandBarSeparator::new();
        cb.primary_commands().add(sep.as_command_bar_element());
        assert!(cb.visible_primary_commands().contains(&sep.as_command_bar_element()));
    }

    #[test]
    fn command_bar_toggle_button_can_be_added_to_primary_commands() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let toggle = CommandBarToggleButton::new();
        toggle.set_label(Some("Bold"));
        cb.primary_commands().add(toggle.as_command_bar_element());
        assert!(cb.visible_primary_commands().contains(&toggle.as_command_bar_element()));
    }
}
