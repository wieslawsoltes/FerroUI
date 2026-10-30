// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DocumentTabStrip;
use crate::{
    automation::peers::DocumentTabStripItemAutomationPeer,
    internal::{
        dock_helpers::DockHelpers, event_type::EventType, item_drag_helper::ItemDragHelper,
    },
    model_projection,
};
use ferrodock_model::{
    core::{DockCapability, DragAction},
    DockCapabilityResolver,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs, TappedEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies},
    layout::{LayoutableImpl, Orientation},
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::{AutomationPeer, ControlAutomationPeer},
    primitives::{TabStripItem, TemplatedControlImpl},
    ContentControlImpl, ContextMenu, Control, ControlImpl, ItemsControl, ListBoxItem,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
/// Original source tab item with the shared docking engine and native input routes.
#[repr(C)]
pub struct DocumentTabStripItem {
    base: TabStripItem,
    drag_helper: RefCell<Option<Rc<ItemDragHelper>>>,
    pressed: Cell<Option<RoutedEventHandlerToken>>,
    double_tapped: Cell<Option<RoutedEventHandlerToken>>,
}
ferro_class!(DocumentTabStripItem:TabStripItem);
ferro_class_info!(DocumentTabStripItem {new:DocumentTabStripItem::new ,markup:{attributes:[PseudoClasses(":active",":ultracompact")]}});
ferro_impl_classes!(DocumentTabStripItem:LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ContentControlImpl);
impl StyledElementImpl for DocumentTabStripItem {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for DocumentTabStripItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.pseudo_classes().set(":active", this.is_active());
        this.update_ultra_compact(this.bounds().width);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_active_property().as_property() {
            this.pseudo_classes()
                .set(":active", change.get_new_value::<bool>());
        }
        if change.property() == Visual::bounds_property().as_property() {
            this.update_ultra_compact(change.get_new_value::<Rect>().width);
        }
        if change.property() == ListBoxItem::is_selected_property().as_property() {
            if let Some(peer) = ControlAutomationPeer::from_element(this)
                .and_then(|p| p.cast::<DocumentTabStripItemAutomationPeer>())
            {
                peer.notify_is_selected_changed(
                    change.get_old_value::<bool>().unwrap_or_default(),
                    change.get_new_value::<bool>(),
                );
            }
        }
    }
}
impl ControlImpl for DocumentTabStripItem {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DocumentTabStripItemAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for DocumentTabStripItem {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        this.pressed.set(Some(this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| owner.pressed_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        )));
        let owner = this.to_ref();
        this.double_tapped.set(Some(this.add_handler_with(
            InputElement::double_tapped_event(),
            move |_, e| owner.double_tapped_handler(e),
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            false,
        )));
        let owner = this.to_ref();
        let orientation = this.to_ref();
        let outside = this.to_ref();
        let bounds = this.to_ref();
        let helper = ItemDragHelper::new(
            this,
            Rc::new(move || owner.parent().and_then(|p| p.cast::<ItemsControl>())),
            Rc::new(move || {
                orientation
                    .parent()
                    .and_then(|p| p.cast::<DocumentTabStrip>())
                    .map_or(Orientation::Horizontal, |p| p.orientation())
            }),
            None,
            None,
            Some(Rc::new(move |start, e| {
                outside.start_dock_drag(start.expect("source start args"), e)
            })),
            Some(Rc::new(move || {
                bounds
                    .find_ancestor_of_type::<DocumentTabStrip>(false)
                    .map(|b| b.upcast::<Control>())
            })),
        );
        this.drag_helper.replace(Some(helper));
        let helper = this.drag_helper.borrow().clone().expect("source helper");
        helper.attach();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        if let Some(token) = this.pressed.take() {
            this.remove_handler(InputElement::pointer_pressed_event(), token);
        }
        if let Some(token) = this.double_tapped.take() {
            this.remove_handler(InputElement::double_tapped_event(), token);
        }
        let helper = this.drag_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.detach();
        }
        this.drag_helper.replace(None);
    }
}
ferro_properties! {impl DocumentTabStripItem {
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsActive",false)}
    /// Identifies DocumentContextMenu.
    pub fn document_context_menu_property()->StyledProperty<Option<Ref<ContextMenu>>> {FerroProperty::register::<Self,Option<Ref<ContextMenu>>>("DocumentContextMenu",None)}
}}
impl DocumentTabStripItem {
    /// Constructs the native source class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TabStripItem::construct(),
            drag_helper: RefCell::new(None),
            pressed: Cell::new(None),
            double_tapped: Cell::new(None),
        }
    }
    /// Creates an original source tab item.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
    fn update_ultra_compact(&self, width: f64) {
        self.pseudo_classes()
            .set(":ultracompact", width > 0.0 && width <= 32.0);
    }
    /// Gets DocumentContextMenu.
    pub fn document_context_menu(&self) -> Option<Ref<ContextMenu>> {
        self.get_value(Self::document_context_menu_property())
    }
    /// Sets DocumentContextMenu.
    pub fn set_document_context_menu(&self, value: Option<Ref<ContextMenu>>) {
        self.set_value(Self::document_context_menu_property(), value);
    }
    fn to_drag_action(e: &PointerEventArgs) -> DragAction {
        super::dock_control::to_drag_action(e.key_modifiers())
    }
    fn start_dock_drag(&self, start: &PointerEventArgs, e: &PointerEventArgs) {
        if let Some((dock, controls)) = DockHelpers::resolve_dock_control(self).and_then(|d| {
            d.layout()
                .and_then(|l| l.factory())
                .map(|f| (d, f.dock_controls()))
        }) {
            e.pointer()
                .capture(Some(&dock.clone().upcast::<InputElement>()));
            let start_position = start.get_position(Some(&dock));
            let position = e.get_position(Some(&dock));
            let action = Self::to_drag_action(e);
            let state = dock.concrete_state();
            state.start_drag(self, start_position, position, &dock);
            state.process(
                position,
                Default::default(),
                EventType::Moved,
                action,
                Some(&dock),
                controls,
            );
        }
    }
    fn pressed_handler(&self, e: &PointerPressedEventArgs) {
        if e.get_current_point(Some(self))
            .properties
            .is_middle_button_pressed
        {
            if let Some((dockable, factory)) =
                model_projection::dockable(self.data_context().as_ref()).and_then(|d| {
                    d.owner()
                        .and_then(|o| o.as_dock())
                        .and_then(|o| o.factory())
                        .map(|f| (d, f))
                })
            {
                if DockCapabilityResolver::is_enabled(
                    Some(dockable.clone()),
                    DockCapability::Close,
                    DockCapabilityResolver::resolve_operation_dock(Some(dockable.clone())),
                ) {
                    factory.close_dockable(Some(dockable));
                }
            }
        }
    }
    fn double_tapped_handler(&self, _e: &TappedEventArgs) {
        if let Some((dockable, owner, factory)) =
            model_projection::dockable(self.data_context().as_ref()).and_then(|d| {
                d.owner()
                    .and_then(|o| o.as_dock())
                    .and_then(|o| o.factory().map(|f| (d, o, f)))
            })
        {
            if DockCapabilityResolver::is_enabled(
                Some(dockable.clone()),
                DockCapability::Float,
                DockCapabilityResolver::resolve_operation_dock(Some(dockable.clone())),
            ) && (owner.can_close_last_dockable()
                || owner.visible_dockables().map_or(0, |v| v.count()) > 1)
            {
                factory.float_dockable(Some(dockable.clone()));
                factory.activate_window(Some(dockable));
            }
        }
    }
}
