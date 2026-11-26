//! WinUI's `ScrollBarController` (`controls/dev/ScrollView/ScrollBarController.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md): the scroll controller of a scroll bar of a `ScrollView`.

use super::{ScrollingAnimationMode, ScrollingScrollOptions, ScrollingSnapPointsMode};
use crate::primitives::{
    AddScrollVelocityRequestedHandler, IScrollController, ScrollBar, ScrollByRequestedHandler,
    ScrollControllerRequests, ScrollEventArgs, ScrollEventType, ScrollToRequestedHandler,
};
use ferroui_base::reactive::Subscription;
use ferroui_base::Ref;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// `s_defaultViewportToSmallChangeRatio`.
const DEFAULT_VIEWPORT_TO_SMALL_CHANGE_RATIO: f64 = 8.0;
/// `s_minMaxEpsilon`.
const MIN_MAX_EPSILON: f64 = 0.001;

/// Drives a [`ScrollBar`] from the values of a scroll presenter and turns its scrolls into
/// requests.
pub(crate) struct ScrollBarController {
    scroll_bar: Ref<ScrollBar>,
    requests: ScrollControllerRequests,
    is_scrollable: Cell<bool>,
    is_scrolling_with_mouse: Cell<bool>,
    last_scroll_bar_value: Cell<f64>,
    // Set while the presenter's values are written to the bar: its value changes are not scrolls.
    updating: Cell<bool>,
    _scroll: Cell<Option<Subscription>>,
}

impl ScrollBarController {
    /// The controller of `scroll_bar` (`SetScrollBar`).
    pub(crate) fn new(scroll_bar: Ref<ScrollBar>) -> Rc<Self> {
        let controller = Rc::new(Self {
            scroll_bar: scroll_bar.clone(),
            requests: ScrollControllerRequests::new(),
            is_scrollable: Cell::new(false),
            is_scrolling_with_mouse: Cell::new(false),
            last_scroll_bar_value: Cell::new(scroll_bar.value()),
            updating: Cell::new(false),
            _scroll: Cell::new(None),
        });
        let weak: Weak<Self> = Rc::downgrade(&controller);
        let subscription = scroll_bar.scroll(move |args| {
            if let Some(controller) = weak.upgrade() {
                controller.on_scroll(args);
            }
        });
        controller._scroll.set(Some(subscription));
        controller
    }

    /// The scroll bar.
    pub(crate) fn scroll_bar(&self) -> &Ref<ScrollBar> {
        &self.scroll_bar
    }

    /// `OnScroll`: a scroll of the thumb is a `ScrollToRequested` without animation; the buttons and
    /// the track are a `ScrollByRequested` of a small or a large change, animated.
    fn on_scroll(&self, args: &ScrollEventArgs) {
        if self.updating.get() {
            return;
        }
        let event_type = args.scroll_event_type();
        let bar = &self.scroll_bar;
        if !self.is_scrollable.get() {
            bar.set_range_value(self.last_scroll_bar_value.get());
            return;
        }
        let requested = match event_type {
            ScrollEventType::EndScroll => {
                self.is_scrolling_with_mouse.set(false);
                true
            }
            ScrollEventType::ThumbTrack => {
                self.is_scrolling_with_mouse.set(true);
                let options = ScrollingScrollOptions::with_snap_points_mode(
                    ScrollingAnimationMode::Disabled,
                    ScrollingSnapPointsMode::Ignore,
                );
                self.requests.raise_scroll_to(args.new_value(), options) != -1
            }
            _ => {
                let last = self.last_scroll_bar_value.get();
                let mut change = match event_type {
                    ScrollEventType::LargeDecrement => {
                        -(last - bar.minimum()).min(bar.large_change())
                    }
                    ScrollEventType::LargeIncrement => {
                        (bar.maximum() - last).min(bar.large_change())
                    }
                    ScrollEventType::SmallDecrement => {
                        -(last - bar.minimum()).min(bar.small_change())
                    }
                    _ => (bar.maximum() - last).min(bar.small_change()),
                };
                if args.new_value() - bar.minimum() < MIN_MAX_EPSILON {
                    change -= MIN_MAX_EPSILON;
                } else if bar.maximum() - args.new_value() < MIN_MAX_EPSILON {
                    change += MIN_MAX_EPSILON;
                }
                let options = ScrollingScrollOptions::new(ScrollingAnimationMode::Auto);
                self.requests.raise_scroll_by(change, options) != -1
            }
        };
        if !requested {
            bar.set_range_value(self.last_scroll_bar_value.get());
        }
        self.last_scroll_bar_value.set(bar.value());
    }
}

impl IScrollController for ScrollBarController {
    /// `CanScroll`: the bar is enabled, has a range and the axis can scroll.
    fn can_scroll(&self) -> bool {
        let bar = &self.scroll_bar;
        bar.is_effectively_enabled() && bar.maximum() > bar.minimum() && self.is_scrollable.get()
    }

    fn is_scrolling_with_mouse(&self) -> bool {
        self.is_scrolling_with_mouse.get()
    }

    fn set_is_scrollable(&self, is_scrollable: bool) {
        self.is_scrollable.set(is_scrollable);
    }

    /// `SetValues`: the range, the value and the viewport of the bar, a large change of a viewport
    /// and a small change of an eighth of it.
    fn set_values(&self, min_offset: f64, max_offset: f64, offset: f64, viewport_length: f64) {
        let bar = &self.scroll_bar;
        let max_offset = max_offset.max(min_offset);
        self.updating.set(true);
        bar.set_minimum(min_offset);
        bar.set_maximum(max_offset);
        bar.set_range_value(offset.clamp(min_offset, max_offset));
        bar.set_viewport_size(viewport_length);
        bar.set_large_change(viewport_length);
        bar.set_small_change((viewport_length / DEFAULT_VIEWPORT_TO_SMALL_CHANGE_RATIO).max(1.0));
        self.updating.set(false);
        self.last_scroll_bar_value.set(bar.value());
    }

    fn scroll_to_requested(&self, handler: Rc<ScrollToRequestedHandler>) -> Subscription {
        self.requests.add_scroll_to(handler)
    }

    fn scroll_by_requested(&self, handler: Rc<ScrollByRequestedHandler>) -> Subscription {
        self.requests.add_scroll_by(handler)
    }

    fn add_scroll_velocity_requested(
        &self,
        handler: Rc<AddScrollVelocityRequestedHandler>,
    ) -> Subscription {
        self.requests.add_add_scroll_velocity(handler)
    }
}
