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
/// Displays the source confirmation overlay and hosted content using native properties.
#[repr(C)]
pub struct ConfirmationOverlayControl {
    base: TemplatedControl,
    is_overlay_visible: Cell<bool>,
    show_confirmations: Cell<bool>,
    show_global_message: Cell<bool>,
    overlay_message: RefCell<Option<String>>,
    is_content_enabled: Cell<bool>,
    confirmations: RefCell<Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>>,
    local: ServiceAttachment<dyn IDockConfirmationService>,
    global: ServiceAttachment<dyn IDockGlobalConfirmationService>,
    handle: RefCell<Weak<ControlHandle>>,
}
ferro_class!(ConfirmationOverlayControl:TemplatedControl);
ferro_class_info!(ConfirmationOverlayControl {new:ConfirmationOverlayControl::new,
    interfaces:[Rc<dyn IOverlayContentHost> => ConfirmationOverlayControl::as_content_host, Rc<dyn IVisualTreeLifecycle> => ConfirmationOverlayControl::as_visual_tree_lifecycle],
    markup:{content:Content,property_attributes:[Content:[DependsOn("ContentTemplate")]]}
});
ferro_impl_classes!(ConfirmationOverlayControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl ConfirmationOverlayControl {
    /// Identifies the source ConfirmationService styled property.
    pub fn confirmation_service_property()->StyledProperty<Option<Rc<dyn IDockConfirmationService>>>{FerroProperty::register::<ConfirmationOverlayControl,_>("ConfirmationService",None)}
    /// Identifies the source GlobalConfirmationService styled property.
    pub fn global_confirmation_service_property()->StyledProperty<Option<Rc<dyn IDockGlobalConfirmationService>>>{FerroProperty::register::<ConfirmationOverlayControl,_>("GlobalConfirmationService",None)}
    /// Identifies the source Content styled property.
    pub fn content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<ConfirmationOverlayControl,_>("Content",None)}
    /// Identifies the source ContentTemplate styled property.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<ConfirmationOverlayControl,_>("ContentTemplate",None)}
    /// Identifies the source BlocksInput styled property.
    pub fn blocks_input_property()->StyledProperty<bool>{FerroProperty::register::<ConfirmationOverlayControl,_>("BlocksInput",true)}
    /// Identifies the source read-only IsOverlayVisible property.
    pub fn is_overlay_visible_property()->DirectProperty<ConfirmationOverlayControl,bool>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("IsOverlayVisible",|c|c.is_overlay_visible(),None,false)}
    /// Identifies the source read-only ShowConfirmations property.
    pub fn show_confirmations_property()->DirectProperty<ConfirmationOverlayControl,bool>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("ShowConfirmations",|c|c.show_confirmations(),None,false)}
    /// Identifies the source read-only ShowGlobalMessage property.
    pub fn show_global_message_property()->DirectProperty<ConfirmationOverlayControl,bool>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("ShowGlobalMessage",|c|c.show_global_message(),None,false)}
    /// Identifies the source read-only OverlayMessage property.
    pub fn overlay_message_property()->DirectProperty<ConfirmationOverlayControl,Option<String>>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("OverlayMessage",|c|c.overlay_message(),None,None)}
    /// Identifies the source read-only IsContentEnabled property.
    pub fn is_content_enabled_property()->DirectProperty<ConfirmationOverlayControl,bool>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("IsContentEnabled",|c|c.is_content_enabled(),None,false)}
    /// Identifies the source read-only Confirmations property.
    pub fn confirmations_property()->DirectProperty<ConfirmationOverlayControl,Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>>{FerroProperty::register_direct::<ConfirmationOverlayControl,_>("Confirmations",|c|c.confirmations(),None,None)}
}}
impl ConfirmationOverlayControl {
    fn static_constructor() {
        Self::confirmation_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_local(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockConfirmationService>>>()
                        .cloned()
                        .flatten(),
                );
                c.set_confirmations(c.local.current().map(|s| s.confirmations()));
                c.sync_from_services();
            });
        Self::global_confirmation_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_global(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockGlobalConfirmationService>>>()
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
            show_confirmations: Cell::new(false),
            show_global_message: Cell::new(false),
            overlay_message: RefCell::new(None),
            is_content_enabled: Cell::new(true),
            confirmations: RefCell::new(None),
            local: ServiceAttachment::new(),
            global: ServiceAttachment::new(),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates the native control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets ConfirmationService.
    pub fn confirmation_service(&self) -> Option<Rc<dyn IDockConfirmationService>> {
        self.get_value(Self::confirmation_service_property())
    }
    /// Sets ConfirmationService.
    pub fn set_confirmation_service(&self, value: Option<Rc<dyn IDockConfirmationService>>) {
        self.set_value(Self::confirmation_service_property(), value);
    }
    /// Gets GlobalConfirmationService.
    pub fn global_confirmation_service(&self) -> Option<Rc<dyn IDockGlobalConfirmationService>> {
        self.get_value(Self::global_confirmation_service_property())
    }
    /// Sets GlobalConfirmationService.
    pub fn set_global_confirmation_service(
        &self,
        value: Option<Rc<dyn IDockGlobalConfirmationService>>,
    ) {
        self.set_value(Self::global_confirmation_service_property(), value);
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
    /// Gets ShowConfirmations.
    pub fn show_confirmations(&self) -> bool {
        self.show_confirmations.get()
    }
    fn set_show_confirmations(&self, value: bool) {
        self.set_and_raise_cell(
            Self::show_confirmations_property(),
            &self.show_confirmations,
            value,
        );
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
    /// Gets Confirmations.
    pub fn confirmations(&self) -> Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>> {
        self.confirmations.borrow().clone()
    }
    fn set_confirmations(
        &self,
        value: Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>,
    ) {
        self.set_and_raise(Self::confirmations_property(), &self.confirmations, value);
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
        self.attach_local(self.confirmation_service());
        self.attach_global(self.global_confirmation_service());
        self.set_confirmations(self.local.current().map(|s| s.confirmations()));
        self.sync_from_services();
    }
    fn detach_services(&self) {
        self.local.detach();
        self.global.detach();
        self.set_confirmations(None);
    }
    fn attach_local(&self, service: Option<Rc<dyn IDockConfirmationService>>) {
        let weak = self.to_ref().downgrade();
        self.local.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "HasConfirmations" | "ActiveConfirmation") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn attach_global(&self, service: Option<Rc<dyn IDockGlobalConfirmationService>>) {
        let weak = self.to_ref().downgrade();
        self.global.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "IsConfirmationOpen" | "Message") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn sync_from_services(&self) {
        let has_local = self.local.current().is_some_and(|s| s.has_confirmations());
        let has_global = self
            .global
            .current()
            .is_some_and(|s| s.is_confirmation_open());
        let visible = has_local || has_global;
        self.set_is_overlay_visible(visible);
        self.set_show_confirmations(has_local);
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
impl VisualImpl for ConfirmationOverlayControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.on_attached_to_visual_tree();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.on_detached_from_visual_tree();
    }
}
struct ControlHandle(Ref<ConfirmationOverlayControl>);
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
