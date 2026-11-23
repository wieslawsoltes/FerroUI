// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

//! `ItemContainer` of WinUI: the element of an [`ItemsView`] item. Its template and visual states
//! are in `Themes/WinUI/ItemContainer.xaml`.

use crate::ItemsView;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, MouseButton,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, StyledProperty,
    VisualImpl, WeakRef,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::visual_state::VisualStateManager;
use ferroui_controls::{CheckBox, Control, ControlImpl, Panel};
use std::cell::{Cell, RefCell};

const CONTAINER_ROOT: &str = "PART_ContainerRoot";
const CONTAINER_BACKGROUND: &str = "PART_ContainerBackground";
const SELECTION_CHECKBOX: &str = "PART_SelectionCheckbox";

/// What made an [`ItemContainer`] report an interaction (WinUI `ItemContainerInteractionTrigger`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemContainerInteractionTrigger {
    PointerReleased,
    Tap,
    DoubleTap,
    EnterKey,
    SpaceKey,
}

/// The element of an item of an [`ItemsView`] (WinUI `ItemContainer`).
#[repr(C)]
pub struct ItemContainer {
    base: TemplatedControl,
    root_panel: RefCell<Option<Ref<Panel>>>,
    child_index: Cell<usize>,
    selection_checkbox: RefCell<Option<Ref<CheckBox>>>,
    owner: RefCell<Option<WeakRef<ItemsView>>>,
    is_pointer_over: Cell<bool>,
    is_pressed: Cell<bool>,
    is_multiple: Cell<bool>,
    click_count: Cell<i32>,
}

ferro_class!(ItemContainer: TemplatedControl);
ferro_class_info!(ItemContainer {
    new: ItemContainer::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Child,
    },
});
ferro_impl_classes!(ItemContainer: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

ferro_properties! { impl ItemContainer {
    pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<Self, _>("Child", None)
    }
    pub fn is_selected_property() -> StyledProperty<bool> {
        FerroProperty::register::<Self, _>("IsSelected", false)
    }
} }

impl FerroObjectImpl for ItemContainer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::child_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<Option<Ref<Control>>>();
            if let Some(panel) = this.root_panel.borrow().as_ref() {
                if let Some(old) = old {
                    panel.children().remove(old);
                }
                if let Some(new) = new {
                    panel.children().insert(this.child_index.get(), new);
                }
            }
        } else if property == Self::is_selected_property().as_property() {
            let selected = this.is_selected();
            this.pseudo_classes().set(":selected", selected);
            this.update_visual_state(true);
            this.update_checkbox();
            if let Some(owner) = this.owner() {
                owner.on_container_selected_changed(this, selected);
            }
        } else if property == InputElement::is_effectively_enabled_property().as_property() {
            this.update_visual_state(true);
        }
    }
}

impl TemplatedControlImpl for ItemContainer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let panel = e.name_scope().find_as::<Panel>(CONTAINER_ROOT);
        // WinUI puts the child first. Here the rounded background of the root comes before it.
        let background = e.name_scope().find_as::<Control>(CONTAINER_BACKGROUND);
        let child_index = panel.as_ref().zip(background.as_ref()).and_then(|(panel, background)| {
            panel.children().index_of(background).map(|index| index + 1)
        });
        this.child_index.set(child_index.unwrap_or(0));
        if let (Some(panel), Some(child)) = (&panel, this.child()) {
            panel.children().insert(this.child_index.get(), child);
        }
        *this.root_panel.borrow_mut() = panel;
        *this.selection_checkbox.borrow_mut() = e.name_scope().find_as::<CheckBox>(SELECTION_CHECKBOX);
        Self::parent_on_apply_template(this, e);
        this.update_visual_state(false);
        this.update_multi_select_state(false);
    }
}

impl InputElementImpl for ItemContainer {
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.is_pointer_over.set(true);
        this.update_visual_state(true);
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.is_pointer_over.set(false);
        this.update_visual_state(true);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !e.handled() && e.get_current_point(Some(this)).properties.is_left_button_pressed {
            this.click_count.set(e.click_count());
            this.is_pressed.set(true);
            this.update_visual_state(true);
            this.focus();
            e.set_handled(true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if !this.is_pressed.get() || e.initial_press_mouse_button() != MouseButton::Left {
            return;
        }
        this.is_pressed.set(false);
        this.update_visual_state(true);
        e.set_handled(true);
        if !Rect::from_size(this.bounds().size()).contains(e.get_position(Some(this))) {
            return;
        }
        let modifiers = e.key_modifiers();
        this.raise_interaction(ItemContainerInteractionTrigger::PointerReleased, modifiers);
        let trigger = if this.click_count.get() == 2 {
            ItemContainerInteractionTrigger::DoubleTap
        } else {
            ItemContainerInteractionTrigger::Tap
        };
        this.raise_interaction(trigger, modifiers);
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        if this.is_pressed.replace(false) {
            this.update_visual_state(true);
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !this.is_focused() {
            return;
        }
        let trigger = match e.key {
            Key::Enter => ItemContainerInteractionTrigger::EnterKey,
            Key::Space => ItemContainerInteractionTrigger::SpaceKey,
            _ => return,
        };
        this.raise_interaction(trigger, e.key_modifiers);
        e.set_handled(true);
    }
}

impl ItemContainer {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            root_panel: RefCell::new(None),
            child_index: Cell::new(0),
            selection_checkbox: RefCell::new(None),
            owner: RefCell::new(None),
            is_pointer_over: Cell::new(false),
            is_pressed: Cell::new(false),
            is_multiple: Cell::new(false),
            click_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The content of the item.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::child_property(), value);
    }

    /// Whether the item is selected.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value);
    }

    pub(crate) fn set_owner(&self, owner: Option<&ItemsView>) {
        *self.owner.borrow_mut() = owner.map(|owner| owner.to_ref().downgrade());
    }

    /// `MultiSelectMode`: whether the check box of the multiple selection mode shows.
    pub(crate) fn set_is_multiple(&self, value: bool) {
        if self.is_multiple.replace(value) != value {
            self.update_multi_select_state(true);
        }
    }

    fn owner(&self) -> Option<Ref<ItemsView>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn raise_interaction(&self, trigger: ItemContainerInteractionTrigger, modifiers: KeyModifiers) {
        if let Some(owner) = self.owner() {
            owner.on_container_interaction(self, trigger, modifiers);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_pointer_state(&self, is_pointer_over: bool, is_pressed: bool) {
        self.is_pointer_over.set(is_pointer_over);
        self.is_pressed.set(is_pressed);
        self.update_visual_state(false);
    }

    /// `UpdateVisualState`.
    fn update_visual_state(&self, use_transitions: bool) {
        let selected = self.is_selected();
        if !self.is_effectively_enabled() {
            VisualStateManager::go_to_state(self, "Disabled", use_transitions);
            VisualStateManager::go_to_state(
                self,
                if selected { "SelectedNormal" } else { "UnselectedNormal" },
                use_transitions,
            );
            return;
        }
        VisualStateManager::go_to_state(self, "Enabled", use_transitions);
        let state = match (self.is_pressed.get(), self.is_pointer_over.get(), selected) {
            (true, _, true) => "SelectedPressed",
            (true, _, false) => "UnselectedPressed",
            (false, true, true) => "SelectedPointerOver",
            (false, true, false) => "UnselectedPointerOver",
            (false, false, true) => "SelectedNormal",
            (false, false, false) => "UnselectedNormal",
        };
        VisualStateManager::go_to_state(self, state, use_transitions);
    }

    /// `UpdateMultiSelectState`.
    fn update_multi_select_state(&self, use_transitions: bool) {
        let state = if self.is_multiple.get() { "Multiple" } else { "Single" };
        VisualStateManager::go_to_state(self, state, use_transitions);
        self.update_checkbox();
    }

    /// `UpdateCheckboxState`.
    fn update_checkbox(&self) {
        if let Some(checkbox) = self.selection_checkbox.borrow().as_ref() {
            let selected = self.is_selected();
            if (checkbox.is_checked() == Some(true)) != selected {
                checkbox.set_is_checked(Some(selected));
            }
        }
    }
}
