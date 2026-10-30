// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::i_overlay_content_host::IOverlayContentHost;
use super::service_attachment::ServiceAttachment;
use ferrodock_model::collections::ReadOnlyObservableCollection;
use ferrodock_model::services::*;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    primitives::{TemplatedControl, TemplatedControlImpl},
    templates::IDataTemplate,
    ControlImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
/// Displays the source dialog overlay and hosted content using native properties.
#[repr(C)]
pub struct DialogOverlayControl {
    base: TemplatedControl,
    is_overlay_visible: Cell<bool>,
    show_dialogs: Cell<bool>,
    show_global_message: Cell<bool>,
    overlay_message: RefCell<Option<String>>,
    is_content_enabled: Cell<bool>,
    dialogs: RefCell<Option<ReadOnlyObservableCollection<Rc<DialogRequest>>>>,
    local: ServiceAttachment<dyn IDockDialogService>,
    global: ServiceAttachment<dyn IDockGlobalDialogService>,
    handle: RefCell<Weak<ControlHandle>>,
}
ferro_class!(DialogOverlayControl:TemplatedControl);
ferro_class_info!(DialogOverlayControl {new:DialogOverlayControl::new,
    interfaces:[Rc<dyn IOverlayContentHost> => DialogOverlayControl::as_content_host, Rc<dyn IVisualTreeLifecycle> => DialogOverlayControl::as_visual_tree_lifecycle],
    markup:{content:Content,property_attributes:[Content:[DependsOn("ContentTemplate")]]}
});
ferro_impl_classes!(DialogOverlayControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl DialogOverlayControl {
    /// Identifies the source DialogService styled property.
    pub fn dialog_service_property()->StyledProperty<Option<Rc<dyn IDockDialogService>>>{FerroProperty::register::<DialogOverlayControl,_>("DialogService",None)}
    /// Identifies the source GlobalDialogService styled property.
    pub fn global_dialog_service_property()->StyledProperty<Option<Rc<dyn IDockGlobalDialogService>>>{FerroProperty::register::<DialogOverlayControl,_>("GlobalDialogService",None)}
    /// Identifies the source Content styled property.
    pub fn content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<DialogOverlayControl,_>("Content",None)}
    /// Identifies the source ContentTemplate styled property.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<DialogOverlayControl,_>("ContentTemplate",None)}
    /// Identifies the source BlocksInput styled property.
    pub fn blocks_input_property()->StyledProperty<bool>{FerroProperty::register::<DialogOverlayControl,_>("BlocksInput",true)}
    /// Identifies the source read-only IsOverlayVisible property.
    pub fn is_overlay_visible_property()->DirectProperty<DialogOverlayControl,bool>{FerroProperty::register_direct::<DialogOverlayControl,_>("IsOverlayVisible",|c|c.is_overlay_visible(),None,false)}
    /// Identifies the source read-only ShowDialogs property.
    pub fn show_dialogs_property()->DirectProperty<DialogOverlayControl,bool>{FerroProperty::register_direct::<DialogOverlayControl,_>("ShowDialogs",|c|c.show_dialogs(),None,false)}
    /// Identifies the source read-only ShowGlobalMessage property.
    pub fn show_global_message_property()->DirectProperty<DialogOverlayControl,bool>{FerroProperty::register_direct::<DialogOverlayControl,_>("ShowGlobalMessage",|c|c.show_global_message(),None,false)}
    /// Identifies the source read-only OverlayMessage property.
    pub fn overlay_message_property()->DirectProperty<DialogOverlayControl,Option<String>>{FerroProperty::register_direct::<DialogOverlayControl,_>("OverlayMessage",|c|c.overlay_message(),None,None)}
    /// Identifies the source read-only IsContentEnabled property.
    pub fn is_content_enabled_property()->DirectProperty<DialogOverlayControl,bool>{FerroProperty::register_direct::<DialogOverlayControl,_>("IsContentEnabled",|c|c.is_content_enabled(),None,false)}
    /// Identifies the source read-only Dialogs property.
    pub fn dialogs_property()->DirectProperty<DialogOverlayControl,Option<ReadOnlyObservableCollection<Rc<DialogRequest>>>>{FerroProperty::register_direct::<DialogOverlayControl,_>("Dialogs",|c|c.dialogs(),None,None)}
}}
impl DialogOverlayControl {
    fn static_constructor() {
        Self::dialog_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_local(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockDialogService>>>()
                        .cloned()
                        .flatten(),
                );
                c.set_dialogs(c.local.current().map(|s| s.dialogs()));
                c.sync_from_services();
            });
        Self::global_dialog_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_global(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockGlobalDialogService>>>()
                        .cloned()
                        .flatten(),
                );
                c.sync_from_services();
            });
        Self::blocks_input_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.sync_from_services());
    }
    /// Constructs the original field defaults without subscribing to services.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            is_overlay_visible: Cell::new(false),
            show_dialogs: Cell::new(false),
            show_global_message: Cell::new(false),
            overlay_message: RefCell::new(None),
            is_content_enabled: Cell::new(true),
            dialogs: RefCell::new(None),
            local: ServiceAttachment::new(),
            global: ServiceAttachment::new(),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates the native control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets DialogService.
    pub fn dialog_service(&self) -> Option<Rc<dyn IDockDialogService>> {
        self.get_value(Self::dialog_service_property())
    }
    /// Sets DialogService.
    pub fn set_dialog_service(&self, value: Option<Rc<dyn IDockDialogService>>) {
        self.set_value(Self::dialog_service_property(), value);
    }
    /// Gets GlobalDialogService.
    pub fn global_dialog_service(&self) -> Option<Rc<dyn IDockGlobalDialogService>> {
        self.get_value(Self::global_dialog_service_property())
    }
    /// Sets GlobalDialogService.
    pub fn set_global_dialog_service(&self, value: Option<Rc<dyn IDockGlobalDialogService>>) {
        self.set_value(Self::global_dialog_service_property(), value);
    }
    /// Gets Content.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }
    /// Sets Content.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value);
    }
    /// Gets ContentTemplate.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }
    /// Sets ContentTemplate.
    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value);
    }
    /// Gets BlocksInput.
    pub fn blocks_input(&self) -> bool {
        self.get_value(Self::blocks_input_property())
    }
    /// Sets BlocksInput.
    pub fn set_blocks_input(&self, value: bool) {
        self.set_value(Self::blocks_input_property(), value);
    }
    /// Gets IsOverlayVisible.
    pub fn is_overlay_visible(&self) -> bool {
        self.is_overlay_visible.get()
    }
    fn set_is_overlay_visible(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_overlay_visible_property(),
            &self.is_overlay_visible,
            value,
        );
    }
    /// Gets ShowDialogs.
    pub fn show_dialogs(&self) -> bool {
        self.show_dialogs.get()
    }
    fn set_show_dialogs(&self, value: bool) {
        self.set_and_raise_cell(Self::show_dialogs_property(), &self.show_dialogs, value);
    }
    /// Gets ShowGlobalMessage.
    pub fn show_global_message(&self) -> bool {
        self.show_global_message.get()
    }
    fn set_show_global_message(&self, value: bool) {
        self.set_and_raise_cell(
            Self::show_global_message_property(),
            &self.show_global_message,
            value,
        );
    }
    /// Gets OverlayMessage.
    pub fn overlay_message(&self) -> Option<String> {
        self.overlay_message.borrow().clone()
    }
    fn set_overlay_message(&self, value: Option<String>) {
        self.set_and_raise(
            Self::overlay_message_property(),
            &self.overlay_message,
            value,
        );
    }
    /// Gets IsContentEnabled.
    pub fn is_content_enabled(&self) -> bool {
        self.is_content_enabled.get()
    }
    fn set_is_content_enabled(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_content_enabled_property(),
            &self.is_content_enabled,
            value,
        );
    }
    /// Gets Dialogs.
    pub fn dialogs(&self) -> Option<ReadOnlyObservableCollection<Rc<DialogRequest>>> {
        self.dialogs.borrow().clone()
    }
    fn set_dialogs(&self, value: Option<ReadOnlyObservableCollection<Rc<DialogRequest>>>) {
        self.set_and_raise(Self::dialogs_property(), &self.dialogs, value);
    }
    /// Rebinds both services when attached, including explicit lifecycle calls.
    pub fn on_attached_to_visual_tree(&self) {
        self.rebind_services();
    }
    /// Removes service subscriptions when detached, retaining source state.
    pub fn on_detached_from_visual_tree(&self) {
        self.detach_services();
    }
    fn rebind_services(&self) {
        self.attach_local(self.dialog_service());
        self.attach_global(self.global_dialog_service());
        self.set_dialogs(self.local.current().map(|s| s.dialogs()));
        self.sync_from_services();
    }
    fn detach_services(&self) {
        self.local.detach();
        self.global.detach();
        self.set_dialogs(None);
    }
    fn attach_local(&self, service: Option<Rc<dyn IDockDialogService>>) {
        let weak = self.to_ref().downgrade();
        self.local.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "HasDialogs" | "ActiveDialog") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn attach_global(&self, service: Option<Rc<dyn IDockGlobalDialogService>>) {
        let weak = self.to_ref().downgrade();
        self.global.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "IsDialogOpen" | "Message") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn sync_from_services(&self) {
        let has_local = self.local.current().is_some_and(|s| s.has_dialogs());
        let has_global = self.global.current().is_some_and(|s| s.is_dialog_open());
        let visible = has_local || has_global;
        self.set_is_overlay_visible(visible);
        self.set_show_dialogs(has_local);
        self.set_show_global_message(has_global && !has_local);
        self.set_overlay_message(if has_local {
            None
        } else {
            self.global.current().and_then(|s| s.message())
        });
        self.set_is_content_enabled(!(visible && self.blocks_input()));
    }
    fn handle(this: Ref<Self>) -> Rc<ControlHandle> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(ControlHandle(this.clone()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
    pub(crate) fn as_content_host(this: Ref<Self>) -> Rc<dyn IOverlayContentHost> {
        Self::handle(this)
    }
    /// Projects this same control to its framework-independent lifecycle contract.
    pub fn as_visual_tree_lifecycle(this: Ref<Self>) -> Rc<dyn IVisualTreeLifecycle> {
        Self::handle(this)
    }
}
impl VisualImpl for DialogOverlayControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.on_attached_to_visual_tree();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.on_detached_from_visual_tree();
    }
}
struct ControlHandle(Ref<DialogOverlayControl>);
impl IOverlayContentHost for ControlHandle {
    fn content(&self) -> Option<BoxedValue> {
        self.0.content()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        self.0.set_content(value);
    }
    fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.0.content_template()
    }
    fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.0.set_content_template(value);
    }
    fn blocks_input(&self) -> bool {
        self.0.blocks_input()
    }
    fn set_blocks_input(&self, value: bool) {
        self.0.set_blocks_input(value);
    }
}
impl IVisualTreeLifecycle for ControlHandle {
    fn on_attached_to_visual_tree(&self) {
        self.0.on_attached_to_visual_tree();
    }
    fn on_detached_from_visual_tree(&self) {
        self.0.on_detached_from_visual_tree();
    }
}
