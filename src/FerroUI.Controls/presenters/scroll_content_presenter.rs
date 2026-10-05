use super::{ContentPresenter, ItemsPresenter};
use crate::primitives::{
    as_logical_scrollable, as_scroll_snap_points_info, register_scroll_snap_points_info, ScrollBarVisibility,
    SnapPointsAlignment, SnapPointsChangedHandler, SnapPointsType,
};
use crate::{
    register_scroll_anchor_provider, Control, ControlImpl, IScrollAnchorProvider, ItemsControl,
    RequestBringIntoViewEventArgs, ScrollViewer, StackPanel,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
use ferroui_base::input::{
    IScrollable, InputElement, InputElementImpl, KeyModifiers, PointerWheelEventArgs, ScrollGestureEndedEventArgs,
    ScrollGestureEventArgs, ScrollGestureInertiaStartingEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl, LayoutableImplExt, Orientation};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, BoxedValue, DirectProperty,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect,
    Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyMetadata, Thickness, Vector, Visual, VisualImpl,
    VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

const EDGE_DETECTION_TOLERANCE: f64 = 0.1;

/// Presents a scrolling view of content inside a [`ScrollViewer`].
#[repr(C)]
pub struct ScrollContentPresenter {
    base: ContentPresenter,
    arranging: Cell<bool>,
    extent: Cell<Size>,
    logical_scroll_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    viewport: Cell<Size>,
    active_logical_gesture_scrolls: RefCell<Option<HashMap<i32, Vector>>>,
    scroll_gesture_snap_points: RefCell<Option<HashMap<i32, Vector>>>,
    anchor_candidates: RefCell<Option<Vec<Ref<Control>>>>,
    anchor_element: RefCell<Option<Ref<Control>>>,
    anchor_element_bounds: Cell<Rect>,
    is_anchor_element_dirty: Cell<bool>,
    are_vertical_snap_points_regular: Cell<bool>,
    are_horizontal_snap_points_regular: Cell<bool>,
    horizontal_snap_points: RefCell<Option<Vec<f64>>>,
    horizontal_snap_point: Cell<f64>,
    vertical_snap_points: RefCell<Option<Vec<f64>>>,
    vertical_snap_point: Cell<f64>,
    vertical_snap_point_offset: Cell<f64>,
    horizontal_snap_point_offset: Cell<f64>,
    owner_subscriptions: RefCell<Option<Rc<CompositeDisposable>>>,
    owner: RefCell<Option<WeakRef<ScrollViewer>>>,
    scroll_snap_points_info: RefCell<Option<Ref<Control>>>,
    /// The snap points changed subscriptions on `scroll_snap_points_info`
    /// (vertical, then horizontal).
    scroll_snap_points_subscriptions: RefCell<Option<[Rc<dyn IDisposable>; 2]>>,
    is_snap_points_updated: Cell<bool>,
}

ferro_class!(ScrollContentPresenter: ContentPresenter);
ferroui_base::ferro_class_info!(ScrollContentPresenter { new: ScrollContentPresenter::new });
ferro_impl_classes!(ScrollContentPresenter: StyledElementImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for ScrollContentPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.add_handler(
            Control::request_bring_into_view_event(),
            |sender, e: &RequestBringIntoViewEventArgs| {
                if let Some(presenter) = sender.downcast_ref::<ScrollContentPresenter>() {
                    presenter.bring_into_view_requested(e);
                }
            },
        );
        this.add_handler(
            InputElement::scroll_gesture_event(),
            |sender, e: &ScrollGestureEventArgs| {
                if let Some(presenter) = sender.downcast_ref::<ScrollContentPresenter>() {
                    presenter.on_scroll_gesture(e);
                }
            },
        );
        this.add_handler(
            InputElement::scroll_gesture_ended_event(),
            |sender, e: &ScrollGestureEndedEventArgs| {
                if let Some(presenter) = sender.downcast_ref::<ScrollContentPresenter>() {
                    presenter.on_scroll_gesture_ended(e);
                }
            },
        );
        this.add_handler(
            InputElement::scroll_gesture_inertia_starting_event(),
            |sender, e: &ScrollGestureInertiaStartingEventArgs| {
                if let Some(presenter) = sender.downcast_ref::<ScrollContentPresenter>() {
                    presenter.on_scroll_gesture_inertia_starting_ended(e);
                }
            },
        );

        let weak = this.to_ref().downgrade();
        let _ = FerroObjectExtensions::get_observable(fo(&this), ContentPresenter::child_property()).subscribe_fn(
            move |child| {
                if let Some(this) = weak.upgrade() {
                    this.update_scrollable_subscription(child);
                }
            },
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();

        if property == Self::offset_property().as_property() {
            if !this.arranging.get() {
                this.invalidate_arrange();
            }

            if let Some(owner) = this.owner() {
                owner.set_current_value(ScrollViewer::offset_property(), change.get_new_value::<Vector>());
            }
        } else if property == ContentPresenter::child_property().as_property() {
            this.child_changed(change);
        } else if property == Self::horizontal_snap_points_alignment_property().as_property()
            || property == Self::vertical_snap_points_alignment_property().as_property()
        {
            this.update_snap_points();
        } else if property == Self::extent_property().as_property() {
            if let Some(owner) = this.owner() {
                owner.set_extent(change.get_new_value::<Size>());
            }
            this.coerce_value(Self::offset_property().as_property());
        } else if property == Self::viewport_property().as_property() {
            if let Some(owner) = this.owner() {
                owner.set_viewport(change.get_new_value::<Size>());
            }
            this.coerce_value(Self::offset_property().as_property());
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl VisualImpl for ScrollContentPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attach_to_scroll_viewer();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        // A replaced template no longer has the ScrollViewer as an ancestor
        // and must release its content. Keep the bindings when the entire
        // owner is detached.
        if this.find_ancestor_of_type::<ScrollViewer>(false).is_none() {
            this.detach_from_scroll_viewer();
        }
    }
}

impl LayoutableImpl for ScrollContentPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = match this.child() {
            Some(child) if this.logical_scroll_subscription.borrow().is_none() => child,
            _ => return Self::parent_measure_override(this, available_size),
        };

        let padding = this.get_child_padding();
        let deflated = available_size.deflate(padding);

        let constraint = Size::new(
            if this.can_horizontally_scroll() {
                f64::INFINITY
            } else {
                deflated.width
            },
            if this.can_vertically_scroll() {
                f64::INFINITY
            } else {
                deflated.height
            },
        );

        child.measure(constraint);

        if !this.is_snap_points_updated.get() {
            this.is_snap_points_updated.set(true);
            this.update_snap_points();
        }

        child.desired_size().inflate(padding).constrain(available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let child = match this.child() {
            Some(child) if this.logical_scroll_subscription.borrow().is_none() => child,
            _ => return Self::parent_arrange_override(this, final_size),
        };

        this.arrange_with_anchoring(&child, final_size)
    }
}

impl InputElementImpl for ScrollContentPresenter {
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        let extent = this.extent();
        let viewport = this.viewport();

        if extent.height > viewport.height || extent.width > viewport.width {
            let child = this.child();
            let scrollable = child.as_ref().and_then(|child| as_logical_scrollable(child));
            let logical = scrollable.filter(|scrollable| scrollable.is_logical_scroll_enabled());

            let offset = this.offset();
            let mut x = offset.x;
            let mut y = offset.y;
            let mut delta = e.delta();

            // KeyModifiers.Shift should scroll in horizontal direction. This
            // does not work on every platform. If Shift-Key is pressed and X
            // is close to 0 we swap the Vector.
            if e.key_modifiers() == KeyModifiers::SHIFT && MathUtilities::is_zero(delta.x) {
                delta = Vector::new(delta.y, delta.x);
            } else {
                delta = Self::adjust_delta_for_flow_direction(delta, this.flow_direction());
            }

            if extent.height > viewport.height {
                let height = match logical {
                    Some(scrollable) => scrollable.scroll_size().height,
                    None => 50.0,
                };
                y += -delta.y * height;
                y = y.max(0.0);
                y = y.min(extent.height - viewport.height);
            }

            if extent.width > viewport.width {
                let width = match logical {
                    Some(scrollable) => scrollable.scroll_size().width,
                    None => 50.0,
                };
                x += -delta.x * width;
                x = x.max(0.0);
                x = x.min(extent.width - viewport.width);
            }

            let new_offset = this.snap_offset(Vector::new(x, y), delta, true);

            let offset_changed = new_offset != this.offset();
            this.set_current_value(Self::offset_property(), new_offset);

            e.set_handled(!this.is_scroll_chaining_enabled() || offset_changed);
        }
    }

    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl IScrollable for ScrollContentPresenter {
    fn extent(&self) -> Size {
        ScrollContentPresenter::extent(self)
    }

    fn offset(&self) -> Vector {
        ScrollContentPresenter::offset(self)
    }

    fn set_offset(&self, value: Vector) {
        ScrollContentPresenter::set_offset(self, value)
    }

    fn viewport(&self) -> Size {
        ScrollContentPresenter::viewport(self)
    }

    fn can_horizontally_scroll(&self) -> bool {
        ScrollContentPresenter::can_horizontally_scroll(self)
    }

    fn can_vertically_scroll(&self) -> bool {
        ScrollContentPresenter::can_vertically_scroll(self)
    }
}

impl IScrollAnchorProvider for ScrollContentPresenter {
    fn current_anchor(&self) -> Option<Ref<Control>> {
        self.ensure_anchor_element_selection();
        self.anchor_element.borrow().clone()
    }

    fn register_anchor_candidate(&self, element: &Ref<Control>) {
        if !self.is_visual_ancestor_of(element) {
            panic!("An anchor control must be a visual descendent of the ScrollContentPresenter.");
        }

        {
            let mut candidates = self.anchor_candidates.borrow_mut();
            let candidates = candidates.get_or_insert_with(Vec::new);
            if !candidates.contains(element) {
                candidates.push(element.clone());
            }
        }
        self.is_anchor_element_dirty.set(true);
    }

    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(candidates) = self.anchor_candidates.borrow_mut().as_mut() {
            candidates.retain(|candidate| candidate != element);
        }
        self.is_anchor_element_dirty.set(true);

        let is_anchor = self.anchor_element.borrow().as_ref() == Some(element);
        if is_anchor {
            *self.anchor_element.borrow_mut() = None;
        }
    }
}

ferroui_base::ferro_properties! { impl ScrollContentPresenter {
    ferro_property!(
        /// Defines the `CanHorizontallyScroll` property.
        pub fn can_horizontally_scroll_property() -> StyledProperty<bool> {
            FerroProperty::register::<ScrollContentPresenter, _>("CanHorizontallyScroll", false)
        }
    );

    ferro_property!(
        /// Defines the `CanVerticallyScroll` property.
        pub fn can_vertically_scroll_property() -> StyledProperty<bool> {
            FerroProperty::register::<ScrollContentPresenter, _>("CanVerticallyScroll", false)
        }
    );

    ferro_property!(
        /// Defines the `Extent` property.
        pub fn extent_property() -> DirectProperty<ScrollContentPresenter, Size> {
            ScrollViewer::extent_property().add_owner::<ScrollContentPresenter>(|o| o.extent(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `Offset` property.
        pub fn offset_property() -> StyledProperty<Vector> {
            ScrollViewer::offset_property().add_owner_with::<ScrollContentPresenter>(
                StyledPropertyMetadata::new(None).with_coerce(ScrollViewer::coerce_offset),
            )
        }
    );

    ferro_property!(
        /// Defines the `Viewport` property.
        pub fn viewport_property() -> DirectProperty<ScrollContentPresenter, Size> {
            ScrollViewer::viewport_property().add_owner::<ScrollContentPresenter>(|o| o.viewport(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalSnapPointsType` property.
        pub fn horizontal_snap_points_type_property() -> AttachedProperty<SnapPointsType> {
            ScrollViewer::horizontal_snap_points_type_property().add_owner::<ScrollContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalSnapPointsType` property.
        pub fn vertical_snap_points_type_property() -> AttachedProperty<SnapPointsType> {
            ScrollViewer::vertical_snap_points_type_property().add_owner::<ScrollContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalSnapPointsAlignment` property.
        pub fn horizontal_snap_points_alignment_property() -> AttachedProperty<SnapPointsAlignment> {
            ScrollViewer::horizontal_snap_points_alignment_property().add_owner::<ScrollContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalSnapPointsAlignment` property.
        pub fn vertical_snap_points_alignment_property() -> AttachedProperty<SnapPointsAlignment> {
            ScrollViewer::vertical_snap_points_alignment_property().add_owner::<ScrollContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `IsScrollChainingEnabled` property.
        pub fn is_scroll_chaining_enabled_property() -> AttachedProperty<bool> {
            ScrollViewer::is_scroll_chaining_enabled_property().add_owner::<ScrollContentPresenter>()
        }
    );
} }

impl ScrollContentPresenter {
    /// Initializes static members of the class.
    fn static_constructor() {
        register_scroll_anchor_provider::<ScrollContentPresenter>();
        register_scroll_snap_points_info::<StackPanel>();

        Visual::clip_to_bounds_property().override_default_value::<ScrollContentPresenter>(true);
        Layoutable::affects_measure::<ScrollContentPresenter>(&[
            Self::can_horizontally_scroll_property().as_property(),
            Self::can_vertically_scroll_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentPresenter::construct(),
            arranging: Cell::new(false),
            extent: Cell::new(Size::default()),
            logical_scroll_subscription: RefCell::new(None),
            viewport: Cell::new(Size::default()),
            active_logical_gesture_scrolls: RefCell::new(None),
            scroll_gesture_snap_points: RefCell::new(None),
            anchor_candidates: RefCell::new(None),
            anchor_element: RefCell::new(None),
            anchor_element_bounds: Cell::new(Rect::default()),
            is_anchor_element_dirty: Cell::new(false),
            are_vertical_snap_points_regular: Cell::new(false),
            are_horizontal_snap_points_regular: Cell::new(false),
            horizontal_snap_points: RefCell::new(None),
            horizontal_snap_point: Cell::new(0.0),
            vertical_snap_points: RefCell::new(None),
            vertical_snap_point: Cell::new(0.0),
            vertical_snap_point_offset: Cell::new(0.0),
            horizontal_snap_point_offset: Cell::new(0.0),
            owner_subscriptions: RefCell::new(None),
            owner: RefCell::new(None),
            scroll_snap_points_info: RefCell::new(None),
            scroll_snap_points_subscriptions: RefCell::new(None),
            is_snap_points_updated: Cell::new(false),
        }
    }

    /// Initializes a new instance of the class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets a value indicating whether the content can be scrolled
    /// horizontally.
    pub fn can_horizontally_scroll(&self) -> bool {
        self.get_value(Self::can_horizontally_scroll_property())
    }

    /// Sets a value indicating whether the content can be scrolled
    /// horizontally.
    pub fn set_can_horizontally_scroll(&self, value: bool) {
        self.set_value(Self::can_horizontally_scroll_property(), value)
    }

    /// Gets a value indicating whether the content can be scrolled
    /// vertically.
    pub fn can_vertically_scroll(&self) -> bool {
        self.get_value(Self::can_vertically_scroll_property())
    }

    /// Sets a value indicating whether the content can be scrolled
    /// vertically.
    pub fn set_can_vertically_scroll(&self, value: bool) {
        self.set_value(Self::can_vertically_scroll_property(), value)
    }

    /// Gets the extent of the scrollable content.
    pub fn extent(&self) -> Size {
        self.extent.get()
    }

    fn set_extent(&self, value: Size) {
        self.set_and_raise_cell(Self::extent_property(), &self.extent, value);
    }

    /// Gets the current scroll offset.
    pub fn offset(&self) -> Vector {
        self.get_value(Self::offset_property())
    }

    /// Sets the current scroll offset.
    pub fn set_offset(&self, value: Vector) {
        self.set_value(Self::offset_property(), value)
    }

    /// Gets the size of the viewport on the scrollable content.
    pub fn viewport(&self) -> Size {
        self.viewport.get()
    }

    fn set_viewport(&self, value: Size) {
        self.set_and_raise_cell(Self::viewport_property(), &self.viewport, value);
    }

    /// Gets how scroll gesture reacts to the snap points along the
    /// horizontal axis.
    pub fn horizontal_snap_points_type(&self) -> SnapPointsType {
        self.get_value(Self::horizontal_snap_points_type_property())
    }

    /// Sets how scroll gesture reacts to the snap points along the
    /// horizontal axis.
    pub fn set_horizontal_snap_points_type(&self, value: SnapPointsType) {
        self.set_value(Self::horizontal_snap_points_type_property(), value)
    }

    /// Gets how scroll gesture reacts to the snap points along the vertical
    /// axis.
    pub fn vertical_snap_points_type(&self) -> SnapPointsType {
        self.get_value(Self::vertical_snap_points_type_property())
    }

    /// Sets how scroll gesture reacts to the snap points along the vertical
    /// axis.
    pub fn set_vertical_snap_points_type(&self, value: SnapPointsType) {
        self.set_value(Self::vertical_snap_points_type_property(), value)
    }

    /// Gets how the existing snap points are horizontally aligned versus the
    /// initial viewport.
    pub fn horizontal_snap_points_alignment(&self) -> SnapPointsAlignment {
        self.get_value(Self::horizontal_snap_points_alignment_property())
    }

    /// Sets how the existing snap points are horizontally aligned versus the
    /// initial viewport.
    pub fn set_horizontal_snap_points_alignment(&self, value: SnapPointsAlignment) {
        self.set_value(Self::horizontal_snap_points_alignment_property(), value)
    }

    /// Gets how the existing snap points are vertically aligned versus the
    /// initial viewport.
    pub fn vertical_snap_points_alignment(&self) -> SnapPointsAlignment {
        self.get_value(Self::vertical_snap_points_alignment_property())
    }

    /// Sets how the existing snap points are vertically aligned versus the
    /// initial viewport.
    pub fn set_vertical_snap_points_alignment(&self, value: SnapPointsAlignment) {
        self.set_value(Self::vertical_snap_points_alignment_property(), value)
    }

    /// Gets if scroll chaining is enabled. The default value is true.
    ///
    /// After a user hits a scroll limit on an element that has been nested
    /// within another scrollable element, you can specify whether that
    /// parent element should continue the scrolling operation begun in its
    /// child element. This is called scroll chaining.
    pub fn is_scroll_chaining_enabled(&self) -> bool {
        self.get_value(Self::is_scroll_chaining_enabled_property())
    }

    /// Sets if scroll chaining is enabled.
    pub fn set_is_scroll_chaining_enabled(&self, value: bool) {
        self.set_value(Self::is_scroll_chaining_enabled_property(), value)
    }

    fn owner(&self) -> Option<Ref<ScrollViewer>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Attempts to bring a portion of the target visual into view by
    /// scrolling the content.
    ///
    /// `target` is the target visual and `target_rect` the portion of the
    /// target visual to bring into view. Returns true if the scroll offset
    /// was changed; otherwise false.
    pub fn bring_descendant_into_view(&self, target: &Visual, target_rect: Rect) -> bool {
        let child = match self.child() {
            Some(child) if child.is_effectively_visible() => child,
            _ => return false,
        };

        if let Some(scrollable) = as_logical_scrollable(&child) {
            if scrollable.is_logical_scroll_enabled() {
                if let Some(control) = target.to_ref().cast::<Control>() {
                    return scrollable.bring_into_view(&control, target_rect);
                }
            }
        }

        // The `viewport` rectangle computed below is in extent coordinates,
        // so transform `target_rect` into that space too. Going via the
        // child rather than via this + offset keeps the result independent
        // of the offset, which may have changed since the last arrange.
        let Some(transform) = target.transform_to_visual(&child) else {
            return false;
        };

        let child_padding = self.get_child_padding();
        let child_margin = self.get_child_margin();
        let child_content_origin = Vector::new(
            child_padding.left + child_margin.left,
            child_padding.top + child_margin.top,
        );
        let rectangle = target_rect.transform_to_aabb(transform).translate(child_content_origin);
        let current = self.offset();
        let viewport_size = self.viewport();
        let viewport = Rect::new(current.x, current.y, viewport_size.width, viewport_size.height);

        let min_x = Self::compute_scroll_offset_with_minimal_scroll(
            viewport.x,
            viewport.right(),
            rectangle.x,
            rectangle.right(),
        );
        let min_y = Self::compute_scroll_offset_with_minimal_scroll(
            viewport.y,
            viewport.bottom(),
            rectangle.y,
            rectangle.bottom(),
        );
        let offset = Vector::new(min_x, min_y);

        if self.offset().nearly_equals(offset) {
            return false;
        }

        let old_offset = self.offset();
        self.set_current_value(Self::offset_property(), offset);

        // It's possible that the offset coercion has changed the offset back
        // to its previous value, this is common for floating point rounding
        // errors.
        !self.offset().nearly_equals(old_offset)
    }

    /// Computes the closest offset to ensure most of the child is visible in
    /// the viewport along an axis.
    ///
    /// `viewport_start` is the left or top of the viewport, `viewport_end`
    /// the right or bottom of the viewport, `child_start` the left or top of
    /// the child and `child_end` the right or bottom of the child.
    pub(crate) fn compute_scroll_offset_with_minimal_scroll(
        viewport_start: f64,
        viewport_end: f64,
        child_start: f64,
        child_end: f64,
    ) -> f64 {
        // If child is at least partially above viewport, i.e. top of child is
        // above viewport top and bottom of child is above viewport bottom.
        let is_child_above =
            MathUtilities::less_than(child_start, viewport_start) && MathUtilities::less_than(child_end, viewport_end);

        // If child is at least partially below viewport, i.e. top of child is
        // below viewport top and bottom of child is below viewport bottom.
        let is_child_below = MathUtilities::greater_than(child_end, viewport_end)
            && MathUtilities::greater_than(child_start, viewport_start);
        let is_child_larger = (child_end - child_start) > (viewport_end - viewport_start);

        // Value if no updates is needed. The child is fully visible in the
        // viewport, or the viewport is completely within the child's bounds.
        let mut res = viewport_start;

        // The child is above the viewport and is smaller than the viewport,
        // or if the child's top is below the viewport top and is larger than
        // the viewport, we align the child top to the top of the viewport.
        if (is_child_above && !is_child_larger) || (is_child_below && is_child_larger) {
            res = child_start;
        }
        // The child is above the viewport and is larger than the viewport, or
        // if the child's smaller but is below the viewport, we align the
        // child's bottom to the bottom of the viewport.
        else if is_child_above || is_child_below {
            res = child_end - (viewport_end - viewport_start);
        }

        res
    }

    /// Locates the first [`ScrollViewer`] ancestor and binds to it.
    /// Properties which have been set through other means are not bound.
    ///
    /// This method is automatically called when the control is attached to a
    /// visual tree.
    pub(crate) fn attach_to_scroll_viewer(&self) {
        let Some(owner) = self.find_ancestor_of_type::<ScrollViewer>(false) else {
            self.detach_from_scroll_viewer();
            return;
        };

        if self.owner().as_ref() == Some(&owner) {
            return;
        }

        let old_subscriptions = self.owner_subscriptions.borrow_mut().take();
        if let Some(old_subscriptions) = old_subscriptions {
            old_subscriptions.dispose();
        }
        *self.owner.borrow_mut() = Some(owner.downgrade());

        fn not_disabled(v: ScrollBarVisibility) -> bool {
            v != ScrollBarVisibility::Disabled
        }

        let mut subscription_disposables: Vec<Rc<dyn IDisposable>> = Vec::new();

        if !self.is_set(Self::can_horizontally_scroll_property().as_property()) {
            subscription_disposables.push(self.bind(
                Self::can_horizontally_scroll_property(),
                FerroObjectExtensions::get_observable_with(
                    fo(&owner),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    not_disabled,
                ),
                BindingPriority::Template,
            ));
        }

        if !self.is_set(Self::can_vertically_scroll_property().as_property()) {
            subscription_disposables.push(self.bind(
                Self::can_vertically_scroll_property(),
                FerroObjectExtensions::get_observable_with(
                    fo(&owner),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    not_disabled,
                ),
                BindingPriority::Template,
            ));
        }

        if !self.is_set(Self::offset_property().as_property()) {
            subscription_disposables.push(self.bind_value(
                Self::offset_property(),
                FerroObjectExtensions::get_binding_observable(fo(&owner), ScrollViewer::offset_property()),
                BindingPriority::Template,
            ));
        }

        if !self.is_set(Self::is_scroll_chaining_enabled_property().as_property()) {
            subscription_disposables.push(self.bind_value(
                Self::is_scroll_chaining_enabled_property(),
                FerroObjectExtensions::get_binding_observable(
                    fo(&owner),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                ),
                BindingPriority::Template,
            ));
        }

        if !self.is_set(ContentPresenter::content_property().as_property()) {
            subscription_disposables.push(self.bind_value(
                ContentPresenter::content_property(),
                FerroObjectExtensions::get_binding_observable(fo(&owner), ContentPresenter::content_property()),
                BindingPriority::Template,
            ));
        }

        *self.owner_subscriptions.borrow_mut() =
            Some(Rc::new(CompositeDisposable::from_disposables(subscription_disposables)));
    }

    /// Clears the owning [`ScrollViewer`] and disposes the bindings to it.
    fn detach_from_scroll_viewer(&self) {
        *self.owner.borrow_mut() = None;
        let subscriptions = self.owner_subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }
    }

    fn arrange_with_anchoring(&self, child: &Ref<Control>, final_size: Size) -> Size {
        let padding = self.get_child_padding();
        let desired_size = child.desired_size().inflate(padding);

        let size = Size::new(
            if self.can_horizontally_scroll() {
                desired_size.width.max(final_size.width)
            } else {
                final_size.width
            },
            if self.can_vertically_scroll() {
                desired_size.height.max(final_size.height)
            } else {
                final_size.height
            },
        );

        let track_anchor = || -> Vector {
            // If we have an anchor and its position relative to the child
            // has changed during the arrange then that change wasn't just
            // due to scrolling (as scrolling doesn't adjust relative
            // positions within the child).
            let anchor_element = self.anchor_element.borrow().clone();
            if let Some(anchor_element) = anchor_element {
                if let Some(updated_bounds) = Self::try_translate_bounds(&anchor_element, child) {
                    let anchor_bounds = self.anchor_element_bounds.get();
                    if updated_bounds.position() != anchor_bounds.position() {
                        return (updated_bounds.position() - anchor_bounds.position()).into();
                    }
                }
            }

            Vector::default()
        };

        let offset = self.offset();
        let is_anchoring = offset.x >= EDGE_DETECTION_TOLERANCE || offset.y >= EDGE_DETECTION_TOLERANCE;

        if is_anchoring {
            // Calculate the new anchor element if necessary.
            self.ensure_anchor_element_selection();

            // Do the arrange.
            self.arrange_override_impl(size, -self.offset());

            // If the anchor moved during the arrange, we need to adjust the
            // offset and do another arrange.
            let anchor_shift = track_anchor();

            if anchor_shift != Vector::default() {
                let new_offset = self.offset() + anchor_shift;
                let extent = self.extent();
                let viewport = self.viewport();
                let mut new_extent = extent;
                let max_offset = Vector::new(extent.width - viewport.width, extent.height - viewport.height);

                if new_offset.x > max_offset.x {
                    new_extent = new_extent.with_width(new_offset.x + viewport.width);
                }

                if new_offset.y > max_offset.y {
                    new_extent = new_extent.with_height(new_offset.y + viewport.height);
                }

                self.set_extent(new_extent);

                {
                    struct Arranging<'a>(&'a Cell<bool>);

                    impl Drop for Arranging<'_> {
                        fn drop(&mut self) {
                            self.0.set(false);
                        }
                    }

                    self.arranging.set(true);
                    let _arranging = Arranging(&self.arranging);
                    self.set_current_value(Self::offset_property(), new_offset);
                }

                self.arrange_override_impl(size, -self.offset());
            }
        } else {
            self.arrange_override_impl(size, -self.offset());
        }

        self.set_viewport(final_size);
        self.set_extent(self.compute_extent(child, final_size, padding));
        self.is_anchor_element_dirty.set(true);

        final_size
    }

    fn get_child_padding(&self) -> Thickness {
        let mut padding = self.padding();
        let mut border_thickness = self.border_thickness();

        if self.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(self);
            padding = LayoutHelper::round_layout_thickness(padding, scale);
            border_thickness = LayoutHelper::round_layout_thickness(border_thickness, scale);
        }

        padding + border_thickness
    }

    fn get_child_margin(&self) -> Thickness {
        let Some(child) = self.child() else {
            return Thickness::default();
        };

        let mut margin = child.margin();

        if child.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(&child);
            margin = LayoutHelper::round_layout_thickness(margin, scale);
        }

        margin
    }

    fn compute_extent(&self, child: &Ref<Control>, viewport_size: Size, padding: Thickness) -> Size {
        let child_margin = self.get_child_margin();

        let mut extent = child.bounds().size().inflate(child_margin).inflate(padding);

        if MathUtilities::are_close_eps(extent.width, viewport_size.width, LayoutHelper::LAYOUT_EPSILON) {
            extent = extent.with_width(viewport_size.width);
        }

        if MathUtilities::are_close_eps(extent.height, viewport_size.height, LayoutHelper::LAYOUT_EPSILON) {
            extent = extent.with_height(viewport_size.height);
        }

        extent
    }

    fn on_scroll_gesture(&self, e: &ScrollGestureEventArgs) {
        let extent = self.extent();
        let viewport = self.viewport();

        if extent.height > viewport.height || extent.width > viewport.width {
            let child = self.child();
            let scrollable = child.as_ref().and_then(|child| as_logical_scrollable(child));
            let is_logical = scrollable.is_some_and(|scrollable| scrollable.is_logical_scroll_enabled());
            let mut logical_scroll_item_size = Vector::new(1.0, 1.0);
            let mut can_x_scroll = false;
            let mut can_y_scroll = false;

            let offset = self.offset();
            let mut x = offset.x;
            let mut y = offset.y;

            let mut delta = Vector::default();
            if is_logical {
                if let Some(active) = self.active_logical_gesture_scrolls.borrow().as_ref() {
                    if let Some(value) = active.get(&e.id()) {
                        delta = *value;
                    }
                }
            }
            delta += Self::adjust_delta_for_flow_direction(e.delta(), self.flow_direction());

            if let Some(scrollable) = scrollable.filter(|_| is_logical) {
                logical_scroll_item_size = self.bounds().size() / scrollable.viewport();
            }

            if extent.height > viewport.height {
                let dy;
                if is_logical {
                    let logical_units = delta.y / logical_scroll_item_size.y;
                    delta = delta.with_y(delta.y - logical_units * logical_scroll_item_size.y);
                    dy = logical_units;
                } else {
                    dy = delta.y;
                }

                y += dy;
                y = y.max(0.0);
                y = y.min(extent.height - viewport.height);

                can_y_scroll = dy != 0.0;
            }

            if extent.width > viewport.width {
                let dx;
                if is_logical {
                    let logical_units = delta.x / logical_scroll_item_size.x;
                    delta = delta.with_x(delta.x - logical_units * logical_scroll_item_size.x);
                    dx = logical_units;
                } else {
                    dx = delta.x;
                }
                x += dx;
                x = x.max(0.0);
                x = x.min(extent.width - viewport.width);

                can_x_scroll = dx != 0.0;
            }

            if is_logical {
                self.active_logical_gesture_scrolls
                    .borrow_mut()
                    .get_or_insert_with(HashMap::new)
                    .insert(e.id(), delta);
            }

            let mut new_offset = Vector::new(x, y);

            let snap_point = self
                .scroll_gesture_snap_points
                .borrow()
                .as_ref()
                .and_then(|points| points.get(&e.id()).copied());
            if let Some(snap_point) = snap_point {
                let mut x_offset = x;
                let mut y_offset = y;

                if self.horizontal_snap_points_type() != SnapPointsType::None {
                    x_offset = if delta.x < 0.0 {
                        snap_point.x.max(new_offset.x)
                    } else {
                        snap_point.x.min(new_offset.x)
                    };
                }

                if self.vertical_snap_points_type() != SnapPointsType::None {
                    y_offset = if delta.y < 0.0 {
                        snap_point.y.max(new_offset.y)
                    } else {
                        snap_point.y.min(new_offset.y)
                    };
                }

                new_offset = Vector::new(x_offset, y_offset);
            }

            let offset_changed = new_offset != self.offset();
            self.set_current_value(Self::offset_property(), new_offset);

            e.set_handled(!self.is_scroll_chaining_enabled() || offset_changed);

            if !e.handled() && !self.is_scroll_chaining_enabled() {
                // Gesture may cause an overscroll so we mark the event as
                // handled if it did.
                e.set_handled(can_x_scroll || can_y_scroll);
            }

            e.set_should_end_scroll_gesture(!self.is_scroll_chaining_enabled() && !offset_changed);
        }
    }

    fn on_scroll_gesture_ended(&self, e: &ScrollGestureEndedEventArgs) {
        if let Some(active) = self.active_logical_gesture_scrolls.borrow_mut().as_mut() {
            active.remove(&e.id());
        }
        if let Some(points) = self.scroll_gesture_snap_points.borrow_mut().as_mut() {
            points.remove(&e.id());
        }

        let offset = self.snap_offset(self.offset(), Vector::default(), false);
        self.set_current_value(Self::offset_property(), offset);
    }

    fn on_scroll_gesture_inertia_starting_ended(&self, e: &ScrollGestureInertiaStartingEventArgs) {
        let mut scrollable = self.content().as_ref().and_then(Control::from_boxed);

        if let Some(items_control) = scrollable.as_ref().and_then(|content| content.downcast_ref::<ItemsControl>()) {
            scrollable = items_control.presenter().and_then(|presenter| presenter.panel()).map(Ref::upcast);
        }

        if !scrollable.is_some_and(|scrollable| as_scroll_snap_points_info(&scrollable).is_some()) {
            return;
        }

        self.scroll_gesture_snap_points
            .borrow_mut()
            .get_or_insert_with(HashMap::new);

        let mut offset = self.offset();

        if self.horizontal_snap_points_type() != SnapPointsType::None
            && self.vertical_snap_points_type() != SnapPointsType::None
        {
            return;
        }

        fn get_distance(speed: f64) -> f64 {
            let time =
                (ScrollGestureRecognizer::INERTIAL_SCROLL_SPEED_END / speed.abs()).ln() / ScrollGestureRecognizer::INERTIAL_RESISTANCE.ln();

            let mut time_elapsed = 0.0;
            let mut distance = 0.0;
            let mut step = 0.0;

            while time_elapsed <= time {
                let s = speed * ScrollGestureRecognizer::INERTIAL_RESISTANCE.powf(time_elapsed);
                distance += s * step;

                time_elapsed += 0.016f32 as f64;
                step = 0.016f32 as f64;
            }

            distance
        }

        let mut x_distance = 0.0;
        let mut y_distance = 0.0;

        if self.horizontal_snap_points_type() != SnapPointsType::None {
            x_distance = if self.horizontal_snap_points_type() == SnapPointsType::Mandatory {
                get_distance(e.inertia().x)
            } else {
                0.0
            };
        }

        if self.vertical_snap_points_type() != SnapPointsType::None {
            y_distance = if self.vertical_snap_points_type() == SnapPointsType::Mandatory {
                get_distance(e.inertia().y)
            } else {
                0.0
            };
        }

        offset = Vector::new(offset.x + x_distance, offset.y + y_distance);

        let snapped = self.snap_offset(offset, Vector::default(), false);
        let mut points = self.scroll_gesture_snap_points.borrow_mut();
        let points = points.get_or_insert_with(HashMap::new);
        points.insert(e.id(), snapped);
    }

    fn bring_into_view_requested(&self, e: &RequestBringIntoViewEventArgs) {
        if let Some(target_object) = &e.target_object {
            e.set_handled(self.bring_descendant_into_view(target_object, e.target_rect()));
        }
    }

    fn child_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<Ref<Control>>>();
        self.update_scrollable_subscription(new_value);

        if old_value.is_some() {
            self.set_current_value(Self::offset_property(), Vector::default());
        }
    }

    fn update_scrollable_subscription(&self, child: Option<Ref<Control>>) {
        let old_subscription = self.logical_scroll_subscription.borrow_mut().take();
        if let Some(old_subscription) = old_subscription {
            old_subscription.dispose();
        }

        let Some(child) = child else { return };
        let Some(scrollable) = as_logical_scrollable(&child) else {
            return;
        };

        let weak_this = self.to_ref().downgrade();
        let weak_sender = child.downgrade();
        let scroll_invalidated = scrollable.scroll_invalidated(Rc::new(move || {
            if let (Some(this), Some(sender)) = (weak_this.upgrade(), weak_sender.upgrade()) {
                this.update_from_scrollable(&sender);
            }
        }));

        if scrollable.is_logical_scroll_enabled() {
            let target = child.clone();
            let can_horizontally_scroll =
                FerroObjectExtensions::get_observable(fo(&self), Self::can_horizontally_scroll_property())
                    .subscribe_fn(move |x| {
                        if let Some(scrollable) = as_logical_scrollable(&target) {
                            scrollable.set_can_horizontally_scroll(x);
                        }
                    });

            let target = child.clone();
            let can_vertically_scroll =
                FerroObjectExtensions::get_observable(fo(&self), Self::can_vertically_scroll_property()).subscribe_fn(
                    move |x| {
                        if let Some(scrollable) = as_logical_scrollable(&target) {
                            scrollable.set_can_vertically_scroll(x);
                        }
                    },
                );

            let target = child.clone();
            let skipped = Cell::new(false);
            let offset =
                FerroObjectExtensions::get_observable(fo(&self), Self::offset_property()).subscribe_fn(move |x| {
                    if !skipped.replace(true) {
                        return;
                    }
                    if let Some(scrollable) = as_logical_scrollable(&target) {
                        scrollable.set_offset(x);
                    }
                });

            let subscription: Rc<dyn IDisposable> = Rc::new(CompositeDisposable::from_disposables([
                can_horizontally_scroll,
                can_vertically_scroll,
                offset,
                scroll_invalidated,
            ]));
            *self.logical_scroll_subscription.borrow_mut() = Some(subscription);
            self.update_from_scrollable(&child);
        }
    }

    fn update_from_scrollable(&self, child: &Ref<Control>) {
        let Some(scrollable) = as_logical_scrollable(child) else {
            return;
        };
        let logical_scroll = self.logical_scroll_subscription.borrow().is_some();

        if logical_scroll != scrollable.is_logical_scroll_enabled() {
            self.update_scrollable_subscription(self.child());
            self.set_current_value(Self::offset_property(), Vector::default());
            self.invalidate_measure();
        } else if scrollable.is_logical_scroll_enabled() {
            self.set_viewport(scrollable.viewport());
            self.set_extent(scrollable.extent());
            self.set_current_value(Self::offset_property(), scrollable.offset());
        }
    }

    fn ensure_anchor_element_selection(&self) {
        if !self.is_anchor_element_dirty.get() {
            return;
        }
        let Some(candidates) = self.anchor_candidates.borrow().clone() else {
            return;
        };

        *self.anchor_element.borrow_mut() = None;
        self.anchor_element_bounds.set(Rect::default());
        self.is_anchor_element_dirty.set(false);

        let Some(child) = self.child() else {
            panic!("The scroll content presenter has anchor candidates but no child.");
        };

        let mut best_candidate: Option<Ref<Control>> = None;
        let mut best_candidate_distance = f64::MAX;

        // Find the anchor candidate that is scrolled closest to the top-left
        // of this ScrollContentPresenter.
        for element in candidates.iter() {
            if !element.is_visible() {
                continue;
            }

            if let Some(bounds) = self.get_viewport_bounds(element, &child) {
                let distance: Vector = bounds.position().into();
                let candidate_distance = distance.length().abs();

                if candidate_distance < best_candidate_distance {
                    best_candidate = Some(element.clone());
                    best_candidate_distance = candidate_distance;
                }
            }
        }

        if let Some(best_candidate) = best_candidate {
            // We have a candidate, calculate its bounds relative to the
            // child. Because these bounds aren't relative to the
            // ScrollContentPresenter itself, if they change then we know it
            // wasn't just due to scrolling.
            let unscrolled_bounds = Self::translate_bounds(&best_candidate, &child);
            *self.anchor_element.borrow_mut() = Some(best_candidate);
            self.anchor_element_bounds.set(unscrolled_bounds);
        }
    }

    /// The bounds of `element` relative to the viewport, when they intersect
    /// it.
    fn get_viewport_bounds(&self, element: &Ref<Control>, child: &Ref<Control>) -> Option<Rect> {
        let child_bounds = Self::try_translate_bounds(element, child)?;

        // We want the bounds relative to the new offset, regardless of
        // whether the child control has actually been arranged to this
        // offset yet, so translate first to the child control and then apply
        // the offset rather than translating directly to this control.
        let this_bounds = Rect::from_size(self.bounds().size());
        let bounds = Rect::from_position_size(child_bounds.position() - self.offset(), child_bounds.size());
        bounds.intersects(this_bounds).then_some(bounds)
    }

    fn translate_bounds(control: &Ref<Control>, to: &Ref<Control>) -> Rect {
        match Self::try_translate_bounds(control, to) {
            Some(bounds) => bounds,
            None => panic!("The control's bounds could not be translated to the requested control."),
        }
    }

    fn try_translate_bounds(control: &Ref<Control>, to: &Ref<Control>) -> Option<Rect> {
        if !control.is_visible() {
            return None;
        }

        let p = control.translate_point(Default::default(), to)?;
        Some(Rect::from_position_size(p, control.bounds().size()))
    }

    fn update_snap_points(&self) {
        let content = self.content();
        let scrollable = self.get_scroll_snap_points_info(content.as_ref());

        if let Some(scroll_snap_points_info) = scrollable.as_ref().and_then(|s| as_scroll_snap_points_info(s)) {
            self.are_vertical_snap_points_regular
                .set(scroll_snap_points_info.are_vertical_snap_points_regular());
            self.are_horizontal_snap_points_regular
                .set(scroll_snap_points_info.are_horizontal_snap_points_regular());

            if !self.are_vertical_snap_points_regular.get() {
                let points = scroll_snap_points_info
                    .get_irregular_snap_points(Orientation::Vertical, self.vertical_snap_points_alignment());
                *self.vertical_snap_points.borrow_mut() = Some(points);
            } else {
                *self.vertical_snap_points.borrow_mut() = Some(Vec::new());
                let (snap_point, offset) = scroll_snap_points_info
                    .get_regular_snap_points(Orientation::Vertical, self.vertical_snap_points_alignment());
                self.vertical_snap_point.set(snap_point);
                self.vertical_snap_point_offset.set(offset);
            }

            if !self.are_horizontal_snap_points_regular.get() {
                let points = scroll_snap_points_info
                    .get_irregular_snap_points(Orientation::Horizontal, self.horizontal_snap_points_alignment());
                *self.horizontal_snap_points.borrow_mut() = Some(points);
            } else {
                *self.horizontal_snap_points.borrow_mut() = Some(Vec::new());
                let (snap_point, offset) = scroll_snap_points_info
                    .get_regular_snap_points(Orientation::Horizontal, self.horizontal_snap_points_alignment());
                self.horizontal_snap_point.set(snap_point);
                self.horizontal_snap_point_offset.set(offset);
            }
        } else {
            *self.horizontal_snap_points.borrow_mut() = Some(Vec::new());
            *self.vertical_snap_points.borrow_mut() = Some(Vec::new());
        }
    }

    fn snap_offset(&self, mut offset: Vector, direction: Vector, snap_to_next: bool) -> Vector {
        let content = self.content();
        let scrollable = self.get_scroll_snap_points_info(content.as_ref());

        if scrollable.is_none()
            || (self.vertical_snap_points_type() == SnapPointsType::None
                && self.horizontal_snap_points_type() == SnapPointsType::None)
        {
            return offset;
        }

        let diff = self.get_alignment_diff();

        let has_vertical_points = self
            .vertical_snap_points
            .borrow()
            .as_ref()
            .is_some_and(|points| !points.is_empty());
        if self.vertical_snap_points_type() != SnapPointsType::None
            && (self.are_vertical_snap_points_regular.get() || has_vertical_points)
            && (!snap_to_next || direction.y != 0.0)
        {
            let estimated_offset = Vector::new(offset.x, offset.y + diff.y);
            let mut previous_snap_point = 0.0;
            let mut next_snap_point = 0.0;
            let mut mid_point = 0.0;

            if self.are_vertical_snap_points_regular.get() {
                let snap_point = self.vertical_snap_point.get();
                previous_snap_point = ((estimated_offset.y / snap_point) as i32) as f64 * snap_point
                    + self.vertical_snap_point_offset.get();
                next_snap_point = previous_snap_point + snap_point;
                mid_point = (previous_snap_point + next_snap_point) / 2.0;
            } else if has_vertical_points {
                if let Some(points) = self.vertical_snap_points.borrow().as_ref() {
                    (previous_snap_point, next_snap_point) = Self::find_nearest_snap_point(points, estimated_offset.y);
                }
                mid_point = (previous_snap_point + next_snap_point) / 2.0;
            }

            let nearest_snap_point = if snap_to_next {
                if direction.y > 0.0 {
                    previous_snap_point
                } else {
                    next_snap_point
                }
            } else if estimated_offset.y < mid_point {
                previous_snap_point
            } else {
                next_snap_point
            };

            offset = Vector::new(offset.x, nearest_snap_point - diff.y);
        }

        let has_horizontal_points = self
            .horizontal_snap_points
            .borrow()
            .as_ref()
            .is_some_and(|points| !points.is_empty());
        if self.horizontal_snap_points_type() != SnapPointsType::None
            && (self.are_horizontal_snap_points_regular.get() || has_horizontal_points)
            && (!snap_to_next || direction.x != 0.0)
        {
            let estimated_offset = Vector::new(offset.x + diff.x, offset.y);
            let mut previous_snap_point = 0.0;
            let mut next_snap_point = 0.0;
            let mut mid_point = 0.0;

            if self.are_horizontal_snap_points_regular.get() {
                let snap_point = self.horizontal_snap_point.get();
                previous_snap_point = ((estimated_offset.x / snap_point) as i32) as f64 * snap_point
                    + self.horizontal_snap_point_offset.get();
                next_snap_point = previous_snap_point + snap_point;
                mid_point = (previous_snap_point + next_snap_point) / 2.0;
            } else if has_horizontal_points {
                if let Some(points) = self.horizontal_snap_points.borrow().as_ref() {
                    (previous_snap_point, next_snap_point) = Self::find_nearest_snap_point(points, estimated_offset.x);
                }
                mid_point = (previous_snap_point + next_snap_point) / 2.0;
            }

            let nearest_snap_point = if snap_to_next {
                if direction.x > 0.0 {
                    previous_snap_point
                } else {
                    next_snap_point
                }
            } else if estimated_offset.x < mid_point {
                previous_snap_point
            } else {
                next_snap_point
            };

            offset = Vector::new(nearest_snap_point - diff.x, offset.y);
        }

        offset
    }

    fn get_alignment_diff(&self) -> Vector {
        let mut vector = Vector::default();
        let viewport = self.viewport();

        match self.vertical_snap_points_alignment() {
            SnapPointsAlignment::Center => vector += Vector::new(0.0, viewport.height / 2.0),
            SnapPointsAlignment::Far => vector += Vector::new(0.0, viewport.height),
            SnapPointsAlignment::Near => {}
        }

        match self.horizontal_snap_points_alignment() {
            SnapPointsAlignment::Center => vector += Vector::new(viewport.width / 2.0, 0.0),
            SnapPointsAlignment::Far => vector += Vector::new(viewport.width, 0.0),
            SnapPointsAlignment::Near => {}
        }

        vector
    }

    /// The snap points before and after `value` in the sorted, non-empty
    /// `snap_points`.
    fn find_nearest_snap_point(snap_points: &[f64], value: f64) -> (f64, f64) {
        match snap_points.binary_search_by(|point| point.partial_cmp(&value).unwrap_or(Ordering::Equal)) {
            Err(point) => {
                let previous_snap_point = snap_points[point.saturating_sub(1)];
                let next_snap_point = if point >= snap_points.len() {
                    snap_points[snap_points.len() - 1]
                } else {
                    snap_points[point]
                };
                (previous_snap_point, next_snap_point)
            }
            Ok(point) => (snap_points[point], snap_points[point]),
        }
    }

    /// The content viewed as a provider of snap points, if it is one. Keeps
    /// the snap points changed subscriptions on the current provider.
    fn get_scroll_snap_points_info(&self, content: Option<&BoxedValue>) -> Option<Ref<Control>> {
        let mut scrollable = content.and_then(Control::from_boxed);

        let own_content = self.content().as_ref().and_then(Control::from_boxed);

        if let Some(items_control) = own_content.as_ref().and_then(|content| content.downcast_ref::<ItemsControl>()) {
            scrollable = items_control.presenter().and_then(|presenter| presenter.panel()).map(Ref::upcast);
        }

        if let Some(items_presenter) = own_content.as_ref().and_then(|content| content.downcast_ref::<ItemsPresenter>())
        {
            scrollable = items_presenter.panel().map(Ref::upcast);
        }

        let snap_points_info = scrollable.filter(|scrollable| as_scroll_snap_points_info(scrollable).is_some());

        let current = self.scroll_snap_points_info.borrow().clone();
        if snap_points_info != current {
            let old_subscriptions = self.scroll_snap_points_subscriptions.borrow_mut().take();
            if let Some([vertical, horizontal]) = old_subscriptions {
                vertical.dispose();
                horizontal.dispose();
            }

            *self.scroll_snap_points_info.borrow_mut() = snap_points_info.clone();

            if let Some(info) = snap_points_info
                .as_ref()
                .and_then(|info| as_scroll_snap_points_info(info))
            {
                let weak = self.to_ref().downgrade();
                let handler: SnapPointsChangedHandler = Rc::new(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.update_snap_points();
                    }
                });
                let vertical = info.vertical_snap_points_changed(handler.clone());
                let horizontal = info.horizontal_snap_points_changed(handler);
                *self.scroll_snap_points_subscriptions.borrow_mut() = Some([vertical, horizontal]);
            }
        }

        snap_points_info
    }

    fn adjust_delta_for_flow_direction(delta: Vector, flow_direction: FlowDirection) -> Vector {
        if flow_direction == FlowDirection::RightToLeft {
            return delta.with_x(-delta.x);
        }
        delta
    }
}

/// The object as its root class, for calls that would otherwise resolve to
/// a member of an intermediate class.
fn fo(object: &ferroui_base::FerroObject) -> &ferroui_base::FerroObject {
    object
}
