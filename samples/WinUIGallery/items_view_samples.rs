//! The ItemsView examples of the pinned WinUI Gallery and the handlers of
//! `ItemsViewPage.xaml.cs` in Rust.

use crate::list_samples::CustomDataObject;
use ferroui_base::controls::ResourceKey;
use ferroui_base::layout::Orientation;
use ferroui_base::{BoxedValue, ObjectType, Ref, Vector};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{CheckBox, ComboBox, Control, ItemsSource, NumberBox, RadioButton, StackPanel, TextBlock};
use ferroui_controls_items_view::{ItemsView, ItemsViewSelectionMode};
use ferroui_controls_repeater::{Layout, LinedFlowLayout, StackLayout, UniformGridLayout};
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(basic) = find::<ItemsView>(root, "BasicItemsView") else {
        return;
    };
    let items = || {
        ItemsSource::from_items(
            CustomDataObject::get_data_objects().into_iter().map(|item| Some(Rc::new(item) as BoxedValue)),
        )
    };
    basic.set_items_source(Some(items()));
    let swappable = root.get_control::<ItemsView>("SwappableLayoutsItemsView");
    swappable.set_items_source(Some(items()));
    let selection = root.get_control::<ItemsView>("SwappableSelectionModesItemsView");
    selection.set_items_source(Some(items()));

    wire_invoked(&basic, &root.get_control::<TextBlock>("tblBasicInvokeOutput"));
    wire_layouts(root, &swappable);
    wire_selection(root, &selection);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

/// `BasicItemsView_ItemInvoked` and `SwappableSelectionModesItemsView_ItemInvoked`.
fn wire_invoked(view: &Ref<ItemsView>, output: &Ref<TextBlock>) {
    let output = output.downgrade();
    view.item_invoked(move |args| {
        let item = args.invoked_item.as_ref().and_then(|item| item.downcast_ref::<Rc<CustomDataObject>>());
        if let (Some(item), Some(output)) = (item, output.upgrade()) {
            output.set_text(Some(&format!("You invoked {}.", item.title)));
        }
    });
}

fn on_checked(radio: &Ref<RadioButton>, handler: impl Fn() + 'static) {
    let weak = radio.downgrade();
    radio.property_changed(move |change| {
        if change.property() == ToggleButton::is_checked_property().as_property()
            && weak.upgrade().is_some_and(|radio| radio.is_checked() == Some(true))
        {
            handler();
        }
    });
}

fn on_value(number: &Ref<NumberBox>, handler: impl Fn(f64) + 'static) {
    number.property_changed({
        let weak = number.downgrade();
        move |change| {
            if change.property() == NumberBox::value_property().as_property() {
                if let Some(number) = weak.upgrade() {
                    handler(number.value());
                }
            }
        }
    });
}

fn page_resource(root: &Ref<Control>, key: &str) -> Option<BoxedValue> {
    root.try_find_resource(&ResourceKey::from(key), root.actual_theme_variant().as_ref()).flatten()
}

/// `RbLayout_Checked`, `RbLineHeight_Checked` and the `ValueChanged` handlers of the layouts. The
/// source scrolls to the top before it changes the lined flow layout and applies the change when
/// the view stops; here the change follows the scroll at once.
fn wire_layouts(root: &Ref<Control>, view: &Ref<ItemsView>) {
    let lined = view.layout().and_then(|layout| layout.cast::<LinedFlowLayout>()).expect("the page's layout");
    let lined_template = view.item_template();
    let stack = StackLayout::new();
    stack.set_orientation(Orientation::Vertical);
    stack.set_spacing(5.0);
    let uniform = UniformGridLayout::new();
    uniform.set_min_row_spacing(5.0);
    uniform.set_min_column_spacing(5.0);
    uniform.set_maximum_rows_or_columns(3);
    let stack_template = page_resource(root, "StackLayoutItemTemplate");
    let uniform_template = page_resource(root, "UniformGridLayoutItemTemplate");

    let panels: Rc<[Ref<StackPanel>; 3]> = Rc::new([
        root.get_control::<StackPanel>("spLinedFlowLayoutOptions"),
        root.get_control::<StackPanel>("spUniformGridLayoutOptions"),
        root.get_control::<StackPanel>("spStackLayoutOptions"),
    ]);
    let select = {
        let view = view.downgrade();
        move |layout: Ref<Layout>, template: Option<BoxedValue>, panel: usize, panels: &[Ref<StackPanel>; 3]| {
            if let Some(view) = view.upgrade() {
                view.set_layout(Some(layout));
                view.set_item_template(template);
            }
            for (index, options) in panels.iter().enumerate() {
                options.set_is_visible(index == panel);
            }
        }
    };
    let choices: [(&str, Ref<Layout>, Option<BoxedValue>); 3] = [
        ("rbLinedFlowLayout", lined.clone().upcast(), lined_template),
        ("rbUniformGridLayout", uniform.clone().upcast(), uniform_template),
        ("rbStackLayout", stack.clone().upcast(), stack_template),
    ];
    for (panel, (name, layout, template)) in choices.into_iter().enumerate() {
        let select = select.clone();
        let panels = panels.clone();
        on_checked(&root.get_control::<RadioButton>(name), move || {
            select(layout.clone(), template.clone(), panel, &panels);
        });
    }

    let scroll_to_top = {
        let view = view.downgrade();
        move || {
            if let Some(scroll) = view.upgrade().and_then(|view| view.scroll_view()) {
                scroll.set_offset(Vector::new(0.0, 0.0));
            }
        }
    };
    for (name, height) in [("rbSmallLineHeight", 80.0), ("rbLargeLineHeight", 160.0)] {
        let lined = lined.clone();
        let scroll_to_top = scroll_to_top.clone();
        on_checked(&root.get_control::<RadioButton>(name), move || {
            scroll_to_top();
            lined.set_line_height(height);
        });
    }
    {
        let lined = lined.clone();
        let scroll_to_top = scroll_to_top.clone();
        on_value(&root.get_control::<NumberBox>("nbLineSpacing"), move |value| {
            scroll_to_top();
            lined.set_line_spacing(value);
        });
    }
    on_value(&root.get_control::<NumberBox>("nbMinItemSpacing"), move |value| {
        scroll_to_top();
        lined.set_min_item_spacing(value);
    });
    on_value(&root.get_control::<NumberBox>("nbSpacing"), move |value| stack.set_spacing(value));
    {
        let uniform = uniform.clone();
        on_value(&root.get_control::<NumberBox>("nbMinColumnSpacing"), move |value| uniform.set_min_column_spacing(value));
    }
    {
        let uniform = uniform.clone();
        on_value(&root.get_control::<NumberBox>("nbMinRowSpacing"), move |value| uniform.set_min_row_spacing(value));
    }
    on_value(&root.get_control::<NumberBox>("nbMaximumRowsOrColumns"), move |value| {
        uniform.set_maximum_rows_or_columns(value as i32);
    });
}

/// `SwappableSelectionModesItemsView_ItemInvoked`, `_SelectionChanged`,
/// `CmbSelectionMode_SelectionChanged` and `ChkIsItemInvokedEnabled_IsCheckedChanged`.
fn wire_selection(root: &Ref<Control>, view: &Ref<ItemsView>) {
    let invocation = root.get_control::<TextBlock>("tblInvocationOutput");
    wire_invoked(view, &invocation);

    let output = root.get_control::<TextBlock>("tblSelectionOutput").downgrade();
    let weak = view.downgrade();
    view.selection_changed(move |_| {
        if let (Some(view), Some(output)) = (weak.upgrade(), output.upgrade()) {
            output.set_text(Some(&format!("You have selected {} item(s).", view.selected_items().len())));
        }
    });

    let combo = root.get_control::<ComboBox>("cmbSelectionMode");
    let weak_combo = combo.downgrade();
    let weak = view.downgrade();
    combo.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(combo), Some(view)) = (weak_combo.upgrade(), weak.upgrade()) {
            view.set_selection_mode(match combo.selected_index() {
                0 => ItemsViewSelectionMode::None,
                1 => ItemsViewSelectionMode::Single,
                3 => ItemsViewSelectionMode::Extended,
                _ => ItemsViewSelectionMode::Multiple,
            });
        }
    });

    let check = root.get_control::<CheckBox>("chkIsItemInvokedEnabled");
    let weak_check = check.downgrade();
    let weak = view.downgrade();
    let invocation = invocation.downgrade();
    check.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let Some(invocation) = invocation.upgrade() {
            invocation.set_text(Some(""));
        }
        if let (Some(check), Some(view)) = (weak_check.upgrade(), weak.upgrade()) {
            view.set_is_item_invoked_enabled(check.is_checked() == Some(true));
        }
    });
}
