//! The keyboard events of a sink and the key repeat (the port of
//! `WindowImplBase.Keyboard.cs`). On Wayland the client repeats a key that is
//! held: the compositor only says at which rate and after which delay.

use crate::window_impl_base::{RawEvent, Sink};
use ferroui_base::input::raw::{IRawInputEventArgs, RawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use crate::server::persistent::wayland_key_info::WaylandKeyInfo;
use ferroui_base::input::{IInputDevice, IInputRoot, Key, KeyDeviceType, RawInputModifiers};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The keyboard left the surface: an event of the queue of the sink, which stops the key
/// repeat in the order of the other events and goes no further.
pub(crate) struct RawKeyboardLeaveEventArgs {
    base: RawInputEventArgs,
}

impl RawKeyboardLeaveEventArgs {
    fn new(device: Rc<dyn IInputDevice>, timestamp: u64, root: Rc<dyn IInputRoot>) -> Self {
        Self { base: RawInputEventArgs::new(device, timestamp, root) }
    }
}

impl IRawInputEventArgs for RawKeyboardLeaveEventArgs {
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        &self.base
    }

    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<RawKeyboardLeaveEventArgs>() {
            Some(self)
        } else {
            IRawInputEventArgs::query_args(&self.base, type_id)
        }
    }
}

/// A change of the values of the key that is held down (`RawHeldKeyChangedEventArgs`, 12.2.0):
/// queued like the key down, so that it cannot overtake the key down that starts the repeat.
pub(crate) struct RawHeldKeyChangedEventArgs {
    base: RawInputEventArgs,
    modifiers: RawInputModifiers,
    key: WaylandKeyInfo,
}

impl RawHeldKeyChangedEventArgs {
    fn new(
        device: Rc<dyn IInputDevice>,
        root: Rc<dyn IInputRoot>,
        modifiers: RawInputModifiers,
        key: WaylandKeyInfo,
    ) -> Self {
        Self { base: RawInputEventArgs::new(device, 0, root), modifiers, key }
    }
}

impl IRawInputEventArgs for RawHeldKeyChangedEventArgs {
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        &self.base
    }

    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<RawHeldKeyChangedEventArgs>() {
            Some(self)
        } else {
            IRawInputEventArgs::query_args(&self.base, type_id)
        }
    }
}

/// The raw key event of a resolved key (`CreateKeyEventArgs`).
pub(crate) fn key_event_args(
    device: Rc<dyn IInputDevice>,
    timestamp: u64,
    root: Rc<dyn IInputRoot>,
    type_: RawKeyEventType,
    modifiers: RawInputModifiers,
    key: &WaylandKeyInfo,
) -> RawKeyEventArgs {
    let args = RawKeyEventArgs::new(
        device,
        timestamp,
        root,
        type_,
        key.key,
        modifiers,
        key.physical_key,
        key.key_symbol.clone(),
        KeyDeviceType::Keyboard,
    );
    key.apply_to(&args);
    args
}

/// Whether a key event is of the key `repeat` repeats: keys are matched by their key code
/// (12.2.0), since the key of a held key can change with the modifiers.
pub(crate) fn is_repeated_key(repeat: &WaylandKeyInfo, args: &RawKeyEventArgs) -> bool {
    args.platform_key_info().is_some_and(|info| info.scan_code == repeat.platform_key_info.scan_code)
}

/// The interval between two repeats of a key at a rate in keys a second.
pub(crate) fn repeat_interval(rate: i32) -> Duration {
    Duration::from_millis((1000 / rate.max(1)).max(1) as u64)
}

/// Whether a key repeats while it is held: modifier keys don't.
pub(crate) fn key_repeats(key: Key) -> bool {
    !matches!(
        key,
        Key::LeftShift | Key::RightShift | Key::LeftCtrl | Key::RightCtrl | Key::LeftAlt | Key::RightAlt | Key::LWin | Key::RWin
    )
}

struct RepeatTimer {
    timer: Rc<DispatcherTimer>,
    tick: Rc<dyn IDisposable>,
}

/// Key repeat state (managed on UI thread via a dispatcher timer).
pub(crate) struct KeyRepeat {
    sink: Weak<Sink>,
    key_repeat_timer: RefCell<Option<RepeatTimer>>,
    repeat_key: RefCell<Option<WaylandKeyInfo>>,
    repeat_modifiers: Cell<RawInputModifiers>,
    key_repeat_delay: Cell<i32>,
    key_repeat_rate: Cell<i32>,
}

impl KeyRepeat {
    pub(crate) fn new(sink: Weak<Sink>) -> Self {
        Self {
            sink,
            key_repeat_timer: RefCell::new(None),
            repeat_key: RefCell::new(None),
            repeat_modifiers: Cell::new(RawInputModifiers::NONE),
            key_repeat_delay: Cell::new(0),
            key_repeat_rate: Cell::new(0),
        }
    }

    pub(crate) fn set_info(&self, rate: i32, delay: i32) {
        self.key_repeat_rate.set(rate);
        self.key_repeat_delay.set(delay);
    }

    /// Looks at an event of the queue before it is raised. Returns whether the event is
    /// consumed.
    pub(crate) fn handle_keyboard_dispatch(&self, args: &RawEvent) -> bool {
        if args.downcast_ref::<RawKeyboardLeaveEventArgs>().is_some() {
            self.stop();
            return true;
        }

        if let Some(held) = args.downcast_ref::<RawHeldKeyChangedEventArgs>() {
            let mut repeat_key = self.repeat_key.borrow_mut();
            let scan_code = held.key.platform_key_info.scan_code;
            if repeat_key.as_ref().is_some_and(|k| k.platform_key_info.scan_code == scan_code) {
                *repeat_key = Some(held.key.clone());
                self.repeat_modifiers.set(held.modifiers);
            }
            return true;
        }

        if let Some(key_args) = args.downcast_ref::<RawKeyEventArgs>() {
            if key_args.type_() == RawKeyEventType::KeyDown {
                self.start(key_args);
            } else if key_args.type_() == RawKeyEventType::KeyUp
                && self.repeat_key.borrow().as_ref().is_some_and(|repeat| is_repeated_key(repeat, key_args))
            {
                self.stop();
            }
        }

        false
    }

    fn start(&self, key_args: &RawKeyEventArgs) {
        self.stop();

        if self.key_repeat_rate.get() <= 0 {
            return;
        }

        // Modifier keys don't repeat
        if !key_repeats(key_args.key()) {
            return;
        }

        // Every key of this backend has its platform data; one without is not repeated.
        let Some(platform_key_info) = key_args.platform_key_info() else {
            return;
        };
        *self.repeat_key.borrow_mut() = Some(WaylandKeyInfo {
            key: key_args.key(),
            physical_key: key_args.physical_key(),
            key_symbol: key_args.key_symbol(),
            key_text: key_args.key_text(),
            alternate_key_text: key_args.alternate_key_text(),
            consumed_modifiers: key_args.consumed_modifiers(),
            platform_key_info,
        });
        self.repeat_modifiers.set(key_args.modifiers());

        let timer = DispatcherTimer::with_interval(
            repeat_interval(self.key_repeat_rate.get()),
            // Ensure it doesn't block any actual input events that would stop the repeat
            DispatcherPriority::from_value(DispatcherPriority::INPUT.value() - 1),
            &Dispatcher::ui_thread(),
        );
        let sink = self.sink.clone();
        let tick = timer.tick(move |_| {
            if let Some(sink) = sink.upgrade() {
                sink.key_repeat.on_tick();
            }
        });
        // Start with the initial delay, then switch to repeat interval on first tick
        timer.set_interval(Duration::from_millis(self.key_repeat_delay.get().max(0) as u64));
        timer.start();
        *self.key_repeat_timer.borrow_mut() = Some(RepeatTimer { timer, tick });
    }

    pub(crate) fn stop(&self) {
        let timer = self.key_repeat_timer.borrow_mut().take();
        if let Some(timer) = timer {
            timer.timer.stop();
            timer.tick.dispose();
        }
        *self.repeat_key.borrow_mut() = None;
    }

    fn on_tick(&self) {
        let Some(sink) = self.sink.upgrade() else {
            self.stop();
            return;
        };
        let (Some(parent), Some(input_root)) = (sink.parent(), sink.input_root()) else {
            self.stop();
            return;
        };
        let Some(timer) = self.key_repeat_timer.borrow().as_ref().map(|timer| timer.timer.clone()) else {
            return;
        };
        let Some(repeat_key) = self.repeat_key.borrow().clone() else {
            self.stop();
            return;
        };

        // After first tick (initial delay), switch to repeat interval
        let interval = repeat_interval(self.key_repeat_rate.get());
        if timer.interval() != interval {
            timer.set_interval(interval);
        }

        // Deviation (12.2.0 `8dae868308`): the repeat is raised on the window directly, as
        // before, so it never reaches `handle_keyboard_dispatch` and needs no marker type
        // (upstream's `RepeatedRawKeyEventArgs`) to keep it from restarting the repeat.
        parent.base().raise_input(Rc::new(key_event_args(
            parent.base().keyboard(),
            0,
            input_root,
            RawKeyEventType::KeyDown,
            self.repeat_modifiers.get(),
            &repeat_key,
        )));
    }
}

impl Sink {
    pub(crate) fn on_key_down(&self, timestamp: u64, modifiers: RawInputModifiers, key: WaylandKeyInfo) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        let keyboard = parent.base().keyboard();
        let args = key_event_args(keyboard, timestamp, input_root, RawKeyEventType::KeyDown, modifiers, &key);
        self.schedule_input(Rc::new(args));
    }

    pub(crate) fn on_key_up(&self, timestamp: u64, modifiers: RawInputModifiers, key: WaylandKeyInfo) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        let keyboard = parent.base().keyboard();
        let args = key_event_args(keyboard, timestamp, input_root, RawKeyEventType::KeyUp, modifiers, &key);
        self.schedule_input(Rc::new(args));
    }

    pub(crate) fn on_held_key_changed(&self, modifiers: RawInputModifiers, key: WaylandKeyInfo) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        let held = RawHeldKeyChangedEventArgs::new(parent.base().keyboard(), input_root, modifiers, key);
        self.schedule_input(Rc::new(held));
    }

    pub(crate) fn on_keyboard_leave(&self) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawKeyboardLeaveEventArgs::new(parent.base().keyboard(), 0, input_root)));
    }

    pub(crate) fn on_key_repeat_info(&self, rate: i32, delay: i32) {
        self.key_repeat.set_info(rate, delay);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_repeat_interval_follows_the_rate_and_is_never_zero() {
        assert_eq!(repeat_interval(25), Duration::from_millis(40));
        assert_eq!(repeat_interval(1), Duration::from_millis(1000));
        assert_eq!(repeat_interval(5000), Duration::from_millis(1));
    }

    #[test]
    fn modifier_keys_do_not_repeat() {
        assert!(key_repeats(Key::A));
        assert!(key_repeats(Key::Return));
        assert!(!key_repeats(Key::LeftShift));
        assert!(!key_repeats(Key::RightAlt));
        assert!(!key_repeats(Key::LWin));
    }
}
