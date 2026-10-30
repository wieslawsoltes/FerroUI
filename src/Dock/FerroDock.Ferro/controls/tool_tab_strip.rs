// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ToolTabStripItem;
use crate::{
    automation::peers::ToolTabStripAutomationPeer,
    internal::scroll_viewer_mouse_wheel_hook_helper::ScrollViewerMouseWheelHookHelper,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{Layoutable, LayoutableImpl, Orientation},
    media::ITransform,
    reactive::{AnonymousObserver, IDisposable},
    BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, StyledProperty,
    Thickness, Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    generators::RecycleKey,
    presenters::ItemsPresenter,
    primitives::{
        SelectingItemsControl, SelectingItemsControlImpl, TabStrip, TemplateAppliedEventArgs,
        TemplatedControlImpl, TemplatedControlImplExt,
    },
    templates::IDataTemplate,
    Border, Control, ControlImpl, ItemsControlImpl, ItemsControlImplExt, ScrollViewer,
};
use ferroui_system::collections::generic::Dictionary;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Subscriptions = Vec<Rc<dyn IDisposable>>;
struct ContainerEvents {
    item: WeakRef<ToolTabStripItem>,
    attached: Rc<dyn IDisposable>,
    detached: Rc<dyn IDisposable>,
}
/// Original tool tab strip with container observations and selection-border geometry.
#[repr(C)]
pub struct ToolTabStrip {
    base: TabStrip,
    container_observables: RefCell<Dictionary<Ref<ToolTabStripItem>, Subscriptions>>,
    template_observables: RefCell<Subscriptions>,
    template_version: Cell<u32>,
    container_events: RefCell<Vec<ContainerEvents>>,
    border_left_fill: RefCell<Option<Ref<Border>>>,
    border_right_fill: RefCell<Option<Ref<Border>>>,
    items_presenter: RefCell<Option<Ref<ItemsPresenter>>>,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    scroll_viewer_wheel_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferro_class!(ToolTabStrip:TabStrip);
ferro_class_info!(ToolTabStrip {new:ToolTabStrip::new,markup:{attributes:[PseudoClasses(":create")]}});
ferro_impl_classes!(ToolTabStrip:LayoutableImpl,InteractiveImpl,InputElementImpl,SelectingItemsControlImpl);
impl StyledElementImpl for ToolTabStrip {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for ToolTabStrip {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.can_create_item());
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::can_create_item_property().as_property() {
            this.update_pseudo_classes(change.get_new_value::<bool>());
        }
        if change.property() == Self::mouse_wheel_scroll_orientation_property().as_property() {
            let scroll = this.scroll_viewer.borrow().clone();
            this.attach_scroll_viewer_wheel(scroll.as_deref());
        }
    }
}
impl ControlImpl for ToolTabStrip {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ToolTabStripAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for ToolTabStrip {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.attach_scroll_viewer_wheel(None);
        this.clear_template_observables();
        this.border_left_fill
            .replace(e.name_scope().find_as::<Border>("PART_BorderLeftFill"));
        this.border_right_fill
            .replace(e.name_scope().find_as::<Border>("PART_BorderRightFill"));
        this.items_presenter.replace(
            e.name_scope()
                .find_as::<ItemsPresenter>("PART_ItemsPresenter"),
        );
        this.scroll_viewer
            .replace(e.name_scope().find_as::<ScrollViewer>("PART_ScrollViewer"));
        let scroll = this.scroll_viewer.borrow().clone();
        this.attach_scroll_viewer_wheel(scroll.as_deref());
        let presenter = this.items_presenter.borrow().clone();
        if let Some(presenter) = presenter {
            let owner = this.to_ref();
            let subscription = FerroObject::get_observable(&presenter, Visual::bounds_property())
                .subscribe(Rc::new(AnonymousObserver::new(move |_: Rect| {
                    owner.update_borders()
                })));
            this.add_template_observable(subscription);
        }
        let scroll = this.scroll_viewer.borrow().clone();
        if let Some(scroll) = scroll {
            let owner = this.to_ref();
            let subscription =
                FerroObject::get_observable(&scroll, ScrollViewer::offset_property()).subscribe(
                    Rc::new(AnonymousObserver::new(move |_: Vector| {
                        owner.update_borders()
                    })),
                );
            this.add_template_observable(subscription);
        }
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, SelectingItemsControl::selected_item_property())
                .subscribe(Rc::new(AnonymousObserver::new(
                    move |_: Option<BoxedValue>| owner.update_borders(),
                )));
        this.add_template_observable(subscription);
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, Visual::bounds_property()).subscribe(Rc::new(
                AnonymousObserver::new(move |_: Rect| owner.update_borders()),
            ));
        this.add_template_observable(subscription);
        this.update_borders();
    }
}
impl VisualImpl for ToolTabStrip {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.attach_scroll_viewer_wheel(None);
        this.clear_template_observables();
        this.clear_container_observables();
    }
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let scroll = this.scroll_viewer.borrow().clone();
        this.attach_scroll_viewer_wheel(scroll.as_deref());
    }
}
impl ItemsControlImpl for ToolTabStrip {
    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if let Some(item) = container.cast::<ToolTabStripItem>() {
            this.remove_container_events(&item);
            let owner = this.to_ref();
            let sender = item.downgrade();
            let attached = item.attached_to_visual_tree(move |e| {
                if let Some(sender) = sender.upgrade() {
                    owner.on_container_attached(&sender, e);
                }
            });
            let owner = this.to_ref();
            let sender = item.downgrade();
            let detached = item.detached_from_visual_tree(move |e| {
                if let Some(sender) = sender.upgrade() {
                    owner.on_container_detached(&sender, e);
                }
            });
            this.container_events.borrow_mut().push(ContainerEvents {
                item: item.downgrade(),
                attached,
                detached,
            });
        }
    }
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ToolTabStripItem::new().upcast()
    }
    fn needs_container_override(
        this: &Self,
        item: &Option<BoxedValue>,
        _index: i32,
    ) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ToolTabStripItem>(item)
    }
}
impl ToolTabStrip {
    /// Constructs the native source class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TabStrip::construct(),
            container_observables: RefCell::new(Dictionary::new()),
            template_observables: RefCell::new(Vec::new()),
            template_version: Cell::new(0),
            container_events: RefCell::new(Vec::new()),
            border_left_fill: RefCell::new(None),
            border_right_fill: RefCell::new(None),
            items_presenter: RefCell::new(None),
            scroll_viewer: RefCell::new(None),
            scroll_viewer_wheel_subscription: RefCell::new(None),
        }
    }
    /// Creates the original tool tab strip.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn add_template_observable(&self, subscription: Rc<dyn IDisposable>) {
        self.template_observables.borrow_mut().push(subscription);
        self.template_version
            .set(self.template_version.get().wrapping_add(1));
    }
    fn on_container_attached(
        &self,
        item: &Ref<ToolTabStripItem>,
        _e: &VisualTreeAttachmentEventArgs,
    ) {
        let existing = self.container_observables.borrow().get(item).cloned();
        if let Some(existing) = existing {
            for disposable in existing {
                disposable.dispose();
            }
            let removed = self.container_observables.borrow_mut().remove(item);
            drop(removed);
        }
        let owner = self.to_ref();
        let transform = FerroObject::get_observable(item, Visual::render_transform_property())
            .subscribe(Rc::new(AnonymousObserver::new(
                move |_: Option<Rc<dyn ITransform>>| owner.update_borders(),
            )));
        let owner = self.to_ref();
        let bounds =
            FerroObject::get_observable(item, Visual::bounds_property()).subscribe(Rc::new(
                AnonymousObserver::new(move |_: Rect| owner.update_borders()),
            ));
        let old = self
            .container_observables
            .borrow_mut()
            .insert(item.clone(), vec![transform, bounds]);
        drop(old);
    }
    fn on_container_detached(
        &self,
        item: &Ref<ToolTabStripItem>,
        _e: &VisualTreeAttachmentEventArgs,
    ) {
        let subscriptions = self.container_observables.borrow().get(item).cloned();
        if let Some(subscriptions) = subscriptions {
            for disposable in subscriptions {
                disposable.dispose();
            }
            let removed = self.container_observables.borrow_mut().remove(item);
            drop(removed);
        }
        self.remove_container_events(item);
    }
    fn remove_container_events(&self, item: &Ref<ToolTabStripItem>) {
        let index = self
            .container_events
            .borrow()
            .iter()
            .rposition(|e| e.item.upgrade().is_some_and(|i| i.ptr_eq(item)));
        if let Some(index) = index {
            let removed = self.container_events.borrow_mut().remove(index);
            removed.attached.dispose();
            removed.detached.dispose();
        }
    }
    fn update_borders(&self) {
        let selected_index = self.selected_index();
        let bounds = self
            .items_presenter
            .borrow()
            .clone()
            .map_or_else(Rect::default, |p| p.bounds());
        let scroll_offset = self
            .scroll_viewer
            .borrow()
            .clone()
            .map_or(0.0, |v| v.offset().x);
        let (left, right) = if selected_index >= 0
            && selected_index < self.items().count() as i32
            && self.items().count() != 1
        {
            let selected = self
                .container_from_index(selected_index)
                .and_then(|c| c.cast::<ToolTabStripItem>());
            let width = selected.as_ref().map_or(0.0, |c| c.bounds().width);
            let translate = selected
                .as_ref()
                .and_then(|c| c.render_transform())
                .map_or(0.0, |t| t.value().m31);
            let x = selected.map_or(0.0, |c| c.bounds().x) + translate - scroll_offset;
            (
                Thickness::new(0.0, 0.0, self.bounds().width - x, 0.0),
                Thickness::new(x + width, 0.0, 0.0, 0.0),
            )
        } else {
            (
                Thickness::new(0.0, 0.0, bounds.width, 0.0),
                Thickness::new(bounds.width, 0.0, 0.0, 0.0),
            )
        };
        let border = self.border_left_fill.borrow().clone();
        if let Some(border) = border {
            border.set_current_value(Layoutable::margin_property(), left);
        }
        let border = self.border_right_fill.borrow().clone();
        if let Some(border) = border {
            border.set_current_value(Layoutable::margin_property(), right);
        }
    }
    fn update_pseudo_classes(&self, value: bool) {
        self.pseudo_classes().set(":create", value);
    }
    fn attach_scroll_viewer_wheel(&self, scroll: Option<&ScrollViewer>) {
        let subscription = self.scroll_viewer_wheel_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let subscription =
            ScrollViewerMouseWheelHookHelper::attach(scroll, self.mouse_wheel_scroll_orientation());
        self.scroll_viewer_wheel_subscription.replace(subscription);
    }
    fn clear_template_observables(&self) {
        let version = self.template_version.get();
        let mut index = 0;
        loop {
            assert_eq!(
                version,
                self.template_version.get(),
                "Collection was modified; enumeration operation may not execute."
            );
            let disposable = self.template_observables.borrow().get(index).cloned();
            index += 1;
            let Some(disposable) = disposable else {
                break;
            };
            disposable.dispose();
        }
        let old = std::mem::take(&mut *self.template_observables.borrow_mut());
        self.template_version
            .set(self.template_version.get().wrapping_add(1));
        drop(old);
    }
    fn clear_container_observables(&self) {
        if self.container_observables.borrow().is_empty() {
            return;
        }
        let entries: Vec<_> = self
            .container_observables
            .borrow()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (item, subscriptions) in entries {
            for disposable in subscriptions {
                disposable.dispose();
            }
            self.remove_container_events(&item);
        }
        let old = std::mem::take(&mut *self.container_observables.borrow_mut());
        drop(old);
    }
}
ferro_properties! {impl ToolTabStrip {
    /// Identifies CanCreateItem.
    pub fn can_create_item_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("CanCreateItem",false)}
    /// Identifies MouseWheelScrollOrientation.
    pub fn mouse_wheel_scroll_orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<Self,Orientation>("MouseWheelScrollOrientation",Orientation::Horizontal)}
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("ModifiedTemplate",None)}
}}
impl ToolTabStrip {
    /// Gets CanCreateItem.
    pub fn can_create_item(&self) -> bool {
        self.get_value(Self::can_create_item_property())
    }
    /// Sets CanCreateItem.
    pub fn set_can_create_item(&self, value: bool) {
        self.set_value(Self::can_create_item_property(), value);
    }
    /// Gets MouseWheelScrollOrientation.
    pub fn mouse_wheel_scroll_orientation(&self) -> Orientation {
        self.get_value(Self::mouse_wheel_scroll_orientation_property())
    }
    /// Sets MouseWheelScrollOrientation.
    pub fn set_mouse_wheel_scroll_orientation(&self, value: Orientation) {
        self.set_value(Self::mouse_wheel_scroll_orientation_property(), value);
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
}
