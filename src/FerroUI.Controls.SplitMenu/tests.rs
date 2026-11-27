//! The two parts of a split item.

use crate::split_menu_flyout_item::{is_on_primary_part, SUBMENU_PART_WIDTH};
use crate::{register_types, SplitMenuFlyoutItem};
use ferroui_base::input::{Key, KeyEventArgs};
use ferroui_base::input::InputElement;
use ferroui_base::{Point, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::MenuItem;
use std::cell::Cell;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn press(item: &SplitMenuFlyoutItem, key: Key) -> bool {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    item.raise_event(&args);
    args.handled()
}

fn counted(item: &SplitMenuFlyoutItem) -> Rc<Cell<u32>> {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    item.click(move |_, _| seen.set(seen.get() + 1));
    count
}

#[test]
fn the_trailing_part_is_not_the_primary_part() {
    let size = Size::new(200.0, 32.0);
    assert!(is_on_primary_part(Point::new(10.0, 10.0), size));
    assert!(is_on_primary_part(Point::new(200.0 - SUBMENU_PART_WIDTH - 1.0, 10.0), size));
    assert!(!is_on_primary_part(Point::new(200.0 - SUBMENU_PART_WIDTH, 10.0), size));
    assert!(!is_on_primary_part(Point::new(199.0, 10.0), size));
    assert!(!is_on_primary_part(Point::new(-1.0, 10.0), size));
    assert!(!is_on_primary_part(Point::new(10.0, 40.0), size));
}

#[test]
fn an_item_too_narrow_for_two_parts_has_no_primary_part() {
    assert!(!is_on_primary_part(Point::new(5.0, 5.0), Size::new(SUBMENU_PART_WIDTH, 32.0)));
}

#[test]
fn the_primary_action_raises_click() {
    let _app = start();
    let item = SplitMenuFlyoutItem::new();
    let count = counted(&item);

    item.invoke_primary();
    assert_eq!(1, count.get());
}

#[test]
fn enter_and_space_are_the_primary_action() {
    let _app = start();
    let item = SplitMenuFlyoutItem::new();
    let count = counted(&item);

    assert!(press(&item, Key::Enter));
    assert!(press(&item, Key::Space));
    assert_eq!(2, count.get());
}

#[test]
fn the_arrow_keys_stay_with_the_menu() {
    let _app = start();
    let item = SplitMenuFlyoutItem::new();
    let count = counted(&item);

    assert!(!press(&item, Key::Right));
    assert_eq!(0, count.get());
}

#[test]
fn a_disabled_item_has_no_primary_action_from_the_keyboard() {
    let _app = start();
    let item = SplitMenuFlyoutItem::new();
    item.set_is_enabled(false);
    let count = counted(&item);

    press(&item, Key::Enter);
    assert_eq!(0, count.get());
}

#[test]
fn the_item_has_the_theme_of_a_menu_item() {
    use ferroui_base::StyledElementImpl;
    let _app = start();
    let item = SplitMenuFlyoutItem::new();
    assert!(std::ptr::eq(
        SplitMenuFlyoutItem::style_key_override(&item),
        <MenuItem as ferroui_base::StaticType>::TYPE
    ));
}
