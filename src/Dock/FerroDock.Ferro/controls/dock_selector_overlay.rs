// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    automation::peers::DockSelectorOverlayAutomationPeer,
    selectors::{DockSelectorItem, DockSelectorMode},
};
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    automation::peers::{AutomationPeer, ControlAutomationPeer},
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    ControlImpl, ListBox,
};
use std::{cell::RefCell, rc::Rc};
/// Original selector overlay for documents and tools.
#[repr(C)]
pub struct DockSelectorOverlay {
    base: TemplatedControl,
    items_list: RefCell<Option<Ref<ListBox>>>,
}
ferro_class!(DockSelectorOverlay:TemplatedControl);
ferro_class_info!(DockSelectorOverlay {new:DockSelectorOverlay::new,markup:{attributes:[PseudoClasses(":open"),TemplatePart("PART_ItemsList",type(Ref<ListBox>))]}});
ferro_impl_classes!(DockSelectorOverlay:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl FerroObjectImpl for DockSelectorOverlay {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_current_value(ferroui_base::Visual::is_visible_property(), false);
        this.set_current_value(
            ferroui_base::input::InputElement::is_hit_test_visible_property(),
            false,
        );
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_open_property().as_property() {
            this.pseudo_classes().set(":open", this.is_open());
            this.set_current_value(ferroui_base::Visual::is_visible_property(), this.is_open());
            this.set_current_value(
                ferroui_base::input::InputElement::is_hit_test_visible_property(),
                this.is_open(),
            );
        }
        let Some(peer) = ControlAutomationPeer::from_element(this)
            .and_then(|p| p.cast::<DockSelectorOverlayAutomationPeer>())
        else {
            return;
        };
        if change.property() == Self::is_open_property().as_property() {
            peer.notify_is_open_changed(
                change.get_old_value::<bool>().unwrap_or_default(),
                change.get_new_value::<bool>(),
            );
        } else if change.property() == Self::selected_item_property().as_property() {
            peer.notify_selected_item_changed(
                change
                    .get_old_value::<Option<Rc<DockSelectorItem>>>()
                    .flatten(),
                change.get_new_value::<Option<Rc<DockSelectorItem>>>(),
            );
        } else if change.property() == Self::items_property().as_property() {
            peer.notify_items_changed();
        } else if change.property() == Self::mode_property().as_property() {
            peer.notify_mode_changed(
                change
                    .get_old_value::<DockSelectorMode>()
                    .unwrap_or_default(),
                change.get_new_value::<DockSelectorMode>(),
            );
        }
    }
}
impl ControlImpl for DockSelectorOverlay {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DockSelectorOverlayAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for DockSelectorOverlay {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.items_list
            .replace(e.name_scope().find_as::<ListBox>("PART_ItemsList"));
    }
}
ferro_properties! {impl DockSelectorOverlay {
    /// Identifies IsOpen.
    pub fn is_open_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsOpen",false)}
    /// Identifies the original read-only Items list.
    pub fn items_property()->StyledProperty<Option<ReadOnlyListHandle<Rc<DockSelectorItem>>>> {FerroProperty::register::<Self,Option<ReadOnlyListHandle<Rc<DockSelectorItem>>>>("Items",None)}
    /// Identifies SelectedItem.
    pub fn selected_item_property()->StyledProperty<Option<Rc<DockSelectorItem>>> {FerroProperty::register::<Self,Option<Rc<DockSelectorItem>>>("SelectedItem",None)}
    /// Identifies Mode.
    pub fn mode_property()->StyledProperty<DockSelectorMode> {FerroProperty::register::<Self,DockSelectorMode>("Mode",DockSelectorMode::Documents)}
}}
impl DockSelectorOverlay {
    /// Constructs the native class state, including for derived controls.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items_list: RefCell::new(None),
        }
    }
    /// Creates the original initially hidden overlay.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets IsOpen.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }
    /// Sets IsOpen.
    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value);
    }
    /// Gets the original read-only Items reference.
    pub fn items(&self) -> Option<ReadOnlyListHandle<Rc<DockSelectorItem>>> {
        self.get_value(Self::items_property())
    }
    /// Sets the original read-only Items reference.
    pub fn set_items(&self, value: Option<ReadOnlyListHandle<Rc<DockSelectorItem>>>) {
        self.set_value(Self::items_property(), value);
    }
    /// Gets SelectedItem.
    pub fn selected_item(&self) -> Option<Rc<DockSelectorItem>> {
        self.get_value(Self::selected_item_property())
    }
    /// Sets SelectedItem.
    pub fn set_selected_item(&self, value: Option<Rc<DockSelectorItem>>) {
        self.set_value(Self::selected_item_property(), value);
    }
    /// Gets Mode.
    pub fn mode(&self) -> DockSelectorMode {
        self.get_value(Self::mode_property())
    }
    /// Sets Mode.
    pub fn set_mode(&self, value: DockSelectorMode) {
        self.set_value(Self::mode_property(), value);
    }
    pub(crate) fn items_list(&self) -> Option<Ref<ListBox>> {
        self.items_list.borrow().clone()
    }
}
