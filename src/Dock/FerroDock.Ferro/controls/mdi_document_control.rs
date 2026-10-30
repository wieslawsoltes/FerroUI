// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    automation::peers::MdiDocumentControlAutomationPeer,
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager},
    model_projection,
};
use ferrodock_model::{
    collections::ListHandle,
    core::{IDock, IDockable},
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    reactive::{AnonymousObserver, IDisposable},
    styling::ControlTheme,
    BoxedValue, DirectProperty, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{TemplatedControl, TemplatedControlImpl},
    templates::IDataTemplate,
    ControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Subscription(Box<dyn Fn()>);
impl Subscription {
    fn dispose(self) {
        (self.0)()
    }
}
/// Original MDI document host with source dock/collection subscriptions and Z ordering.
#[repr(C)]
pub struct MdiDocumentControl {
    base: TemplatedControl,
    dock_subscription: RefCell<Option<Subscription>>,
    dockables_subscription: RefCell<Option<Subscription>>,
    current_dock: RefCell<Option<Rc<dyn IDock>>>,
    data_context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    has_visible_documents: Cell<bool>,
}
ferro_class!(MdiDocumentControl:TemplatedControl);
ferro_class_info!(MdiDocumentControl{new:MdiDocumentControl::new,markup:{attributes:[PseudoClasses(":active")]}});
ferro_impl_classes!(MdiDocumentControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl FerroObjectImpl for MdiDocumentControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.is_active());
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_active_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<bool>());
        }
    }
}
impl ControlImpl for MdiDocumentControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MdiDocumentControlAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for MdiDocumentControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, StyledElement::data_context_property()).subscribe(
                Rc::new(AnonymousObserver::new(move |v| owner.on_dock_changed(v))),
            );
        this.data_context_subscription.replace(Some(subscription));
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let subscription = this.data_context_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        this.detach_dock_subscriptions();
    }
}
ferro_properties! {impl MdiDocumentControl{
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("ModifiedTemplate",None)}
    /// Identifies CloseTemplate.
    pub fn close_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("CloseTemplate",None)}
    /// Identifies EmptyContentTemplate.
    pub fn empty_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("EmptyContentTemplate",None)}
    /// Identifies CloseButtonTheme.
    pub fn close_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>>{FerroProperty::register::<Self,_>("CloseButtonTheme",None)}
    /// Identifies LayoutManager, with the original shared classic default.
    pub fn layout_manager_property()->StyledProperty<Option<Rc<dyn IMdiLayoutManager>>>{FerroProperty::register::<Self,_>("LayoutManager",Some(ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>))}
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsActive",false)}
    /// Identifies the original read-only HasVisibleDocuments direct property.
    pub fn has_visible_documents_property()->DirectProperty<MdiDocumentControl,bool>{FerroProperty::register_direct::<Self,_>("HasVisibleDocuments",Self::has_visible_documents,None,false)}
}}
impl MdiDocumentControl {
    /// Constructs source field state for this or a derived native control.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            dock_subscription: RefCell::new(None),
            dockables_subscription: RefCell::new(None),
            current_dock: RefCell::new(None),
            data_context_subscription: RefCell::new(None),
            has_visible_documents: Cell::new(false),
        }
    }
    /// Creates the original native MDI document host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Whether the current dock contains at least one MDI document.
    pub fn has_visible_documents(&self) -> bool {
        self.has_visible_documents.get()
    }
    fn set_has_visible_documents(&self, value: bool) {
        self.set_and_raise_cell(
            Self::has_visible_documents_property(),
            &self.has_visible_documents,
            value,
        );
    }
    fn dock(&self) -> Option<Rc<dyn IDock>> {
        self.current_dock.borrow().clone()
    }
    fn update_pseudo_classes(&self, active: bool) {
        self.pseudo_classes().set(":active", active);
    }
    fn on_dock_changed(&self, value: Option<BoxedValue>) {
        self.detach_dock_subscriptions();
        let dock = model_projection::dockable(value.as_ref()).and_then(|d| d.as_dock());
        self.current_dock.replace(dock.clone());
        let Some(dock) = dock else {
            self.set_has_visible_documents(false);
            return;
        };
        if let Some(notifier) = dock.as_notify_property_changed() {
            let owner = self.to_ref();
            let token = notifier
                .property_changed()
                .add(Rc::new(move |name| owner.dock_property_changed(name)));
            self.dock_subscription
                .replace(Some(Subscription(Box::new(move || {
                    if let Some(n) = dock.as_notify_property_changed() {
                        n.property_changed().remove(token);
                    }
                }))));
        }
        self.attach_dockables_collection(
            self.dock()
                .expect("source current dock")
                .visible_dockables(),
        );
        self.update_has_visible_documents();
        self.update_z_order();
    }
    fn dock_property_changed(&self, name: &str) {
        if self.dock().is_none() {
            return;
        }
        if name == "VisibleDockables" {
            self.attach_dockables_collection(
                self.dock()
                    .expect("source current dock")
                    .visible_dockables(),
            );
            self.update_has_visible_documents();
            self.update_z_order();
            return;
        }
        if name == "ActiveDockable" {
            self.update_z_order();
        }
    }
    fn attach_dockables_collection(&self, list: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        let sub = self.dockables_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let Some(list) = list else {
            return;
        };
        let Some(notifier) = list.as_notify_collection_changed() else {
            return;
        };
        let owner = self.to_ref();
        let token = notifier
            .collection_changed()
            .add(Rc::new(move |_| owner.dockables_collection_changed()));
        self.dockables_subscription
            .replace(Some(Subscription(Box::new(move || {
                if let Some(n) = list.as_notify_collection_changed() {
                    n.collection_changed().remove(token);
                }
            }))));
    }
    fn dockables_collection_changed(&self) {
        self.update_has_visible_documents();
        self.update_z_order();
    }
    fn update_has_visible_documents(&self) {
        if self.dock().and_then(|d| d.visible_dockables()).is_none() {
            self.set_has_visible_documents(false);
            return;
        }
        let visible = self
            .dock()
            .expect("source current dock")
            .visible_dockables()
            .expect("source visible dockables");
        let has_visible = visible
            .iter()
            .any(|d| d.and_then(|item| item.as_mdi_document()).is_some());
        self.set_has_visible_documents(has_visible);
    }
    fn update_z_order(&self) {
        if self.dock().and_then(|d| d.visible_dockables()).is_none() {
            return;
        }
        let visible = self
            .dock()
            .expect("source current dock")
            .visible_dockables()
            .expect("source visible dockables");
        let mut documents = Vec::new();
        for d in visible.iter() {
            if let Some(d) = d.and_then(|item| item.as_mdi_document()) {
                documents.push(d);
            }
        }
        let manager = self
            .layout_manager()
            .unwrap_or_else(|| ClassicMdiLayoutManager::instance());
        manager.update_z_order(
            &ListHandle::from_plain(documents),
            self.dock()
                .expect("source current dock")
                .active_dockable()
                .and_then(|d| d.as_mdi_document()),
        );
    }
    fn detach_dock_subscriptions(&self) {
        let sub = self.dock_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let sub = self.dockables_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        self.current_dock.replace(None);
        self.set_has_visible_documents(false);
    }
}

impl MdiDocumentControl {
    /// Gets IconTemplate.
    pub fn icon_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_template_property())
    }
    /// Sets IconTemplate.
    pub fn set_icon_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_template_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets HeaderTemplate.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    /// Sets HeaderTemplate.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets ModifiedTemplate.
    pub fn modified_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::modified_template_property())
    }
    /// Sets ModifiedTemplate.
    pub fn set_modified_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::modified_template_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets CloseTemplate.
    pub fn close_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::close_template_property())
    }
    /// Sets CloseTemplate.
    pub fn set_close_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::close_template_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets EmptyContentTemplate.
    pub fn empty_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::empty_content_template_property())
    }
    /// Sets EmptyContentTemplate.
    pub fn set_empty_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::empty_content_template_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets CloseButtonTheme.
    pub fn close_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_theme_property())
    }
    /// Sets CloseButtonTheme.
    pub fn set_close_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_theme_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets LayoutManager.
    pub fn layout_manager(&self) -> Option<Rc<dyn IMdiLayoutManager>> {
        self.get_value(Self::layout_manager_property())
    }
    /// Sets LayoutManager.
    pub fn set_layout_manager(&self, value: Option<Rc<dyn IMdiLayoutManager>>) {
        self.set_value(Self::layout_manager_property(), value);
    }
}

impl MdiDocumentControl {
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
}

#[cfg(test)]
#[path = "mdi_document_control_tests.rs"]
mod tests;
