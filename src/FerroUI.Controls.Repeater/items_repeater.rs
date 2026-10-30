// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::repeater_layout_context::RepeaterLayoutContext;
use super::view_manager::ViewManager;
use super::viewport_manager::ViewportManager;
use super::{
    factory_from_template, ElementOwner, ElementRealizationOptions, IElementFactory,
    ItemCollectionTransitionProvider, ItemsRepeaterElementClearingEventArgs,
    ItemsRepeaterElementIndexChangedEventArgs, ItemsRepeaterElementPreparedEventArgs, Layout,
    LayoutContext, Phaser, RepeaterAutomationPeer, StackLayout, TransitionManager,
    VirtualizationInfo, VirtualizingLayout, VirtualizingLayoutContext,
};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{
    Control, ControlImpl, ItemsChangedEventArgs, ItemsSource, ItemsSourceView, Panel, PanelImpl,
    RequestBringIntoViewEventArgs,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::{core::ValueTypes, BindingPriority};
use ferroui_base::input::navigation::{XYFocus, XYFocusNavigationModes};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl,
    InputElementImplExt, KeyboardNavigation, KeyboardNavigationMode,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::FerroObjectExtensions;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    // LayoutState lives on each context, so all implicit hosts share one
    // layout exactly as the source's thread-local effective StackLayout.
    static DEFAULT_LAYOUT: Ref<Layout> = StackLayout::new().upcast();
}

/// A lightweight, virtualizing items host with pluggable layouts and factories.
///
/// Elements are ordinary FerroUI controls in the visual and logical trees.
/// ScrollViewer supplies the effective viewport and scroll anchors; the
/// repeater supplies no selection policy or implicit item container.
#[repr(C)]
pub struct ItemsRepeater {
    base: Panel,
    items_view: RefCell<Option<Rc<ItemsSourceView>>>,
    source_token: Cell<Option<u64>>,
    factory: RefCell<Option<Rc<dyn IElementFactory>>>,
    effective_layout: RefCell<Option<Ref<Layout>>>,
    context: OnceCell<Ref<VirtualizingLayoutContext>>,
    layout_state: RefCell<Option<BoxedValue>>,
    layout_origin: Cell<Point>,
    layout_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    phaser: OnceCell<Rc<Phaser>>,
    transitions: OnceCell<Rc<TransitionManager>>,
    pub(crate) view_manager: ViewManager,
    pub(crate) viewport_manager: ViewportManager,
    in_layout: Cell<bool>,
    pending_template_reset: Cell<bool>,
    stack_measure_count: Cell<u8>,
    processing_change: Cell<Option<NotifyCollectionChangedAction>>,
    item_template_empty: Cell<bool>,
    owns_transition_provider: Cell<bool>,
    element_prepared: HandlerList<dyn Fn(&ItemsRepeaterElementPreparedEventArgs)>,
    element_clearing: HandlerList<dyn Fn(&ItemsRepeaterElementClearingEventArgs)>,
    element_index_changed: HandlerList<dyn Fn(&ItemsRepeaterElementIndexChangedEventArgs)>,
}

ferro_class!(ItemsRepeater: Panel);
ferro_class_info!(ItemsRepeater {
    new: ItemsRepeater::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: ItemTemplate,
        property_attributes: [ItemTemplate: [InheritDataTypeFromItems("ItemsSource")]],
        properties: [ItemsSourceView: Option<Rc<ItemsSourceView>> { get: ItemsRepeater::items_source_view }],
        methods: [
            fn GetElementIndex(Ref<Control>) -> i32 => |this: &Ref<ItemsRepeater>, element: Ref<Control>| this.get_element_index(&element),
            fn TryGetElement(i32) -> Option<Ref<Control>> => ItemsRepeater::try_get_element,
            fn GetOrCreateElement(i32) -> Ref<Control> => ItemsRepeater::get_or_create_element,
        ],
        events: [
            ElementPrepared(Option<Ref<ItemsRepeater>>, ItemsRepeaterElementPreparedEventArgs) => |this: &Ref<ItemsRepeater>, handler: ferroui_base::metadata::MarkupDelegate| {
                let sender = this.downgrade();
                this.element_prepared(move |args| { handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade()), ferroui_base::metadata::into_markup_value(args.clone())]); })
            },
            ElementClearing(Option<Ref<ItemsRepeater>>, ItemsRepeaterElementClearingEventArgs) => |this: &Ref<ItemsRepeater>, handler: ferroui_base::metadata::MarkupDelegate| {
                let sender = this.downgrade();
                this.element_clearing(move |args| { handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade()), ferroui_base::metadata::into_markup_value(args.clone())]); })
            },
            ElementIndexChanged(Option<Ref<ItemsRepeater>>, ItemsRepeaterElementIndexChangedEventArgs) => |this: &Ref<ItemsRepeater>, handler: ferroui_base::metadata::MarkupDelegate| {
                let sender = this.downgrade();
                this.element_index_changed(move |args| { handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade()), ferroui_base::metadata::into_markup_value(args.clone())]); })
            },
        ],
    },
});
ferro_impl_classes!(ItemsRepeater: StyledElementImpl, InteractiveImpl);

struct LayoutGuard<'a>(&'a Cell<bool>);
impl Drop for LayoutGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
struct CollectionGuard<'a>(&'a Cell<Option<NotifyCollectionChangedAction>>);
impl Drop for CollectionGuard<'_> {
    fn drop(&mut self) {
        self.0.set(None);
    }
}

impl FerroObjectImpl for ItemsRepeater {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        KeyboardNavigation::set_tab_navigation(this, KeyboardNavigationMode::Once);
        XYFocus::set_navigation_modes(
            this,
            XYFocus::get_navigation_modes(this) | XYFocusNavigationModes::KEYBOARD,
        );
        AutomationProperties::set_accessibility_view(this, AccessibilityView::Raw);
        let weak = this.to_ref().downgrade();
        let items_weak = weak.clone();
        let _ = this.phaser.set(Phaser::new(
            Rc::new(move || {
                weak.upgrade()
                    .map_or(Rect::default(), |owner| owner.visible_window())
            }),
            Rc::new(move |index| {
                items_weak
                    .upgrade()
                    .and_then(|owner| owner.items_source_view())
                    .filter(|items| index >= 0 && (index as usize) < items.count())
                    .and_then(|items| items.get_at(index as usize))
            }),
        ));
        let weak = this.to_ref().downgrade();
        let _ = this
            .transitions
            .set(TransitionManager::new(Rc::new(move |element| {
                if let Some(owner) = weak.upgrade() {
                    // Completion can arrive after a source/template change or
                    // after a shared provider's element moved to another host.
                    // Only the current animator owner may recycle it. The
                    // ownership check also makes repeated completion harmless
                    // if this element has already been reused for another item.
                    let is_child = element
                        .visual_parent()
                        .is_some_and(|parent| parent.ptr_eq(&owner.clone().upcast::<Visual>()));
                    if is_child
                        && Self::get_virtualization_info(&element).owner() == ElementOwner::Animator
                    {
                        owner
                            .view_manager
                            .clear_element_to_element_factory(&owner, &element);
                        // Recycle pools can retain the element in our panel.
                        // Arrange must move it outside the visible viewport.
                        owner.invalidate_arrange();
                    }
                }
            })));
        let weak = this.to_ref().downgrade();
        this.layout_updated(move || {
            if let Some(owner) = weak.upgrade() {
                owner.stack_measure_count.set(0);
                owner.viewport_manager.on_layout_updated(&owner);
            }
        });
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            Control::request_bring_into_view_event(),
            move |_, args: &RequestBringIntoViewEventArgs| {
                if let (Some(owner), Some(target)) = (weak.upgrade(), args.target_object.as_ref()) {
                    owner.viewport_manager.on_bring_into_view(&owner, target);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        this.change_layout();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::items_source_property().as_property() {
            this.change_source();
        } else if property == Self::item_template_property().as_property() {
            this.change_template();
        } else if property == Self::layout_property().as_property() {
            this.change_layout();
        } else if property == Self::item_transition_provider_property().as_property() {
            this.owns_transition_provider.set(false);
            this.transition_manager()
                .on_transition_provider_changed(this.item_transition_provider());
        } else if property == Self::horizontal_cache_length_property().as_property()
            || property == Self::vertical_cache_length_property().as_property()
        {
            this.viewport_manager.reset_cache();
            this.invalidate_measure();
        } else if property == KeyboardNavigation::tab_navigation_property().as_property()
            && KeyboardNavigation::get_tab_navigation(this) != KeyboardNavigationMode::Once
        {
            KeyboardNavigation::set_tab_once_active_element(this, None);
        } else if property == Visual::render_transform_property().as_property() {
            this.viewport_manager.on_render_transform_changed(this);
        } else if property == Visual::render_transform_origin_property().as_property() {
            this.viewport_manager.refresh_viewport(this);
        } else if property == Visual::bounds_property().as_property()
            && this.render_transform().is_some()
        {
            let origin = this.render_transform_origin();
            let (old, new) = change.get_old_and_new_value::<Rect>();
            if origin.unit == ferroui_base::RelativeUnit::Relative
                && origin.point != Point::default()
                && old.size() != new.size()
            {
                // Native viewport geometry can stay unchanged when only the
                // receiving visual grows. Its own relative transform origin
                // nevertheless moves, changing the viewport in item space.
                this.viewport_manager.refresh_viewport(this);
            }
        }
    }
}

impl PanelImpl for ItemsRepeater {
    fn invalidate_measure_on_children_changed(_this: &Self) {
        // The layout owns realization and invalidation, including mutations
        // performed during its own measure pass.
    }
}

impl LayoutableImpl for ItemsRepeater {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let _guard = this.enter_layout();
        if this.effective_layout().is::<StackLayout>() {
            let count = this.stack_measure_count.get().saturating_add(1);
            this.stack_measure_count.set(count);
            if count >= 60 {
                let extent = this.viewport_manager.layout_extent();
                return Size::new(extent.width - extent.x, extent.height - extent.y);
            }
        }
        this.view_manager.prune_pinned_elements(this);
        this.viewport_manager.on_owner_measuring(this);
        let context = this.layout_context();
        let mut desired = if this.item_template_empty.get() {
            Size::default()
        } else {
            this.effective_layout().measure(&context, available_size)
        };
        if this.pending_template_reset.replace(false) {
            // A mixed source can discover its first data item after a layout
            // has already measured direct control items. Resetting that
            // layout inside GetElement would invalidate its active traversal.
            // Keep the public template change synchronous, but deliver Reset
            // at this safe point with the old (absent) factory visible, then
            // measure the complete source with the newly installed factory.
            let factory = this.factory.borrow_mut().take();
            this.reset_template_elements();
            *this.factory.borrow_mut() = factory;
            desired = if this.item_template_empty.get() {
                Size::default()
            } else {
                this.effective_layout().measure(&context, available_size)
            };
        }
        let children = this.children().snapshot();
        for child in children.iter() {
            let info = Self::get_virtualization_info(child);
            if info.is_held_by_layout() && info.auto_recycle_candidate() && !info.keep_alive() {
                this.clear_element_impl(child);
            }
        }
        let origin = this.layout_origin();
        this.viewport_manager.set_layout_extent(Rect::new(
            origin.x,
            origin.y,
            desired.width,
            desired.height,
        ));
        desired
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let _guard = this.enter_layout();
        let arranged = this
            .effective_layout()
            .arrange(&this.layout_context(), final_size);
        this.view_manager.on_owner_arranged(this);
        let children = this.children().snapshot();
        for element in children.iter() {
            let info = Self::get_virtualization_info(element);
            info.set_keep_alive(false);
            if matches!(
                info.owner(),
                ElementOwner::ElementFactory | ElementOwner::PinnedPool
            ) {
                let desired = element.desired_size();
                element.arrange(Rect::new(
                    -10000.0 - desired.width,
                    -10000.0 - desired.height,
                    0.0,
                    0.0,
                ));
            } else {
                let bounds = element.bounds();
                let old = info.arrange_bounds();
                if old != VirtualizationInfo::INVALID_RECT && old != bounds {
                    this.transition_manager()
                        .on_element_bounds_changed(element, old, bounds);
                }
                info.set_arrange_bounds(bounds);
            }
        }
        this.viewport_manager.on_owner_arranged(this);
        this.transition_manager().on_owner_arranged();
        arranged
    }
}

impl VisualImpl for ItemsRepeater {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.viewport_manager.attach(this);
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.stack_measure_count.set(0);
        this.viewport_manager.detach(this);
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}

impl InputElementImpl for ItemsRepeater {
    // Deviation (winui-repeater/DEVIATIONS.md, WR-20): the realized children in source order
    // are the tab order (WinUI's `GetChildrenInTabFocusOrder`); recycled children stay out.
    fn children_in_tab_focus_order(this: &Self) -> Option<Vec<Ref<InputElement>>> {
        Some(
            this.realized_elements()
                .into_iter()
                .map(Ref::upcast)
                .collect(),
        )
    }
    fn on_got_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, args);
        this.view_manager
            .update_focus(this, args.new_focused_element.as_ref());
    }
    fn on_lost_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, args);
        this.view_manager
            .update_focus(this, args.new_focused_element.as_ref());
    }
}

impl ControlImpl for ItemsRepeater {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        RepeaterAutomationPeer::new(this).upcast()
    }
}

ferro_properties! { impl ItemsRepeater {
    pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<Self, _>("ItemsSource", None)
    }
    pub fn item_template_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Self, _>("ItemTemplate", None)
    }
    pub fn layout_property() -> StyledProperty<Option<Ref<Layout>>> {
        FerroProperty::register::<Self, _>("Layout", None)
    }
    pub fn item_transition_provider_property() -> StyledProperty<Option<Ref<ItemCollectionTransitionProvider>>> {
        FerroProperty::register::<Self, _>("ItemTransitionProvider", None)
    }
    pub fn horizontal_cache_length_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<Self, _>("HorizontalCacheLength", StyledPropertyOptions::new(2.0_f64).validate(|value| value.is_finite() && *value >= 0.0))
    }
    pub fn vertical_cache_length_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<Self, _>("VerticalCacheLength", StyledPropertyOptions::new(2.0_f64).validate(|value| value.is_finite() && *value >= 0.0))
    }
    pub fn virtualization_info_property() -> AttachedProperty<Option<Rc<VirtualizationInfo>>> {
        FerroProperty::register_attached::<Self, Control, _>("VirtualizationInfo", None)
    }
} }

macro_rules! repeater_event {
    ($event:ident, $raise:ident, $args:ty) => {
        pub fn $event(&self, handler: impl Fn(&$args) + 'static) -> Rc<dyn IDisposable> {
            let token = self.$event.add(Rc::new(handler));
            let weak = self.to_ref().downgrade();
            Disposable::create(move || {
                if let Some(owner) = weak.upgrade() {
                    owner.$event.remove(token);
                }
            })
        }
        pub(crate) fn $raise(&self, args: &$args) {
            for (_, handler) in self.$event.snapshot().iter() {
                handler(args);
            }
        }
    };
}

impl ItemsRepeater {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            items_view: RefCell::new(None),
            source_token: Cell::new(None),
            factory: RefCell::new(None),
            effective_layout: RefCell::new(None),
            context: OnceCell::new(),
            layout_state: RefCell::new(None),
            layout_origin: Cell::new(Point::default()),
            layout_subscriptions: RefCell::new(Vec::new()),
            phaser: OnceCell::new(),
            transitions: OnceCell::new(),
            view_manager: ViewManager::default(),
            viewport_manager: ViewportManager::default(),
            in_layout: Cell::new(false),
            pending_template_reset: Cell::new(false),
            stack_measure_count: Cell::new(0),
            processing_change: Cell::new(None),
            item_template_empty: Cell::new(false),
            owns_transition_provider: Cell::new(true),
            element_prepared: HandlerList::new(),
            element_clearing: HandlerList::new(),
            element_index_changed: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }
    pub fn set_items_source(&self, source: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), source);
    }
    pub fn items_source_view(&self) -> Option<Rc<ItemsSourceView>> {
        self.items_view.borrow().clone()
    }
    pub fn item_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::item_template_property())
    }
    pub fn set_item_template(&self, template: Option<BoxedValue>) {
        self.set_value(Self::item_template_property(), template);
    }
    pub fn set_data_template(&self, template: Option<Rc<dyn IDataTemplate>>) {
        self.set_item_template(template.map(|template| Rc::new(template) as BoxedValue));
    }
    pub fn set_element_factory(&self, factory: Option<Rc<dyn IElementFactory>>) {
        self.set_item_template(factory.map(|factory| Rc::new(factory) as BoxedValue));
    }
    pub fn layout(&self) -> Option<Ref<Layout>> {
        self.get_value(Self::layout_property())
    }
    pub fn set_layout(&self, layout: Option<Ref<Layout>>) {
        self.set_value(Self::layout_property(), layout);
    }
    pub fn item_transition_provider(&self) -> Option<Ref<ItemCollectionTransitionProvider>> {
        self.get_value(Self::item_transition_provider_property())
    }
    pub fn set_item_transition_provider(
        &self,
        provider: Option<Ref<ItemCollectionTransitionProvider>>,
    ) {
        self.set_value(Self::item_transition_provider_property(), provider);
    }
    pub fn horizontal_cache_length(&self) -> f64 {
        self.get_value(Self::horizontal_cache_length_property())
    }
    pub fn set_horizontal_cache_length(&self, value: f64) {
        self.set_value(Self::horizontal_cache_length_property(), value);
    }
    pub fn vertical_cache_length(&self) -> f64 {
        self.get_value(Self::vertical_cache_length_property())
    }
    pub fn set_vertical_cache_length(&self, value: f64) {
        self.set_value(Self::vertical_cache_length_property(), value);
    }

    pub fn get_element_index(&self, element: &Ref<Control>) -> i32 {
        if element
            .visual_parent()
            .as_ref()
            .is_some_and(|parent| parent.ptr_eq(&self.to_ref().upcast::<Visual>()))
        {
            let info = Self::get_virtualization_info(element);
            if info.is_realized() || info.is_in_unique_id_reset_pool() {
                return info.index();
            }
        }
        -1
    }
    pub fn try_get_element(&self, index: i32) -> Option<Ref<Control>> {
        self.view_manager.try_get_element(index, false)
    }
    pub fn get_or_create_element(&self, index: i32) -> Ref<Control> {
        assert!(
            !self.in_layout.get(),
            "GetOrCreateElement invocation is not allowed during layout."
        );
        let items = self
            .items_source_view()
            .expect("ItemsSource doesn't have a value.");
        assert!(
            index >= 0 && (index as usize) < items.count(),
            "Argument index is invalid."
        );
        let existing = self.try_get_element(index);
        let outside = existing.is_none();
        let element = existing.unwrap_or_else(|| {
            let element = self.get_element_impl(index, ElementRealizationOptions::NONE);
            element.measure(Size::INFINITY);
            // A recycled native control may still have a valid arrange at
            // its old offscreen pool position. BringIntoView must wait for
            // the new item's arrange instead of scrolling to those bounds.
            element.invalidate_arrange();
            element
        });
        self.viewport_manager.make_anchor(self, &element, outside);
        self.invalidate_measure();
        element
    }
    pub fn pin_element(&self, element: &Ref<Control>) {
        ViewManager::update_pin(element, true);
    }
    pub fn unpin_element(&self, element: &Ref<Control>) {
        ViewManager::update_pin(element, false);
    }
    /// The realized elements in the order of the children, for work that needs no source order.
    pub(crate) fn realized_elements_unsorted(&self) -> Vec<Ref<Control>> {
        self.children()
            .snapshot()
            .iter()
            .filter(|element| {
                Self::try_get_virtualization_info(element).is_some_and(|info| info.is_realized())
            })
            .cloned()
            .collect()
    }
    pub fn realized_elements(&self) -> Vec<Ref<Control>> {
        let mut elements: Vec<_> = self
            .children()
            .snapshot()
            .iter()
            .filter(|element| {
                Self::try_get_virtualization_info(element).is_some_and(|info| info.is_realized())
            })
            .cloned()
            .collect();
        elements.sort_by_key(|element| Self::get_virtualization_info(element).index());
        elements
    }
    pub fn try_get_virtualization_info(element: &Control) -> Option<Rc<VirtualizationInfo>> {
        element.get_value(Self::virtualization_info_property())
    }
    pub fn get_virtualization_info(element: &Control) -> Rc<VirtualizationInfo> {
        if let Some(info) = Self::try_get_virtualization_info(element) {
            return info;
        }
        let info = VirtualizationInfo::new();
        element.set_value(Self::virtualization_info_property(), Some(info.clone()));
        info
    }
    pub fn layout_state(&self) -> Option<BoxedValue> {
        self.layout_state.borrow().clone()
    }
    pub fn set_layout_state(&self, value: Option<BoxedValue>) {
        *self.layout_state.borrow_mut() = value;
    }
    pub fn layout_origin(&self) -> Point {
        self.layout_origin.get()
    }
    pub fn set_layout_origin(&self, value: Point) {
        assert!(
            value.x.is_finite() && value.y.is_finite(),
            "LayoutOrigin must be finite."
        );
        self.layout_origin.set(value);
    }
    pub fn visible_window(&self) -> Rect {
        self.viewport_manager.visible_window()
    }
    pub fn realization_window(&self) -> Rect {
        self.viewport_manager.realization_window()
    }

    repeater_event!(
        element_prepared,
        raise_element_prepared,
        ItemsRepeaterElementPreparedEventArgs
    );
    repeater_event!(
        element_clearing,
        raise_element_clearing,
        ItemsRepeaterElementClearingEventArgs
    );
    repeater_event!(
        element_index_changed,
        raise_element_index_changed,
        ItemsRepeaterElementIndexChangedEventArgs
    );

    pub(crate) fn phaser(&self) -> Rc<Phaser> {
        self.phaser.get().expect("Repeater is constructed.").clone()
    }
    pub(crate) fn transition_manager(&self) -> Rc<TransitionManager> {
        self.transitions
            .get()
            .expect("Repeater is constructed.")
            .clone()
    }
    pub(crate) fn item_template_shim(&self) -> Option<Rc<dyn IElementFactory>> {
        self.factory.borrow().clone()
    }
    pub(crate) fn ensure_default_factory(&self) -> Rc<dyn IElementFactory> {
        if let Some(factory) = self.item_template_shim() {
            return factory;
        }
        // Installation is a public property change, including the layout
        // Reset notification. The implicit template also creates an empty
        // TextBlock for null items, unlike the general fallback template.
        let template = FuncDataTemplate::new(
            |_| true,
            |_, _| {
                let text = ferroui_controls::TextBlock::new();
                let object: &ferroui_base::FerroObject = &text;
                text.bind(
                    ferroui_controls::TextBlock::text_property(),
                    FerroObjectExtensions::get_observable_with(
                        object,
                        ferroui_base::StyledElement::data_context_property(),
                        |data: Option<BoxedValue>| {
                            data.map(|data| ValueTypes::to_display_string(Some(&data)))
                        },
                    ),
                    BindingPriority::LocalValue,
                );
                Some(text.upcast())
            },
            true,
        );
        self.set_data_template(Some(template));
        self.item_template_shim()
            .expect("The implicit item template creates an element factory.")
    }
    pub(crate) fn get_element_impl(
        &self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        self.view_manager.get_element(self, index, options)
    }
    pub(crate) fn clear_element_impl(&self, element: &Ref<Control>) {
        let strict = matches!(
            self.processing_change.get(),
            Some(
                NotifyCollectionChangedAction::Remove
                    | NotifyCollectionChangedAction::Replace
                    | NotifyCollectionChangedAction::Reset
            )
        );
        self.view_manager.clear_element(self, element, strict);
        self.viewport_manager.on_element_cleared(element);
    }
    pub(crate) fn immediate_child(&self, descendant: &Ref<Visual>) -> Option<Ref<Control>> {
        let mut child = descendant.clone();
        while let Some(parent) = child.visual_parent() {
            if parent.ptr_eq(&self.to_ref().upcast::<Visual>()) {
                return child.cast::<Control>();
            }
            child = parent;
        }
        None
    }
    pub(crate) fn focusable_descendants(element: &Ref<Control>) -> Vec<Ref<InputElement>> {
        fn visit(visual: &Ref<Visual>, result: &mut Vec<Ref<InputElement>>) {
            if let Some(input) = visual.cast::<InputElement>() {
                if input.focusable()
                    && input.is_effectively_enabled()
                    && input.is_effectively_visible()
                    && KeyboardNavigation::get_is_tab_stop(&input)
                {
                    result.push(input);
                }
            }
            if let Some(children) = visual
                .cast::<InputElement>()
                .and_then(|input| input.children_in_tab_focus_order())
            {
                for child in children {
                    visit(&child.upcast(), result);
                }
                return;
            }
            for child in visual.visual_children().snapshot().iter() {
                visit(child, result);
            }
        }
        let mut result = Vec::new();
        visit(&element.clone().upcast(), &mut result);
        result.sort_by_key(|element| KeyboardNavigation::get_tab_index(element));
        result
    }
    fn enter_layout(&self) -> LayoutGuard<'_> {
        assert!(!self.in_layout.get(), "Reentrancy detected during layout.");
        assert!(
            self.processing_change.get().is_none(),
            "Cannot run layout in the middle of a collection change."
        );
        self.in_layout.set(true);
        LayoutGuard(&self.in_layout)
    }
    fn layout_context(&self) -> Ref<LayoutContext> {
        self.context
            .get_or_init(|| {
                VirtualizingLayoutContext::with_host(Rc::new(RepeaterLayoutContext(
                    self.to_ref().downgrade(),
                )))
            })
            .clone()
            .upcast()
    }
    fn effective_layout(&self) -> Ref<Layout> {
        self.effective_layout
            .borrow()
            .clone()
            .expect("Repeater has an effective layout.")
    }
    fn change_layout(&self) {
        assert!(
            !self.in_layout.get(),
            "Layout cannot be changed during layout."
        );
        let context = self.layout_context();
        let old = self.effective_layout.borrow_mut().take();
        self.view_manager.on_layout_changing(self);
        self.transition_manager().on_layout_changing();
        if let Some(old) = old {
            self.stack_measure_count.set(0);
            old.uninitialize_for_context(&context);
            for subscription in std::mem::take(&mut *self.layout_subscriptions.borrow_mut()) {
                subscription.dispose();
            }
            for element in self.realized_elements() {
                self.clear_element_impl(&element);
            }
            self.set_layout_state(None);
        }
        let layout = self
            .layout()
            .unwrap_or_else(|| DEFAULT_LAYOUT.with(Clone::clone));
        *self.effective_layout.borrow_mut() = Some(layout.clone());
        layout.initialize_for_context(&context);
        let weak = self.to_ref().downgrade();
        let measure = layout.measure_invalidated(move || {
            if let Some(owner) = weak.upgrade() {
                owner.invalidate_measure();
            }
        });
        let weak = self.to_ref().downgrade();
        let arrange = layout.arrange_invalidated(move || {
            if let Some(owner) = weak.upgrade() {
                owner.invalidate_arrange();
            }
        });
        self.layout_subscriptions
            .borrow_mut()
            .extend([measure, arrange]);
        if self.owns_transition_provider.get() {
            self.transition_manager()
                .on_transition_provider_changed(layout.create_default_item_transition_provider());
        }
        self.viewport_manager
            .on_layout_changed(self, layout.is::<VirtualizingLayout>());
        self.invalidate_measure();
    }
    fn change_source(&self) {
        assert!(
            !self.in_layout.get(),
            "Cannot set ItemsSourceView during layout."
        );
        assert!(
            self.processing_change.get().is_none(),
            "ItemsSource cannot change during another collection change."
        );
        self.view_manager.clear_all(self);
        let old = self.items_view.borrow_mut().take();
        if let (Some(old), Some(token)) = (old, self.source_token.take()) {
            old.remove_collection_changed(token);
        }
        let source = self.items_source();
        let view = source
            .as_ref()
            .map(|source| ItemsSourceView::get_or_create(Some(source)));
        *self.items_view.borrow_mut() = view.clone();
        if let Some(view) = view {
            let weak = self.to_ref().downgrade();
            self.source_token
                .set(Some(view.add_collection_changed(Rc::new(move |args| {
                    if let Some(owner) = weak.upgrade() {
                        owner.on_items_source_changed(args);
                    }
                }))));
        }
        self.recheck_template_empty();
        self.viewport_manager.reset_cache();
        self.effective_layout()
            .on_items_changed(&self.layout_context(), &ItemsChangedEventArgs::RESET);
        self.invalidate_measure();
    }
    fn change_template(&self) {
        assert!(
            !self.in_layout.get() || self.item_template_shim().is_none(),
            "ItemTemplate cannot be changed during layout."
        );
        let template = self.item_template();
        let factory =
            factory_from_template(template.as_ref()).unwrap_or_else(|error| panic!("{error}"));
        if self.in_layout.get() {
            self.pending_template_reset.set(true);
        } else {
            self.reset_template_elements();
        }
        *self.factory.borrow_mut() = factory;
        self.recheck_template_empty();
        self.invalidate_measure();
    }
    fn reset_template_elements(&self) {
        {
            self.view_manager.on_template_changing();
            self.processing_change
                .set(Some(NotifyCollectionChangedAction::Reset));
            let _guard = CollectionGuard(&self.processing_change);
            let layout = self.effective_layout();
            if layout.is::<VirtualizingLayout>() {
                // The layout recycles its own elements into the OLD factory.
                layout.on_items_changed(&self.layout_context(), &ItemsChangedEventArgs::RESET);
            } else {
                for element in self.realized_elements() {
                    self.clear_element_impl(&element);
                }
            }
            // Reset first releases the layout's references. A keyed source
            // reset or an active removal transition may still own children
            // outside that layout; they must also reach the OLD factory.
            self.view_manager.clear_all(self);
        }
    }
    fn recheck_template_empty(&self) {
        let item = self
            .items_source_view()
            .filter(|items| items.count() > 0)
            .and_then(|items| items.get_at(0));
        let empty = self
            .item_template_shim()
            .is_some_and(|factory| factory.is_template_empty(item.as_ref()));
        self.item_template_empty.set(empty);
    }
    fn on_items_source_changed(&self, args: &ItemsChangedEventArgs<'_>) {
        assert!(
            !self.in_layout.get(),
            "Changes in data source are not allowed during layout."
        );
        assert!(
            self.processing_change.get().is_none(),
            "Changes in the data source are not allowed during another change."
        );
        self.processing_change.set(Some(args.action));
        let _guard = CollectionGuard(&self.processing_change);
        self.transition_manager()
            .on_items_source_changed(args.action);
        self.view_manager.on_items_source_changed(self, args);
        self.effective_layout()
            .on_items_changed(&self.layout_context(), args);
        if self.item_template_empty.get() {
            self.recheck_template_empty();
        }
        self.invalidate_measure();
    }
}

impl Drop for ItemsRepeater {
    fn drop(&mut self) {
        if let (Some(view), Some(token)) =
            (self.items_view.get_mut().take(), self.source_token.take())
        {
            view.remove_collection_changed(token);
        }
        for subscription in self.layout_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
