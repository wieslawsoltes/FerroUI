//! Port of the reference `CommandBarTests`: the test classes of the label
//! positions, the overflow button, the item widths and the separators in
//! overflow, one module for each class.

use super::{CommandBar, CommandBarButton, CommandBarSeparator, CommandBarToggleButton, ICommandBarElement};
use ferroui_base::{Ref, Size};
use std::rc::Rc;

fn button() -> Rc<dyn ICommandBarElement> {
    CommandBarButton::new().as_command_bar_element()
}

fn separator() -> Rc<dyn ICommandBarElement> {
    CommandBarSeparator::new().as_command_bar_element()
}

fn el(button: &Ref<CommandBarButton>) -> Rc<dyn ICommandBarElement> {
    button.as_command_bar_element()
}

fn toggle_el(button: &Ref<CommandBarToggleButton>) -> Rc<dyn ICommandBarElement> {
    button.as_command_bar_element()
}

fn create_with_width(width: f64) -> Ref<CommandBar> {
    let cb = CommandBar::new();
    cb.measure(Size::new(width, f64::INFINITY));
    cb
}

fn is_separator(element: &Rc<dyn ICommandBarElement>) -> bool {
    super::command_bar_tests::is_type::<CommandBarSeparator>(element)
}

fn is_button(element: &Rc<dyn ICommandBarElement>) -> bool {
    super::command_bar_tests::is_type::<CommandBarButton>(element)
}

mod command_bar_label_position_tests {
    use super::super::{
        CommandBar, CommandBarButton, CommandBarDefaultLabelPosition as Position, CommandBarSeparator,
        CommandBarToggleButton,
    };
    use super::{el, toggle_el};
    use crate::test_support::test_scope;
    use ferroui_base::Size;

    #[test]
    fn default_label_position_collapsed_sets_is_compact_on_existing_primary_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));

        cb.set_default_label_position(Position::Collapsed);

        assert!(btn.is_compact());
    }

    #[test]
    fn default_label_position_bottom_clears_is_compact_on_primary_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));
        cb.set_default_label_position(Position::Collapsed);

        cb.set_default_label_position(Position::Bottom);

        assert!(!btn.is_compact());
    }

    #[test]
    fn default_label_position_right_sets_label_position_on_primary_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));

        cb.set_default_label_position(Position::Right);

        assert_eq!(Position::Right, btn.label_position());
    }

    #[test]
    fn default_label_position_collapsed_sets_label_position_on_primary_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));

        cb.set_default_label_position(Position::Collapsed);

        assert_eq!(Position::Collapsed, btn.label_position());
    }

    #[test]
    fn default_label_position_collapsed_propagates_is_compact_to_toggle_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let toggle = CommandBarToggleButton::new();
        cb.primary_commands().add(toggle_el(&toggle));

        cb.set_default_label_position(Position::Collapsed);

        assert!(toggle.is_compact());
        assert_eq!(Position::Collapsed, toggle.label_position());
    }

    #[test]
    fn default_label_position_right_propagates_label_position_to_toggle_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let toggle = CommandBarToggleButton::new();
        cb.primary_commands().add(toggle_el(&toggle));

        cb.set_default_label_position(Position::Right);

        assert_eq!(Position::Right, toggle.label_position());
    }

    #[test]
    fn default_label_position_collapsed_sets_is_compact_on_separator() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let sep = CommandBarSeparator::new();
        cb.primary_commands().add(sep.as_command_bar_element());

        cb.set_default_label_position(Position::Collapsed);

        assert!(sep.is_compact());
    }

    #[test]
    fn new_primary_command_gets_current_label_position_when_already_collapsed() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(Position::Collapsed);

        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));

        assert!(btn.is_compact());
        assert_eq!(Position::Collapsed, btn.label_position());
    }

    #[test]
    fn new_primary_command_gets_current_label_position_when_right() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(Position::Right);

        let btn = CommandBarButton::new();
        cb.primary_commands().add(el(&btn));

        assert_eq!(Position::Right, btn.label_position());
    }

    #[test]
    fn default_label_position_collapsed_does_not_compact_secondary_commands() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        cb.secondary_commands().add(el(&btn));

        cb.set_default_label_position(Position::Collapsed);

        assert!(btn.is_in_overflow());
        assert!(!btn.is_compact());
        assert_eq!(Position::Right, btn.label_position());
    }

    #[test]
    fn new_secondary_command_gets_overflow_label_position_when_already_collapsed() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(Position::Collapsed);

        let btn = CommandBarButton::new();
        cb.secondary_commands().add(el(&btn));

        assert!(btn.is_in_overflow());
        assert!(!btn.is_compact());
        assert_eq!(Position::Right, btn.label_position());
    }

    #[test]
    fn default_label_position_collapsed_does_not_compact_secondary_toggle_button() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let toggle = CommandBarToggleButton::new();
        cb.secondary_commands().add(toggle_el(&toggle));

        cb.set_default_label_position(Position::Collapsed);

        assert!(toggle.is_in_overflow());
        assert!(!toggle.is_compact());
        assert_eq!(Position::Right, toggle.label_position());
    }

    #[test]
    fn default_label_position_collapsed_does_not_compact_overflowed_primary_command() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(Position::Collapsed);
        cb.set_is_dynamic_overflow_enabled(true);
        cb.measure(Size::new(81.0, f64::INFINITY));

        let visible = CommandBarButton::new();
        let overflowed = CommandBarButton::new();
        cb.primary_commands().add(el(&visible));
        cb.primary_commands().add(el(&overflowed));

        assert!(cb.visible_primary_commands().contains(&el(&visible)));
        assert!(visible.is_compact());
        assert_eq!(Position::Collapsed, visible.label_position());
        assert!(cb.overflow_items().contains(&el(&overflowed)));
        assert!(overflowed.is_in_overflow());
        assert!(!overflowed.is_compact());
        assert_eq!(Position::Right, overflowed.label_position());
    }

    #[test]
    fn overflowed_primary_command_reapplies_default_label_position_when_restored() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_default_label_position(Position::Collapsed);
        cb.set_is_dynamic_overflow_enabled(true);
        cb.measure(Size::new(81.0, f64::INFINITY));

        let visible = CommandBarButton::new();
        let overflowed = CommandBarButton::new();
        cb.primary_commands().add(el(&visible));
        cb.primary_commands().add(el(&overflowed));
        assert!(cb.overflow_items().contains(&el(&overflowed)));

        cb.measure(Size::new(400.0, f64::INFINITY));

        assert!(cb.visible_primary_commands().contains(&el(&overflowed)));
        assert!(!overflowed.is_in_overflow());
        assert!(overflowed.is_compact());
        assert_eq!(Position::Collapsed, overflowed.label_position());
    }

    #[test]
    fn default_label_position_does_not_clear_label_text() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = CommandBarButton::new();
        btn.set_label(Some("Save"));
        cb.primary_commands().add(el(&btn));

        cb.set_default_label_position(Position::Collapsed);

        assert_eq!(Some("Save".to_string()), btn.label());
    }
}

mod command_bar_overflow_button_tests {
    use super::super::{CommandBar, CommandBarOverflowButtonVisibility as Visibility};
    use super::button;
    use crate::test_support::test_scope;

    #[test]
    fn overflow_button_visibility_visible_sets_is_overflow_button_visible_true() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_overflow_button_visibility(Visibility::Visible);
        assert!(cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_collapsed_sets_is_overflow_button_visible_false() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_overflow_button_visibility(Visibility::Collapsed);
        assert!(!cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_auto_true_when_has_secondary_commands() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.secondary_commands().add(button());
        assert!(cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_auto_false_when_no_secondary_commands() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        assert!(!cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_visible_remains_true_without_secondary() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_overflow_button_visibility(Visibility::Visible);
        assert!(cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_collapsed_remains_false_even_with_secondary() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        cb.set_overflow_button_visibility(Visibility::Collapsed);
        cb.secondary_commands().add(button());
        assert!(!cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_auto_false_after_secondary_removed() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        let btn = button();
        cb.secondary_commands().add(btn.clone());
        assert!(cb.is_overflow_button_visible());

        cb.secondary_commands().remove(&btn);
        assert!(!cb.is_overflow_button_visible());
    }

    #[test]
    fn overflow_button_visibility_switch_from_auto_to_visible_shows_button_immediately() {
        let _scope = test_scope();
        let cb = CommandBar::new();
        assert!(!cb.is_overflow_button_visible());

        cb.set_overflow_button_visibility(Visibility::Visible);
        assert!(cb.is_overflow_button_visible());
    }
}

mod command_bar_item_width_tests {
    use super::super::CommandBarDefaultLabelPosition as Position;
    use super::{button, create_with_width, is_button, is_separator};
    use crate::test_support::test_scope;

    #[test]
    fn item_width_bottom_controls_how_many_buttons_fit() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        let secondary = button();
        cb.secondary_commands().add(secondary.clone()); // forces overflow button
        for _ in 0..4 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(3, cb.visible_primary_commands().count());
        assert_eq!(3, cb.overflow_items().count());
        assert!(is_button(&cb.overflow_items().get(0)));
        assert!(is_separator(&cb.overflow_items().get(1)));
        assert!(secondary == cb.overflow_items().get(2));
    }

    #[test]
    fn item_width_bottom_reduced_allows_more_items_to_fit() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(35.0);
        cb.secondary_commands().add(button());
        for _ in 0..4 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(4, cb.visible_primary_commands().count());
    }

    #[test]
    fn item_width_bottom_large_reduces_to_minimum_one_visible() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);
        cb.secondary_commands().add(button());
        for _ in 0..3 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(1, cb.visible_primary_commands().count());
    }

    #[test]
    fn item_width_right_used_when_label_position_is_right() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_default_label_position(Position::Right);
        cb.secondary_commands().add(button());
        for _ in 0..4 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(2, cb.visible_primary_commands().count());
    }

    #[test]
    fn item_width_right_increased_fewer_items_fit() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_default_label_position(Position::Right);
        cb.set_item_width_right(252.0); // exactly 1 fits: 252/252=1
        cb.secondary_commands().add(button());
        for _ in 0..3 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(1, cb.visible_primary_commands().count());
    }

    #[test]
    fn item_width_collapsed_used_when_label_position_is_collapsed() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_default_label_position(Position::Collapsed);
        cb.secondary_commands().add(button());
        for _ in 0..4 {
            cb.primary_commands().add(button());
        }
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(4, cb.visible_primary_commands().count());
    }

    #[test]
    fn item_widths_are_independent_per_label_position() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(70.0);
        cb.set_item_width_right(102.0);
        cb.set_item_width_collapsed(42.0);
        cb.secondary_commands().add(button());
        for _ in 0..4 {
            cb.primary_commands().add(button());
        }

        cb.set_default_label_position(Position::Bottom);
        cb.set_is_dynamic_overflow_enabled(true);
        let visible_bottom = cb.visible_primary_commands().count(); // 252/70 = 3

        cb.set_is_dynamic_overflow_enabled(false);
        cb.set_default_label_position(Position::Right);
        cb.set_is_dynamic_overflow_enabled(true);
        let visible_right = cb.visible_primary_commands().count(); // 252/102 = 2

        cb.set_is_dynamic_overflow_enabled(false);
        cb.set_default_label_position(Position::Collapsed);
        cb.set_is_dynamic_overflow_enabled(true);
        let visible_collapsed = cb.visible_primary_commands().count(); // 252/42 = 6 -> capped at 4

        assert_eq!(3, visible_bottom);
        assert_eq!(2, visible_right);
        assert_eq!(4, visible_collapsed);
    }
}

mod command_bar_separator_overflow_tests {
    use super::super::{CommandBarButton, CommandBarElementCollection, CommandBarSeparator};
    use super::{button, create_with_width, el, is_button, is_separator, separator};
    use crate::test_support::test_scope;

    fn count_separators(items: &CommandBarElementCollection) -> usize {
        let mut count = 0;
        for i in 0..items.count() {
            if is_separator(&items.get(i)) {
                count += 1;
            }
        }
        count
    }

    fn last(items: &CommandBarElementCollection) -> std::rc::Rc<dyn super::super::ICommandBarElement> {
        items.get(items.count() - 1)
    }

    #[test]
    fn trailing_separator_is_not_last_visible_item() {
        let _scope = test_scope();
        // [Btn, Btn, Sep, Btn] with room for 2 buttons: Sep should NOT trail.
        let cb = create_with_width(300.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(!is_separator(&last(&cb.visible_primary_commands())));
    }

    #[test]
    fn trailing_separator_moved_to_overflow() {
        let _scope = test_scope();
        // [Btn, Sep, Btn, Btn] with room for 1 button: Sep after the single visible button should overflow.
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(1, cb.visible_primary_commands().count());
        assert!(is_button(&cb.visible_primary_commands().get(0)));
    }

    #[test]
    fn multiple_separators_all_trailing_ones_stripped() {
        let _scope = test_scope();
        // [Btn, Sep, Sep, Btn] with room for 1: both trailing separators should be stripped.
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(1, cb.visible_primary_commands().count());
        assert!(is_button(&cb.visible_primary_commands().get(0)));
    }

    #[test]
    fn mid_separator_stays_visible_when_buttons_on_both_sides() {
        let _scope = test_scope();
        // [Btn, Sep, Btn] with room for all: separator stays.
        let cb = create_with_width(300.0);
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(3, cb.visible_primary_commands().count());
        assert!(is_separator(&cb.visible_primary_commands().get(1)));
    }

    #[test]
    fn all_buttons_overflow_separators_also_overflow() {
        let _scope = test_scope();
        // [Sep, Btn, Btn] with room for 0: everything overflows.
        let cb = create_with_width(50.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(cb.visible_primary_commands().is_empty());
    }

    #[test]
    fn leading_separator_is_stripped_from_visible() {
        let _scope = test_scope();
        // [Sep, Btn, Btn, Btn] with room for 2: leading Sep should be stripped.
        let cb = create_with_width(300.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(!is_separator(&cb.visible_primary_commands().get(0)));
    }

    #[test]
    fn consecutive_separators_collapsed_to_one() {
        let _scope = test_scope();
        // [Btn, Sep, Sep, Btn] all fit: only one separator should remain.
        let cb = create_with_width(300.0);
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        let sep_count = count_separators(&cb.visible_primary_commands());
        assert_eq!(1, sep_count);
    }

    #[test]
    fn orphaned_mid_separator_removed_when_neighbor_overflows() {
        let _scope = test_scope();
        // [Btn1, Sep, Btn2, Sep, Btn3] with room for 2: Btn3 overflows,
        // second Sep becomes trailing and is removed. First Sep stays.
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(100.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(!is_separator(&last(&cb.visible_primary_commands())));
        assert_eq!(1, count_separators(&cb.visible_primary_commands()));
    }

    #[test]
    fn separator_between_overflowed_buttons_is_removed() {
        let _scope = test_scope();
        // [Btn1, Btn2, Sep, Btn3, Btn4] with room for 2: Btn3 and Btn4 overflow,
        // Sep has no non-separator after it in visible set, so it is removed.
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(100.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(0, count_separators(&cb.visible_primary_commands()));
    }

    #[test]
    fn multiple_separator_groups_only_valid_ones_remain() {
        let _scope = test_scope();
        // [Btn, Sep, Btn, Sep, Btn, Sep, Btn] with room for 3:
        // last Btn overflows, last Sep becomes trailing, the rest stay.
        let cb = create_with_width(300.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(button());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(!is_separator(&last(&cb.visible_primary_commands())));
        assert!(!is_separator(&cb.visible_primary_commands().get(0)));
    }

    #[test]
    fn only_separators_all_overflow() {
        let _scope = test_scope();
        // [Sep, Sep, Sep] with no buttons: all should overflow.
        let cb = create_with_width(300.0);
        cb.secondary_commands().add(button());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(separator());
        cb.primary_commands().add(separator());
        cb.set_is_dynamic_overflow_enabled(true);

        assert!(cb.visible_primary_commands().is_empty());
    }

    #[test]
    fn primary_separator_is_removed_instead_of_becoming_first_overflow_item() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);

        let leading_separator = CommandBarSeparator::new().as_command_bar_element();
        let first_button = button();
        let overflowed_button = button();

        cb.primary_commands().add(leading_separator.clone());
        cb.primary_commands().add(first_button);
        cb.primary_commands().add(overflowed_button.clone());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(1, cb.overflow_items().count());
        assert!(overflowed_button == cb.overflow_items().get(0));
        assert!(!cb.overflow_items().contains(&leading_separator));
    }

    #[test]
    fn overflowed_primary_commands_precede_secondary_commands_with_synthetic_separator() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);

        let visible_primary = button();
        let original_primary_separator = separator();
        let overflowed_primary_one = button();
        let overflowed_primary_two = button();
        let secondary = button();

        cb.primary_commands().add(visible_primary);
        cb.primary_commands().add(original_primary_separator.clone());
        cb.primary_commands().add(overflowed_primary_one.clone());
        cb.primary_commands().add(overflowed_primary_two.clone());
        cb.secondary_commands().add(secondary.clone());
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(4, cb.overflow_items().count());
        assert!(overflowed_primary_one == cb.overflow_items().get(0));
        assert!(overflowed_primary_two == cb.overflow_items().get(1));
        assert!(is_separator(&cb.overflow_items().get(2)));
        assert!(original_primary_separator != cb.overflow_items().get(2));
        assert!(secondary == cb.overflow_items().get(3));
        assert!(!cb.overflow_items().contains(&original_primary_separator));
    }

    #[test]
    fn hidden_secondary_commands_do_not_get_synthetic_overflow_separator() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);

        let visible_primary = button();
        let overflowed_primary = button();
        let hidden_secondary = CommandBarButton::new();
        hidden_secondary.set_is_visible(false);

        cb.primary_commands().add(visible_primary);
        cb.primary_commands().add(overflowed_primary.clone());
        cb.secondary_commands().add(el(&hidden_secondary));
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(2, cb.overflow_items().count());
        assert!(overflowed_primary == cb.overflow_items().get(0));
        assert!(el(&hidden_secondary) == cb.overflow_items().get(1));
        assert_eq!(0, count_separators(&cb.overflow_items()));
    }

    #[test]
    fn toggling_secondary_visibility_rebuilds_synthetic_overflow_separator() {
        let _scope = test_scope();
        let cb = create_with_width(300.0);
        cb.set_item_width_bottom(260.0);

        let visible_primary = button();
        let overflowed_primary = button();
        let secondary = CommandBarButton::new();

        cb.primary_commands().add(visible_primary);
        cb.primary_commands().add(overflowed_primary.clone());
        cb.secondary_commands().add(el(&secondary));
        cb.set_is_dynamic_overflow_enabled(true);

        assert_eq!(3, cb.overflow_items().count());
        assert!(overflowed_primary == cb.overflow_items().get(0));
        assert!(is_separator(&cb.overflow_items().get(1)));
        assert!(el(&secondary) == cb.overflow_items().get(2));

        secondary.set_is_visible(false);

        assert_eq!(2, cb.overflow_items().count());
        assert!(overflowed_primary == cb.overflow_items().get(0));
        assert!(el(&secondary) == cb.overflow_items().get(1));
        assert_eq!(0, count_separators(&cb.overflow_items()));

        secondary.set_is_visible(true);

        assert_eq!(3, cb.overflow_items().count());
        assert!(overflowed_primary == cb.overflow_items().get(0));
        assert!(is_separator(&cb.overflow_items().get(1)));
        assert!(el(&secondary) == cb.overflow_items().get(2));
    }
}
