//! The keyboard events of a sink and the key repeat (the port of
//! `WindowImplBase.Keyboard.cs`). On Wayland the client repeats a key that is
//! held: the compositor only says at which rate and after which delay.

use crate::window_impl_base::{RawEvent, Sink};
use ferroui_base::input::raw::{IRawInputEventArgs, RawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::{IInputDevice, IInputRoot, Key, KeyDeviceType, PhysicalKey, RawInputModifiers};
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
    repeat_key: Cell<Key>,
    repeat_physical_key: Cell<PhysicalKey>,
    repeat_modifiers: Cell<RawInputModifiers>,
    repeat_key_symbol: RefCell<Option<String>>,
    key_repeat_delay: Cell<i32>,
    key_repeat_rate: Cell<i32>,
}

impl KeyRepeat {
    pub(crate) fn new(sink: Weak<Sink>) -> Self {
        Self {
            sink,
            key_repeat_timer: RefCell::new(None),
            repeat_key: Cell::new(Key::None),
            repeat_physical_key: Cell::new(PhysicalKey::None),
            repeat_modifiers: Cell::new(RawInputModifiers::NONE),
            repeat_key_symbol: RefCell::new(None),
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

        if let Some(key_args) = args.downcast_ref::<RawKeyEventArgs>() {
            if key_args.type_() == RawKeyEventType::KeyDown {
                self.start(key_args.key(), key_args.physical_key(), key_args.modifiers(), key_args.key_symbol());
            } else if key_args.type_() == RawKeyEventType::KeyUp && self.repeat_key.get() == key_args.key() {
                self.stop();
            }
        }

        false
    }

    fn start(&self, key: Key, physical_key: PhysicalKey, modifiers: RawInputModifiers, key_symbol: Option<String>) {
        self.stop();

        if self.key_repeat_rate.get() <= 0 {
            return;
        }

        // Modifier keys don't repeat
        if !key_repeats(key) {
            return;
        }

        self.repeat_key.set(key);
        self.repeat_physical_key.set(physical_key);
        self.repeat_modifiers.set(modifiers);
        *self.repeat_key_symbol.borrow_mut() = key_symbol;

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

        // After first tick (initial delay), switch to repeat interval
        let interval = repeat_interval(self.key_repeat_rate.get());
        if timer.interval() != interval {
            timer.set_interval(interval);
        }

        parent.base().raise_input(Rc::new(RawKeyEventArgs::new(
            parent.base().keyboard(),
            0,
            input_root,
            RawKeyEventType::KeyDown,
            self.repeat_key.get(),
            self.repeat_modifiers.get(),
            self.repeat_physical_key.get(),
            self.repeat_key_symbol.borrow().clone(),
            KeyDeviceType::Keyboard,
        )));
    }
}

impl Sink {
    pub(crate) fn on_key_down(
        &self,
        timestamp: u64,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
    ) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawKeyEventArgs::new(
            parent.base().keyboard(),
            timestamp,
            input_root,
            RawKeyEventType::KeyDown,
            key,
            modifiers,
            physical_key,
            key_symbol,
            KeyDeviceType::Keyboard,
        )));
    }

    pub(crate) fn on_key_up(
        &self,
        timestamp: u64,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
    ) {
        let (Some(parent), Some(input_root)) = (self.parent(), self.input_root()) else {
            return;
        };
        self.schedule_input(Rc::new(RawKeyEventArgs::new(
            parent.base().keyboard(),
            timestamp,
            input_root,
            RawKeyEventType::KeyUp,
            key,
            modifiers,
            physical_key,
            key_symbol,
            KeyDeviceType::Keyboard,
        )));
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
