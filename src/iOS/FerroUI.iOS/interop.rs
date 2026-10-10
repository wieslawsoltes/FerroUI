//! The functions of Core Foundation and of libdispatch the dispatcher
//! calls: the run loop of the main thread, an observer and a timer on it,
//! and the main dispatch queue.
//!
//! The declarations are the C declarations of the system headers
//! (`CFRunLoop.h`, `CFDate.h`, `dispatch/queue.h`).

#![allow(non_upper_case_globals, non_snake_case)]

use std::ffi::{c_ulong, c_void};

/// `CFOptionFlags`, the flags of the run loop activities among them.
pub type CFOptionFlags = c_ulong;

/// `kCFRunLoopBeforeSources`.
pub const kCFRunLoopBeforeSources: CFOptionFlags = 1 << 2;
/// `kCFRunLoopBeforeWaiting`.
pub const kCFRunLoopBeforeWaiting: CFOptionFlags = 1 << 5;
/// `kCFRunLoopAfterWaiting`.
pub const kCFRunLoopAfterWaiting: CFOptionFlags = 1 << 6;

/// `CFRunLoopObserverCallBack`.
pub type CFRunLoopObserverCallBack = extern "C" fn(observer: *mut c_void, activity: CFOptionFlags, info: *mut c_void);

/// `CFRunLoopTimerCallBack`.
pub type CFRunLoopTimerCallBack = extern "C" fn(timer: *mut c_void, info: *mut c_void);

/// `dispatch_function_t`.
pub type DispatchFunction = extern "C" fn(context: *mut c_void);

/// The storage of a dispatch queue, of which only the address is used.
#[repr(C)]
pub struct DispatchQueue {
    _opaque: [u8; 0],
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    /// The default mode of a run loop (a `CFStringRef`).
    pub static kCFRunLoopDefaultMode: *const c_void;

    pub fn CFRunLoopGetMain() -> *mut c_void;

    pub fn CFRunLoopGetCurrent() -> *mut c_void;

    pub fn CFRunLoopWakeUp(rl: *mut c_void);

    pub fn CFRunLoopObserverCreate(
        allocator: *const c_void,
        activities: CFOptionFlags,
        repeats: u8,
        order: isize,
        callout: CFRunLoopObserverCallBack,
        context: *mut c_void,
    ) -> *mut c_void;

    pub fn CFRunLoopAddObserver(rl: *mut c_void, observer: *mut c_void, mode: *const c_void);

    pub fn CFRunLoopTimerCreate(
        allocator: *const c_void,
        fire_date: f64,
        interval: f64,
        flags: CFOptionFlags,
        order: isize,
        callout: CFRunLoopTimerCallBack,
        context: *mut c_void,
    ) -> *mut c_void;

    pub fn CFRunLoopTimerSetTolerance(timer: *mut c_void, tolerance: f64);

    pub fn CFRunLoopTimerSetNextFireDate(timer: *mut c_void, fire_date: f64);

    pub fn CFRunLoopAddTimer(rl: *mut c_void, timer: *mut c_void, mode: *const c_void);

    pub fn CFAbsoluteTimeGetCurrent() -> f64;
}

extern "C" {
    /// The main dispatch queue (what `dispatch_get_main_queue()` is the
    /// address of).
    pub static _dispatch_main_q: DispatchQueue;

    pub fn dispatch_async_f(queue: *const DispatchQueue, context: *mut c_void, work: DispatchFunction);
}

/// The main dispatch queue.
pub fn dispatch_get_main_queue() -> *const DispatchQueue {
    // Only the address of the queue is taken; the queue is a global of
    // libdispatch that lives as long as the process.
    std::ptr::addr_of!(_dispatch_main_q)
}
