// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{automation::peers::DocumentControlAutomationPeer, model_projection};
use ferrodock_model::{
    collections::ListHandle,
    core::{DocumentTabLayout, IDock, IDockable},
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl},
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::LayoutableImpl,
    reactive::IDisposable,
    BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_base::{
    reactive::AnonymousObserver, styling::ControlTheme, DirectProperty, FerroObject,
    FerroObjectExtensions, FerroObjectImplExt, FerroPropertyChangedEventArgs, StyledElement,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{TemplatedControl, TemplatedControlImpl},
    templates::IDataTemplate,
    ControlImpl,
};
use std::cell::Cell;
use std::{cell::RefCell, rc::Rc};
struct Subscription(Box<dyn Fn()>);
impl Subscription {
    fn dispose(self) {
        (self.0)()
    }
}
/// Original native document host.
#[repr(C)]
pub struct DocumentControl {
    base: TemplatedControl,
    pressed: RefCell<Option<Rc<dyn IDisposable>>>,
    dock_subscription: RefCell<Option<Subscription>>,
    dockables_subscription: RefCell<Option<Subscription>>,
    current_dock: RefCell<Option<Rc<dyn IDock>>>,
    data_context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    has_visible_dockables: Cell<bool>,
}
ferro_class!(DocumentControl:TemplatedControl);
ferro_class_info!(DocumentControl {
    new: DocumentControl::new,
    markup:{attributes:[PseudoClasses(":active")]},
});
ferro_impl_classes!(DocumentControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl ControlImpl for DocumentControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DocumentControlAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl DocumentControl {
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("ModifiedTemplate",None)}
    /// Identifies CloseTemplate.
    pub fn close_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("CloseTemplate",None)}
    /// Identifies EmptyContentTemplate.
    pub fn empty_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("EmptyContentTemplate",None)}
    /// Identifies CloseButtonTheme.
    pub fn close_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("CloseButtonTheme",None)}
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsActive",false)}
    /// Identifies TabsLayout.
    pub fn tabs_layout_property()->StyledProperty<DocumentTabLayout> {FerroProperty::register::<Self,DocumentTabLayout>("TabsLayout",DocumentTabLayout::Top)}
    /// Identifies source read-only visible content state.
    pub fn has_visible_dockables_property()->DirectProperty<DocumentControl,bool> {FerroProperty::register_direct::<Self,_>("HasVisibleDockables",|this|this.has_visible_dockables(),None,false)}
}}
impl VisualImpl for DocumentControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let pressed = this.add_disposable_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, _| owner.pressed_handler(),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.pressed.replace(Some(pressed));
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, StyledElement::data_context_property()).subscribe(
                Rc::new(AnonymousObserver::new(move |value: Option<BoxedValue>| {
                    owner.on_dock_changed(value.as_ref())
                })),
            );
        this.data_context_subscription.replace(Some(subscription));
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let pressed = this.pressed.borrow().clone();
        if let Some(pressed) = pressed {
            pressed.dispose();
        }
        let context = this.data_context_subscription.borrow().clone();
        if let Some(context) = context {
            context.dispose();
        }
        this.data_context_subscription.replace(None);
        this.detach_dock_subscriptions();
    }
}
impl FerroObjectImpl for DocumentControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if std::ptr::eq(change.property(), Self::is_active_property().as_property()) {
            this.update_pseudo_classes(this.is_active());
        }
    }
}
impl DocumentControl {
    /// Constructs original state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            pressed: RefCell::new(None),
            dock_subscription: RefCell::new(None),
            dockables_subscription: RefCell::new(None),
            current_dock: RefCell::new(None),
            data_context_subscription: RefCell::new(None),
            has_visible_dockables: Cell::new(false),
        }
    }
    /// Creates the original native host.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.update_pseudo_classes(this.is_active());
        this
    }
    /// Gets IconTemplate.
    pub fn icon_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_template_property())
    }
    /// Sets IconTemplate.
    pub fn set_icon_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_template_property(), value);
    }
    /// Gets HeaderTemplate.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    /// Sets HeaderTemplate.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    /// Gets ModifiedTemplate.
    pub fn modified_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::modified_template_property())
    }
    /// Sets ModifiedTemplate.
    pub fn set_modified_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::modified_template_property(), value);
    }
    /// Gets CloseTemplate.
    pub fn close_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::close_template_property())
    }
    /// Sets CloseTemplate.
    pub fn set_close_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::close_template_property(), value);
    }
    /// Gets EmptyContentTemplate.
    pub fn empty_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::empty_content_template_property())
    }
    /// Sets EmptyContentTemplate.
    pub fn set_empty_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::empty_content_template_property(), value);
    }
    /// Gets CloseButtonTheme.
    pub fn close_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_theme_property())
    }
    /// Sets CloseButtonTheme.
    pub fn set_close_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_theme_property(), value);
    }
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
    /// Gets TabsLayout.
    pub fn tabs_layout(&self) -> DocumentTabLayout {
        self.get_value(Self::tabs_layout_property())
    }
    /// Sets TabsLayout.
    pub fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.set_value(Self::tabs_layout_property(), value);
    }
    fn pressed_handler(&self) {
        if let Some(dock) =
            model_projection::dockable(self.data_context().as_ref()).and_then(|d| d.as_dock())
        {
            if let Some(factory) = dock.factory() {
                if dock.active_dockable().is_some() {
                    if let Some(root) = factory.find_root(
                        Some(
                            dock.active_dockable()
                                .expect("source current active dockable"),
                        ),
                        Some(Rc::new(|_| true)),
                    ) {
                        factory.set_focused_dockable(Some(root), dock.active_dockable());
                    }
                }
            }
        }
    }
    /// Gets whether the current host has visible dockables.
    pub fn has_visible_dockables(&self) -> bool {
        self.has_visible_dockables.get()
    }
    fn set_has_visible_dockables(&self, value: bool) {
        self.set_and_raise_cell(
            Self::has_visible_dockables_property(),
            &self.has_visible_dockables,
            value,
        );
    }
    fn update_pseudo_classes(&self, active: bool) {
        self.pseudo_classes().set(":active", active);
    }
    fn current_dock(&self) -> Option<Rc<dyn IDock>> {
        self.current_dock.borrow().clone()
    }
    fn on_dock_changed(&self, value: Option<&BoxedValue>) {
        self.detach_dock_subscriptions();
        self.current_dock
            .replace(model_projection::dockable(value).and_then(|d| d.as_dock()));
        let Some(dock) = self.current_dock() else {
            self.set_has_visible_dockables(false);
            return;
        };
        if let Some(notifier) = dock.as_notify_property_changed() {
            let owner = self.to_ref();
            let token = notifier
                .property_changed()
                .add(Rc::new(move |name| owner.dock_property_changed(name)));
            let original = dock.clone();
            self.dock_subscription
                .replace(Some(Subscription(Box::new(move || {
                    if let Some(notifier) = original.as_notify_property_changed() {
                        notifier.property_changed().remove(token);
                    }
                }))));
        }
        let list = self
            .current_dock()
            .expect("source current dock")
            .visible_dockables();
        self.attach_dockables_collection(list);
        self.update_has_visible_dockables();
    }
    fn dock_property_changed(&self, name: &str) {
        if self.current_dock.borrow().is_none() {
            return;
        }
        if name != "VisibleDockables" {
            return;
        }
        let list = self
            .current_dock()
            .expect("source current dock")
            .visible_dockables();
        self.attach_dockables_collection(list);
        self.update_has_visible_dockables();
    }
    fn attach_dockables_collection(&self, list: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        let old = self.dockables_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
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
            .add(Rc::new(move |_| owner.update_has_visible_dockables()));
        self.dockables_subscription
            .replace(Some(Subscription(Box::new(move || {
                if let Some(notifier) = list.as_notify_collection_changed() {
                    notifier.collection_changed().remove(token);
                }
            }))));
    }
    fn update_has_visible_dockables(&self) {
        let dock = self.current_dock();
        let visible = dock
            .and_then(|d| d.visible_dockables())
            .is_some_and(|list| list.count() > 0);
        self.set_has_visible_dockables(visible);
    }
    fn detach_dock_subscriptions(&self) {
        let old = self.dock_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let old = self.dockables_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        self.current_dock.replace(None);
        self.set_has_visible_dockables(false);
    }
}
