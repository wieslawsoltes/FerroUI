//! WinUI's `IScrollController` and its request arguments
//! (`controls/dev/ScrollPresenter/ScrollPresenterPrimitives.idl` and the
//! `ScrollController*RequestedEventArgs.cpp` files of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::scroll_view::ScrollingScrollOptions;
use ferroui_base::numerics::Vector2;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::rendering::composition::animations::Vector3KeyFrameAnimation;
use ferroui_base::utilities::HandlerList;
use std::cell::Cell;
use std::rc::Rc;

/// A request of a scroll controller to scroll to `Offset` (WinUI's
/// `ScrollControllerScrollToRequestedEventArgs`). The scroll presenter that handles it sets
/// `CorrelationId` to the id of the scroll it starts.
#[derive(Debug)]
pub struct ScrollControllerScrollToRequestedEventArgs {
    offset: f64,
    options: ScrollingScrollOptions,
    correlation_id: Cell<i32>,
}

impl ScrollControllerScrollToRequestedEventArgs {
    /// `ScrollControllerScrollToRequestedEventArgs(offset, options)`.
    pub fn new(offset: f64, options: ScrollingScrollOptions) -> Self {
        Self {
            offset,
            options,
            correlation_id: Cell::new(-1),
        }
    }

    /// The offset to scroll to, on the axis of the controller.
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// The options of the scroll.
    pub fn options(&self) -> ScrollingScrollOptions {
        self.options
    }

    /// The id of the scroll the presenter started, -1 while no presenter handled the request.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id.get()
    }

    /// Sets the id of the scroll the request started.
    pub fn set_correlation_id(&self, value: i32) {
        self.correlation_id.set(value);
    }
}

/// A request of a scroll controller to scroll by `OffsetDelta` (WinUI's
/// `ScrollControllerScrollByRequestedEventArgs`).
#[derive(Debug)]
pub struct ScrollControllerScrollByRequestedEventArgs {
    offset_delta: f64,
    options: ScrollingScrollOptions,
    correlation_id: Cell<i32>,
}

impl ScrollControllerScrollByRequestedEventArgs {
    /// `ScrollControllerScrollByRequestedEventArgs(offsetDelta, options)`.
    pub fn new(offset_delta: f64, options: ScrollingScrollOptions) -> Self {
        Self {
            offset_delta,
            options,
            correlation_id: Cell::new(-1),
        }
    }

    /// The change of the offset, on the axis of the controller.
    pub fn offset_delta(&self) -> f64 {
        self.offset_delta
    }

    /// The options of the scroll.
    pub fn options(&self) -> ScrollingScrollOptions {
        self.options
    }

    /// The id of the scroll the presenter started, -1 while no presenter handled the request.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id.get()
    }

    /// Sets the id of the scroll the request started.
    pub fn set_correlation_id(&self, value: i32) {
        self.correlation_id.set(value);
    }
}

/// A request of a scroll controller to add a velocity to the offset (WinUI's
/// `ScrollControllerAddScrollVelocityRequestedEventArgs`).
#[derive(Debug)]
pub struct ScrollControllerAddScrollVelocityRequestedEventArgs {
    offset_velocity: f32,
    inertia_decay_rate: Option<f32>,
    correlation_id: Cell<i32>,
}

impl ScrollControllerAddScrollVelocityRequestedEventArgs {
    /// `ScrollControllerAddScrollVelocityRequestedEventArgs(offsetVelocity, inertiaDecayRate)`.
    pub fn new(offset_velocity: f32, inertia_decay_rate: Option<f32>) -> Self {
        Self {
            offset_velocity,
            inertia_decay_rate,
            correlation_id: Cell::new(-1),
        }
    }

    /// The velocity to add, in pixels per second.
    pub fn offset_velocity(&self) -> f32 {
        self.offset_velocity
    }

    /// The share of the velocity lost per second (0.95 when `None`).
    pub fn inertia_decay_rate(&self) -> Option<f32> {
        self.inertia_decay_rate
    }

    /// The id of the change the presenter started, -1 while no presenter handled the request.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id.get()
    }

    /// Sets the id of the change the request started.
    pub fn set_correlation_id(&self, value: i32) {
        self.correlation_id.set(value);
    }
}

/// The handlers of the requests of a scroll controller.
pub type ScrollToRequestedHandler = dyn Fn(&ScrollControllerScrollToRequestedEventArgs);
/// The handlers of `ScrollByRequested`.
pub type ScrollByRequestedHandler = dyn Fn(&ScrollControllerScrollByRequestedEventArgs);
/// The handlers of `AddScrollVelocityRequested`.
pub type AddScrollVelocityRequestedHandler =
    dyn Fn(&ScrollControllerAddScrollVelocityRequestedEventArgs);

/// A control that scrolls one axis of a [`ScrollPresenter`](super::ScrollPresenter) (WinUI's
/// `IScrollController`): the presenter tells it the range, the offset and the viewport
/// (`SetValues`) and whether the axis can scroll (`SetIsScrollable`); the controller asks the
/// presenter to scroll with its requests. The scroll bars of a `ScrollView` and an
/// `AnnotatedScrollBar` are scroll controllers; a scroll presenter takes one per axis
/// (`HorizontalScrollController`, `VerticalScrollController`).
///
/// Difference: the events of WinUI's interface are the methods that add a handler and return a
/// [`Subscription`]; [`ScrollControllerRequests`] keeps the handlers for an implementation.
/// `PanningInfo` (the touch panning of the controller's own visual) and `IsScrollingWithMouse`'s
/// change event are left out (docs/porting/winui-controls.md, section 10.12).
pub trait IScrollController {
    /// Whether the controller can scroll now (`CanScroll`).
    fn can_scroll(&self) -> bool;

    /// Whether the user is dragging the controller with the mouse (`IsScrollingWithMouse`).
    fn is_scrolling_with_mouse(&self) -> bool {
        false
    }

    /// Whether the presenter can scroll on the controller's axis (`SetIsScrollable`).
    fn set_is_scrollable(&self, is_scrollable: bool);

    /// The range, the offset and the viewport of the axis (`SetValues`).
    fn set_values(&self, min_offset: f64, max_offset: f64, offset: f64, viewport_length: f64);

    /// The animation of a scroll the controller requested (`GetScrollAnimation`): `None` keeps
    /// `default_animation`.
    fn get_scroll_animation(
        &self,
        _correlation_id: i32,
        _start_position: Vector2,
        _end_position: Vector2,
        _default_animation: &Rc<Vector3KeyFrameAnimation>,
    ) -> Option<Rc<Vector3KeyFrameAnimation>> {
        None
    }

    /// A scroll the controller requested is complete (`NotifyRequestedScrollCompleted`).
    fn notify_requested_scroll_completed(&self, _correlation_id: i32) {}

    /// Adds a handler of `ScrollToRequested`.
    fn scroll_to_requested(&self, handler: Rc<ScrollToRequestedHandler>) -> Subscription;

    /// Adds a handler of `ScrollByRequested`.
    fn scroll_by_requested(&self, handler: Rc<ScrollByRequestedHandler>) -> Subscription;

    /// Adds a handler of `AddScrollVelocityRequested`.
    fn add_scroll_velocity_requested(
        &self,
        handler: Rc<AddScrollVelocityRequestedHandler>,
    ) -> Subscription;
}

/// The handlers of the requests of a scroll controller, for an implementation of
/// [`IScrollController`].
#[derive(Default)]
pub struct ScrollControllerRequests {
    scroll_to: Rc<HandlerList<ScrollToRequestedHandler>>,
    scroll_by: Rc<HandlerList<ScrollByRequestedHandler>>,
    add_scroll_velocity: Rc<HandlerList<AddScrollVelocityRequestedHandler>>,
}

fn subscribe<H: ?Sized + 'static>(list: &Rc<HandlerList<H>>, handler: Rc<H>) -> Subscription {
    let token = list.add(handler);
    let weak = Rc::downgrade(list);
    Subscription::new(Disposable::create(move || {
        if let Some(list) = weak.upgrade() {
            list.remove(token);
        }
    }))
}

impl ScrollControllerRequests {
    /// Creates the lists without handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a handler of `ScrollToRequested`.
    pub fn add_scroll_to(&self, handler: Rc<ScrollToRequestedHandler>) -> Subscription {
        subscribe(&self.scroll_to, handler)
    }

    /// Adds a handler of `ScrollByRequested`.
    pub fn add_scroll_by(&self, handler: Rc<ScrollByRequestedHandler>) -> Subscription {
        subscribe(&self.scroll_by, handler)
    }

    /// Adds a handler of `AddScrollVelocityRequested`.
    pub fn add_add_scroll_velocity(
        &self,
        handler: Rc<AddScrollVelocityRequestedHandler>,
    ) -> Subscription {
        subscribe(&self.add_scroll_velocity, handler)
    }

    /// Raises `ScrollToRequested`; the correlation id the handler set, -1 when none did.
    pub fn raise_scroll_to(&self, offset: f64, options: ScrollingScrollOptions) -> i32 {
        let args = ScrollControllerScrollToRequestedEventArgs::new(offset, options);
        for (_, handler) in self.scroll_to.snapshot().iter() {
            handler(&args);
        }
        args.correlation_id()
    }

    /// Raises `ScrollByRequested`; the correlation id the handler set, -1 when none did.
    pub fn raise_scroll_by(&self, offset_delta: f64, options: ScrollingScrollOptions) -> i32 {
        let args = ScrollControllerScrollByRequestedEventArgs::new(offset_delta, options);
        for (_, handler) in self.scroll_by.snapshot().iter() {
            handler(&args);
        }
        args.correlation_id()
    }

    /// Raises `AddScrollVelocityRequested`; the correlation id the handler set, -1 when none did.
    pub fn raise_add_scroll_velocity(
        &self,
        offset_velocity: f32,
        inertia_decay_rate: Option<f32>,
    ) -> i32 {
        let args = ScrollControllerAddScrollVelocityRequestedEventArgs::new(
            offset_velocity,
            inertia_decay_rate,
        );
        for (_, handler) in self.add_scroll_velocity.snapshot().iter() {
            handler(&args);
        }
        args.correlation_id()
    }
}
