//! The pointer and touch events of a sink (the port of
//! `WindowImplBase.Pointer.cs`): each becomes a raw input event of the
//! framework, queued for the dispatcher.

use crate::server::persistent::i_w_surface_event_sink::PlatformInputEventCookie;
use crate::window_impl_base::Sink;
use ferroui_base::input::raw::{RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType, RawTouchEventArgs};
use ferroui_base::input::RawInputModifiers;
use ferroui_base::{Point, Vector};
use std::any::Any;
use std::rc::Rc;

/// The cookie as the raw event carries it: the shared value behind a handle of the UI thread.
fn cookie_of(platform_cookie: Option<PlatformInputEventCookie>) -> Option<Rc<dyn Any>> {
    platform_cookie.map(|cookie| Rc::new(cookie) as Rc<dyn Any>)
}

impl Sink {
    pub(crate) fn on_pointer_enter(&self, timestamp: u64, _serial: u32, position: Point) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawPointerEventArgs::new(
            parent.base().mouse(),
            timestamp,
            input_root,
            RawPointerEventType::Move,
            position,
            RawInputModifiers::NONE,
        )));
    }

    pub(crate) fn on_pointer_leave(&self, _serial: u32) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawPointerEventArgs::new(
            parent.base().mouse(),
            0,
            input_root,
            RawPointerEventType::LeaveWindow,
            Point::default(),
            RawInputModifiers::NONE,
        )));
    }

    pub(crate) fn on_pointer_motion(&self, timestamp: u64, position: Point, modifiers: RawInputModifiers) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawPointerEventArgs::new(
            parent.base().mouse(),
            timestamp,
            input_root,
            RawPointerEventType::Move,
            position,
            modifiers,
        )));
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn on_pointer_button(
        &self,
        timestamp: u64,
        _serial: u32,
        type_: RawPointerEventType,
        modifiers: RawInputModifiers,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    ) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        let args = RawPointerEventArgs::new(parent.base().mouse(), timestamp, input_root, type_, position, modifiers);
        args.set_platform_input_event_cookie(cookie_of(platform_cookie));
        self.schedule_input(Rc::new(args));
    }

    pub(crate) fn on_pointer_axis(&self, timestamp: u64, delta: Vector, modifiers: RawInputModifiers, position: Point) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawMouseWheelEventArgs::new(
            parent.base().mouse(),
            timestamp,
            input_root,
            position,
            delta,
            modifiers,
        )));
    }

    pub(crate) fn on_touch_down(
        &self,
        timestamp: u64,
        touch_id: i32,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    ) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        let args = RawTouchEventArgs::new(
            parent.base().touch(),
            timestamp,
            input_root,
            RawPointerEventType::TouchBegin,
            position,
            RawInputModifiers::NONE,
            i64::from(touch_id),
        );
        args.set_platform_input_event_cookie(cookie_of(platform_cookie));
        self.schedule_input(Rc::new(args));
    }

    pub(crate) fn on_touch_move(&self, timestamp: u64, touch_id: i32, position: Point) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawTouchEventArgs::new(
            parent.base().touch(),
            timestamp,
            input_root,
            RawPointerEventType::TouchUpdate,
            position,
            RawInputModifiers::NONE,
            i64::from(touch_id),
        )));
    }

    pub(crate) fn on_touch_up(&self, timestamp: u64, touch_id: i32, position: Point) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawTouchEventArgs::new(
            parent.base().touch(),
            timestamp,
            input_root,
            RawPointerEventType::TouchEnd,
            position,
            RawInputModifiers::NONE,
            i64::from(touch_id),
        )));
    }

    pub(crate) fn on_touch_cancel(&self, touch_id: i32, position: Point) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawTouchEventArgs::new(
            parent.base().touch(),
            0,
            input_root,
            RawPointerEventType::TouchCancel,
            position,
            RawInputModifiers::NONE,
            i64::from(touch_id),
        )));
    }
}
