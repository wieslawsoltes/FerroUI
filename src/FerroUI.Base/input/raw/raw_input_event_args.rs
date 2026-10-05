use crate::input::{IInputDevice, IInputRoot};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

/// Implemented by every raw input event args type.
///
/// Raw event args form a small class hierarchy rooted at
/// [`RawInputEventArgs`]; a derived type embeds its base as its `base`
/// field and derefs to it. This trait gives the runtime view of that
/// hierarchy.
pub trait IRawInputEventArgs: Any {
    /// The base raw input event args.
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs;

    /// Returns the args viewed as the args type identified by `type_id`, if
    /// the args are of that type or of a type derived from it.
    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any>;
}

impl dyn IRawInputEventArgs {
    /// Returns the args as `T` if they are of type `T` or of a type derived
    /// from it (C# `e as T`).
    #[inline]
    pub fn downcast_ref<T: IRawInputEventArgs>(&self) -> Option<&T> {
        self.query_args(TypeId::of::<T>()).and_then(<dyn Any>::downcast_ref::<T>)
    }
}

impl Deref for dyn IRawInputEventArgs {
    type Target = RawInputEventArgs;

    #[inline]
    fn deref(&self) -> &RawInputEventArgs {
        self.as_raw_input_event_args()
    }
}

/// Declares a raw input event args type derived from another one.
macro_rules! raw_input_event_args {
    ($name:ident : $base:ty) => {
        impl ::std::ops::Deref for $name {
            type Target = $base;

            #[inline]
            fn deref(&self) -> &$base {
                &self.base
            }
        }

        impl $crate::input::raw::IRawInputEventArgs for $name {
            #[inline]
            fn as_raw_input_event_args(&self) -> &$crate::input::raw::RawInputEventArgs {
                $crate::input::raw::IRawInputEventArgs::as_raw_input_event_args(&self.base)
            }

            #[inline]
            fn query_args(&self, type_id: ::std::any::TypeId) -> ::std::option::Option<&dyn ::std::any::Any> {
                if type_id == ::std::any::TypeId::of::<$name>() {
                    ::std::option::Option::Some(self)
                } else {
                    $crate::input::raw::IRawInputEventArgs::query_args(&self.base, type_id)
                }
            }
        }
    };
}

pub(crate) use raw_input_event_args;

/// A raw input event.
///
/// Raw input events are sent from the windowing subsystem to the input
/// manager for processing: this gives an application the opportunity to
/// pre-process the event. After pre-processing they are consumed by the
/// relevant input device and turned into standard events.
pub struct RawInputEventArgs {
    device: Rc<dyn IInputDevice>,
    root: Rc<dyn IInputRoot>,
    handled: Cell<bool>,
    timestamp: Cell<u64>,
}

impl RawInputEventArgs {
    /// Creates raw input event args.
    ///
    /// `timestamp` is the time stamp of the input as reported by the
    /// platform; `root` is the root from which the event originates.
    pub fn new(device: Rc<dyn IInputDevice>, timestamp: u64, root: Rc<dyn IInputRoot>) -> Self {
        Self { device, root, handled: Cell::new(false), timestamp: Cell::new(timestamp) }
    }

    /// The associated device.
    #[inline]
    pub fn device(&self) -> &Rc<dyn IInputDevice> {
        &self.device
    }

    /// The root from which the event originates.
    #[inline]
    pub fn root(&self) -> &Rc<dyn IInputRoot> {
        &self.root
    }

    /// Whether the event was handled.
    ///
    /// If an event is not marked handled after processing, this event will
    /// be returned to the windowing subsystem for further processing.
    #[inline]
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    /// Marks the event as handled or unhandled.
    #[inline]
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value)
    }

    /// The timestamp associated with the event.
    #[inline]
    pub fn timestamp(&self) -> u64 {
        self.timestamp.get()
    }

    /// Sets the timestamp associated with the event.
    #[inline]
    pub fn set_timestamp(&self, value: u64) {
        self.timestamp.set(value)
    }
}

impl IRawInputEventArgs for RawInputEventArgs {
    #[inline]
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        self
    }

    #[inline]
    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<RawInputEventArgs>() {
            Some(self)
        } else {
            None
        }
    }
}
