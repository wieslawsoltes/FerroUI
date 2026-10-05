use crate::generators::{ItemContainerGenerator, RecycleKey};
use crate::items_source::ItemsChangedEventArgs;
use crate::primitives::{register_logical_scrollable, ILogicalScrollable};
use crate::{
    Carousel, Control, ControlImpl, ItemsControl, ItemsSourceView, PanelImpl, VirtualizingPanel,
    VirtualizingPanelImpl, VirtualizingPanelImplExt,
};
use ferroui_base::animation::easings::{Easing, QuadraticEaseOut, SineEaseOut};
use ferroui_base::animation::{
    Animation, Cue, FillMode, IPageTransition, KeyFrame, PageTransitionItem, SlideAxis, TimeSpan,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    IScrollable, InputElement, InputElementImpl, NavigationDirection, SwipeGestureEndedEventArgs,
    SwipeGestureEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{
    CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTask, DispatcherTaskScheduler,
    FerroSynchronizationContext,
};
use ferroui_base::utilities::{HandlerList, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

const SWIPE_COMMIT_THRESHOLD: f64 = 0.25;
const VELOCITY_COMMIT_THRESHOLD: f64 = 800.0;
const MIN_SWIPE_DISTANCE_FOR_VELOCITY_COMMIT: f64 = 0.05;
const RUBBER_BAND_FACTOR: f64 = 0.3;
const RUBBER_BAND_RETURN_DURATION: f64 = 0.16;
const MAX_COMPLETION_DURATION: f64 = 0.35;
const MIN_COMPLETION_DURATION: f64 = 0.12;

/// A container realized for a slot of the fractional viewport.
#[derive(Clone)]
struct ViewportRealizedItem {
    /// The position of the slot, in pages. With wrapping it can lie outside
    /// the range of the items.
    logical_index: i32,
    item_index: i32,
    control: Ref<Control>,
}

/// A slot of the fractional viewport: its position and the index of the
/// item it shows.
type ViewportSlot = (i32, i32);

/// A panel used by [`Carousel`] to display the current item.
#[repr(C)]
pub struct VirtualizingCarouselPanel {
    base: VirtualizingPanel,
    extent: Cell<Size>,
    offset: Cell<Vector>,
    viewport: Cell<Size>,
    recycle_pool: RefCell<HashMap<RecycleKey, Vec<Ref<Control>>>>,
    /// The containers of the fractional viewport, in the order of their
    /// slots.
    viewport_realized: RefCell<Vec<ViewportRealizedItem>>,
    realized: RefCell<Option<Ref<Control>>>,
    realized_index: Cell<i32>,
    transition_from: RefCell<Option<Ref<Control>>>,
    transition_from_index: Cell<i32>,
    transition: RefCell<Option<Rc<CancellationTokenSource>>>,
    transition_task: RefCell<Option<DispatcherTask<()>>>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
    can_horizontally_scroll: Cell<bool>,
    can_vertically_scroll: Cell<bool>,

    swipe_gesture_recognizer: RefCell<Option<Ref<SwipeGestureRecognizer>>>,
    /// The handlers of the swipe gesture events, while they are added.
    swipe_gesture_handler: Cell<Option<RoutedEventHandlerToken>>,
    swipe_gesture_ended_handler: Cell<Option<RoutedEventHandlerToken>>,
    swipe_gesture_id: Cell<i32>,
    is_dragging: Cell<bool>,
    total_delta: Cell<f64>,
    is_forward: Cell<bool>,
    swipe_target: RefCell<Option<Ref<Control>>>,
    swipe_target_index: Cell<i32>,
    swipe_axis: Cell<Option<SlideAxis>>,
    locked_axis: Cell<SlideAxis>,

    completion_cts: RefCell<Option<Rc<CancellationTokenSource>>>,
    offset_animation_cts: RefCell<Option<Rc<CancellationTokenSource>>>,
    completion_end_progress: Cell<f64>,
    is_rubber_banding: Cell<bool>,
    drag_start_offset: Cell<f64>,
    progress_start_offset: Cell<f64>,
    offset_animation_start: Cell<f64>,
    offset_animation_target: Cell<f64>,
    active_viewport_target_offset: Cell<f64>,
    progress_from_index: Cell<i32>,
    progress_to_index: Cell<i32>,
}

ferro_class!(VirtualizingCarouselPanel: VirtualizingPanel);
ferroui_base::ferro_class_info!(VirtualizingCarouselPanel { new: VirtualizingCarouselPanel::new });
ferro_impl_classes!(VirtualizingCarouselPanel: StyledElementImpl, InteractiveImpl, ControlImpl, PanelImpl);

impl FerroObjectImpl for VirtualizingCarouselPanel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::offset_animation_progress_property().as_property() {
            if Self::is_active(&this.offset_animation_cts) {
                let anim_progress = change.get_new_value::<f64>();
                let primary_offset = this.offset_animation_start.get()
                    + ((this.offset_animation_target.get() - this.offset_animation_start.get()) * anim_progress);
                this.set_offset_core(this.with_primary_offset(this.offset.get(), primary_offset));

                if this.uses_viewport_fraction_layout() {
                    let transition = this.get_transition();
                    if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
                        let transition_progress = this.get_fractional_transition_progress(primary_offset);
                        progressive.update(
                            transition_progress,
                            Self::as_visual(this.find_viewport_control(this.progress_from_index.get())).as_ref(),
                            Self::as_visual(this.find_viewport_control(this.progress_to_index.get())).as_ref(),
                            this.is_forward.get(),
                            this.get_viewport_item_extent(this.bounds().size()),
                            &this.build_fractional_visible_items(primary_offset),
                        );
                    }
                }
            }
        } else if change.property() == Self::completion_progress_property().as_property() {
            let is_completion_animating = Self::is_active(&this.completion_cts);

            if !this.is_dragging.get() && this.swipe_target().is_none() && !is_completion_animating {
                return;
            }

            let progress = change.get_new_value::<f64>();
            let transition = this.get_transition();
            if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
                let realized = this.realized();
                let swipe_target = this.swipe_target().filter(|target| realized.as_ref() != Some(target));
                let size = this.locked_axis_size();
                progressive.update(
                    progress,
                    Self::as_visual(realized).as_ref(),
                    Self::as_visual(swipe_target).as_ref(),
                    this.is_forward.get(),
                    size,
                    &[],
                );
            }
        }
    }
}

impl VisualImpl for VirtualizingCarouselPanel {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.refresh_gesture_recognizer();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.teardown_gesture_recognizer();
    }
}

impl LayoutableImpl for VirtualizingCarouselPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.uses_viewport_fraction_layout() {
            return this.measure_viewport_fraction_override(available_size);
        }

        this.clear_viewport_realized();
        this.cancel_offset_animation();

        this.measure_single_page_override(available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if this.uses_viewport_fraction_layout() {
            return this.arrange_viewport_fraction_override(final_size);
        }

        // The single page arrange.
        let result = Self::parent_arrange_override(this, final_size);

        if this.transition.borrow().is_none() {
            let transition_from = this.transition_from();
            let to = this.realized();
            let transition = this.get_transition();

            if let (Some(transition_from), Some(to), Some(transition)) = (transition_from, to, transition) {
                let transition_cts = Rc::new(CancellationTokenSource::new());
                *this.transition.borrow_mut() = Some(transition_cts.clone());

                let forward = this.realized_index.get() > this.transition_from_index.get();

                let task = this.run_transition_async(transition_cts, transition_from, to, forward, transition);
                *this.transition_task.borrow_mut() = Some(task);
            }
        }

        result
    }
}

impl InputElementImpl for VirtualizingCarouselPanel {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl VirtualizingPanelImpl for VirtualizingCarouselPanel {
    fn on_items_control_changed(this: &Self, old_value: Option<&Ref<ItemsControl>>) {
        Self::parent_on_items_control_changed(this, old_value);
        // The items presenter attaches the panel to the visual tree before
        // attaching it to the items control, so the attachment to the visual
        // tree is notified with no items control; re-run setup here.
        this.refresh_gesture_recognizer();
    }

    fn get_control_in_direction(
        _this: &Self,
        _direction: NavigationDirection,
        _from: Option<&Ref<InputElement>>,
        _wrap: bool,
    ) -> Option<Ref<InputElement>> {
        None
    }

    fn container_from_index(this: &Self, index: i32) -> Option<Ref<Control>> {
        let items = this.items();

        if index < 0 || index as usize >= items.count() {
            return None;
        }
        if let Some(viewport_realized) = this.find_viewport_control(index) {
            return Some(viewport_realized);
        }
        if index == this.realized_index.get() {
            return this.realized();
        }
        if let Some(c) = items.get_at(index as usize).as_ref().and_then(Control::from_boxed) {
            if c.get_value(Self::recycle_key_property()) == Some(Self::item_is_its_own_container()) {
                return Some(c);
            }
        }
        None
    }

    fn get_realized_containers(this: &Self) -> Option<Vec<Ref<Control>>> {
        {
            let viewport_realized = this.viewport_realized.borrow();
            if !viewport_realized.is_empty() {
                return Some(viewport_realized.iter().map(|x| x.control.clone()).collect());
            }
        }

        this.realized().map(|realized| vec![realized])
    }

    fn index_from_container(this: &Self, container: &Ref<Control>) -> i32 {
        {
            let viewport_realized = this.viewport_realized.borrow();
            if let Some(entry) = viewport_realized.iter().find(|entry| entry.control == *container) {
                return entry.item_index;
            }
        }

        if this.realized.borrow().as_ref() == Some(container) {
            this.realized_index.get()
        } else {
            -1
        }
    }

    fn scroll_into_view(_this: &Self, _index: i32) -> Option<Ref<Control>> {
        None
    }

    fn on_items_changed(this: &Self, items: &Rc<ItemsSourceView>, e: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_changed(this, items, e);

        if this.uses_viewport_fraction_layout() || !this.viewport_realized.borrow().is_empty() {
            this.clear_viewport_realized();
            this.invalidate_measure();
            return;
        }

        // Returns whether the handling of the change is complete.
        let add = |index: i32, count: i32| -> bool {
            if this.realized.borrow().is_none() {
                this.invalidate_measure();
                return true;
            }

            if index <= this.realized_index.get() {
                this.realized_index.set(this.realized_index.get() + count);
            }
            false
        };

        let remove = |index: i32, count: i32| {
            let end = index + (count - 1);
            let realized = this.realized();
            let realized_index = this.realized_index.get();

            match realized {
                Some(realized) if index <= realized_index && end >= realized_index => {
                    this.recycle_element(&realized);
                    *this.realized.borrow_mut() = None;
                    this.realized_index.set(-1);
                }
                _ => {
                    if index < realized_index {
                        this.realized_index.set(realized_index - count);
                    }
                }
            }
        };

        let new_count = e.new_items.len() as i32;
        let old_count = e.old_items.len() as i32;

        let mut action = e.action;
        if matches!(action, NotifyCollectionChangedAction::Replace | NotifyCollectionChangedAction::Move)
            && e.old_starting_index < 0
        {
            action = NotifyCollectionChangedAction::Reset;
        }

        match action {
            NotifyCollectionChangedAction::Add => {
                if add(e.new_starting_index, new_count) {
                    return;
                }
            }
            NotifyCollectionChangedAction::Remove => {
                remove(e.old_starting_index, old_count);
            }
            NotifyCollectionChangedAction::Replace => {
                remove(e.old_starting_index, old_count);
                if add(e.new_starting_index, new_count) {
                    return;
                }
            }
            NotifyCollectionChangedAction::Move => {
                remove(e.old_starting_index, old_count);
                let mut insert_index = e.new_starting_index;

                if e.new_starting_index > e.old_starting_index {
                    insert_index -= old_count - 1;
                }

                if add(insert_index, new_count) {
                    return;
                }
            }
            NotifyCollectionChangedAction::Reset => {
                let realized = this.realized();
                if let Some(realized) = realized {
                    this.recycle_element(&realized);
                    *this.realized.borrow_mut() = None;
                    this.realized_index.set(-1);
                }
            }
        }

        this.invalidate_measure();
    }
}

impl IScrollable for VirtualizingCarouselPanel {
    fn extent(&self) -> Size {
        self.extent.get()
    }

    fn offset(&self) -> Vector {
        self.offset.get()
    }

    fn set_offset(&self, value: Vector) {
        self.set_offset_core(value)
    }

    fn viewport(&self) -> Size {
        self.viewport.get()
    }

    fn can_horizontally_scroll(&self) -> bool {
        self.can_horizontally_scroll.get()
    }

    fn can_vertically_scroll(&self) -> bool {
        self.can_vertically_scroll.get()
    }
}

impl ILogicalScrollable for VirtualizingCarouselPanel {
    fn set_can_horizontally_scroll(&self, value: bool) {
        self.can_horizontally_scroll.set(value)
    }

    fn set_can_vertically_scroll(&self, value: bool) {
        self.can_vertically_scroll.set(value)
    }

    fn is_logical_scroll_enabled(&self) -> bool {
        true
    }

    fn scroll_size(&self) -> Size {
        Size::new(1.0, 1.0)
    }

    fn page_scroll_size(&self) -> Size {
        Size::new(1.0, 1.0)
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

    fn bring_into_view(&self, _target: &Ref<Control>, _target_rect: Rect) -> bool {
        false
    }

    fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        None
    }

    fn raise_scroll_invalidated(&self) {
        self.invoke_scroll_invalidated();
    }
}

/// Clears a cancellation source field when an asynchronous method is left,
/// if the field still holds the source of that call.
struct ClearSourceOnExit {
    panel: WeakRef<VirtualizingCarouselPanel>,
    field: fn(&VirtualizingCarouselPanel) -> &RefCell<Option<Rc<CancellationTokenSource>>>,
    source: Rc<CancellationTokenSource>,
}

impl Drop for ClearSourceOnExit {
    fn drop(&mut self) {
        if let Some(panel) = self.panel.upgrade() {
            let field = (self.field)(&panel);
            let is_current = field.borrow().as_ref().is_some_and(|current| Rc::ptr_eq(current, &self.source));
            if is_current {
                *field.borrow_mut() = None;
            }
        }
    }
}

/// Waits for a page transition to end. An interrupted or failed transition
/// ends the wait like a completed one: an interruption is expected when a
/// transition is superseded by a newer navigation action.
struct TransitionEnd {
    task: DispatcherTask<()>,
}

impl Future for TransitionEnd {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // The failure of a transition is not re-raised.
        if self.task.is_faulted() {
            return Poll::Ready(());
        }

        match Pin::new(&mut self.task).poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

ferroui_base::ferro_properties! { impl VirtualizingCarouselPanel {
    ferro_property!(
        fn recycle_key_property() -> AttachedProperty<Option<RecycleKey>> {
            FerroProperty::register_attached::<VirtualizingCarouselPanel, Control, _>("RecycleKey", None)
        }
    );

    ferro_property!(
        fn completion_progress_property() -> StyledProperty<f64> {
            FerroProperty::register::<VirtualizingCarouselPanel, _>("CompletionProgress", 0.0)
        }
    );

    ferro_property!(
        fn offset_animation_progress_property() -> StyledProperty<f64> {
            FerroProperty::register::<VirtualizingCarouselPanel, _>("OffsetAnimationProgress", 0.0)
        }
    );
} }

impl VirtualizingCarouselPanel {
    /// The recycle key that marks an item which is its own container.
    fn item_is_its_own_container() -> RecycleKey {
        thread_local! {
            static KEY: RecycleKey = RecycleKey::new_unique();
        }
        KEY.with(|key| *key)
    }

    fn static_constructor() {
        register_logical_scrollable::<VirtualizingCarouselPanel>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: VirtualizingPanel::construct(),
            extent: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            viewport: Cell::new(Size::default()),
            recycle_pool: RefCell::new(HashMap::new()),
            viewport_realized: RefCell::new(Vec::new()),
            realized: RefCell::new(None),
            realized_index: Cell::new(-1),
            transition_from: RefCell::new(None),
            transition_from_index: Cell::new(-1),
            transition: RefCell::new(None),
            transition_task: RefCell::new(None),
            scroll_invalidated: Rc::new(HandlerList::new()),
            can_horizontally_scroll: Cell::new(false),
            can_vertically_scroll: Cell::new(false),
            swipe_gesture_recognizer: RefCell::new(None),
            swipe_gesture_handler: Cell::new(None),
            swipe_gesture_ended_handler: Cell::new(None),
            swipe_gesture_id: Cell::new(0),
            is_dragging: Cell::new(false),
            total_delta: Cell::new(0.0),
            is_forward: Cell::new(false),
            swipe_target: RefCell::new(None),
            swipe_target_index: Cell::new(-1),
            swipe_axis: Cell::new(None),
            locked_axis: Cell::new(SlideAxis::Horizontal),
            completion_cts: RefCell::new(None),
            offset_animation_cts: RefCell::new(None),
            completion_end_progress: Cell::new(0.0),
            is_rubber_banding: Cell::new(false),
            drag_start_offset: Cell::new(0.0),
            progress_start_offset: Cell::new(0.0),
            offset_animation_start: Cell::new(0.0),
            offset_animation_target: Cell::new(0.0),
            active_viewport_target_offset: Cell::new(0.0),
            progress_from_index: Cell::new(-1),
            progress_to_index: Cell::new(-1),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn is_managing_interaction_offset(&self) -> bool {
        self.uses_viewport_fraction_layout() && (self.is_dragging.get() || Self::is_active(&self.offset_animation_cts))
    }

    /// Whether the field holds a cancellation source that has not been
    /// cancelled.
    #[inline]
    fn is_active(source: &RefCell<Option<Rc<CancellationTokenSource>>>) -> bool {
        source.borrow().as_ref().is_some_and(|source| !source.is_cancellation_requested())
    }

    /// The carousel that the panel is displaying items for.
    #[inline]
    fn carousel(&self) -> Option<Ref<Carousel>> {
        self.items_control().and_then(|items_control| items_control.cast::<Carousel>())
    }

    #[inline]
    fn realized(&self) -> Option<Ref<Control>> {
        self.realized.borrow().clone()
    }

    #[inline]
    fn transition_from(&self) -> Option<Ref<Control>> {
        self.transition_from.borrow().clone()
    }

    #[inline]
    fn swipe_target(&self) -> Option<Ref<Control>> {
        self.swipe_target.borrow().clone()
    }

    #[inline]
    fn as_visual(control: Option<Ref<Control>>) -> Option<Ref<Visual>> {
        control.map(Ref::upcast)
    }

    /// The generator of the items control. Panics if the panel does not
    /// belong to an items control.
    #[inline]
    fn generator(&self) -> Rc<ItemContainerGenerator> {
        match self.item_container_generator() {
            Some(generator) => generator,
            None => panic!("The VirtualizingPanel does not belong to an ItemsControl."),
        }
    }

    /// The number of items.
    #[inline]
    fn item_count(&self) -> i32 {
        self.items().count() as i32
    }

    fn set_extent(&self, value: Size) {
        if self.extent.get() != value {
            self.extent.set(value);
            self.invoke_scroll_invalidated();
        }
    }

    fn set_viewport(&self, value: Size) {
        if self.viewport.get() != value {
            self.viewport.set(value);
            self.invoke_scroll_invalidated();
        }
    }

    fn invoke_scroll_invalidated(&self) {
        if self.scroll_invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }

    fn uses_viewport_fraction_layout(&self) -> bool {
        self.carousel().is_some_and(|carousel| !MathUtilities::are_close(carousel.viewport_fraction(), 1.0))
    }

    fn get_layout_axis(&self) -> SlideAxis {
        self.carousel().map_or(SlideAxis::Horizontal, |carousel| carousel.get_layout_axis())
    }

    fn get_viewport_fraction(&self) -> f64 {
        self.carousel().map_or(1.0, |carousel| carousel.viewport_fraction())
    }

    fn get_viewport_units(&self) -> f64 {
        1.0 / self.get_viewport_fraction()
    }

    fn get_primary_offset(&self, offset: Vector) -> f64 {
        if self.get_layout_axis() == SlideAxis::Vertical {
            offset.y
        } else {
            offset.x
        }
    }

    fn get_primary_size(&self, size: Size) -> f64 {
        if self.get_layout_axis() == SlideAxis::Vertical {
            size.height
        } else {
            size.width
        }
    }

    fn get_cross_size(&self, size: Size) -> f64 {
        if self.get_layout_axis() == SlideAxis::Vertical {
            size.width
        } else {
            size.height
        }
    }

    fn create_logical_size(&self, primary: f64) -> Size {
        if self.get_layout_axis() == SlideAxis::Vertical {
            Size::new(1.0, primary)
        } else {
            Size::new(primary, 1.0)
        }
    }

    fn create_item_size(&self, primary: f64, cross: f64) -> Size {
        if self.get_layout_axis() == SlideAxis::Vertical {
            Size::new(cross, primary)
        } else {
            Size::new(primary, cross)
        }
    }

    fn create_item_rect(&self, primary_offset: f64, primary_size: f64, cross_size: f64) -> Rect {
        if self.get_layout_axis() == SlideAxis::Vertical {
            Rect::new(0.0, primary_offset, cross_size, primary_size)
        } else {
            Rect::new(primary_offset, 0.0, primary_size, cross_size)
        }
    }

    fn with_primary_offset(&self, offset: Vector, primary: f64) -> Vector {
        if self.get_layout_axis() == SlideAxis::Vertical {
            Vector::new(offset.x, primary)
        } else {
            Vector::new(primary, offset.y)
        }
    }

    /// The size of the panel along the axis that the swipe is locked to.
    fn locked_axis_size(&self) -> f64 {
        if self.locked_axis.get() == SlideAxis::Horizontal {
            self.bounds().width
        } else {
            self.bounds().height
        }
    }

    fn resolve_layout_size(&self, available_size: Size) -> Size {
        let owner = self.items_control();

        fn resolve_dimension(available: f64, bounds: f64, owner_bounds: f64, owner_explicit: f64) -> f64 {
            if !available.is_infinite() && available > 0.0 {
                return available;
            }

            if bounds > 0.0 {
                return bounds;
            }

            if owner_bounds > 0.0 {
                return owner_bounds;
            }

            if owner_explicit.is_nan() {
                0.0
            } else {
                owner_explicit
            }
        }

        let bounds = self.bounds();
        let owner_bounds = owner.as_ref().map(|owner| owner.bounds());
        let width = resolve_dimension(
            available_size.width,
            bounds.width,
            owner_bounds.map_or(0.0, |bounds| bounds.width),
            owner.as_ref().map_or(f64::NAN, |owner| owner.width()),
        );
        let height = resolve_dimension(
            available_size.height,
            bounds.height,
            owner_bounds.map_or(0.0, |bounds| bounds.height),
            owner.as_ref().map_or(f64::NAN, |owner| owner.height()),
        );
        Size::new(width, height)
    }

    fn get_viewport_item_extent(&self, size: Size) -> f64 {
        let viewport_units = self.get_viewport_units();
        if viewport_units <= 0.0 {
            0.0
        } else {
            self.get_primary_size(size) / viewport_units
        }
    }

    fn uses_viewport_wrap_layout(&self) -> bool {
        self.uses_viewport_fraction_layout()
            && self.carousel().is_some_and(|carousel| carousel.wrap_selection())
            && self.item_count() > 1
    }

    fn normalize_index(index: i32, count: i32) -> i32 {
        ((index % count) + count) % count
    }

    fn get_nearest_logical_offset(&self, item_index: i32, reference_offset: f64) -> f64 {
        let count = self.item_count();

        if !self.uses_viewport_wrap_layout() || count == 0 {
            return item_index.clamp(0, (count - 1).max(0)) as f64;
        }

        let wrap_span = count as f64;
        let wrap_multiplier = ((reference_offset - item_index as f64) / wrap_span).round_ties_even();
        item_index as f64 + (wrap_multiplier * wrap_span)
    }

    fn is_preferred_viewport_slot(
        &self,
        candidate_logical_index: i32,
        existing_logical_index: i32,
        primary_offset: f64,
    ) -> bool {
        let candidate_distance = (candidate_logical_index as f64 - primary_offset).abs();
        let existing_distance = (existing_logical_index as f64 - primary_offset).abs();

        if !MathUtilities::are_close(candidate_distance, existing_distance) {
            return candidate_distance < existing_distance;
        }

        let count = self.item_count();
        let candidate_in_range = candidate_logical_index >= 0 && candidate_logical_index < count;
        let existing_in_range = existing_logical_index >= 0 && existing_logical_index < count;

        if candidate_in_range != existing_in_range {
            return candidate_in_range;
        }

        if self.is_dragging.get() {
            return if self.is_forward.get() {
                candidate_logical_index > existing_logical_index
            } else {
                candidate_logical_index < existing_logical_index
            };
        }

        candidate_logical_index < existing_logical_index
    }

    /// The slots that the viewport shows at the offset, ordered by position.
    fn get_required_viewport_slots(&self, primary_offset: f64) -> Vec<ViewportSlot> {
        let count = self.item_count();

        if count == 0 {
            return Vec::new();
        }

        let viewport_units = self.get_viewport_units();
        let edge_inset = (viewport_units - 1.0) / 2.0;
        let mut start = (primary_offset - edge_inset).floor() as i32;
        let mut end = (primary_offset + viewport_units - edge_inset).ceil() as i32 - 1;

        if !self.uses_viewport_wrap_layout() {
            start = start.max(0);
            end = end.min(count - 1);

            if start > end {
                return Vec::new();
            }

            return (start..=end).map(|index| (index, index)).collect();
        }

        // The best slot of each item.
        let mut best_slots: Vec<ViewportSlot> = Vec::new();

        for logical_index in start..=end {
            let item_index = Self::normalize_index(logical_index, count);

            match best_slots.iter().position(|slot| slot.1 == item_index) {
                None => best_slots.push((logical_index, item_index)),
                Some(existing) => {
                    if self.is_preferred_viewport_slot(logical_index, best_slots[existing].0, primary_offset) {
                        best_slots[existing].0 = logical_index;
                    }
                }
            }
        }

        best_slots.sort_by_key(|slot| slot.0);
        best_slots
    }

    fn viewport_slots_changed(&self, old_primary_offset: f64, new_primary_offset: f64) -> bool {
        let old_slots = self.get_required_viewport_slots(old_primary_offset);
        let new_slots = self.get_required_viewport_slots(new_primary_offset);

        old_slots != new_slots
    }

    /// Sets the scroll offset.
    fn set_offset_core(&self, value: Vector) {
        if self.uses_viewport_fraction_layout() {
            let old_primary_offset = self.get_primary_offset(self.offset.get());
            let new_primary_offset = self.get_primary_offset(value);

            if MathUtilities::are_close(old_primary_offset, new_primary_offset) {
                self.offset.set(value);
                return;
            }

            self.offset.set(value);

            let range_changed = self.viewport_slots_changed(old_primary_offset, new_primary_offset);

            if range_changed {
                self.invalidate_measure();
            } else {
                self.invalidate_arrange();
            }

            self.invoke_scroll_invalidated();
            return;
        }

        if (self.offset.get().x as i32) as f64 != value.x {
            self.invalidate_measure();
        }

        self.offset.set(value);
    }

    /// The container at the position in the fractional viewport, if there
    /// is one.
    #[inline]
    fn viewport_realized_at(&self, position: usize) -> Option<ViewportRealizedItem> {
        self.viewport_realized.borrow().get(position).cloned()
    }

    fn clear_viewport_realized(&self) {
        if self.viewport_realized.borrow().is_empty() {
            return;
        }

        let mut position = 0;
        while let Some(entry) = self.viewport_realized_at(position) {
            self.recycle_element(&entry.control);
            position += 1;
        }

        self.viewport_realized.borrow_mut().clear();
    }

    fn reset_single_page_state(&self) {
        let transition = self.transition.borrow().clone();
        if let Some(transition) = transition {
            transition.cancel();
        }
        *self.transition.borrow_mut() = None;
        *self.transition_task.borrow_mut() = None;

        if let Some(transition_from) = self.transition_from() {
            self.recycle_element(&transition_from);
        }

        if let Some(swipe_target) = self.swipe_target() {
            self.recycle_element(&swipe_target);
        }

        if let Some(realized) = self.realized() {
            self.recycle_element(&realized);
        }

        *self.transition_from.borrow_mut() = None;
        self.transition_from_index.set(-1);
        *self.swipe_target.borrow_mut() = None;
        self.swipe_target_index.set(-1);
        *self.realized.borrow_mut() = None;
        self.realized_index.set(-1);
    }

    fn cancel_offset_animation(&self) {
        let offset_animation_cts = self.offset_animation_cts.borrow().clone();
        if let Some(offset_animation_cts) = offset_animation_cts {
            offset_animation_cts.cancel();
        }
        *self.offset_animation_cts.borrow_mut() = None;
    }

    fn measure_single_page_override(&self, available_size: Size) -> Size {
        let items = self.items();
        let item_count = items.count() as i32;
        let index = match self.carousel() {
            Some(carousel) => carousel.selected_index(),
            None => self.offset.get().x as i32,
        };

        self.complete_finished_transition_if_needed();

        if index != self.realized_index.get() {
            if let Some(realized) = self.realized() {
                // Cancel any already running transition, and recycle the
                // element we're transitioning from.
                let transition = self.transition.borrow().clone();
                if let Some(transition) = transition {
                    transition.cancel();
                    *self.transition.borrow_mut() = None;
                    *self.transition_task.borrow_mut() = None;
                    if let Some(transition_from) = self.transition_from() {
                        self.recycle_element(&transition_from);
                    }
                    *self.transition_from.borrow_mut() = None;
                    self.transition_from_index.set(-1);
                    self.reset_transition_state(Some(&realized));
                }

                if self.get_transition().is_none() {
                    self.recycle_element(&realized);
                } else {
                    // Record the current element as the element we're
                    // transitioning from and we'll start the transition in
                    // the arrange pass.
                    *self.transition_from.borrow_mut() = Some(realized);
                    self.transition_from_index.set(self.realized_index.get());
                }

                *self.realized.borrow_mut() = None;
                self.realized_index.set(-1);
            }

            if index >= 0 && index < item_count {
                let element = self.get_or_create_element(&items, index);
                *self.realized.borrow_mut() = Some(element);
                self.realized_index.set(index);
            }
        }

        let Some(realized) = self.realized() else {
            self.set_viewport(Size::new(0.0, 0.0));
            self.set_extent(Size::new(0.0, 0.0));
            *self.transition_from.borrow_mut() = None;
            self.transition_from_index.set(-1);
            return Size::default();
        };

        realized.measure(available_size);
        self.set_extent(Size::new(item_count as f64, 1.0));
        self.set_viewport(Size::new(1.0, 1.0));

        realized.desired_size()
    }

    fn measure_viewport_fraction_override(&self, available_size: Size) -> Size {
        self.reset_single_page_state();

        let items = self.items();
        let item_count = items.count() as i32;

        if item_count == 0 {
            self.clear_viewport_realized();
            self.set_viewport(Size::new(0.0, 0.0));
            self.set_extent(Size::new(0.0, 0.0));
            return Size::default();
        }

        let layout_size = self.resolve_layout_size(available_size);
        let primary_size = self.get_primary_size(layout_size);
        let cross_size = self.get_cross_size(layout_size);
        let viewport_units = self.get_viewport_units();

        if primary_size <= 0.0 || viewport_units <= 0.0 {
            self.clear_viewport_realized();
            self.set_viewport(Size::new(0.0, 0.0));
            self.set_extent(Size::new(0.0, 0.0));
            return Size::default();
        }

        let item_primary_size = primary_size / viewport_units;
        let item_size = self.create_item_size(item_primary_size, cross_size);
        let required_slots = self.get_required_viewport_slots(self.get_primary_offset(self.offset.get()));

        // Recycle the containers of the slots that are no longer required.
        let mut position = 0;
        while let Some(entry) = self.viewport_realized_at(position) {
            let required =
                required_slots.iter().any(|slot| slot.0 == entry.logical_index && slot.1 == entry.item_index);

            if required {
                position += 1;
            } else {
                self.recycle_element(&entry.control);

                let mut viewport_realized = self.viewport_realized.borrow_mut();
                match viewport_realized.iter().position(|x| x.logical_index == entry.logical_index) {
                    Some(removed) => {
                        viewport_realized.remove(removed);
                        if removed > position {
                            position += 1;
                        }
                    }
                    None => position += 1,
                }
            }
        }

        for &(logical_index, item_index) in &required_slots {
            let is_realized = self.viewport_realized.borrow().iter().any(|x| x.logical_index == logical_index);

            if !is_realized {
                let control = self.get_or_create_element(&items, item_index);
                let mut viewport_realized = self.viewport_realized.borrow_mut();

                match viewport_realized.binary_search_by_key(&logical_index, |x| x.logical_index) {
                    Ok(existing) => viewport_realized[existing] = ViewportRealizedItem { logical_index, item_index, control },
                    Err(insert_at) => {
                        viewport_realized.insert(insert_at, ViewportRealizedItem { logical_index, item_index, control })
                    }
                }
            }
        }

        let mut max_cross_desired_size: f64 = 0.0;

        let mut position = 0;
        while let Some(entry) = self.viewport_realized_at(position) {
            entry.control.measure(item_size);
            max_cross_desired_size = max_cross_desired_size.max(self.get_cross_size(entry.control.desired_size()));
            position += 1;
        }

        self.set_viewport(self.create_logical_size(viewport_units));
        self.set_extent(self.create_logical_size((item_count as f64 + viewport_units - 1.0).max(0.0)));

        let desired_primary = if primary_size.is_infinite() { item_primary_size * viewport_units } else { primary_size };
        let desired_cross = if cross_size.is_infinite() { max_cross_desired_size } else { cross_size };
        self.create_item_size(desired_primary, desired_cross)
    }

    fn arrange_viewport_fraction_override(&self, final_size: Size) -> Size {
        let primary_size = self.get_primary_size(final_size);
        let cross_size = self.get_cross_size(final_size);
        let viewport_units = self.get_viewport_units();

        if primary_size <= 0.0 || viewport_units <= 0.0 {
            return final_size;
        }

        if self.viewport_realized.borrow().is_empty() && self.item_count() > 0 {
            self.invalidate_measure();
            return final_size;
        }

        let item_primary_size = primary_size / viewport_units;
        let edge_inset = (viewport_units - 1.0) / 2.0;
        let primary_offset = self.get_primary_offset(self.offset.get());

        let mut position = 0;
        while let Some(entry) = self.viewport_realized_at(position) {
            let item_offset = (edge_inset + entry.logical_index as f64 - primary_offset) * item_primary_size;
            let rect = self.create_item_rect(item_offset, item_primary_size, cross_size);
            entry.control.set_is_visible(true);
            entry.control.arrange(rect);
            position += 1;
        }

        final_size
    }

    fn get_or_create_element(&self, items: &ItemsSourceView, index: i32) -> Ref<Control> {
        if let Some(element) = self.get_realized_element(index) {
            return element;
        }

        let item = items.get_at(index as usize);
        let generator = self.generator();
        let (needs_container, recycle_key) = generator.needs_container(&item, index);

        if needs_container {
            match self.get_recycled_element(&generator, &item, index, recycle_key) {
                Some(recycled) => recycled,
                None => self.create_element(&generator, &item, index, recycle_key),
            }
        } else {
            self.get_item_as_own_container(&generator, &item, index)
        }
    }

    fn get_realized_element(&self, index: i32) -> Option<Ref<Control>> {
        if let Some(viewport_realized) = self.find_viewport_control(index) {
            return Some(viewport_realized);
        }

        if self.realized_index.get() == index {
            self.realized()
        } else {
            None
        }
    }

    fn get_item_as_own_container(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
    ) -> Ref<Control> {
        let control_item = match item.as_ref().and_then(Control::from_boxed) {
            Some(control_item) => control_item,
            None => panic!("An item that is its own container must be a control."),
        };

        if !control_item.is_set(Self::recycle_key_property().as_property()) {
            generator.prepare_item_container(&control_item, item, index);
            self.add_internal_child(&control_item);
            control_item.set_value(Self::recycle_key_property(), Some(Self::item_is_its_own_container()));
            generator.item_container_prepared(&control_item, item, index);
        }

        control_item.set_is_visible(true);
        control_item
    }

    fn get_recycled_element(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Option<Ref<Control>> {
        let recycle_key = recycle_key?;

        let recycled = self.recycle_pool.borrow_mut().get_mut(&recycle_key).and_then(Vec::pop)?;

        recycled.set_is_visible(true);
        generator.prepare_item_container(&recycled, item, index);
        generator.item_container_prepared(&recycled, item, index);
        Some(recycled)
    }

    fn create_element(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        let container = generator.create_container(item, index, recycle_key);

        container.set_value(Self::recycle_key_property(), recycle_key);
        generator.prepare_item_container(&container, item, index);
        self.add_internal_child(&container);
        generator.item_container_prepared(&container, item, index);

        container
    }

    fn recycle_element(&self, element: &Ref<Control>) {
        let recycle_key = element.get_value(Self::recycle_key_property());
        debug_assert!(recycle_key.is_some());

        // Hide first so cleanup doesn't visibly snap transforms/opacity for
        // a frame.
        element.set_is_visible(false);
        self.reset_transition_state(Some(element));

        if recycle_key == Some(Self::item_is_its_own_container()) {
            return;
        }

        self.generator().clear_item_container(element);

        let Some(recycle_key) = recycle_key else {
            panic!("The recycle key of a container of the carousel panel cannot be null.");
        };

        self.recycle_pool.borrow_mut().entry(recycle_key).or_default().push(element.clone());
    }

    fn get_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.carousel().and_then(|carousel| carousel.page_transition())
    }

    fn complete_finished_transition_if_needed(&self) {
        let is_task_completed = self.transition_task.borrow().as_ref().is_some_and(|task| task.is_completed());

        if self.transition.borrow().is_some() && is_task_completed {
            if let Some(transition_from) = self.transition_from() {
                self.recycle_element(&transition_from);
            }

            *self.transition.borrow_mut() = None;
            *self.transition_task.borrow_mut() = None;
            *self.transition_from.borrow_mut() = None;
            self.transition_from_index.set(-1);
        }
    }

    /// The scheduler that runs the asynchronous methods of the panel: the
    /// one of the current synchronization context.
    fn task_scheduler() -> DispatcherTaskScheduler {
        let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
            FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
        });
        context.to_task_scheduler()
    }

    fn run_transition_async(
        &self,
        transition_cts: Rc<CancellationTokenSource>,
        transition_from: Ref<Control>,
        transition_to: Ref<Control>,
        forward: bool,
        transition: Rc<dyn IPageTransition>,
    ) -> DispatcherTask<()> {
        let this = self.to_ref().downgrade();

        Self::task_scheduler().start_local(async move {
            let from: Ref<Visual> = transition_from.upcast();
            let to: Ref<Visual> = transition_to.upcast();
            let task = transition.start(Some(&from), Some(&to), forward, transition_cts.token());

            // An interruption is expected when a transition is interrupted
            // by a newer navigation action.
            TransitionEnd { task }.await;

            let Some(this) = this.upgrade() else {
                return;
            };

            let is_current =
                this.transition.borrow().as_ref().is_some_and(|current| Rc::ptr_eq(current, &transition_cts));
            if transition_cts.is_cancellation_requested() || !is_current {
                return;
            }

            if let Some(transition_from) = this.transition_from() {
                this.recycle_element(&transition_from);
            }
            *this.transition.borrow_mut() = None;
            *this.transition_task.borrow_mut() = None;
            *this.transition_from.borrow_mut() = None;
            this.transition_from_index.set(-1);
        })
    }

    pub(crate) fn sync_selection_offset(&self, selected_index: i32) {
        if !self.uses_viewport_fraction_layout() {
            self.set_offset_core(self.with_primary_offset(self.offset.get(), selected_index as f64));
            return;
        }

        let current_offset = self.get_primary_offset(self.offset.get());
        let target_offset = self.get_nearest_logical_offset(selected_index, current_offset);

        if MathUtilities::are_close(current_offset, target_offset) {
            self.set_offset_core(self.with_primary_offset(self.offset.get(), target_offset));
            return;
        }

        if self.is_dragging.get() {
            return;
        }

        let transition = self.get_transition();
        let can_animate = transition.is_some() && (target_offset - current_offset).abs() <= 1.001;

        if !can_animate {
            self.reset_viewport_transition_state();
            self.clear_fractional_progress_context();
            self.set_offset_core(self.with_primary_offset(self.offset.get(), target_offset));
            return;
        }

        let item_count = self.item_count();
        let from_index = if item_count > 0 {
            Self::normalize_index(current_offset.round_ties_even() as i32, item_count)
        } else {
            -1
        };
        let forward = target_offset > current_offset;

        self.reset_viewport_transition_state();
        self.set_fractional_progress_context(from_index, selected_index, forward, current_offset, target_offset);
        self.animate_viewport_offset_async(
            current_offset,
            target_offset,
            TimeSpan::from_seconds(MAX_COMPLETION_DURATION),
            QuadraticEaseOut::new().into(),
            |this| {
                this.reset_viewport_transition_state();
                this.clear_fractional_progress_context();
                // The carousel does not sync the scroll offset during the
                // animation and the layout after the animation still sees a
                // live cancellation source, so re-sync explicitly in case
                // the selected index changed.
                if let Some(carousel) = this.carousel() {
                    this.sync_selection_offset(carousel.selected_index());
                }
            },
        );
    }

    /// Refreshes the gesture recognizer based on the carousel's
    /// `is_swipe_enabled` and `page_transition` settings.
    pub(crate) fn refresh_gesture_recognizer(&self) {
        self.teardown_gesture_recognizer();

        let Some(carousel) = self.carousel() else {
            return;
        };
        if !carousel.is_swipe_enabled() {
            return;
        }

        let swipe_axis = if self.uses_viewport_fraction_layout() {
            Some(carousel.get_layout_axis())
        } else {
            carousel.get_transition_axis()
        };
        self.swipe_axis.set(swipe_axis);

        let swipe_gesture_recognizer = SwipeGestureRecognizer::new();
        swipe_gesture_recognizer.set_can_horizontally_swipe(swipe_axis != Some(SlideAxis::Vertical));
        swipe_gesture_recognizer.set_can_vertically_swipe(swipe_axis != Some(SlideAxis::Horizontal));
        *self.swipe_gesture_recognizer.borrow_mut() = Some(swipe_gesture_recognizer.clone());

        self.gesture_recognizers().add(swipe_gesture_recognizer);

        let this = self.to_ref().downgrade();
        let handler = self.add_handler(InputElement::swipe_gesture_event(), move |_, e| {
            if let Some(this) = this.upgrade() {
                this.on_swipe_gesture(e);
            }
        });
        self.swipe_gesture_handler.set(Some(handler));

        let this = self.to_ref().downgrade();
        let handler = self.add_handler(InputElement::swipe_gesture_ended_event(), move |_, e| {
            if let Some(this) = this.upgrade() {
                this.on_swipe_gesture_ended(e);
            }
        });
        self.swipe_gesture_ended_handler.set(Some(handler));
    }

    fn teardown_gesture_recognizer(&self) {
        let completion_cts = self.completion_cts.borrow().clone();
        if let Some(completion_cts) = completion_cts {
            completion_cts.cancel();
        }
        *self.completion_cts.borrow_mut() = None;
        self.cancel_offset_animation();

        let swipe_gesture_recognizer = self.swipe_gesture_recognizer.take();
        if let Some(swipe_gesture_recognizer) = swipe_gesture_recognizer {
            self.gesture_recognizers().remove(&swipe_gesture_recognizer);
        }

        if let Some(handler) = self.swipe_gesture_handler.take() {
            self.remove_handler(InputElement::swipe_gesture_event(), handler);
        }
        if let Some(handler) = self.swipe_gesture_ended_handler.take() {
            self.remove_handler(InputElement::swipe_gesture_ended_event(), handler);
        }
        self.reset_swipe_state();
    }

    fn find_viewport_control(&self, item_index: i32) -> Option<Ref<Control>> {
        let viewport_realized = self.viewport_realized.borrow();
        viewport_realized.iter().find(|x| x.item_index == item_index).map(|x| x.control.clone())
    }

    fn set_fractional_progress_context(
        &self,
        from_index: i32,
        to_index: i32,
        forward: bool,
        start_offset: f64,
        target_offset: f64,
    ) {
        self.progress_from_index.set(from_index);
        self.progress_to_index.set(to_index);
        self.is_forward.set(forward);
        self.progress_start_offset.set(start_offset);
        self.active_viewport_target_offset.set(target_offset);
    }

    fn clear_fractional_progress_context(&self) {
        self.progress_from_index.set(-1);
        self.progress_to_index.set(-1);
        self.progress_start_offset.set(0.0);
        self.active_viewport_target_offset.set(0.0);
    }

    fn get_fractional_transition_progress(&self, current_offset: f64) -> f64 {
        let total_distance = (self.active_viewport_target_offset.get() - self.progress_start_offset.get()).abs();
        if total_distance <= 0.0 {
            return 0.0;
        }

        ((current_offset - self.progress_start_offset.get()).abs() / total_distance).clamp(0.0, 1.0)
    }

    fn reset_viewport_transition_state(&self) {
        let mut position = 0;
        while let Some(entry) = self.viewport_realized_at(position) {
            self.reset_transition_state(Some(&entry.control));
            position += 1;
        }
    }

    fn on_swipe_gesture(&self, e: &SwipeGestureEventArgs) {
        let Some(carousel) = self.carousel() else {
            return;
        };
        if !carousel.is_swipe_enabled() {
            return;
        }

        if self.uses_viewport_fraction_layout() {
            self.on_viewport_fraction_swipe_gesture(&carousel, e);
            return;
        }

        let item_count = self.item_count();

        if self.realized_index.get() < 0 || item_count == 0 {
            return;
        }

        if Self::is_active(&self.completion_cts) {
            let completion_cts = self.completion_cts.take();
            if let Some(completion_cts) = completion_cts {
                completion_cts.cancel();
            }

            let was_commit = self.completion_end_progress.get() > 0.5;
            let swipe_target = self.swipe_target();
            match swipe_target {
                Some(swipe_target) if was_commit => {
                    if let Some(realized) = self.realized() {
                        self.recycle_element(&realized);
                    }

                    *self.realized.borrow_mut() = Some(swipe_target);
                    self.realized_index.set(self.swipe_target_index.get());
                    carousel.set_selected_index(self.swipe_target_index.get());
                }
                _ => self.reset_swipe_state(),
            }

            *self.swipe_target.borrow_mut() = None;
            self.swipe_target_index.set(-1);
            self.total_delta.set(0.0);
        }

        if self.is_dragging.get() && e.id() != self.swipe_gesture_id.get() {
            return;
        }

        if !self.is_dragging.get() {
            // Lock the axis on gesture start to keep diagonal drags stable.
            self.locked_axis.set(self.swipe_axis.get().unwrap_or(if e.delta().x.abs() >= e.delta().y.abs() {
                SlideAxis::Horizontal
            } else {
                SlideAxis::Vertical
            }));
        }

        let delta = if self.locked_axis.get() == SlideAxis::Horizontal { e.delta().x } else { e.delta().y };

        if !self.is_dragging.get() {
            self.is_forward.set(delta > 0.0);
            self.is_rubber_banding.set(false);
            let current_index = self.realized_index.get();
            let mut target_index = if self.is_forward.get() { current_index + 1 } else { current_index - 1 };

            if target_index >= item_count {
                if carousel.wrap_selection() {
                    target_index = 0;
                } else {
                    self.is_rubber_banding.set(true);
                }
            } else if target_index < 0 {
                if carousel.wrap_selection() {
                    target_index = item_count - 1;
                } else {
                    self.is_rubber_banding.set(true);
                }
            }

            if !self.is_rubber_banding.get()
                && (target_index == current_index || target_index < 0 || target_index >= item_count)
            {
                return;
            }

            self.is_dragging.set(true);
            self.swipe_gesture_id.set(e.id());
            self.total_delta.set(0.0);
            self.swipe_target_index.set(if self.is_rubber_banding.get() { -1 } else { target_index });
            carousel.set_is_swiping(true);

            let transition = self.transition.borrow().clone();
            if let Some(transition) = transition {
                transition.cancel();
                *self.transition.borrow_mut() = None;
                if let Some(transition_from) = self.transition_from() {
                    self.recycle_element(&transition_from);
                }
                *self.transition_from.borrow_mut() = None;
                self.transition_from_index.set(-1);
            }

            if !self.is_rubber_banding.get() {
                let swipe_target = self.get_or_create_element(&self.items(), self.swipe_target_index.get());
                *self.swipe_target.borrow_mut() = Some(swipe_target.clone());
                let size = self.bounds().size();
                swipe_target.measure(size);
                swipe_target.arrange(Rect::from_size(size));
                swipe_target.set_is_visible(true);
            }
        }

        self.accumulate_delta(delta);

        let size = self.locked_axis_size();
        if size <= 0.0 {
            return;
        }

        let raw_progress = (self.total_delta.get().abs() / size).clamp(0.0, 1.0);
        let progress =
            if self.is_rubber_banding.get() { RUBBER_BAND_FACTOR * raw_progress.sqrt() } else { raw_progress };

        let transition = self.get_transition();
        if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
            let swipe_target = if self.is_rubber_banding.get() { None } else { self.swipe_target() };
            progressive.update(
                progress,
                Self::as_visual(self.realized()).as_ref(),
                Self::as_visual(swipe_target).as_ref(),
                self.is_forward.get(),
                size,
                &[],
            );
        }

        e.set_handled(true);
    }

    /// Adds the delta of a swipe event to the total, clamped so that the
    /// total cannot cross zero (absorbs touch jitter).
    fn accumulate_delta(&self, delta: f64) {
        let total_delta = self.total_delta.get() + delta;

        self.total_delta.set(if self.is_forward.get() { total_delta.max(0.0) } else { total_delta.min(0.0) });
    }

    fn on_viewport_fraction_swipe_gesture(&self, carousel: &Ref<Carousel>, e: &SwipeGestureEventArgs) {
        if Self::is_active(&self.offset_animation_cts) {
            self.cancel_offset_animation();
            self.set_offset_core(self.with_primary_offset(
                self.offset.get(),
                self.get_nearest_logical_offset(carousel.selected_index(), self.get_primary_offset(self.offset.get())),
            ));
        }

        if self.is_dragging.get() && e.id() != self.swipe_gesture_id.get() {
            return;
        }

        let delta = if self.locked_axis.get() == SlideAxis::Horizontal { e.delta().x } else { e.delta().y };
        let item_count = self.item_count();

        if !self.is_dragging.get() {
            self.locked_axis.set(carousel.get_layout_axis());
            self.swipe_gesture_id.set(e.id());
            self.drag_start_offset.set(
                self.get_nearest_logical_offset(carousel.selected_index(), self.get_primary_offset(self.offset.get())),
            );
            self.total_delta.set(0.0);
            self.is_dragging.set(true);
            self.is_rubber_banding.set(false);
            carousel.set_is_swiping(true);
            self.is_forward.set(delta > 0.0);
            let mut target_index =
                if self.is_forward.get() { carousel.selected_index() + 1 } else { carousel.selected_index() - 1 };

            if target_index >= item_count || target_index < 0 {
                if carousel.wrap_selection() && item_count > 1 {
                    target_index = Self::normalize_index(target_index, item_count);
                } else {
                    self.is_rubber_banding.set(true);
                }
            }

            let drag_start_offset = self.drag_start_offset.get();
            let target_offset = if self.is_forward.get() { drag_start_offset + 1.0 } else { drag_start_offset - 1.0 };
            self.set_fractional_progress_context(
                carousel.selected_index(),
                if self.is_rubber_banding.get() { -1 } else { target_index },
                self.is_forward.get(),
                drag_start_offset,
                target_offset,
            );
            self.reset_viewport_transition_state();
        }

        self.accumulate_delta(delta);

        let item_extent = self.get_viewport_item_extent(self.bounds().size());
        if item_extent <= 0.0 {
            return;
        }

        let drag_start_offset = self.drag_start_offset.get();
        let logical_delta = (self.total_delta.get().abs() / item_extent).clamp(0.0, 1.0);
        let mut proposed_offset = drag_start_offset + if self.is_forward.get() { logical_delta } else { -logical_delta };

        if !self.is_rubber_banding.get() {
            let active_viewport_target_offset = self.active_viewport_target_offset.get();
            proposed_offset = proposed_offset.clamp(
                drag_start_offset.min(active_viewport_target_offset),
                drag_start_offset.max(active_viewport_target_offset),
            );
        } else if proposed_offset < 0.0 {
            proposed_offset = -(RUBBER_BAND_FACTOR * (-proposed_offset).sqrt());
        } else {
            let max_offset = (item_count - 1).max(0) as f64;
            proposed_offset = max_offset + (RUBBER_BAND_FACTOR * (proposed_offset - max_offset).sqrt());
        }

        self.set_offset_core(self.with_primary_offset(self.offset.get(), proposed_offset));

        let transition = self.get_transition();
        if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
            let current_offset = self.get_primary_offset(self.offset.get());
            let progress = (current_offset - drag_start_offset).abs().clamp(0.0, 1.0);
            progressive.update(
                progress,
                Self::as_visual(self.find_viewport_control(self.progress_from_index.get())).as_ref(),
                Self::as_visual(self.find_viewport_control(self.progress_to_index.get())).as_ref(),
                self.is_forward.get(),
                self.get_viewport_item_extent(self.bounds().size()),
                &self.build_fractional_visible_items(current_offset),
            );
        }

        e.set_handled(true);
    }

    fn on_viewport_fraction_swipe_gesture_ended(&self, carousel: Ref<Carousel>, e: &SwipeGestureEndedEventArgs) {
        let item_extent = self.get_viewport_item_extent(self.bounds().size());
        let current_offset = self.get_primary_offset(self.offset.get());
        let current_progress = (current_offset - self.drag_start_offset.get()).abs();
        let velocity =
            if self.locked_axis.get() == SlideAxis::Horizontal { e.velocity().x.abs() } else { e.velocity().y.abs() };
        let target_index = self.progress_to_index.get();
        let is_rubber_banding = self.is_rubber_banding.get();
        let can_commit = !is_rubber_banding && target_index >= 0;
        let commit = can_commit
            && (current_progress >= SWIPE_COMMIT_THRESHOLD
                || (velocity > VELOCITY_COMMIT_THRESHOLD && current_progress >= MIN_SWIPE_DISTANCE_FOR_VELOCITY_COMMIT));
        let end_offset = if commit {
            self.active_viewport_target_offset.get()
        } else {
            self.get_nearest_logical_offset(carousel.selected_index(), current_offset)
        };
        let remaining_distance = (end_offset - current_offset).abs();
        let duration_seconds = if is_rubber_banding {
            RUBBER_BAND_RETURN_DURATION
        } else if velocity > 0.0 && item_extent > 0.0 {
            (remaining_distance * item_extent / velocity).clamp(MIN_COMPLETION_DURATION, MAX_COMPLETION_DURATION)
        } else {
            MAX_COMPLETION_DURATION
        };
        let easing: Easing = if is_rubber_banding { SineEaseOut::new().into() } else { QuadraticEaseOut::new().into() };

        self.is_dragging.set(false);
        self.animate_viewport_offset_async(
            current_offset,
            end_offset,
            TimeSpan::from_seconds(duration_seconds),
            easing,
            move |this| {
                this.total_delta.set(0.0);
                this.is_rubber_banding.set(false);
                carousel.set_is_swiping(false);

                if commit {
                    this.set_offset_core(this.with_primary_offset(
                        this.offset.get(),
                        this.get_nearest_logical_offset(target_index, end_offset),
                    ));
                    carousel.set_selected_index(target_index);
                } else {
                    this.set_offset_core(this.with_primary_offset(
                        this.offset.get(),
                        this.get_nearest_logical_offset(carousel.selected_index(), end_offset),
                    ));
                }

                this.reset_viewport_transition_state();
                this.clear_fractional_progress_context();
            },
        );
    }

    /// An animation of a progress property of the panel between two values.
    fn create_progress_animation(
        property: &'static StyledProperty<f64>,
        from: f64,
        to: f64,
        duration: TimeSpan,
        easing: Easing,
    ) -> Ref<Animation> {
        let animation = Animation::new();
        animation.set_fill_mode(FillMode::Forward);
        animation.set_duration(duration);
        animation.set_easing(easing);
        animation.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(property, from) as _]));
        animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(property, to) as _]));
        animation
    }

    fn animate_viewport_offset_async(
        &self,
        from_offset: f64,
        to_offset: f64,
        duration: TimeSpan,
        easing: Easing,
        on_completed: impl FnOnce(&Self) + 'static,
    ) {
        self.cancel_offset_animation();
        let offset_animation_cts = Rc::new(CancellationTokenSource::new());
        *self.offset_animation_cts.borrow_mut() = Some(offset_animation_cts.clone());
        let cancellation_token = offset_animation_cts.token();

        let animation =
            Self::create_progress_animation(Self::offset_animation_progress_property(), 0.0, 1.0, duration, easing);

        self.offset_animation_start.set(from_offset);
        self.offset_animation_target.set(to_offset);
        self.set_value(Self::offset_animation_progress_property(), 0.0);

        let this = self.to_ref().downgrade();
        let _finally = ClearSourceOnExit {
            panel: this.clone(),
            field: |panel| &panel.offset_animation_cts,
            source: offset_animation_cts,
        };
        let run = animation.run_async(self, cancellation_token.clone());

        let _ = Self::task_scheduler().start_local(async move {
            let _finally = _finally;

            if run.await.is_err() {
                return;
            }

            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let Some(this) = this.upgrade() else {
                return;
            };

            this.set_offset_core(this.with_primary_offset(this.offset.get(), to_offset));

            if this.uses_viewport_fraction_layout() {
                let transition = this.get_transition();
                if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
                    let transition_progress = this.get_fractional_transition_progress(to_offset);
                    progressive.update(
                        transition_progress,
                        Self::as_visual(this.find_viewport_control(this.progress_from_index.get())).as_ref(),
                        Self::as_visual(this.find_viewport_control(this.progress_to_index.get())).as_ref(),
                        this.is_forward.get(),
                        this.get_viewport_item_extent(this.bounds().size()),
                        &this.build_fractional_visible_items(to_offset),
                    );
                }
            }

            on_completed(&this);
        });
    }

    fn on_swipe_gesture_ended(&self, e: &SwipeGestureEndedEventArgs) {
        if !self.is_dragging.get() || e.id() != self.swipe_gesture_id.get() {
            return;
        }
        let Some(carousel) = self.carousel() else {
            return;
        };

        if self.uses_viewport_fraction_layout() {
            self.on_viewport_fraction_swipe_gesture_ended(carousel, e);
            return;
        }

        let is_rubber_banding = self.is_rubber_banding.get();
        let size = self.locked_axis_size();
        let raw_progress = if size > 0.0 { self.total_delta.get().abs() / size } else { 0.0 };
        let current_progress = if is_rubber_banding { RUBBER_BAND_FACTOR * raw_progress.sqrt() } else { raw_progress };
        let velocity =
            if self.locked_axis.get() == SlideAxis::Horizontal { e.velocity().x.abs() } else { e.velocity().y.abs() };
        let commit = !is_rubber_banding
            && (current_progress >= SWIPE_COMMIT_THRESHOLD
                || (velocity > VELOCITY_COMMIT_THRESHOLD && current_progress >= MIN_SWIPE_DISTANCE_FOR_VELOCITY_COMMIT))
            && self.swipe_target.borrow().is_some();

        self.completion_end_progress.set(if commit { 1.0 } else { 0.0 });
        let completion_end_progress = self.completion_end_progress.get();
        let remaining_distance = (completion_end_progress - current_progress).abs();
        let duration_seconds = if is_rubber_banding {
            RUBBER_BAND_RETURN_DURATION
        } else if velocity > 0.0 {
            (remaining_distance * size / velocity).clamp(MIN_COMPLETION_DURATION, MAX_COMPLETION_DURATION)
        } else {
            MAX_COMPLETION_DURATION
        };
        let easing: Easing = if is_rubber_banding { SineEaseOut::new().into() } else { QuadraticEaseOut::new().into() };

        let previous_completion_cts = self.completion_cts.borrow().clone();
        if let Some(previous_completion_cts) = previous_completion_cts {
            previous_completion_cts.cancel();
        }
        let completion_cts = Rc::new(CancellationTokenSource::new());
        *self.completion_cts.borrow_mut() = Some(completion_cts.clone());

        self.set_value(Self::completion_progress_property(), current_progress);

        let animation = Self::create_progress_animation(
            Self::completion_progress_property(),
            current_progress,
            completion_end_progress,
            TimeSpan::from_seconds(duration_seconds),
            easing,
        );

        self.is_dragging.set(false);
        self.run_completion_animation(animation, carousel, completion_cts);
    }

    fn run_completion_animation(
        &self,
        animation: Ref<Animation>,
        carousel: Ref<Carousel>,
        completion_cts: Rc<CancellationTokenSource>,
    ) {
        let cancellation_token = completion_cts.token();

        let this = self.to_ref().downgrade();
        let _finally =
            ClearSourceOnExit { panel: this.clone(), field: |panel| &panel.completion_cts, source: completion_cts };
        let run = animation.run_async(self, cancellation_token.clone());

        let _ = Self::task_scheduler().start_local(async move {
            let _finally = _finally;

            if run.await.is_err() {
                return;
            }

            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let Some(this) = this.upgrade() else {
                return;
            };

            let transition = this.get_transition();
            if let Some(progressive) = transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
                let realized = this.realized();
                let swipe_target = this.swipe_target().filter(|target| realized.as_ref() != Some(target));
                let size = this.locked_axis_size();
                progressive.update(
                    this.completion_end_progress.get(),
                    Self::as_visual(realized).as_ref(),
                    Self::as_visual(swipe_target).as_ref(),
                    this.is_forward.get(),
                    size,
                    &[],
                );
            }

            let commit = this.completion_end_progress.get() > 0.5;

            match this.swipe_target() {
                Some(target_element) if commit => {
                    let target_index = this.swipe_target_index.get();

                    // Clear swipe target state before promoting it to the
                    // realized element so interactive transitions never
                    // receive the same control as both from/to.
                    *this.swipe_target.borrow_mut() = None;
                    this.swipe_target_index.set(-1);

                    if let Some(realized) = this.realized() {
                        this.recycle_element(&realized);
                    }

                    *this.realized.borrow_mut() = Some(target_element);
                    this.realized_index.set(target_index);

                    carousel.set_selected_index(target_index);
                }
                _ => this.reset_swipe_state(),
            }

            this.total_delta.set(0.0);
            *this.swipe_target.borrow_mut() = None;
            this.swipe_target_index.set(-1);
            this.is_rubber_banding.set(false);
            carousel.set_is_swiping(false);
        });
    }

    fn build_fractional_visible_items(&self, current_offset: f64) -> Vec<PageTransitionItem> {
        let viewport_realized = self.viewport_realized.borrow();
        viewport_realized
            .iter()
            .map(|entry| {
                PageTransitionItem::new(
                    entry.item_index,
                    entry.control.clone().upcast(),
                    entry.logical_index as f64 - current_offset,
                )
            })
            .collect()
    }

    fn reset_swipe_state(&self) {
        if let Some(carousel) = self.carousel() {
            carousel.set_is_swiping(false);
        }

        self.cancel_offset_animation();

        self.reset_viewport_transition_state();
        self.reset_transition_state(self.realized().as_ref());

        if let Some(swipe_target) = self.swipe_target() {
            self.recycle_element(&swipe_target);
        }

        self.is_dragging.set(false);
        self.total_delta.set(0.0);
        *self.swipe_target.borrow_mut() = None;
        self.swipe_target_index.set(-1);
        self.is_rubber_banding.set(false);
        self.clear_fractional_progress_context();

        if self.uses_viewport_fraction_layout() {
            if let Some(viewport_carousel) = self.carousel() {
                self.set_offset_core(self.with_primary_offset(
                    self.offset.get(),
                    self.get_nearest_logical_offset(
                        viewport_carousel.selected_index(),
                        self.get_primary_offset(self.offset.get()),
                    ),
                ));
            }
        }
    }

    fn reset_transition_state(&self, control: Option<&Ref<Control>>) {
        let Some(control) = control else {
            return;
        };

        let transition = self.get_transition();
        match transition.as_ref().and_then(|t| t.as_progress_page_transition()) {
            Some(progressive) => progressive.reset(&control.clone().upcast()),
            None => Self::reset_visual_state(Some(control)),
        }
    }

    fn reset_visual_state(control: Option<&Ref<Control>>) {
        let Some(control) = control else {
            return;
        };
        control.set_render_transform(None);
        control.set_opacity(1.0);
        control.set_z_index(0);
        control.set_value(Visual::clip_property(), None);
    }
}
