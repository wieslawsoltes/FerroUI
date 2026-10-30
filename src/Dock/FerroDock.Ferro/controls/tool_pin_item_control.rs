// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockControl, ToolPinnedControl};
use crate::{
    automation::peers::ToolPinItemControlAutomationPeer,
    internal::{event_type::EventType, item_drag_helper::ItemDragHelper},
};
use ferrodock_model::core::DragAction;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, PointerEventArgs},
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, Orientation},
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{TemplatedControl, TemplatedControlImpl},
    ContextMenu, Control, ControlImpl, ItemsControl,
};
use std::{cell::RefCell, rc::Rc};
/// Original pinned-tool drag source.
#[repr(C)]
pub struct ToolPinItemControl {
    base: TemplatedControl,
    drag_helper: RefCell<Option<Rc<ItemDragHelper>>>,
}
ferro_class!(ToolPinItemControl:TemplatedControl);
ferro_class_info!(ToolPinItemControl {
    new: ToolPinItemControl::new
});
ferro_impl_classes!(ToolPinItemControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl ControlImpl for ToolPinItemControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ToolPinItemControlAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for ToolPinItemControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let orientation = this.to_ref();
        let outside = this.to_ref();
        let bounds = this.to_ref();
        let helper = ItemDragHelper::new(
            this,
            Rc::new(move || owner.parent().and_then(|p| p.cast::<ItemsControl>())),
            Rc::new(move || orientation.orientation()),
            None,
            None,
            Some(Rc::new(move |start, e| {
                outside.start_dock_drag(start.expect("source start args"), e)
            })),
            Some(Rc::new(move || {
                bounds
                    .find_ancestor_of_type::<ToolPinnedControl>(false)
                    .map(|c| c.upcast::<Control>())
            })),
        );
        this.drag_helper.replace(Some(helper));
        let helper = this.drag_helper.borrow().clone().expect("source helper");
        helper.attach();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let helper = this.drag_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.detach();
        }
        this.drag_helper.replace(None);
    }
}
ferro_properties! {impl ToolPinItemControl {
    /// Identifies Orientation.
    pub fn orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<Self,Orientation>("Orientation",Orientation::Vertical)}
    /// Identifies PinContextMenu.
    pub fn pin_context_menu_property()->StyledProperty<Option<Ref<ContextMenu>>> {FerroProperty::register::<Self,Option<Ref<ContextMenu>>>("PinContextMenu",None)}
}}
impl ToolPinItemControl {
    /// Constructs the native source class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            drag_helper: RefCell::new(None),
        }
    }
    /// Creates an original pinned-tool item.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets Orientation.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    /// Sets Orientation.
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
    /// Gets PinContextMenu.
    pub fn pin_context_menu(&self) -> Option<Ref<ContextMenu>> {
        self.get_value(Self::pin_context_menu_property())
    }
    /// Sets PinContextMenu.
    pub fn set_pin_context_menu(&self, value: Option<Ref<ContextMenu>>) {
        self.set_value(Self::pin_context_menu_property(), value);
    }
    fn to_drag_action(e: &PointerEventArgs) -> DragAction {
        super::dock_control::to_drag_action(e.key_modifiers())
    }
    fn start_dock_drag(&self, start: &PointerEventArgs, e: &PointerEventArgs) {
        if let Some((dock, controls)) =
            self.find_ancestor_of_type::<DockControl>(false)
                .and_then(|d| {
                    d.layout()
                        .and_then(|l| l.factory())
                        .map(|f| (d, f.dock_controls()))
                })
        {
            let state = dock.concrete_state();
            let start_position = start.get_position(Some(&dock));
            let position = e.get_position(Some(&dock));
            let action = Self::to_drag_action(e);
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
}
