// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::i_overlay_content_host::content_host;
use super::*;
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{ContentControl, Control, ControlImpl, Decorator, Grid};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type LayerSubscription = (Ref<FerroObject>, Rc<dyn IDisposable>);

/// Composes the complete source overlay pipeline around hosted content.
#[repr(C)]
pub struct OverlayHost {
    base: Decorator,
    overlays: RefCell<OverlayControlList>,
    layers: RefCell<OverlayLayerCollection>,
    service_layers: RefCell<Option<OverlayLayerCollection>>,
    attached: Cell<bool>,
    rebuilding: Cell<bool>,
    requested: Cell<bool>,
    scheduled: Cell<bool>,
    overlays_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    layers_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    services_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    layer_subscriptions: RefCell<Vec<LayerSubscription>>,
    registry_subscription: Cell<Option<u64>>,
}
ferro_class!(OverlayHost:Decorator);
ferro_class_info!(OverlayHost {new:OverlayHost::new,markup:{content:Content,property_attributes:[Content:[DependsOn("ContentTemplate")]]}});
ferro_impl_classes!(OverlayHost:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl OverlayHost {
    /// Identifies hosted content and the markup content property.
    pub fn content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<OverlayHost,_>("Content",None)}
    /// Identifies the hosted content template.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<OverlayHost,_>("ContentTemplate",None)}
    /// Identifies the source overlay-control IList.
    pub fn overlays_property()->StyledProperty<Option<OverlayControlList>>{FerroProperty::register::<OverlayHost,_>("Overlays",None)}
    /// Identifies explicit layer metadata.
    pub fn overlay_layers_property()->StyledProperty<Option<OverlayLayerCollection>>{FerroProperty::register::<OverlayHost,_>("OverlayLayers",None)}
    /// Identifies service-layer usage, true by default.
    pub fn use_service_layers_property()->StyledProperty<bool>{FerroProperty::register::<OverlayHost,_>("UseServiceLayers",true)}
}}
impl OverlayHost {
    fn static_constructor() {
        ValueTypes::register_nullable::<OverlayControlList>();
        ValueTypes::register_nullable::<OverlayLayerCollection>();
        Self::content_property()
            .changed()
            .add_class_handler::<Self>(|h, _| h.request_rebuild());
        Self::content_template_property()
            .changed()
            .add_class_handler::<Self>(|h, _| h.request_rebuild());
        Self::overlays_property()
            .changed()
            .add_class_handler::<Self>(|h, e| {
                h.on_overlays_changed(
                    e.new_value()
                        .downcast_ref::<Option<OverlayControlList>>()
                        .cloned()
                        .flatten(),
                )
            });
        Self::overlay_layers_property()
            .changed()
            .add_class_handler::<Self>(|h, e| {
                h.on_layers_changed(
                    e.new_value()
                        .downcast_ref::<Option<OverlayLayerCollection>>()
                        .cloned()
                        .flatten(),
                )
            });
        Self::use_service_layers_property()
            .changed()
            .add_class_handler::<Self>(|h, _| {
                h.reset_service_layers();
                h.request_rebuild();
            });
    }
    /// Constructs native source state for this class.
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            overlays: RefCell::new(OverlayControlList::plain(Vec::new())),
            layers: RefCell::new(OverlayLayerCollection::new()),
            service_layers: RefCell::new(None),
            attached: Cell::new(false),
            rebuilding: Cell::new(false),
            requested: Cell::new(false),
            scheduled: Cell::new(false),
            overlays_subscription: RefCell::new(None),
            layers_subscription: RefCell::new(None),
            services_subscription: RefCell::new(None),
            layer_subscriptions: RefCell::new(Vec::new()),
            registry_subscription: Cell::new(None),
        }
    }
    /// Creates the host with a fresh native overlay list.
    pub fn new() -> Ref<Self> {
        let host = instantiate(Self::construct());
        host.set_overlays(Some(OverlayControlList::new()));
        host
    }
    /// Gets hosted content.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }
    /// Sets hosted content.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value);
    }
    /// Gets its template.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }
    /// Sets its template.
    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value);
    }
    /// Gets the source list property, including a late-null assignment.
    pub fn overlays(&self) -> Option<OverlayControlList> {
        self.get_value(Self::overlays_property())
    }
    /// Sets the source list, retaining custom list and notification behavior.
    pub fn set_overlays(&self, value: Option<OverlayControlList>) {
        self.set_value(Self::overlays_property(), value);
    }
    /// Gets explicit layers, falling back to the currently attached internal list.
    pub fn overlay_layers(&self) -> OverlayLayerCollection {
        self.get_value(Self::overlay_layers_property())
            .unwrap_or_else(|| self.layers.borrow().clone())
    }
    /// Sets the layer collection (a structurally equivalent value retains the current pipeline).
    pub fn set_overlay_layers(&self, value: Option<OverlayLayerCollection>) {
        self.set_value(Self::overlay_layers_property(), value);
    }
    /// Gets whether service providers are consulted.
    pub fn use_service_layers(&self) -> bool {
        self.get_value(Self::use_service_layers_property())
    }
    /// Sets service-provider usage.
    pub fn set_use_service_layers(&self, value: bool) {
        self.set_value(Self::use_service_layers_property(), value);
    }
    fn dispose_slot(slot: &RefCell<Option<Rc<dyn IDisposable>>>) {
        let old = slot.take();
        if let Some(old) = old {
            old.dispose();
        }
    }
    fn on_overlays_changed(&self, value: Option<OverlayControlList>) {
        Self::dispose_slot(&self.overlays_subscription);
        self.overlays
            .replace(value.unwrap_or_else(|| OverlayControlList::plain(Vec::new())));
        if self.attached.get() {
            self.subscribe_overlays();
        }
        self.request_rebuild();
    }
    fn on_layers_changed(&self, value: Option<OverlayLayerCollection>) {
        let new = value.unwrap_or_default();
        let old = self.layers.borrow().clone();
        if Self::structurally_equivalent(&old, &new) {
            return;
        }
        Self::dispose_slot(&self.layers_subscription);
        let current = self.layers.borrow().clone();
        self.unsubscribe_layer_changes(current.snapshot().iter().cloned());
        self.layers.replace(new.clone());
        if self.attached.get() {
            self.subscribe_layer_changes(new.snapshot().iter().cloned());
            self.subscribe_collection(&new, false);
        }
        self.request_rebuild();
    }
    fn structurally_equivalent(old: &OverlayLayerCollection, new: &OverlayLayerCollection) -> bool {
        if old == new {
            return true;
        }
        if old.count() != new.count() {
            return false;
        }
        for i in 0..old.count() {
            let a = old.get(i);
            let b = new.get(i);
            match (a, b) {
                (None, None) => continue,
                (Some(a), Some(b)) => {
                    if a.z_index() != b.z_index()
                        || a.is_visible() != b.is_visible()
                        || a.blocks_input() != b.blocks_input()
                        || !ferroui_controls::items_equal(&a.style_key(), &b.style_key())
                    {
                        return false;
                    }
                    let a = a.overlay();
                    let b = b.overlay();
                    if a == b {
                        continue;
                    }
                    match (a, b) {
                        (Some(a), Some(b)) if std::ptr::eq(a.get_type(), b.get_type()) => (),
                        _ => return false,
                    }
                }
                _ => return false,
            }
        }
        true
    }
    fn subscribe_overlays(&self) {
        let weak = self.to_ref().downgrade();
        let overlays = self.overlays.borrow().clone();
        let sub = overlays.subscribe(Rc::new(move || {
            if let Some(host) = weak.upgrade() {
                host.request_rebuild();
            }
        }));
        self.overlays_subscription.replace(sub);
    }
    fn subscribe_collection(&self, list: &OverlayLayerCollection, service: bool) {
        let weak = self.to_ref().downgrade();
        let token = list.add_collection_changed(Rc::new(move |e| {
            if let Some(host) = weak.upgrade() {
                host.on_layers_collection_changed(e);
            }
        }));
        let list = list.clone();
        let sub = ferroui_base::reactive::Disposable::create(move || {
            list.remove_collection_changed(token);
        });
        if service {
            self.services_subscription.replace(Some(sub));
        } else {
            self.layers_subscription.replace(Some(sub));
        }
    }
    fn on_layers_collection_changed(
        &self,
        e: &NotifyCollectionChangedEventArgs<'_, Option<Rc<dyn IOverlayLayer>>>,
    ) {
        self.unsubscribe_layer_changes(e.old_items.iter().cloned());
        self.subscribe_layer_changes(e.new_items.iter().cloned());
        self.request_rebuild();
    }
    fn subscribe_layer_changes(
        &self,
        layers: impl IntoIterator<Item = Option<Rc<dyn IOverlayLayer>>>,
    ) {
        for layer in layers.into_iter().flatten() {
            if let Some(object) = layer.as_object() {
                let weak = self.to_ref().downgrade();
                let sub = object.property_changed(move |_| {
                    if let Some(host) = weak.upgrade() {
                        host.request_rebuild();
                    }
                });
                self.layer_subscriptions.borrow_mut().push((object, sub));
            }
        }
    }
    fn unsubscribe_layer_changes(
        &self,
        layers: impl IntoIterator<Item = Option<Rc<dyn IOverlayLayer>>>,
    ) {
        for layer in layers.into_iter().flatten() {
            if let Some(object) = layer.as_object() {
                let sub = {
                    let mut subs = self.layer_subscriptions.borrow_mut();
                    subs.iter()
                        .rposition(|(o, _)| o.ptr_eq(&object))
                        .map(|i| subs.remove(i).1)
                };
                if let Some(sub) = sub {
                    sub.dispose();
                }
            }
        }
    }
    fn request_rebuild(&self) {
        if !self.attached.get() {
            return;
        }
        if self.rebuilding.get() {
            self.requested.set(true);
            return;
        }
        if self.scheduled.replace(true) {
            return;
        }
        let host = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                host.scheduled.set(false);
                if host.attached.get() {
                    host.rebuild_pipeline_core();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
    fn rebuild_pipeline_core(&self) {
        if self.rebuilding.replace(true) {
            self.requested.set(true);
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.rebuilding);
        loop {
            self.requested.set(false);
            self.rebuild_pipeline();
            if !self.requested.get() {
                break;
            }
        }
    }
    fn rebuild_pipeline(&self) {
        if let Some(content) = self.content().as_ref().and_then(Control::from_boxed) {
            let parent = content
                .parent()
                .map(|p| p.upcast::<FerroObject>())
                .or_else(|| content.visual_parent().map(|p| p.upcast()));
            if parent.is_some() && !Self::try_detach_control(&content) {
                return;
            }
        }
        self.set_child(None::<Ref<Control>>);
        let presenter = ContentPresenter::new();
        presenter.set_content(self.content());
        presenter.set_content_template(self.content_template());
        let mut current = presenter.upcast::<Control>();
        self.for_each_entry(|entry| {
            let Some(overlay) = entry.overlay else {
                return;
            };
            if !Self::try_detach_control(&overlay) {
                return;
            }
            self.apply_style_key(entry.style_key.as_ref(), &overlay);
            if let Some(host) = content_host(&overlay) {
                host.set_content(Some(Control::boxed(current.clone())));
                host.set_content_template(None);
                host.set_blocks_input(entry.blocks_input);
                current = overlay;
                return;
            }
            if let Some(content) = overlay.cast::<ContentControl>() {
                content.set_content(Some(Control::boxed(current.clone())));
                content.set_content_template(None);
                current = overlay;
                return;
            }
            if let Some(decorator) = overlay.cast::<Decorator>() {
                decorator.set_child(current.clone());
                current = decorator.upcast();
                return;
            }
            overlay.set_is_hit_test_visible(entry.blocks_input);
            let grid = Grid::new();
            grid.children().add(current.clone());
            if entry.z_index != 0 {
                overlay.set_z_index(entry.z_index);
            }
            grid.children().add(overlay);
            current = grid.upcast();
        });
        self.set_child(current);
    }
    fn try_detach_control(control: &Control) -> bool {
        let parent = control
            .parent()
            .map(|p| p.upcast::<FerroObject>())
            .or_else(|| control.visual_parent().map(|p| p.upcast()));
        let Some(parent) = parent else {
            return true;
        };
        if let Some(panel) = parent.cast::<ferroui_controls::Panel>() {
            panel.children().remove(control.to_ref());
            return true;
        }
        if let Some(decorator) = parent.cast::<Decorator>() {
            if decorator.child().as_ref() == Some(&control.to_ref()) {
                decorator.set_child(None::<Ref<Control>>);
                return true;
            }
        }
        if let Some(presenter) = parent.cast::<ContentPresenter>() {
            if presenter.child().as_ref() == Some(&control.to_ref()) {
                presenter.set_current_value(ContentPresenter::content_property(), None);
                presenter.update_child();
                return control.visual_parent().is_none();
            }
            if presenter
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .as_ref()
                == Some(&control.to_ref())
            {
                presenter.set_current_value(ContentPresenter::content_property(), None);
                presenter.update_child();
                return true;
            }
            return false;
        }
        if let Some(content) = parent.cast::<ContentControl>() {
            if content
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .as_ref()
                == Some(&control.to_ref())
            {
                content.set_content(None);
                return true;
            }
        }
        false
    }
    fn for_each_entry(&self, mut visit: impl FnMut(Entry)) {
        let service = self.ensure_service_layers();
        let ordered = if let Some(service) = service.filter(|l| l.count() > 0) {
            Some(service.get_ordered_layers())
        } else {
            let layers = self.layers.borrow().clone();
            if layers.count() > 0 {
                Some(layers.get_ordered_layers())
            } else {
                None
            }
        };
        if let Some(ordered) = ordered {
            for layer in ordered {
                if !layer.is_visible() || layer.overlay().is_none() {
                    continue;
                }
                visit(Entry {
                    overlay: layer.overlay(),
                    z_index: layer.z_index(),
                    _is_visible: layer.is_visible(),
                    blocks_input: layer.blocks_input(),
                    style_key: layer.style_key(),
                });
            }
            return;
        }
        let overlays = self.overlays.borrow().clone();
        for overlay in overlays.iter().flatten() {
            visit(Entry {
                overlay: Some(overlay),
                z_index: 0,
                _is_visible: true,
                blocks_input: true,
                style_key: None,
            });
        }
    }
    fn ensure_service_layers(&self) -> Option<OverlayLayerCollection> {
        if !self.use_service_layers() {
            return None;
        }
        if let Some(layers) = self.service_layers.borrow().clone() {
            return Some(layers);
        }
        if let Some(provider) = OverlayLayerRegistry::factory_provider() {
            let factories = provider()
                .map(|s| (s.0)().flatten().collect::<Vec<_>>())
                .unwrap_or_default();
            if !factories.is_empty() {
                let created = OverlayLayerCollection::new();
                for factory in factories {
                    if let Some(layer) = factory.create() {
                        created.add(Some(layer));
                    }
                }
                if created.count() > 0 {
                    self.attach_service_layers(&created);
                    self.service_layers.replace(Some(created));
                    return self.service_layers.borrow().clone();
                }
            }
        }
        let provider = OverlayLayerRegistry::provider()?;
        let layers = provider();
        if let Some(OverlayLayerSource::Collection(collection)) = &layers {
            if collection.count() > 0 {
                self.attach_service_layers(collection);
                self.service_layers.replace(Some(collection.clone()));
                return self.service_layers.borrow().clone();
            }
        }
        let layers = layers
            .map(|s| s.iter().flatten().collect::<Vec<_>>())
            .unwrap_or_default();
        if layers.is_empty() {
            return None;
        }
        let created = OverlayLayerCollection::new();
        for layer in layers {
            created.add(Some(layer));
        }
        self.attach_service_layers(&created);
        self.service_layers.replace(Some(created));
        self.service_layers.borrow().clone()
    }
    fn attach_service_layers(&self, layers: &OverlayLayerCollection) {
        self.subscribe_collection(layers, true);
        self.subscribe_layer_changes(layers.snapshot().iter().cloned());
    }
    fn reset_service_layers(&self) {
        let layers = self.service_layers.borrow().clone();
        if let Some(layers) = layers {
            Self::dispose_slot(&self.services_subscription);
            self.unsubscribe_layer_changes(layers.snapshot().iter().cloned());
            self.service_layers.replace(None);
        }
    }
    fn apply_style_key(&self, key: Option<&BoxedValue>, overlay: &Control) {
        let Some(key) = key else {
            return;
        };
        let key = if let Some(key) = key.downcast_ref::<ResourceKey>() {
            key.clone()
        } else if let Some(text) = key.downcast_ref::<String>() {
            ResourceKey::from(text.as_str())
        } else if let Some(text) = key.downcast_ref::<&'static str>() {
            ResourceKey::from(*text)
        } else if let Some(kind) = key.downcast_ref::<&'static ferroui_base::TypeInfo>() {
            ResourceKey::Type(kind)
        } else if let Some(kind) = key.downcast_ref::<ferroui_base::data::core::ValueType>() {
            let Some((class, _)) = ferroui_base::TypeInfo::find_by_handle(kind.id()) else {
                return;
            };
            ResourceKey::Type(class)
        } else if let Some(variant) = key.downcast_ref::<ferroui_base::styling::ThemeVariant>() {
            variant.key().clone()
        } else {
            return;
        };
        if let Some(Some(value)) = self.try_find_resource(&key, None) {
            if let Some(theme) =
                ValueTypes::as_object(&*value).and_then(|o| o.cast::<ControlTheme>())
            {
                overlay.set_theme(theme);
            }
        }
    }
}
struct Entry {
    // Preserve the source getter call even though pipeline composition ignores it.
    _is_visible: bool,
    overlay: Option<Ref<Control>>,
    z_index: i32,
    blocks_input: bool,
    style_key: Option<BoxedValue>,
}
impl VisualImpl for OverlayHost {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(true);
        this.subscribe_overlays();
        let layers = this.layers.borrow().clone();
        this.subscribe_collection(&layers, false);
        this.subscribe_layer_changes(layers.snapshot().iter().cloned());
        let weak = this.to_ref().downgrade();
        let token = OverlayLayerRegistry::provider_changed().add(Rc::new(move |_| {
            if let Some(host) = weak.upgrade() {
                if host.attached.get() {
                    host.reset_service_layers();
                    host.request_rebuild();
                }
            }
        }));
        this.registry_subscription.set(Some(token));
        this.reset_service_layers();
        this.rebuild_pipeline_core();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.attached.set(false);
        if let Some(token) = this.registry_subscription.take() {
            OverlayLayerRegistry::provider_changed().remove(token);
        }
        Self::dispose_slot(&this.overlays_subscription);
        Self::dispose_slot(&this.layers_subscription);
        let layers = this.layers.borrow().clone();
        this.unsubscribe_layer_changes(layers.snapshot().iter().cloned());
        this.reset_service_layers();
    }
}
