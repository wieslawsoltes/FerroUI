//! The ItemsView examples: their native controls and the source handlers. No rendering parity
//! with WinUI is claimed.

use crate::{examples, tests::start_winui};
use ferroui_base::Ref;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{CheckBox, ComboBox, Control, NumberBox, RadioButton, StackPanel, TextBlock, Window};
use ferroui_controls_items_view::{ItemContainer, ItemsView, ItemsViewSelectionMode};
use ferroui_controls_repeater::{LinedFlowLayout, StackLayout, UniformGridLayout};

fn page() -> (Ref<Window>, Ref<Control>) {
    let root = examples::create(examples::find("ItemsView").unwrap()).unwrap();
    // Tall enough for all three examples: a view outside the viewport realizes no items.
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(1400.0, 2400.0));
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    window.update_layout();
    (window, root)
}

#[test]
fn the_three_views_show_the_items_in_containers_of_their_theme() {
    let _application = start_winui();
    let (_window, root) = page();
    for name in ["BasicItemsView", "SwappableLayoutsItemsView", "SwappableSelectionModesItemsView"] {
        let view = root.get_control::<ItemsView>(name);
        assert!(view.scroll_view().is_some(), "{name}: the template is applied");
        let repeater = view
            .get_visual_descendants()
            .find_map(|descendant| descendant.cast::<ferroui_controls_repeater::ItemsRepeater>())
            .unwrap_or_else(|| panic!("{name}: the repeater of the template"));
        assert_eq!(repeater.items_source_view().map(|items| items.count()), Some(8), "{name}");
        let first = repeater.try_get_element(0).unwrap_or_else(|| panic!("{name}: the first item is realized"));
        let container = first.cast::<ItemContainer>().expect("an ItemContainer");
        assert!(container.template().is_some(), "{name}: the container has its control theme");
    }
}

#[test]
fn the_layout_options_swap_the_layout_and_its_settings() {
    let _application = start_winui();
    let (window, root) = page();
    let view = root.get_control::<ItemsView>("SwappableLayoutsItemsView");
    let visible = |name: &str| root.get_control::<StackPanel>(name).is_visible();
    let lined = view.layout().and_then(|layout| layout.cast::<LinedFlowLayout>()).expect("the source's layout");
    assert!(visible("spLinedFlowLayoutOptions"));

    root.get_control::<RadioButton>("rbSmallLineHeight").set_is_checked(Some(true));
    assert_eq!(lined.line_height(), 80.0);
    root.get_control::<NumberBox>("nbLineSpacing").set_value(NumberBox::value_property(), 12.0);
    assert_eq!(lined.line_spacing(), 12.0);

    root.get_control::<RadioButton>("rbUniformGridLayout").set_is_checked(Some(true));
    window.update_layout();
    let uniform = view.layout().and_then(|layout| layout.cast::<UniformGridLayout>()).expect("the uniform grid");
    assert!(visible("spUniformGridLayoutOptions") && !visible("spLinedFlowLayoutOptions"));
    root.get_control::<NumberBox>("nbMaximumRowsOrColumns").set_value(NumberBox::value_property(), 2.0);
    assert_eq!(uniform.maximum_rows_or_columns(), 2);

    root.get_control::<RadioButton>("rbStackLayout").set_is_checked(Some(true));
    window.update_layout();
    let stack = view.layout().and_then(|layout| layout.cast::<StackLayout>()).expect("the stack");
    assert!(visible("spStackLayoutOptions"));
    root.get_control::<NumberBox>("nbSpacing").set_value(NumberBox::value_property(), 9.0);
    assert_eq!(stack.spacing(), 9.0);
}

#[test]
fn the_selection_options_set_the_mode_and_report_the_selection() {
    let _application = start_winui();
    let (_window, root) = page();
    let view = root.get_control::<ItemsView>("SwappableSelectionModesItemsView");
    assert_eq!(view.selection_mode(), ItemsViewSelectionMode::Multiple);
    view.select(0);
    view.select(2);
    assert_eq!(
        root.get_control::<TextBlock>("tblSelectionOutput").text().as_deref(),
        Some("You have selected 2 item(s).")
    );
    root.get_control::<ComboBox>("cmbSelectionMode").set_selected_index(3);
    assert_eq!(view.selection_mode(), ItemsViewSelectionMode::Extended);
    root.get_control::<CheckBox>("chkIsItemInvokedEnabled").set_is_checked(Some(true));
    assert!(view.is_item_invoked_enabled());
}
