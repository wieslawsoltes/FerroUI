use super::panel_container_generator::PanelContainerGenerator;
use crate::primitives::{as_logical_scrollable, register_logical_scrollable, ILogicalScrollable};
use crate::templates::ITemplateOf;
use crate::{Control, ControlImpl, ItemsControl, Panel, VirtualizingPanel};
use ferroui_base::input::{IScrollable, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{BringIntoViewRequest, Layoutable, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement, StyledElementImpl, StyledProperty, Vector,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Presents items inside an [`ItemsControl`].
#[repr(C)]
pub struct ItemsPresenter {
    base: Control,
    panel: RefCell<Option<Ref<Panel>>>,
    items_control: RefCell<Option<WeakRef<ItemsControl>>>,
    generator: RefCell<Option<Rc<PanelContainerGenerator>>>,
    /// The panel, when it is a logical scrollable.
    logical_scrollable: RefCell<Option<Ref<Panel>>>,
    logical_scroll_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
    pending_scroll_into_view_index: Cell<i32>,
}

ferro_class!(ItemsPresenter: Control);
ferroui_base::ferro_class_info!(ItemsPresenter { new: ItemsPresenter::new });
ferro_impl_classes!(ItemsPresenter: StyledElementImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for ItemsPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == StyledElement::templated_parent_property().as_property() {
            this.reset_state();
            *this.items_control.borrow_mut() = None;

            let new_value = change.get_new_value::<Option<Ref<FerroObject>>>();
            if let Some(items_control) = new_value.and_then(|v| v.cast::<ItemsControl>()) {
                *this.items_control.borrow_mut() = Some(items_control.downgrade());
                items_control.register_items_presenter(this);
            }
        } else if change.property() == Self::items_panel_property().as_property() {
            this.reset_state();
            this.invalidate_measure();
        }
    }
}

impl VisualImpl for ItemsPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.pending_scroll_into_view_index.get() >= 0 {
            if let Some(root) = this.get_layout_root() {
                let layout_manager = root.layout_manager();
                if let Some(layout_manager) = layout_manager.as_bring_into_view_layout_manager() {
                    layout_manager.enqueue_bring_into_view(this.scroll_into_view_request());
                }
            }
        }
    }
}

impl LayoutableImpl for ItemsPresenter {
    fn apply_template(this: &Self) {
        if this.panel.borrow().is_some() {
            return;
        }
        let Some(items_control) = this.items_control() else { return };

        if let Some(subscription) = this.logical_scroll_subscription.take() {
            subscription.dispose();
        }

        let Some(panel) = this.items_panel().build_typed() else { return };
        *this.panel.borrow_mut() = Some(panel.clone());

        panel.set_templated_parent(this.templated_parent());
        panel.set_is_items_host(true);
        this.logical_children().add(panel.clone().upcast());
        this.visual_children().add(panel.clone().upcast());

        if let Some(v) = panel.downcast_ref::<VirtualizingPanel>() {
            v.attach(&items_control);
        } else {
            this.create_simple_panel_generator();
        }

        let is_logical_scrollable = as_logical_scrollable(&panel).is_some();
        *this.logical_scrollable.borrow_mut() = if is_logical_scrollable { Some(panel.clone()) } else { None };

        if let Some(scrollable) = as_logical_scrollable(&panel) {
            let handlers = Rc::downgrade(&this.scroll_invalidated);
            let subscription = scrollable.scroll_invalidated(Rc::new(move || {
                if let Some(handlers) = handlers.upgrade() {
                    for (_, handler) in handlers.snapshot().iter() {
                        handler();
                    }
                }
            }));
            *this.logical_scroll_subscription.borrow_mut() = Some(subscription);
        }
    }
}

impl InputElementImpl for ItemsPresenter {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl IScrollable for ItemsPresenter {
    fn extent(&self) -> Size {
        self.with_logical_scrollable(|s| s.extent()).unwrap_or_default()
    }

    fn offset(&self) -> Vector {
        self.with_logical_scrollable(|s| s.offset()).unwrap_or_default()
    }

    fn set_offset(&self, value: Vector) {
        self.with_logical_scrollable(|s| s.set_offset(value));
    }

    fn viewport(&self) -> Size {
        self.with_logical_scrollable(|s| s.viewport()).unwrap_or_default()
    }

    fn can_horizontally_scroll(&self) -> bool {
        self.with_logical_scrollable(|s| s.can_horizontally_scroll()).unwrap_or(false)
    }

    fn can_vertically_scroll(&self) -> bool {
        self.with_logical_scrollable(|s| s.can_vertically_scroll()).unwrap_or(false)
    }
}

impl ILogicalScrollable for ItemsPresenter {
    fn set_can_horizontally_scroll(&self, value: bool) {
        self.with_logical_scrollable(|s| s.set_can_horizontally_scroll(value));
    }

    fn set_can_vertically_scroll(&self, value: bool) {
        self.with_logical_scrollable(|s| s.set_can_vertically_scroll(value));
    }

    fn is_logical_scroll_enabled(&self) -> bool {
        self.with_logical_scrollable(|s| s.is_logical_scroll_enabled()).unwrap_or(false)
    }

    fn scroll_size(&self) -> Size {
        self.with_logical_scrollable(|s| s.scroll_size()).unwrap_or_default()
    }

    fn page_scroll_size(&self) -> Size {
        self.with_logical_scrollable(|s| s.page_scroll_size()).unwrap_or_default()
    }

    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.scroll_invalidated.add(handler);
        let handlers = Rc::downgrade(&self.scroll_invalidated);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }

    fn bring_into_view(&self, target: &Ref<Control>, target_rect: Rect) -> bool {
        self.with_logical_scrollable(|s| s.bring_into_view(target, target_rect)).unwrap_or(false)
    }

    fn get_control_in_direction(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.with_logical_scrollable(|s| s.get_control_in_direction(direction, from)).flatten()
    }

    fn raise_scroll_invalidated(&self) {
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }
}

/// The deferred scroll of an items presenter, executed by the layout
/// manager once the layout is stable.
struct ScrollIntoViewRequest {
    presenter: Ref<ItemsPresenter>,
}

impl BringIntoViewRequest for ScrollIntoViewRequest {
    fn target(&self) -> Ref<Layoutable> {
        self.presenter.clone().upcast()
    }

    fn try_execute(&self) -> bool {
        self.presenter.try_execute_pending_scroll_into_view()
    }
}

ferroui_base::ferro_properties! { impl ItemsPresenter {
    ferro_property!(
        /// Defines the `ItemsPanel` property.
        pub fn items_panel_property() -> StyledProperty<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>> {
            ItemsControl::items_panel_property().add_owner::<ItemsPresenter>()
        }
    );
} }

impl ItemsPresenter {
    fn static_constructor() {
        register_logical_scrollable::<ItemsPresenter>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            panel: RefCell::new(None),
            items_control: RefCell::new(None),
            generator: RefCell::new(None),
            logical_scrollable: RefCell::new(None),
            logical_scroll_subscription: RefCell::new(None),
            scroll_invalidated: Rc::new(HandlerList::new()),
            pending_scroll_into_view_index: Cell::new(-1),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets a template which creates the [`Panel`] used to display
    /// the items.
    pub fn items_panel(&self) -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
        self.get_value(Self::items_panel_property())
    }

    pub fn set_items_panel(&self, value: Rc<dyn ITemplateOf<Option<Ref<Panel>>>>) {
        self.set_value(Self::items_panel_property(), value)
    }

    /// Gets the panel used to display the items.
    pub fn panel(&self) -> Option<Ref<Panel>> {
        self.panel.borrow().clone()
    }

    /// Gets the owner [`ItemsControl`].
    pub(crate) fn items_control(&self) -> Option<Ref<ItemsControl>> {
        self.items_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn with_logical_scrollable<R>(&self, f: impl FnOnce(&dyn ILogicalScrollable) -> R) -> Option<R> {
        let panel = self.logical_scrollable.borrow().clone()?;
        let scrollable = as_logical_scrollable(&panel)?;
        Some(f(scrollable))
    }

    fn scroll_into_view_request(&self) -> Rc<dyn BringIntoViewRequest> {
        Rc::new(ScrollIntoViewRequest { presenter: self.to_ref() })
    }

    pub(crate) fn scroll_into_view(&self, index: i32) {
        if index < 0 {
            return;
        }

        // Not attached to a layout root yet: the request will be enqueued
        // when attached to the visual tree.
        let layout_manager = self.get_layout_root().map(|root| root.layout_manager());
        let Some(layout_manager) = layout_manager.as_ref().and_then(|m| m.as_bring_into_view_layout_manager()) else {
            self.pending_scroll_into_view_index.set(index);
            return;
        };

        // Try to execute synchronously when no layout pass is running.
        if !layout_manager.is_in_layout_pass() && self.try_scroll_into_view_now(index) {
            self.pending_scroll_into_view_index.set(-1);
            return;
        }

        // Defer to the end of the layout pass.
        self.pending_scroll_into_view_index.set(index);
        layout_manager.enqueue_bring_into_view(self.scroll_into_view_request());
    }

    pub(crate) fn cancel_scroll_into_view(&self, index: i32) {
        if self.pending_scroll_into_view_index.get() == index {
            self.pending_scroll_into_view_index.set(-1);
        }
    }

    fn try_scroll_into_view_now(&self, index: i32) -> bool {
        if !self.is_effectively_visible() {
            return false;
        }
        let Some(panel) = self.panel() else { return false };

        if let Some(virtualizing_panel) = panel.downcast_ref::<VirtualizingPanel>() {
            return virtualizing_panel.scroll_into_view(index).is_some();
        }

        if (index as usize) < panel.children().count() {
            panel.children().get(index as usize).bring_into_view();
            return true;
        }

        false
    }

    fn try_execute_pending_scroll_into_view(&self) -> bool {
        let index = self.pending_scroll_into_view_index.get();
        if index < 0 {
            return true;
        }

        let panel_valid = self.panel().is_some_and(|panel| panel.is_measure_valid() && panel.is_arrange_valid());
        if !panel_valid || !self.is_effectively_visible() {
            return false;
        }

        self.pending_scroll_into_view_index.set(-1);

        // Try to scroll, but ignore the return value. We don't want to
        // return false if this fails: this would requeue a request that
        // might never been be fulfilled because items changed.
        let _ = self.try_scroll_into_view_now(index);

        // Executing the scroll may have prepared containers whose handlers
        // requested a new scroll: in that case keep the request queued so it
        // gets executed as well.
        self.pending_scroll_into_view_index.get() < 0
    }

    pub(crate) fn refresh(&self) {
        let panel = self.panel();
        if let Some(v) = panel.as_ref().and_then(|p| p.downcast_ref::<VirtualizingPanel>()) {
            v.refresh();
        } else {
            let generator = self.generator.borrow().clone();
            if let Some(generator) = generator {
                generator.refresh();
            }
        }
    }

    fn reset_state(&self) {
        if let Some(generator) = self.generator.take() {
            generator.dispose();
        }
        self.logical_children().clear();
        self.visual_children().clear();
        let panel = self.panel.take();
        if let Some(v) = panel.as_ref().and_then(|p| p.downcast_ref::<VirtualizingPanel>()) {
            v.detach();
        }
    }

    fn create_simple_panel_generator(&self) {
        debug_assert!(!self.panel().is_some_and(|p| p.is::<VirtualizingPanel>()));

        if self.items_control().is_none() || self.panel().is_none() {
            return;
        }

        if let Some(generator) = self.generator.take() {
            generator.dispose();
        }
        let generator = PanelContainerGenerator::new(self);
        *self.generator.borrow_mut() = Some(generator);
    }

    pub(crate) fn container_from_index(&self, index: i32) -> Option<Ref<Control>> {
        let panel = self.panel()?;
        if let Some(v) = panel.downcast_ref::<VirtualizingPanel>() {
            return v.container_from_index(index);
        }
        if index >= 0 {
            panel.children().try_get(index as usize)
        } else {
            None
        }
    }

    pub(crate) fn get_realized_containers(&self) -> Option<Vec<Ref<Control>>> {
        let panel = self.panel()?;
        if let Some(v) = panel.downcast_ref::<VirtualizingPanel>() {
            return v.get_realized_containers();
        }
        Some(panel.children().to_vec())
    }

    pub(crate) fn index_from_container(&self, container: &Ref<Control>) -> i32 {
        let Some(panel) = self.panel() else { return -1 };
        if let Some(v) = panel.downcast_ref::<VirtualizingPanel>() {
            return v.index_from_container(container);
        }
        panel.children().index_of(container).map_or(-1, |index| index as i32)
    }
}
