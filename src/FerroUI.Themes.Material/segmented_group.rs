// Copyright 2023 The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Segmented rows over real radio/toggle buttons, with native equal-width joins.
//! Reference: AndroidX `SegmentedButton.kt`, e4d933c98df7baa7ea2b83eea6e086915961e0ff.

use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::input::{
    FocusManager, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyModifiers, KeyboardNavigation, KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl};
use ferroui_base::media::FlowDirection;
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    AttachedProperty, CornerRadius, DirectProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, Control, ControlImpl, Panel, PanelImpl, PanelImplExt, RadioButton};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
struct OwnedChild {
    child: WeakRef<Control>,
    original_group: Option<String>,
    original_corner: CornerRadius,
    original_z: i32,
    installed_theme: Option<Ref<ControlTheme>>,
}

/// A Material segmented row. Use `RadioButton` children for single choice (the default),
/// or set `IsMultiSelect="True"` and use `ToggleButton` children for independent choices.
///
/// The group scopes radio selection to its logical children, supplies equal widths, overlapping outlines,
/// first/last logical corner shapes, selection/interaction stacking and arrow navigation.
/// Child `IsChecked`, `IsEnabled`, `Content`, commands and automation remain native.
/// Set `IsMultiSelect` before adding children; replace the children to change their kind.
#[repr(C)]
pub struct MaterialSegmentedGroup {
    base: Panel,
    owned: RefCell<Vec<OwnedChild>>,
    updating: Cell<bool>,
    selected_index: Cell<i32>,
    selection: RefCell<Vec<usize>>,
}
ferro_class!(MaterialSegmentedGroup: Panel);
ferro_class_info!(MaterialSegmentedGroup {
    new: MaterialSegmentedGroup::new
});
ferro_impl_classes!(MaterialSegmentedGroup: StyledElementImpl, InteractiveImpl);

impl FerroObjectImpl for MaterialSegmentedGroup {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_multi_select_property().as_property()
            || change.property() == Self::item_theme_property().as_property()
        {
            this.refresh(None);
        }
    }
}
impl VisualImpl for MaterialSegmentedGroup {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        this.refresh(None);
    }
}
impl ControlImpl for MaterialSegmentedGroup {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialSegmentedGroupAutomationPeer::new(this).upcast()
    }
}
impl PanelImpl for MaterialSegmentedGroup {
    fn children_changed(this: &Self, event: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        for child in event.new_items {
            this.validate_child(child);
        }
        Self::parent_children_changed(this, event);
        if matches!(
            event.action,
            NotifyCollectionChangedAction::Remove | NotifyCollectionChangedAction::Replace
        ) {
            for child in event.old_items {
                this.release_child(child);
            }
        }
        this.refresh(None);
    }
}
impl InputElementImpl for MaterialSegmentedGroup {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if event.handled() {
            return;
        }
        let direction = match event.key {
            Key::Left => {
                if this.flow_direction() == FlowDirection::RightToLeft {
                    1
                } else {
                    -1
                }
            }
            Key::Right => {
                if this.flow_direction() == FlowDirection::RightToLeft {
                    -1
                } else {
                    1
                }
            }
            Key::Home => -2,
            Key::End => 2,
            _ => return,
        };
        let available: Vec<_> = this
            .children()
            .snapshot()
            .iter()
            .filter(|child| {
                child.is_visible() && child.is_effectively_enabled() && child.focusable()
            })
            .cloned()
            .collect();
        if available.is_empty() {
            return;
        }
        let focused =
            FocusManager::get_focus_manager(this).and_then(|manager| manager.get_focused_element());
        let current = available.iter().position(|child| {
            focused
                .as_ref()
                .is_some_and(|focus| focus == &child.clone().upcast::<InputElement>())
        });
        let index = match direction {
            -2 => 0,
            2 => available.len() - 1,
            delta => (current.unwrap_or(0) as isize + delta).rem_euclid(available.len() as isize)
                as usize,
        };
        let next = &available[index];
        if next.focus_with(NavigationMethod::Directional, KeyModifiers::NONE) {
            if !this.is_multi_select() {
                next.set_current_value(ToggleButton::is_checked_property(), Some(true));
            }
            event.set_handled(true);
        }
    }
}
impl LayoutableImpl for MaterialSegmentedGroup {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.refresh(None);
        let children = this.visible_children();
        let count = children.len();
        if count == 0 {
            return Size::default();
        }
        let overlap = this.overlap().max(0.0);
        let width = ((available.width + overlap * (count - 1) as f64) / count as f64).max(0.0);
        let mut desired = Size::new(58.0, 48.0);
        for child in children {
            child.measure(Size::new(width, available.height));
            desired.width = desired.width.max(child.desired_size().width);
            desired.height = desired.height.max(child.desired_size().height);
        }
        Size::new(
            (desired.width * count as f64 - overlap * (count - 1) as f64).max(0.0),
            desired.height,
        )
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.visible_children();
        if children.is_empty() {
            return final_size;
        }
        let overlap = this.overlap().max(0.0);
        let width =
            (final_size.width + overlap * (children.len() - 1) as f64) / children.len() as f64;
        for (index, child) in children.iter().enumerate() {
            child.arrange(Rect::new(
                index as f64 * (width - overlap),
                0.0,
                width,
                final_size.height,
            ));
        }
        final_size
    }
}
ferro_properties! { impl MaterialSegmentedGroup {
    pub fn is_multi_select_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialSegmentedGroup, _>("IsMultiSelect", false)
    }
    /// Adjacent outlines overlap by their stroke width (1 dp by default).
    pub fn overlap_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialSegmentedGroup, _>("Overlap", 1.0)
    }
    pub fn item_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
        FerroProperty::register::<MaterialSegmentedGroup, _>("ItemTheme", None)
    }
    pub fn item_corner_radius_property() -> AttachedProperty<CornerRadius> {
        FerroProperty::register_attached::<MaterialSegmentedGroup, Control, _>("ItemCornerRadius", CornerRadius::uniform(20.0))
    }
    /// The first selected child, or -1. Change a child's IsChecked to change selection.
    pub fn selected_index_property() -> DirectProperty<MaterialSegmentedGroup, i32> {
        FerroProperty::register_direct::<MaterialSegmentedGroup, _>("SelectedIndex", |owner| owner.selected_index(), None, -1)
    }
}}
impl MaterialSegmentedGroup {
    fn static_constructor() {
        Layoutable::horizontal_alignment_property()
            .override_default_value::<Self>(HorizontalAlignment::Left);
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<Self>(KeyboardNavigationMode::Once);
        Layoutable::affects_measure::<Self>(&[Self::overlap_property().as_property()]);
        ToggleButton::is_checked_property()
            .changed()
            .add_class_handler::<ToggleButton>(|child, _| {
                if let Some(owner) = child
                    .parent()
                    .and_then(|parent| parent.cast::<MaterialSegmentedGroup>())
                {
                    owner.refresh(Some(child));
                }
            });
        for property in [
            Visual::is_visible_property().as_property(),
            InputElement::is_effectively_enabled_property().as_property(),
            InputElement::is_focused_property().as_property(),
            Button::is_pressed_property().as_property(),
            RadioButton::group_name_property().as_property(),
        ] {
            property.changed().add_class_handler::<Control>(|child, _| {
                if let Some(owner) = child
                    .parent()
                    .and_then(|parent| parent.cast::<MaterialSegmentedGroup>())
                {
                    owner.refresh(None);
                    owner.invalidate_measure();
                }
            });
        }
    }
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owned: RefCell::new(Vec::new()),
            updating: Cell::new(false),
            selected_index: Cell::new(-1),
            selection: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_multi_select(&self) -> bool {
        self.get_value(Self::is_multi_select_property())
    }
    pub fn set_is_multi_select(&self, value: bool) {
        self.set_value(Self::is_multi_select_property(), value);
    }
    pub fn overlap(&self) -> f64 {
        self.get_value(Self::overlap_property())
    }
    pub fn set_overlap(&self, value: f64) {
        self.set_value(
            Self::overlap_property(),
            if value.is_finite() {
                value.max(0.0)
            } else {
                1.0
            },
        );
    }
    pub fn item_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::item_theme_property())
    }
    pub fn set_item_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::item_theme_property(), value);
    }
    pub fn selected_index(&self) -> i32 {
        self.selected_index.get()
    }
    pub fn selected_indices(&self) -> Vec<usize> {
        self.selection.borrow().clone()
    }
    pub fn get_item_corner_radius(child: &Control) -> CornerRadius {
        child.get_value(Self::item_corner_radius_property())
    }
    pub fn set_item_corner_radius(child: &Control, value: CornerRadius) {
        child.set_value(Self::item_corner_radius_property(), value);
    }
    ferro_routed_event!(
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MaterialSegmentedGroup, _>(
                "SelectionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    pub fn selection_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selection_changed_event(), handler)
    }
    fn validate_child(&self, child: &Control) {
        assert!(
            child.is::<ToggleButton>(),
            "MaterialSegmentedGroup children must be RadioButton or ToggleButton controls"
        );
        assert_eq!(
            child.is::<RadioButton>(),
            !self.is_multi_select(),
            "Single-choice MaterialSegmentedGroup requires RadioButton children; IsMultiSelect requires ToggleButton children"
        );
    }
    fn visible_children(&self) -> Vec<Ref<Control>> {
        self.children()
            .snapshot()
            .iter()
            .filter(|child| child.is_visible())
            .cloned()
            .collect()
    }
    fn release_child(&self, child: &Ref<Control>) {
        let index = self
            .owned
            .borrow()
            .iter()
            .position(|entry| entry.child.upgrade().as_ref() == Some(child));
        let Some(index) = index else { return };
        let entry = self.owned.borrow_mut().remove(index);
        if let Some(radio) = child.cast::<RadioButton>() {
            radio.set_current_value(RadioButton::group_name_property(), entry.original_group);
        }
        child.set_current_value(Self::item_corner_radius_property(), entry.original_corner);
        child.set_current_value(Visual::z_index_property(), entry.original_z);
        if entry.installed_theme.is_some() && child.theme() == entry.installed_theme {
            child.clear_value(StyledElement::theme_property());
        }
    }
    fn refresh(&self, changed: Option<&ToggleButton>) {
        if self.updating.replace(true) {
            return;
        }
        let children = self.children().snapshot();
        let selected = changed
            .filter(|child| child.is_checked() == Some(true))
            .map(|child| child.to_ref().upcast::<Control>());
        let mut found_selected = false;
        for child in children.iter() {
            self.validate_child(child);
            if !self
                .owned
                .borrow()
                .iter()
                .any(|entry| entry.child.upgrade().as_ref() == Some(child))
            {
                self.owned.borrow_mut().push(OwnedChild {
                    child: child.downgrade(),
                    original_group: child
                        .cast::<RadioButton>()
                        .and_then(|radio| radio.group_name()),
                    original_corner: Self::get_item_corner_radius(child),
                    original_z: child.z_index(),
                    installed_theme: None,
                });
            }
            if let Some(radio) = child.cast::<RadioButton>() {
                if radio.group_name().is_some() {
                    // The native RadioButtonGroupManager scopes unnamed groups to
                    // their logical parent. This is already a private identity for
                    // this row, without exposing generated names in XAML snapshots.
                    radio.set_current_value(RadioButton::group_name_property(), None);
                }
            }
            let toggle = child.cast::<ToggleButton>().expect("validated child");
            if !self.is_multi_select() && toggle.is_checked() == Some(true) {
                if selected.as_ref().is_some_and(|selected| selected != child)
                    || (selected.is_none() && found_selected)
                {
                    toggle.set_current_value(ToggleButton::is_checked_property(), Some(false));
                } else {
                    found_selected = true;
                }
            }
            if let Some(theme) = self.item_theme() {
                let managed = self
                    .owned
                    .borrow()
                    .iter()
                    .find(|entry| entry.child.upgrade().as_ref() == Some(child))
                    .and_then(|entry| entry.installed_theme.clone());
                if child.theme().is_none() || managed.as_ref() == child.theme().as_ref() {
                    child.set_current_value(StyledElement::theme_property(), Some(theme.clone()));
                    if let Some(entry) = self
                        .owned
                        .borrow_mut()
                        .iter_mut()
                        .find(|entry| entry.child.upgrade().as_ref() == Some(child))
                    {
                        entry.installed_theme = Some(theme);
                    }
                }
            }
            let z = if toggle.is_checked() == Some(true) {
                5
            } else {
                0
            } + i32::from(child.is_focused())
                + i32::from(toggle.is_pressed());
            child.set_current_value(Visual::z_index_property(), z);
        }
        let visible = self.visible_children();
        for (index, child) in visible.iter().enumerate() {
            let shape = if visible.len() == 1 {
                CornerRadius::uniform(20.0)
            } else if index == 0 {
                CornerRadius::new(20.0, 0.0, 0.0, 20.0)
            } else if index == visible.len() - 1 {
                CornerRadius::new(0.0, 20.0, 20.0, 0.0)
            } else {
                CornerRadius::default()
            };
            child.set_current_value(Self::item_corner_radius_property(), shape);
        }
        let active = visible
            .iter()
            .find(|child| child.is_focused())
            .or_else(|| {
                visible.iter().find(|child| {
                    child.is_effectively_enabled()
                        && child.get_value(ToggleButton::is_checked_property()) == Some(true)
                })
            })
            .or_else(|| visible.iter().find(|child| child.is_effectively_enabled()));
        KeyboardNavigation::set_tab_once_active_element(
            self,
            active.map(|child| child.clone().upcast()),
        );
        let selected: Vec<_> = children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.get_value(ToggleButton::is_checked_property()) == Some(true))
            .map(|(index, _)| index)
            .collect();
        let changed = *self.selection.borrow() != selected;
        self.set_and_raise_cell(
            Self::selected_index_property(),
            &self.selected_index,
            selected.first().map_or(-1, |index| *index as i32),
        );
        *self.selection.borrow_mut() = selected;
        self.updating.set(false);
        if changed {
            self.raise_event(&RoutedEventArgs::with_event_and_source(
                Self::selection_changed_event(),
                self.to_ref(),
            ));
        }
    }
}

/// Group selection provider while its children retain native radio/toggle peers.
#[repr(C)]
pub struct MaterialSegmentedGroupAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MaterialSegmentedGroupAutomationPeer: ControlAutomationPeer);
ferro_class_info!(MaterialSegmentedGroupAutomationPeer {
    interfaces: [Rc<dyn ISelectionProvider> => MaterialSegmentedGroupAutomationPeer::provider]
});
ferro_impl_classes!(MaterialSegmentedGroupAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for MaterialSegmentedGroupAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
    fn is_control_element_core(_: &Self) -> bool {
        true
    }
    fn is_content_element_core(_: &Self) -> bool {
        true
    }
    fn get_class_name_core(_: &Self) -> String {
        "MaterialSegmentedGroup".into()
    }
}
impl MaterialSegmentedGroupAutomationPeer {
    pub fn new(owner: &MaterialSegmentedGroup) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn provider(peer: Ref<Self>) -> Rc<dyn ISelectionProvider> {
        Rc::new(GroupSelectionProvider(peer))
    }
    fn group(&self) -> Ref<MaterialSegmentedGroup> {
        self.owner().cast().expect("segmented group owner")
    }
}
struct GroupSelectionProvider(Ref<MaterialSegmentedGroupAutomationPeer>);
impl ISelectionProvider for GroupSelectionProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        self.0.group().is_multi_select()
    }
    fn is_selection_required(&self) -> bool {
        false
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let group = self.0.group();
        let children = group.children().snapshot();
        group
            .selected_indices()
            .iter()
            .filter_map(|index| children.get(*index))
            .map(|child| ControlAutomationPeer::create_peer_for_element(child))
            .collect()
    }
}
