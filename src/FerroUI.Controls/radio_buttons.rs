// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from RadioButtons and ColumnMajorUniformToLargestGridLayout in WinUI.

use crate::automation::peers::{
    AutomationPeer, RadioButtonsAutomationPeer,
};
use crate::automation::AutomationProperties;
use crate::generators::RecycleKey;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{
    HeaderedSelectingItemsControl, HeaderedSelectingItemsControlImpl, SelectingItemsControl,
    SelectingItemsControlImpl, TemplatedControl, TemplatedControlImpl, ToggleButton,
};
use crate::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, ITemplateOf};
use crate::{Control, ControlImpl, ItemsChangedEventArgs, ItemsControl, ItemsControlImpl, ItemsControlImplExt, Panel, PanelImpl, RadioButton, StackPanel};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{
    FocusChangedEventArgs, FocusChangingEventArgs, InputElement, InputElementImpl, InputElementImplExt,
    Key, KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, Thickness, Visual, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// A single-choice group using native radio buttons, selection and input services.
///
/// Plain data items become RadioButton containers; existing RadioButton items
/// remain their own containers. The default panel fills columns from top to
/// bottom and fits as many columns as available width and `MaxColumns` permit.
/// `Items`, `ItemsSource`, `ItemTemplate`, `Header`, `HeaderTemplate`,
/// `SelectedIndex`, `SelectedItem`, `SelectionChanged` and `ContainerFromIndex`
/// are inherited native APIs. Data templates use the native `IDataTemplate`
/// contract; WinUI element factories and DataTemplateSelector are not supported.
#[repr(C)]
pub struct RadioButtons {
    base: HeaderedSelectingItemsControl,
    syncing: Cell<bool>,
}

ferro_class!(RadioButtons: HeaderedSelectingItemsControl);
ferroui_base::ferro_class_info!(RadioButtons {
    new: RadioButtons::new,
    markup: { namespace: "FerroUI.Controls", content: Items },
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
        this.add_handler_with(InputElement::key_down_event(), move |_, args| {
            if let Some(this) = weak.upgrade() { this.preview_key_down(args); }
        }, RoutingStrategies::TUNNEL, false);
        let weak = this.to_ref().downgrade();
        this.add_handler(InputElement::getting_focus_event(), move |_, args| {
            if let Some(this) = weak.upgrade() { this.getting_focus(args); }
        });
        let weak = this.to_ref().downgrade();
        this.add_handler(ToggleButton::is_checked_changed_event(), move |_, args| {
            if let Some(this) = weak.upgrade() { this.checked_changed(args); }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            this.synchronize_checked_state();
        } else if change.property() == Self::max_columns_property().as_property() {
            if let Some(panel) = this.items_panel_root() { panel.invalidate_measure(); }
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
        if matches!(args.navigation_method, NavigationMethod::Directional | NavigationMethod::Tab)
            && !args.key_modifiers.contains(KeyModifiers::CONTROL)
            && args.old_focused_element.as_ref().is_some_and(|old| this.container_index(old) >= 0)
        {
            if let Some(focused) = &args.new_focused_element {
                let index = this.container_index(focused);
                if index >= 0 { this.set_selected_index(index); }
            }
        }
    }
}

impl ItemsControlImpl for RadioButtons {
    fn create_container_for_item_override(_this: &Self, _item: &Option<BoxedValue>, _index: i32, _key: Option<RecycleKey>) -> Ref<Control> {
        RadioButton::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<RadioButton>(item)
    }

    fn container_for_item_prepared_override(this: &Self, container: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);
        if container.cast::<RadioButton>().is_some_and(|radio| radio.is_checked() == Some(true)) {
            this.set_selected_index(index);
        }
        this.synchronize_checked_state();
        this.update_automation_positions();
    }

    fn container_index_changed_override(this: &Self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        Self::parent_container_index_changed_override(this, container, old_index, new_index);
        this.synchronize_checked_state();
        this.update_automation_positions();
    }

    fn clear_container_for_item_override(this: &Self, container: &Ref<Control>) {
        let was_syncing = this.syncing.replace(true);
        if let Some(radio) = container.cast::<RadioButton>() {
            radio.set_current_value(ToggleButton::is_checked_property(), Some(false));
        }
        Self::parent_clear_container_for_item_override(this, container);
        this.syncing.set(was_syncing);
    }

    fn on_items_view_collection_changed(this: &Self, args: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, args);
        this.update_automation_positions();
    }
}

ferroui_base::ferro_properties! { impl RadioButtons {
    ferro_property!(
        /// The greatest number of columns. Values less than one are rejected.
        pub fn max_columns_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<RadioButtons, _>("MaxColumns", StyledPropertyOptions::new(1).validate(|value| *value > 0))
        }
    );
} }

impl RadioButtons {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RadioButtons>(false);
        KeyboardNavigation::tab_navigation_property().override_default_value::<RadioButtons>(KeyboardNavigationMode::Once);
        let panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| Some(RadioButtonsPanel::new().upcast()));
        ItemsControl::items_panel_property().override_default_value::<RadioButtons>(panel);
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<RadioButtons>(|_, scope| {
            let header = ContentPresenter::new();
            header.set_name(Some("PART_HeaderPresenter".to_string()));
            header.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
            header.bind_binding(ContentPresenter::content_property().as_property(), &TemplateBinding::new(HeaderedSelectingItemsControl::header_property().as_property()));
            header.bind_binding(ContentPresenter::content_template_property().as_property(), &TemplateBinding::new(HeaderedSelectingItemsControl::header_template_property().as_property()));
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".to_string()));
            let property = ItemsControl::items_panel_property().as_property();
            presenter.bind_binding(property, &TemplateBinding::new(property));
            let stack = StackPanel::new();
            stack.children().add(&header.register_in_name_scope(&**scope));
            stack.children().add(&presenter.register_in_name_scope(&**scope));
            stack.upcast()
        });
        TemplatedControl::template_property().override_default_value::<RadioButtons>(Some(template));
    }

    pub fn construct() -> Self {
        Self { base: HeaderedSelectingItemsControl::construct(), syncing: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }

    pub fn max_columns(&self) -> i32 { self.get_value(Self::max_columns_property()) }

    pub fn set_max_columns(&self, value: i32) { self.set_value(Self::max_columns_property(), value); }

    fn synchronize_checked_state(&self) {
        if self.syncing.replace(true) { return }
        let selected = self.selected_index();
        for index in 0..self.item_count() {
            if let Some(radio) = self.container_from_index(index).and_then(|control| control.cast::<RadioButton>()) {
                radio.set_current_value(ToggleButton::is_checked_property(), Some(index == selected));
            }
        }
        self.syncing.set(false);
    }

    fn checked_changed(&self, args: &RoutedEventArgs) {
        if self.syncing.get() { return }
        let Some(radio) = args.source().and_then(|source| source.cast::<RadioButton>()) else { return };
        let index = self.index_from_container(&radio.clone().upcast());
        if index < 0 { return }
        if radio.is_checked() == Some(true) { self.set_selected_index(index); }
        else if self.selected_index() == index { self.set_selected_index(-1); }
    }

    fn update_automation_positions(&self) {
        for index in 0..self.item_count() {
            if let Some(container) = self.container_from_index(index) {
                AutomationProperties::set_position_in_set(&container, index + 1);
                AutomationProperties::set_size_of_set(&container, self.item_count());
            }
        }
    }

    fn container_index(&self, source: &InputElement) -> i32 {
        let mut current: Option<Ref<Visual>> = Some(source.to_ref().upcast());
        while let Some(visual) = current {
            if let Some(control) = visual.cast::<Control>() {
                let index = self.index_from_container(&control);
                if index >= 0 { return index }
            }
            current = visual.visual_parent();
        }
        -1
    }

    fn getting_focus(&self, args: &FocusChangingEventArgs) {
        if !matches!(args.navigation_method, NavigationMethod::Tab | NavigationMethod::Directional) { return }
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

    fn preview_key_down(&self, args: &KeyEventArgs) {
        if !matches!(args.key, Key::Up | Key::Down | Key::Left | Key::Right) { return }
        let Some(source) = args.source().and_then(|source| source.cast::<InputElement>()) else { return };
        let current = self.container_index(&source);
        if current < 0 { return }
        let candidate = if matches!(args.key, Key::Up | Key::Down) {
            let step = if args.key == Key::Down { 1 } else { -1 };
            let mut index = current + step;
            let mut candidate = None;
            while index >= 0 && index < self.item_count() {
                if let Some(control) = self.container_from_index(index).filter(|c| c.is_visible() && c.is_effectively_enabled() && c.focusable()) {
                    candidate = Some(control);
                    break;
                }
                index += step;
            }
            candidate
        } else {
            self.horizontal_neighbor(current, args.key == Key::Right)
        };
        if let Some(candidate) = candidate { candidate.focus_with(NavigationMethod::Directional, args.key_modifiers); }
        // The group owns directional keys, including its edges: native radio
        // navigation must not wrap or escape the group.
        args.set_handled(true);
    }

    fn horizontal_neighbor(&self, current: i32, right: bool) -> Option<Ref<Control>> {
        let current = self.container_from_index(current)?;
        let bounds = current.bounds();
        let right = right != (self.flow_direction() == FlowDirection::RightToLeft);
        let mut best: Option<(f64, f64, Ref<Control>)> = None;
        for index in 0..self.item_count() {
            let Some(candidate) = self.container_from_index(index).filter(|c| c.is_visible() && c.is_effectively_enabled() && c.focusable()) else { continue };
            let rect = candidate.bounds();
            let distance = if right { rect.x - bounds.x } else { bounds.x - rect.x };
            if distance <= 0.0 { continue }
            let vertical = (rect.y - bounds.y).abs();
            if best.as_ref().is_none_or(|(x, y, _)| distance < *x || (distance == *x && vertical < *y)) {
                best = Some((distance, vertical, candidate));
            }
        }
        best.map(|(_, _, control)| control)
    }
}

#[repr(C)]
pub(crate) struct RadioButtonsPanel {
    base: Panel,
    largest: Cell<Size>,
    columns: Cell<usize>,
}
ferro_class!(RadioButtonsPanel: Panel);
ferro_impl_classes!(RadioButtonsPanel: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl LayoutableImpl for RadioButtonsPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children().snapshot();
        let mut largest = Size::new(0.0, 0.0);
        for child in children.iter() {
            child.measure(available);
            let size = child.desired_size();
            largest.width = largest.width.max(size.width);
            largest.height = largest.height.max(size.height);
        }
        this.largest.set(largest);
        let max_columns = this.templated_parent().and_then(|parent| parent.cast::<RadioButtons>()).map_or(1, |owner| owner.max_columns()) as usize;
        let fit = if available.width.is_infinite() { usize::MAX }
            else { (((available.width - largest.width) / (largest.width + 7.0)).floor().max(0.0) as usize).saturating_add(1) };
        let columns = max_columns.min(children.len()).min(fit).max(1);
        this.columns.set(columns);
        if children.is_empty() { return Size::new(0.0, 0.0) }
        let rows = children.len().div_ceil(columns);
        Size::new(largest.width * columns as f64 + 7.0 * (columns - 1) as f64,
            largest.height * rows as f64 + 8.0 * (rows - 1) as f64)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let columns = this.columns.get();
        let minimum_rows = children.len() / columns;
        let extra = children.len() % columns;
        let largest = this.largest.get();
        let mut index = 0;
        for column in 0..columns {
            for row in 0..minimum_rows + usize::from(column < extra) {
                let child = &children[index];
                let desired = child.desired_size();
                child.arrange(Rect::new(column as f64 * (largest.width + 7.0), row as f64 * (largest.height + 8.0), desired.width, desired.height));
                index += 1;
            }
        }
        final_size
    }
}

impl RadioButtonsPanel {
    fn construct() -> Self { Self { base: Panel::construct(), largest: Cell::new(Size::new(0.0, 0.0)), columns: Cell::new(1) } }
    fn new() -> Ref<Self> { instantiate(Self::construct()) }
}
