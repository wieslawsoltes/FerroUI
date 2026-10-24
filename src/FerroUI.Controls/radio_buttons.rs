//! WinUI's `RadioButtons` (`controls/dev/RadioButtons/RadioButtons.cpp` of microsoft-ui-xaml,
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::peers::{AutomationPeer, RadioButtonsAutomationPeer};
use crate::automation::AutomationProperties;
use crate::generators::RecycleKey;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{
    HeaderedSelectingItemsControl, HeaderedSelectingItemsControlImpl, SelectingItemsControl, SelectingItemsControlImpl,
    TemplatedControlImpl, ToggleButton,
};
use crate::{Control, ControlImpl, ItemsChangedEventArgs, ItemsControlImpl, ItemsControlImplExt, RadioButton};
use ferroui_base::input::{
    FocusChangedEventArgs, FocusChangingEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key,
    KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, Visual,
    VisualImpl,
};
use std::cell::Cell;

/// A group of radio buttons with one selection (WinUI's `RadioButtons`).
///
/// The items are shown as [`RadioButton`]s: an item that is a radio button is its own container,
/// and another item is the content of a radio button. The selection is the selection of the items
/// control (`SelectedIndex`, `SelectedItem`, `SelectionChanged`), kept with the checked state of
/// the containers. The arrow keys move the focus and the selection within the group without
/// wrapping, Tab enters the group at the selected button, and Ctrl with an arrow key moves the
/// focus only.
///
/// The template is the control theme of the WinUI theme (`Controls/RadioButtons.xaml`, from
/// WinUI's `RadioButtons.xaml`), with the parts `PART_HeaderPresenter` and `PART_ItemsPresenter`
/// (WinUI's `HeaderContentPresenter` and the `ItemsRepeater` `InnerRepeater`); the theme sets the
/// items panel, a [`RadioButtonsPanel`](crate::primitives::RadioButtonsPanel) with the spacing of
/// the resources `RadioButtonsColumnSpacing` and `RadioButtonsRowSpacing`. The Fluent and Simple
/// themes have no template for it (docs/porting/winui-controls.md, section 5.1).
///
/// The exclusive check is the radio button group manager's: the containers are the logical
/// children of the group and have no group name, so checking one unchecks the others. The group
/// itself only checks the container of a new selection, and unchecks the container of the old one
/// when no container is checked instead (a cleared selection, or one whose container is not
/// realized), which the group manager never does (WinUI's `Select` writes both containers).
///
/// Missing (docs/porting/winui-controls.md, Input): WinUI's element factories and
/// `DataTemplateSelector` as `ItemTemplate`.
#[repr(C)]
pub struct RadioButtons {
    base: HeaderedSelectingItemsControl,
    currently_selecting: Cell<bool>,
}

ferro_class!(RadioButtons: HeaderedSelectingItemsControl);
ferroui_base::ferro_class_info!(RadioButtons {
    new: RadioButtons::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Items,
        attributes: [
            TemplatePart("PART_HeaderPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_ItemsPresenter", type(Ref<ItemsPresenter>)),
        ],
    },
});
ferro_impl_classes!(RadioButtons:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, TemplatedControlImpl,
    SelectingItemsControlImpl, HeaderedSelectingItemsControlImpl
);

impl FerroObjectImpl for RadioButtons {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_wrap_focus(false);
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, args| {
                if let Some(this) = weak.upgrade() {
                    this.on_child_preview_key_down(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = this.to_ref().downgrade();
        this.add_handler(InputElement::getting_focus_event(), move |_, args| {
            if let Some(this) = weak.upgrade() {
                this.on_getting_focus(args);
            }
        });
        let weak = this.to_ref().downgrade();
        this.add_handler(ToggleButton::is_checked_changed_event(), move |_, args| {
            if let Some(this) = weak.upgrade() {
                this.on_child_checked_changed(args);
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            this.update_checked_state(change.get_old_value::<i32>().unwrap_or(-1), this.selected_index());
        } else if change.property() == Self::max_columns_property().as_property() {
            if let Some(panel) = this.items_panel_root() {
                panel.invalidate_measure();
            }
        }
    }
}

impl ControlImpl for RadioButtons {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        RadioButtonsAutomationPeer::new(this).upcast()
    }
}

impl InputElementImpl for RadioButtons {
    fn on_got_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, args);
        // The selection follows the focus when it moves within the group by the keyboard, not
        // with Ctrl.
        if matches!(args.navigation_method, NavigationMethod::Directional | NavigationMethod::Tab)
            && !args.key_modifiers.contains(KeyModifiers::CONTROL)
            && args.old_focused_element.as_ref().is_some_and(|old| this.container_index(old) >= 0)
        {
            if let Some(focused) = &args.new_focused_element {
                let index = this.container_index(focused);
                if index >= 0 {
                    this.set_selected_index(index);
                }
            }
        }
    }
}

impl ItemsControlImpl for RadioButtons {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _key: Option<RecycleKey>,
    ) -> Ref<Control> {
        RadioButton::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<RadioButton>(item)
    }

    fn container_for_item_prepared_override(this: &Self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);
        // OnRepeaterElementPrepared: a checked radio button added to the group is the selection;
        // the container of the selection is checked when it is realized.
        if let Some(radio) = container.cast::<RadioButton>() {
            if radio.is_checked() == Some(true) {
                this.set_selected_index(index);
            } else if index == this.selected_index() {
                this.set_container_checked(&radio, true);
            }
        }
        this.update_automation_positions();
    }

    fn container_index_changed_override(this: &Self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        Self::parent_container_index_changed_override(this, container, old_index, new_index);
        this.update_automation_positions();
    }

    fn clear_container_for_item_override(this: &Self, container: &Ref<Control>) {
        // A container that is recycled for another item is not checked.
        if let Some(radio) = container.cast::<RadioButton>() {
            this.set_container_checked(&radio, false);
        }
        Self::parent_clear_container_for_item_override(this, container);
    }

    fn on_items_view_collection_changed(this: &Self, args: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, args);
        this.update_automation_positions();
    }
}

ferroui_base::ferro_properties! { impl RadioButtons {
    ferro_property!(
        /// Defines the `MaxColumns` property: the largest number of columns. 1 by default; a value
        /// less than 1 is 1 (WinUI rejects it with an exception).
        pub fn max_columns_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<RadioButtons, _>(
                "MaxColumns",
                StyledPropertyOptions::new(1).coerce(|_, value: i32| {
                    if value > 0 {
                        return value;
                    }
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                        logger.log_with_values(None, "RadioButtons.MaxColumns must be greater than zero, not {Value}: 1 is used", &[&value]);
                    }
                    1
                }),
            )
        }
    );
} }

impl RadioButtons {
    fn static_constructor() {
        // IsTabStop False and TabNavigation Once of WinUI's style: the group is entered once, at
        // a button.
        InputElement::focusable_property().override_default_value::<RadioButtons>(false);
        KeyboardNavigation::tab_navigation_property().override_default_value::<RadioButtons>(KeyboardNavigationMode::Once);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: HeaderedSelectingItemsControl::construct(), currently_selecting: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the largest number of columns.
    pub fn max_columns(&self) -> i32 {
        self.get_value(Self::max_columns_property())
    }

    /// Sets the largest number of columns.
    pub fn set_max_columns(&self, value: i32) {
        self.set_value(Self::max_columns_property(), value);
    }

    /// Checks or unchecks a container without taking the change as one of the user.
    fn set_container_checked(&self, radio: &RadioButton, checked: bool) {
        let was_selecting = self.currently_selecting.replace(true);
        radio.set_current_value(ToggleButton::is_checked_property(), Some(checked));
        self.currently_selecting.set(was_selecting);
    }

    fn container_radio_button(&self, index: i32) -> Option<Ref<RadioButton>> {
        if index < 0 {
            return None;
        }
        self.container_from_index(index).and_then(|container| container.cast::<RadioButton>())
    }

    /// The checked state of the containers after a change of the selection (WinUI's `Select`):
    /// the container of the new selection is checked, and the group manager unchecks the other
    /// containers; without one, the container of the old selection is unchecked here.
    fn update_checked_state(&self, old_index: i32, new_index: i32) {
        if let Some(radio) = self.container_radio_button(new_index) {
            self.set_container_checked(&radio, true);
        } else if let Some(radio) = self.container_radio_button(old_index) {
            self.set_container_checked(&radio, false);
        }
    }

    /// OnChildChecked and OnChildUnchecked: a container the user checks is the selection, and
    /// unchecking the selected one clears it.
    fn on_child_checked_changed(&self, args: &RoutedEventArgs) {
        if self.currently_selecting.get() {
            return;
        }
        let Some(radio) = args.source().and_then(|source| source.cast::<RadioButton>()) else { return };
        let index = self.index_from_container(&radio.clone().upcast());
        if index < 0 {
            return;
        }
        if radio.is_checked() == Some(true) {
            self.set_selected_index(index);
        } else if self.selected_index() == index {
            self.set_selected_index(-1);
        }
    }

    /// The position of each container in the group and the size of the group, for UI Automation.
    fn update_automation_positions(&self) {
        let count = self.item_count();
        for index in 0..count {
            if let Some(container) = self.container_from_index(index) {
                AutomationProperties::set_position_in_set(&container, index + 1);
                AutomationProperties::set_size_of_set(&container, count);
            }
        }
    }

    /// The index of the container that is `source` or contains it, or -1.
    fn container_index(&self, source: &InputElement) -> i32 {
        let mut current: Option<Ref<Visual>> = Some(source.to_ref().upcast());
        while let Some(visual) = current {
            if let Some(control) = visual.cast::<Control>() {
                let index = self.index_from_container(&control);
                if index >= 0 {
                    return index;
                }
            }
            current = visual.visual_parent();
        }
        -1
    }

    /// Entering the group with the keyboard focuses the selected button (WinUI's
    /// `OnGettingFocus`).
    fn on_getting_focus(&self, args: &FocusChangingEventArgs) {
        if !matches!(args.navigation_method, NavigationMethod::Tab | NavigationMethod::Directional) {
            return;
        }
        let from_outside = args.old_focused_element.as_ref().is_none_or(|old| self.container_index(old) < 0);
        if from_outside {
            if let Some(selected) = self.container_from_index(self.selected_index()) {
                let selected: Ref<InputElement> = selected.upcast();
                if selected.is_effectively_enabled() && selected.is_visible() {
                    args.try_set_new_focused_element(Some(&selected));
                }
            }
        }
    }

    /// The arrow keys move the focus to the next enabled button of the group, and stop at its ends
    /// (WinUI's `OnChildPreviewKeyDown` with `MoveFocusNext` and `MoveFocusPrevious`); Left and
    /// Right move to the nearest button of the next column.
    fn on_child_preview_key_down(&self, args: &KeyEventArgs) {
        if !matches!(args.key, Key::Up | Key::Down | Key::Left | Key::Right) {
            return;
        }
        let Some(source) = args.source().and_then(|source| source.cast::<InputElement>()) else { return };
        let current = self.container_index(&source);
        if current < 0 {
            return;
        }
        let candidate = if matches!(args.key, Key::Up | Key::Down) {
            let step = if args.key == Key::Down { 1 } else { -1 };
            let mut index = current + step;
            let mut candidate = None;
            while index >= 0 && index < self.item_count() {
                if let Some(control) = self
                    .container_from_index(index)
                    .filter(|c| c.is_visible() && c.is_effectively_enabled() && c.focusable())
                {
                    candidate = Some(control);
                    break;
                }
                index += step;
            }
            candidate
        } else {
            self.horizontal_neighbor(current, args.key == Key::Right)
        };
        if let Some(candidate) = candidate {
            candidate.focus_with(NavigationMethod::Directional, args.key_modifiers);
        }
        // The group owns the arrow keys, at its ends too: the focus does not wrap or leave it.
        args.set_handled(true);
    }

    fn horizontal_neighbor(&self, current: i32, right: bool) -> Option<Ref<Control>> {
        let current = self.container_from_index(current)?;
        let bounds = current.bounds();
        let right = right != (self.flow_direction() == FlowDirection::RightToLeft);
        let mut best: Option<(f64, f64, Ref<Control>)> = None;
        for index in 0..self.item_count() {
            let Some(candidate) =
                self.container_from_index(index).filter(|c| c.is_visible() && c.is_effectively_enabled() && c.focusable())
            else {
                continue;
            };
            let rect = candidate.bounds();
            let distance = if right { rect.x - bounds.x } else { bounds.x - rect.x };
            if distance <= 0.0 {
                continue;
            }
            let vertical = (rect.y - bounds.y).abs();
            if best.as_ref().is_none_or(|(x, y, _)| distance < *x || (distance == *x && vertical < *y)) {
                best = Some((distance, vertical, candidate));
            }
        }
        best.map(|(_, _, control)| control)
    }
}
