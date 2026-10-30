//! The macOS TabControl retains the inherited equal-width strip layout
//! contract (`Controls/TabControl.xaml`, `Primitives/tab_view_equal_width_panel.rs`):
//! every tab of the strip at the top or the bottom as wide as the others, sharing the width of the
//! strip between `TabViewItemMinWidth` (48) and `TabViewItemMaxWidth` (240), with the scroll
//! buttons of the strip while they do not fit at the minimum width. The strips at the left and at
//! the right stack their tabs, and TabbedPage's top pane keeps items as wide as their content.

use super::support::*;
use crate::primitives::TabViewEqualWidthPanel;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{BoxedValue, Ref, Visual};
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{Application, Border, Control, Dock, ScrollViewer, StackPanel, TabControl, TabItem, Window};
use ferroui_base::controls::ResourceKey;
use ferroui_base::layout::Orientation;
use std::rc::Rc;

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()))
}

/// A TabControl with the tabs of the gallery's page, in a frame of `width` at the top left of a
/// themed window.
fn tab_control_in_window(width: f64, count: usize) -> (Ref<TabControl>, Ref<Border>, Ref<Window>) {
    let tab_control = TabControl::new();
    tab_control.set_height(180.0);
    for index in 0..count {
        let item = TabItem::new();
        item.set_header(header(["Selected", "Normal", "Pointer over", "Disabled", "Fifth", "Sixth"][index % 6]));
        tab_control.items().add(Some(Control::boxed(item)));
    }
    tab_control.set_selected_index(0);

    let frame = Border::new();
    frame.set_width(width);
    frame.set_horizontal_alignment(HorizontalAlignment::Left);
    frame.set_vertical_alignment(VerticalAlignment::Top);
    frame.set_child(&tab_control);
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(1400.0, 600.0));
    window.set_content(Some(Control::boxed(&frame)));
    window.show();
    window.update_layout();
    (tab_control, frame, window)
}

fn descendants<T: ferroui_base::ObjectType>(visual: &Visual) -> Vec<Ref<T>> {
    visual.get_visual_descendants().filter_map(|descendant| descendant.cast::<T>()).collect()
}

fn named<T: ferroui_base::ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|descendant| descendant.name().as_deref() == Some(name))
        .find_map(|descendant| descendant.cast::<T>())
        .unwrap_or_else(|| panic!("{name} is in the template"))
}

fn panel(tab_control: &TabControl) -> Ref<TabViewEqualWidthPanel> {
    descendants::<TabViewEqualWidthPanel>(tab_control).into_iter().next().expect("the items panel of the strip")
}

fn tabs(tab_control: &TabControl) -> Vec<Ref<TabItem>> {
    descendants::<TabItem>(tab_control)
}

/// The strip reserves its two 8-point outer margins, two 2-point content insets,
/// the two structural 2-point columns, the 1-point presenter inset, and the two
/// 4-point header/footer slots. These are the macOS template's explicit layout contract.
const TAB_STRIP_INSETS: f64 = 16.0 + 4.0 + 2.0 + 2.0 + 1.0 + 4.0 + 4.0;
const MIN_TAB_WIDTH: f64 = 48.0;

fn width_for_tabs(width: f64) -> f64 {
    width - TAB_STRIP_INSETS
}

/// Every tab `width` wide, side by side from the left, up to the rounding of their bounds to
/// whole pixels.
fn assert_equal_widths(tabs: &[Ref<TabItem>], width: f64) {
    for (index, tab) in tabs.iter().enumerate() {
        let bounds = tab.bounds();
        assert!((bounds.x - width * index as f64).abs() <= 0.5, "tab {index} at {}", bounds.x);
        assert!((bounds.width - width).abs() <= 1.0, "tab {index} {} wide", bounds.width);
    }
    let last = tabs.last().expect("tabs").bounds();
    assert!((last.right() - width * tabs.len() as f64).abs() <= 1.0, "the last tab ends at {}, not {}", last.right(), width * tabs.len() as f64);
}

/// Whether the scroll buttons of the strip are shown.
fn scroll_buttons_shown(tab_control: &TabControl) -> bool {
    let decrease: Ref<Border> = named(tab_control, "ScrollDecreaseButtonContainer");
    let increase: Ref<Border> = named(tab_control, "ScrollIncreaseButtonContainer");
    assert_eq!(decrease.is_visible(), increase.is_visible());
    increase.is_visible()
}

#[test]
fn the_tabs_at_the_top_share_the_width_of_the_strip() {
    let _app = start_themed_application();
    let (tab_control, frame, window) = tab_control_in_window(600.0, 4);

    let panel = panel(&tab_control);
    let expected = width_for_tabs(600.0) / 4.0;
    assert_eq!(expected, panel.item_width());
    let items = tabs(&tab_control);
    assert_eq!(4, items.len());
    assert_equal_widths(&items, expected);
    let scroll_viewer: Ref<ScrollViewer> = named(&tab_control, "ScrollViewer");
    assert_eq!(scroll_viewer.viewport().width, scroll_viewer.extent().width);
    assert!(!scroll_buttons_shown(&tab_control));

    // Wide: the maximum width.
    frame.set_width(1300.0);
    window.update_layout();
    assert_eq!(240.0, panel.item_width());
    assert!(!scroll_buttons_shown(&tab_control));

    // Narrow: the minimum width, scrolled with the scroll buttons.
    frame.set_width(150.0);
    window.update_layout();
    assert_eq!(MIN_TAB_WIDTH, panel.item_width());
    assert!(scroll_viewer.extent().width > scroll_viewer.viewport().width);
    assert!(scroll_buttons_shown(&tab_control));

    // Wide enough again: the tabs fill the strip, the scroll buttons gone, as soon as the tabs fit
    // at the minimum width beside the header and the footer (TabView leaves the scroll buttons
    // out of the width).
    let fits = 4.0 * MIN_TAB_WIDTH + TAB_STRIP_INSETS;
    frame.set_width(fits + 1.0);
    window.update_layout();
    assert_eq!(width_for_tabs(fits + 1.0) / 4.0, panel.item_width());
    assert!(!scroll_buttons_shown(&tab_control));
    assert_eq!(scroll_viewer.viewport().width, scroll_viewer.extent().width);
    frame.set_width(fits - 4.0);
    window.update_layout();
    assert_eq!(MIN_TAB_WIDTH, panel.item_width());
    assert!(scroll_buttons_shown(&tab_control));
    frame.set_width(fits);
    window.update_layout();
    assert_eq!(MIN_TAB_WIDTH, panel.item_width());
    assert!(!scroll_buttons_shown(&tab_control));

    // More tabs share the same width.
    frame.set_width(800.0);
    window.update_layout();
    let item = TabItem::new();
    item.set_header(header("Fifth"));
    tab_control.items().add(Some(Control::boxed(item)));
    window.update_layout();
    assert_eq!(width_for_tabs(800.0) / 5.0, panel.item_width());
    assert_equal_widths(&tabs(&tab_control), width_for_tabs(800.0) / 5.0);
    window.close();
}

#[test]
fn the_tabs_at_the_bottom_are_the_tabs_of_the_top() {
    let _app = start_themed_application();
    let (tab_control, _frame, window) = tab_control_in_window(600.0, 3);
    tab_control.set_tab_strip_placement(Dock::Bottom);
    window.update_layout();

    let panel = panel(&tab_control);
    assert_eq!(width_for_tabs(600.0) / 3.0, panel.item_width());
    window.close();
}

#[test]
fn the_tabs_at_the_left_and_the_right_are_stacked() {
    let _app = start_themed_application();
    for placement in [Dock::Left, Dock::Right] {
        let (tab_control, _frame, window) = tab_control_in_window(600.0, 3);
        tab_control.set_tab_strip_placement(placement);
        window.update_layout();

        assert!(descendants::<TabViewEqualWidthPanel>(&tab_control).is_empty());
        let stack: Ref<StackPanel> = descendants::<StackPanel>(&tab_control)
            .into_iter()
            .find(|stack| stack.parent().is_some_and(|parent| parent.name().as_deref() == Some("PART_ItemsPresenter")))
            .expect("the items panel of the strip");
        assert_eq!(Orientation::Vertical, stack.orientation());
        window.close();
    }
}

#[test]
fn the_items_of_the_top_pane_of_a_tabbed_page_are_as_wide_as_their_content() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");
    let theme = application
        .try_get_resource(&ResourceKey::from("TabbedPageTabControlTheme"), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<Ref<ControlTheme>>().cloned())
        .expect("TabbedPageTabControlTheme is a resource of the theme");

    for (placement, orientation) in [(Dock::Top, Orientation::Horizontal), (Dock::Left, Orientation::Vertical)] {
        let (tab_control, _frame, window) = tab_control_in_window(600.0, 3);
        tab_control.set_theme(&theme);
        tab_control.set_tab_strip_placement(placement);
        window.update_layout();

        assert!(descendants::<TabViewEqualWidthPanel>(&tab_control).is_empty());
        let stack: Ref<StackPanel> = descendants::<StackPanel>(&tab_control)
            .into_iter()
            .find(|stack| stack.parent().is_some_and(|parent| parent.name().as_deref() == Some("PART_ItemsPresenter")))
            .expect("the items panel of the pane");
        assert_eq!(orientation, stack.orientation());
        window.close();
    }
}
