// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    ManagedDockWindowDocument, ManagedDockableBase, ManagedDockableBaseImpl, MdiDocumentWindow,
};
use crate::{
    internal::managed_window_registry::ManagedWindowRegistry,
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager},
};
use ferrodock_model::{
    collections::{ListHandle, ReadOnlyListChanged},
    controls::IMdiDocument,
    core::{IDock, IDockable},
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, input::InputElementImpl,
    instantiate, interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    Canvas, Control, ControlImpl,
};
use ferroui_system::collections::{
    generic::Dictionary,
    specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs},
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
struct OffsetProbeDockable {
    base: ManagedDockableBase,
}
impl ManagedDockableBaseImpl for OffsetProbeDockable {}
crate::impl_managed_dockable!(OffsetProbeDockable);
impl OffsetProbeDockable {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime: &std::rc::Weak<Self>| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
        })
    }
}
/// Original managed floating-window layer, overlay canvas and topmost ordering.
#[repr(C)]
pub struct ManagedWindowLayer {
    base: TemplatedControl,
    overlays: RefCell<Dictionary<String, Ref<Control>>>,
    window_subscriptions: RefCell<Dictionary<Rc<dyn IDockable>, u64>>,
    dock_subscription: RefCell<Option<Subscription>>,
    dockables_subscription: RefCell<Option<Subscription>>,
    overlay_canvas: RefCell<Option<Ref<Canvas>>>,
    cached_window_content_offset: Cell<Option<Point>>,
}
ferro_class!(ManagedWindowLayer:TemplatedControl);
ferro_class_info!(ManagedWindowLayer{new:ManagedWindowLayer::new,markup:{attributes:[TemplatePart("PART_OverlayCanvas",type(Ref<Canvas>))]}});
ferro_impl_classes!(ManagedWindowLayer:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl FerroObjectImpl for ManagedWindowLayer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::dock_property().as_property() {
            this.attach_dock(change.get_new_value::<Option<Rc<dyn IDock>>>());
        }
    }
}
impl TemplatedControlImpl for ManagedWindowLayer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.overlay_canvas
            .replace(e.name_scope().find_as::<Canvas>("PART_OverlayCanvas"));
        this.cached_window_content_offset.set(None);
        this.attach_dock(this.dock());
    }
}
impl VisualImpl for ManagedWindowLayer {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detach_dock();
        if let Some(factory) = this.dock().and_then(|d| d.factory()) {
            ManagedWindowRegistry::unregister_layer(&factory, &this.to_ref());
        }
    }
}
ferro_properties! {impl ManagedWindowLayer {
    /// Identifies the original owning Dock property.
    pub fn dock_property()->StyledProperty<Option<Rc<dyn IDock>>>{FerroProperty::register::<Self,_>("Dock",None)}
    /// Identifies LayoutManager, defaulting to the shared classic manager.
    pub fn layout_manager_property()->StyledProperty<Option<Rc<dyn IMdiLayoutManager>>>{FerroProperty::register::<Self,_>("LayoutManager",Some(ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>))}
}}
impl ManagedWindowLayer {
    /// Constructs the original field state.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            overlays: RefCell::new(Dictionary::new()),
            window_subscriptions: RefCell::new(Dictionary::new()),
            dock_subscription: RefCell::new(None),
            dockables_subscription: RefCell::new(None),
            overlay_canvas: RefCell::new(None),
            cached_window_content_offset: Cell::new(None),
        }
    }
    /// Creates the original native layer.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the owning dock.
    pub fn dock(&self) -> Option<Rc<dyn IDock>> {
        self.get_value(Self::dock_property())
    }
    /// Sets the owning dock, replacing original source subscriptions immediately.
    pub fn set_dock(&self, value: Option<Rc<dyn IDock>>) {
        self.set_value(Self::dock_property(), value);
    }
    /// Gets the original layout manager.
    pub fn layout_manager(&self) -> Option<Rc<dyn IMdiLayoutManager>> {
        self.get_value(Self::layout_manager_property())
    }
    /// Sets the layout manager; source property changes do not independently reorder.
    pub fn set_layout_manager(&self, value: Option<Rc<dyn IMdiLayoutManager>>) {
        self.set_value(Self::layout_manager_property(), value);
    }
    /// Shows or updates the overlay identified by key using the original rectangle overload.
    pub fn show_overlay(
        &self,
        key: &str,
        control: Ref<Control>,
        bounds: Rect,
        hit_test_visible: bool,
    ) {
        self.show_overlay_at(
            key,
            control,
            bounds.position(),
            Some(bounds.size()),
            hit_test_visible,
        );
    }
    /// Shows or updates an overlay; an existing key retains its original control.
    pub fn show_overlay_at(
        &self,
        key: &str,
        control: Ref<Control>,
        position: Point,
        size: Option<Size>,
        hit_test_visible: bool,
    ) {
        if self.overlay_canvas.borrow().is_none() {
            self.apply_template();
        }
        if self.overlay_canvas.borrow().is_none() {
            return;
        }
        let existing = self.overlays.borrow().get(key).cloned();
        let existing = match existing {
            Some(existing) => existing,
            None => {
                self.overlays
                    .borrow_mut()
                    .insert(key.to_owned(), control.clone());
                let canvas = self
                    .overlay_canvas
                    .borrow()
                    .clone()
                    .expect("source overlay canvas");
                canvas.children().add(control.clone());
                control
            }
        };
        existing.set_is_hit_test_visible(hit_test_visible);
        if let Some(size) = size {
            existing.set_width(size.width);
            existing.set_height(size.height);
        } else {
            existing.set_width(f64::NAN);
            existing.set_height(f64::NAN);
        }
        Canvas::set_left(&existing, position.x);
        Canvas::set_top(&existing, position.y);
    }
    /// Removes only the overlay associated with key when the original canvas exists.
    pub fn hide_overlay(&self, key: &str) {
        if self.overlay_canvas.borrow().is_none() {
            return;
        }
        let existing = self.overlays.borrow().get(key).cloned();
        if let Some(existing) = existing {
            let canvas = self
                .overlay_canvas
                .borrow()
                .clone()
                .expect("source overlay canvas");
            canvas.children().remove(&existing);
            self.overlays.borrow_mut().remove(key);
        }
    }
    #[cfg_attr(not(test), allow(dead_code))] // Original drag/preview callers are still being ported.
    pub(crate) fn try_get_window_content_offset(&self) -> Option<Point> {
        if let Some(offset) = self.cached_window_content_offset.get() {
            return Some(offset);
        }
        let window =
            self.find_descendant_of_type_where::<MdiDocumentWindow>(false, |w| w.is_visible());
        if let Some(offset) = window.and_then(|w| w.try_get_content_offset()) {
            self.cached_window_content_offset.set(Some(offset));
            return Some(offset);
        }
        if let Some(offset) = self.try_measure_window_content_offset() {
            self.cached_window_content_offset.set(Some(offset));
            return Some(offset);
        }
        None
    }
    fn try_measure_window_content_offset(&self) -> Option<Point> {
        if self.overlay_canvas.borrow().is_none() {
            self.apply_template();
        }
        self.overlay_canvas.borrow().as_ref()?;
        let probe = MdiDocumentWindow::new();
        probe.set_width(300.0);
        probe.set_height(200.0);
        probe.set_opacity(0.0);
        probe.set_is_hit_test_visible(false);
        let model = OffsetProbeDockable::new();
        model.set_title(String::new());
        probe.set_data_context(Some(crate::model_projection::to_dockable_value(model)));
        self.show_overlay_at(
            "ManagedWindowOffsetProbe",
            probe.clone().upcast(),
            Point::default(),
            Some(Size::new(300.0, 200.0)),
            false,
        );
        probe.apply_template();
        probe.measure(Size::new(300.0, 200.0));
        probe.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
        let measured = probe.try_get_content_offset();
        self.hide_overlay("ManagedWindowOffsetProbe");
        measured
    }
    fn attach_dock(&self, dock: Option<Rc<dyn IDock>>) {
        self.detach_dock();
        let Some(dock) = dock else { return };
        self.set_data_context(Some(crate::model_projection::to_dockable_value(
            dock.clone(),
        )));
        if let Some(notifier) = dock.as_notify_property_changed() {
            let owner = self.to_ref();
            let token = notifier
                .property_changed()
                .add(Rc::new(move |name| owner.dock_property_changed(name)));
            let original = dock.clone();
            self.dock_subscription
                .replace(Some(Subscription(Box::new(move || {
                    if let Some(n) = original.as_notify_property_changed() {
                        n.property_changed().remove(token);
                    }
                }))));
        }
        self.attach_dockables_collection(dock.visible_dockables());
        self.track_dockables(dock.visible_dockables());
        self.update_z_order();
    }
    fn detach_dock(&self) {
        self.clear_dockable_subscriptions();
        let sub = self.dock_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let sub = self.dockables_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        self.set_data_context(None);
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
            self.track_dockables(
                self.dock()
                    .expect("source current dock")
                    .visible_dockables(),
            );
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
        let Some(list) = list else { return };
        if list.as_notify_collection_changed().is_none() {
            return;
        }
        // The native untyped event has counts only; retain the ORIGINAL typed event
        // so Remove/Replace carry the same old/new objects without mirror storage.
        let owner = self.to_ref();
        let handler: Rc<ReadOnlyListChanged<Option<Rc<dyn IDockable>>>> =
            Rc::new(move |e| owner.dockables_collection_changed(e));
        if let Some(token) = list.add_collection_changed(handler) {
            self.dockables_subscription
                .replace(Some(Subscription(Box::new(move || {
                    list.remove_collection_changed(token)
                }))));
        }
    }
    fn dockables_collection_changed(
        &self,
        e: &NotifyCollectionChangedEventArgs<'_, Option<Rc<dyn IDockable>>>,
    ) {
        match e.action {
            NotifyCollectionChangedAction::Add => {
                for item in e.new_items {
                    if let Some(document) = item.as_ref().and_then(managed_document) {
                        self.attach_dockable_subscription(document);
                    }
                }
            }
            NotifyCollectionChangedAction::Remove => {
                for item in e.old_items {
                    if let Some(document) = item.as_ref().and_then(managed_document) {
                        self.detach_dockable_subscription(document);
                    }
                }
            }
            NotifyCollectionChangedAction::Reset => {
                self.track_dockables(self.dock().and_then(|d| d.visible_dockables()))
            }
            NotifyCollectionChangedAction::Replace => {
                for item in e.old_items {
                    if let Some(document) = item.as_ref().and_then(managed_document) {
                        self.detach_dockable_subscription(document);
                    }
                }
                for item in e.new_items {
                    if let Some(document) = item.as_ref().and_then(managed_document) {
                        self.attach_dockable_subscription(document);
                    }
                }
            }
            _ => {}
        }
        self.update_z_order();
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
        let active = self
            .dock()
            .expect("source current dock")
            .active_dockable()
            .and_then(|d| d.as_mdi_document());
        let topmost: Vec<_> = documents
            .iter()
            .filter(|d| is_topmost(d))
            .cloned()
            .collect();
        if topmost.is_empty() {
            manager.update_z_order(&ListHandle::from_plain(documents), active);
            return;
        }
        let normal: Vec<_> = documents
            .iter()
            .filter(|d| !is_topmost(d))
            .cloned()
            .collect();
        let normal = ListHandle::from_plain(normal);
        let topmost = ListHandle::from_plain(topmost);
        let active_normal = active.clone().filter(|a| normal.contains(a));
        let active_topmost = active.filter(|a| topmost.contains(a));
        manager.update_z_order(&normal, active_normal);
        manager.update_z_order(&topmost, active_topmost);
        let offset = normal.count() as i32;
        for document in topmost.iter() {
            document.set_mdi_z_index(document.mdi_z_index().wrapping_add(offset));
        }
    }
    fn track_dockables(&self, list: Option<ListHandle<Option<Rc<dyn IDockable>>>>) {
        self.clear_dockable_subscriptions();
        let Some(list) = list else { return };
        for d in list.iter() {
            if let Some(d) = d.as_ref().and_then(managed_document) {
                self.attach_dockable_subscription(d);
            }
        }
    }
    fn clear_dockable_subscriptions(&self) {
        if self.window_subscriptions.borrow().is_empty() {
            return;
        }
        let documents: Vec<_> = self.window_subscriptions.borrow().keys().cloned().collect();
        for document in documents {
            self.detach_dockable_subscription(
                managed_document(&document).expect("source subscription key"),
            );
        }
        self.window_subscriptions.borrow_mut().clear();
    }
    fn attach_dockable_subscription(&self, document: Rc<ManagedDockWindowDocument>) {
        let key: Rc<dyn IDockable> = document.clone();
        if self.window_subscriptions.borrow().contains_key(&key) {
            return;
        }
        let Some(window) = document.window() else {
            return;
        };
        let Some(notifier) = window.as_notify_property_changed() else {
            return;
        };
        let owner = self.to_ref();
        let token = notifier.property_changed().add(Rc::new(move |name| {
            if name.is_empty() || name == "Topmost" {
                owner.update_z_order();
            }
        }));
        self.window_subscriptions.borrow_mut().insert(key, token);
    }
    fn detach_dockable_subscription(&self, document: Rc<ManagedDockWindowDocument>) {
        let key: Rc<dyn IDockable> = document.clone();
        let token = self.window_subscriptions.borrow().get(&key).copied();
        let Some(token) = token else { return };
        if let Some(window) = document.window() {
            if let Some(n) = window.as_notify_property_changed() {
                n.property_changed().remove(token);
            }
        }
        self.window_subscriptions.borrow_mut().remove(&key);
    }
}
fn managed_document(d: &Rc<dyn IDockable>) -> Option<Rc<ManagedDockWindowDocument>> {
    d.clone()
        .boxed_reference()
        .downcast_ref::<Rc<ManagedDockWindowDocument>>()
        .cloned()
}
fn is_topmost(d: &Rc<dyn IMdiDocument>) -> bool {
    let original: Rc<dyn IDockable> = d.clone();
    managed_document(&original)
        .and_then(|d| d.window())
        .is_some_and(|w| w.topmost())
}
#[cfg(test)]
#[path = "managed_window_layer_tests.rs"]
mod tests;

use super::DockControl;
use ferroui_base::Visual;
impl ManagedWindowLayer {
    /// Locates the first original layer under the enclosing dock control, then visual root.
    pub fn try_get_layer(visual: Option<&Visual>) -> Option<Ref<Self>> {
        let visual = visual?;
        let dock = visual
            .to_ref()
            .cast::<DockControl>()
            .or_else(|| visual.find_ancestor_of_type::<DockControl>(false));
        if let Some(dock) = dock {
            if let Some(layer) = dock.find_descendant_of_type::<ManagedWindowLayer>(false) {
                return Some(layer);
            }
        }
        visual
            .visual_root()?
            .find_descendant_of_type::<ManagedWindowLayer>(false)
    }
}
