// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::i_overlay_content_host::IOverlayContentHost;
use super::service_attachment::ServiceAttachment;
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
use ferroui_reactiveui::ICommand;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
/// Displays the source busy overlay and hosted content using native properties.
#[repr(C)]
pub struct BusyOverlayControl {
    base: TemplatedControl,
    is_busy: Cell<bool>,
    busy_message: RefCell<Option<String>>,
    is_content_enabled: Cell<bool>,
    show_progress: Cell<bool>,
    is_reload_visible: Cell<bool>,
    is_reload_enabled: Cell<bool>,
    reload_command: RefCell<Option<Rc<dyn ICommand>>>,
    local: ServiceAttachment<dyn IDockBusyService>,
    global: ServiceAttachment<dyn IDockGlobalBusyService>,
    handle: RefCell<Weak<ControlHandle>>,
}
ferro_class!(BusyOverlayControl:TemplatedControl);
ferro_class_info!(BusyOverlayControl {new:BusyOverlayControl::new,
    interfaces:[Rc<dyn IOverlayContentHost> => BusyOverlayControl::as_content_host, Rc<dyn IVisualTreeLifecycle> => BusyOverlayControl::as_visual_tree_lifecycle],
    markup:{content:Content,property_attributes:[Content:[DependsOn("ContentTemplate")]]}
});
ferro_impl_classes!(BusyOverlayControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl BusyOverlayControl {
    /// Identifies the source BusyService styled property.
    pub fn busy_service_property()->StyledProperty<Option<Rc<dyn IDockBusyService>>>{FerroProperty::register::<BusyOverlayControl,_>("BusyService",None)}
    /// Identifies the source GlobalBusyService styled property.
    pub fn global_busy_service_property()->StyledProperty<Option<Rc<dyn IDockGlobalBusyService>>>{FerroProperty::register::<BusyOverlayControl,_>("GlobalBusyService",None)}
    /// Identifies the source Content styled property.
    pub fn content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<BusyOverlayControl,_>("Content",None)}
    /// Identifies the source ContentTemplate styled property.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<BusyOverlayControl,_>("ContentTemplate",None)}
    /// Identifies the source BlocksInput styled property.
    pub fn blocks_input_property()->StyledProperty<bool>{FerroProperty::register::<BusyOverlayControl,_>("BlocksInput",true)}
    /// Identifies the source read-only IsBusy property.
    pub fn is_busy_property()->DirectProperty<BusyOverlayControl,bool>{FerroProperty::register_direct::<BusyOverlayControl,_>("IsBusy",|c|c.is_busy(),None,false)}
    /// Identifies the source read-only BusyMessage property.
    pub fn busy_message_property()->DirectProperty<BusyOverlayControl,Option<String>>{FerroProperty::register_direct::<BusyOverlayControl,_>("BusyMessage",|c|c.busy_message(),None,None)}
    /// Identifies the source read-only IsContentEnabled property.
    pub fn is_content_enabled_property()->DirectProperty<BusyOverlayControl,bool>{FerroProperty::register_direct::<BusyOverlayControl,_>("IsContentEnabled",|c|c.is_content_enabled(),None,false)}
    /// Identifies the source read-only ShowProgress property.
    pub fn show_progress_property()->DirectProperty<BusyOverlayControl,bool>{FerroProperty::register_direct::<BusyOverlayControl,_>("ShowProgress",|c|c.show_progress(),None,false)}
    /// Identifies the source ShowReloadButton styled property.
    pub fn show_reload_button_property()->StyledProperty<Option<bool>>{FerroProperty::register::<BusyOverlayControl,_>("ShowReloadButton",None)}
    /// Identifies the source read-only IsReloadVisible property.
    pub fn is_reload_visible_property()->DirectProperty<BusyOverlayControl,bool>{FerroProperty::register_direct::<BusyOverlayControl,_>("IsReloadVisible",|c|c.is_reload_visible(),None,false)}
    /// Identifies the source read-only IsReloadEnabled property.
    pub fn is_reload_enabled_property()->DirectProperty<BusyOverlayControl,bool>{FerroProperty::register_direct::<BusyOverlayControl,_>("IsReloadEnabled",|c|c.is_reload_enabled(),None,false)}
    /// Identifies the source read-only ReloadCommand property.
    pub fn reload_command_property()->DirectProperty<BusyOverlayControl,Option<Rc<dyn ICommand>>>{FerroProperty::register_direct::<BusyOverlayControl,_>("ReloadCommand",|c|c.reload_command(),None,None)}
}}
impl BusyOverlayControl {
    fn static_constructor() {
        Self::busy_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_local(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockBusyService>>>()
                        .cloned()
                        .flatten(),
                );
                c.sync_from_services();
            });
        Self::global_busy_service_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                c.attach_global(
                    e.new_value()
                        .downcast_ref::<Option<Rc<dyn IDockGlobalBusyService>>>()
                        .cloned()
                        .flatten(),
                );
                c.sync_from_services();
            });
        Self::blocks_input_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.sync_from_services());
        Self::show_reload_button_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.sync_from_services());
    }
    /// Constructs the original field defaults without subscribing to services.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            is_busy: Cell::new(false),
            busy_message: RefCell::new(None),
            is_content_enabled: Cell::new(true),
            show_progress: Cell::new(false),
            is_reload_visible: Cell::new(true),
            is_reload_enabled: Cell::new(false),
            reload_command: RefCell::new(None),
            local: ServiceAttachment::new(),
            global: ServiceAttachment::new(),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates the native control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets BusyService.
    pub fn busy_service(&self) -> Option<Rc<dyn IDockBusyService>> {
        self.get_value(Self::busy_service_property())
    }
    /// Sets BusyService.
    pub fn set_busy_service(&self, value: Option<Rc<dyn IDockBusyService>>) {
        self.set_value(Self::busy_service_property(), value);
    }
    /// Gets GlobalBusyService.
    pub fn global_busy_service(&self) -> Option<Rc<dyn IDockGlobalBusyService>> {
        self.get_value(Self::global_busy_service_property())
    }
    /// Sets GlobalBusyService.
    pub fn set_global_busy_service(&self, value: Option<Rc<dyn IDockGlobalBusyService>>) {
        self.set_value(Self::global_busy_service_property(), value);
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
    /// Gets ShowReloadButton.
    pub fn show_reload_button(&self) -> Option<bool> {
        self.get_value(Self::show_reload_button_property())
    }
    /// Sets ShowReloadButton.
    pub fn set_show_reload_button(&self, value: Option<bool>) {
        self.set_value(Self::show_reload_button_property(), value);
    }
    /// Gets IsBusy.
    pub fn is_busy(&self) -> bool {
        self.is_busy.get()
    }
    fn set_is_busy(&self, value: bool) {
        self.set_and_raise_cell(Self::is_busy_property(), &self.is_busy, value);
    }
    /// Gets BusyMessage.
    pub fn busy_message(&self) -> Option<String> {
        self.busy_message.borrow().clone()
    }
    fn set_busy_message(&self, value: Option<String>) {
        self.set_and_raise(Self::busy_message_property(), &self.busy_message, value);
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
    /// Gets ShowProgress.
    pub fn show_progress(&self) -> bool {
        self.show_progress.get()
    }
    fn set_show_progress(&self, value: bool) {
        self.set_and_raise_cell(Self::show_progress_property(), &self.show_progress, value);
    }
    /// Gets IsReloadVisible.
    pub fn is_reload_visible(&self) -> bool {
        self.is_reload_visible.get()
    }
    fn set_is_reload_visible(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_reload_visible_property(),
            &self.is_reload_visible,
            value,
        );
    }
    /// Gets IsReloadEnabled.
    pub fn is_reload_enabled(&self) -> bool {
        self.is_reload_enabled.get()
    }
    fn set_is_reload_enabled(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_reload_enabled_property(),
            &self.is_reload_enabled,
            value,
        );
    }
    /// Gets ReloadCommand.
    pub fn reload_command(&self) -> Option<Rc<dyn ICommand>> {
        self.reload_command.borrow().clone()
    }
    fn set_reload_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_and_raise(Self::reload_command_property(), &self.reload_command, value);
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
        self.attach_local(self.busy_service());
        self.attach_global(self.global_busy_service());
        self.sync_from_services();
    }
    fn detach_services(&self) {
        self.local.detach();
        self.global.detach();
    }
    fn attach_local(&self, service: Option<Rc<dyn IDockBusyService>>) {
        let weak = self.to_ref().downgrade();
        self.local.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "IsBusy" | "Message" | "IsReloadVisible" | "CanReload") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn attach_global(&self, service: Option<Rc<dyn IDockGlobalBusyService>>) {
        let weak = self.to_ref().downgrade();
        self.global.attach(
            service,
            Rc::new(move |name| {
                if matches!(name, "IsBusy" | "Message") {
                    if let Some(control) = weak.upgrade() {
                        control.sync_from_services();
                    }
                }
            }),
        );
    }
    fn sync_from_services(&self) {
        let local_busy = self.local.current().is_some_and(|s| s.is_busy());
        let global_busy = self.global.current().is_some_and(|s| s.is_busy());
        let is_busy = local_busy || global_busy;
        let has_reload = self.local.current().map(|s| s.reload_command()).is_some();
        let show_reload = self
            .show_reload_button()
            .unwrap_or_else(|| self.local.current().is_some_and(|s| s.is_reload_visible()));
        self.set_is_busy(is_busy);
        self.set_show_progress(local_busy);
        self.set_busy_message(if local_busy {
            self.local.current().and_then(|s| s.message())
        } else {
            self.global.current().and_then(|s| s.message())
        });
        self.set_is_content_enabled(!(is_busy && self.blocks_input()));
        self.set_reload_command(self.local.current().map(|s| s.reload_command()));
        self.set_is_reload_visible(show_reload && has_reload);
        self.set_is_reload_enabled(
            !is_busy && has_reload && self.local.current().is_some_and(|s| s.can_reload()),
        );
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
impl VisualImpl for BusyOverlayControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.on_attached_to_visual_tree();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.on_detached_from_visual_tree();
    }
}
struct ControlHandle(Ref<BusyOverlayControl>);
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
