//! Not a port: the Objective-C runtime calls the embedding of the sample needs, in place of
//! the AppKit binding package the managed original is built on. Each function is one message
//! of one signature; nothing here is general.
//!
//! `objc_msgSend` is declared without a signature and called through a pointer of the exact
//! signature of the method it dispatches to, as the runtime requires.

use std::ffi::{c_char, c_void, CStr, CString};

/// An object (`id`).
pub type Id = *mut c_void;
/// A selector (`SEL`).
pub type Sel = *mut c_void;
/// A class (`Class`).
pub type Class = *mut c_void;
/// The implementation of a method (`IMP`).
pub type Imp = unsafe extern "C" fn();

/// `NSRange`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NSRange {
    pub location: usize,
    pub length: usize,
}

/// `CGRect`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CGRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// `struct objc_super`.
#[repr(C)]
struct ObjcSuper {
    receiver: Id,
    super_class: Class,
}

#[link(name = "objc")]
extern "C" {
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_getClass(name: *const c_char) -> Class;
    fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra_bytes: usize) -> Class;
    fn objc_registerClassPair(class: Class);
    fn class_addMethod(class: Class, name: Sel, imp: Imp, types: *const c_char) -> bool;
    fn objc_msgSend();
    fn objc_msgSendSuper();
}

// The classes the sample names (`NSTextView`, `NSWindow`, `NSApplication`) are classes of
// AppKit.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

/// The selector `name`.
pub fn sel(name: &str) -> Sel {
    let name = CString::new(name).expect("a selector name without a NUL");
    // SAFETY: a NUL-terminated selector name.
    unsafe { sel_registerName(name.as_ptr()) }
}

/// The class `name`; null when the runtime has no such class.
pub fn class(name: &str) -> Class {
    let name = CString::new(name).expect("a class name without a NUL");
    // SAFETY: a NUL-terminated class name.
    unsafe { objc_getClass(name.as_ptr()) }
}

/// Declares the class `name` as a subclass of `superclass` with the given instance methods
/// (`(selector, implementation, type encoding)`), or returns it when it is declared already.
///
/// # Safety
/// Each implementation must be a function of the signature its type encoding states, taking
/// the receiver and the selector first.
pub unsafe fn declare_class(name: &str, superclass: Class, methods: &[(&str, Imp, &str)]) -> Class {
    let existing = class(name);
    if !existing.is_null() {
        return existing;
    }
    let class_name = CString::new(name).expect("a class name without a NUL");
    let class = objc_allocateClassPair(superclass, class_name.as_ptr(), 0);
    assert!(!class.is_null(), "the class {name} cannot be declared");
    for (selector, imp, types) in methods {
        let types = CString::new(*types).expect("a type encoding without a NUL");
        let added = class_addMethod(class, sel(selector), *imp, types.as_ptr());
        assert!(added, "the method {selector} of {name} cannot be added");
    }
    objc_registerClassPair(class);
    class
}

// SAFETY (all `send*`): `objc_msgSend` is called through a pointer of the exact signature of
// the method it dispatches to. The callers send each message to an object (or a class) that
// responds to it; a message to nil returns zero.

/// A message without arguments that returns an object.
pub fn send_id(receiver: Id, selector: &str) -> Id {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector))
    }
}

/// A message without arguments that returns nothing.
pub fn send_void(receiver: Id, selector: &str) {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel) = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector))
    }
}

/// A message without arguments that returns an unsigned integer (`NSUInteger`).
pub fn send_usize(receiver: Id, selector: &str) -> usize {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel) -> usize = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector))
    }
}

/// A message with one object argument that returns an object.
pub fn send_id_id(receiver: Id, selector: &str, argument: Id) -> Id {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, Id) -> Id = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector), argument)
    }
}

/// A message with one object argument that returns nothing.
pub fn send_void_id(receiver: Id, selector: &str, argument: Id) {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, Id) = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector), argument)
    }
}

/// A message with one boolean argument that returns nothing.
pub fn send_void_bool(receiver: Id, selector: &str, argument: bool) {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, bool) = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector), argument)
    }
}

/// A message with one pointer argument that returns nothing: `endModalSession:`.
pub fn send_void_ptr(receiver: Id, selector: &str, argument: *mut c_void) {
    send_void_id(receiver, selector, argument)
}

/// A message with one pointer argument that returns an integer (`NSInteger`):
/// `runModalSession:`.
pub fn send_isize_ptr(receiver: Id, selector: &str, argument: *mut c_void) -> isize {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, *mut c_void) -> isize =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, sel(selector), argument)
    }
}

/// A message with one pointer argument that returns a pointer: `beginModalSessionForWindow:`.
pub fn send_ptr_id(receiver: Id, selector: &str, argument: Id) -> *mut c_void {
    send_id_id(receiver, selector, argument)
}

/// `[storage replaceCharactersInRange:range withString:string]`.
pub fn replace_characters_in_range(storage: Id, range: NSRange, string: Id) {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, NSRange, Id) = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(storage, sel("replaceCharactersInRange:withString:"), range, string)
    }
}

/// `[window initWithContentRect:rect styleMask:style backing:backing defer:defer]`.
pub fn init_window(window: Id, content_rect: CGRect, style_mask: usize, backing: usize, defer: bool) -> Id {
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, CGRect, usize, usize, bool) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(window, sel("initWithContentRect:styleMask:backing:defer:"), content_rect, style_mask, backing, defer)
    }
}

/// `[super selector:argument]` from a method of an object whose superclass is `super_class`.
///
/// # Safety
/// `receiver` must be an object of a subclass of `super_class`, which responds to the message
/// with one object argument and no result.
pub unsafe fn send_super_void_id(receiver: Id, super_class: Class, selector: Sel, argument: Id) {
    let superclass = ObjcSuper { receiver, super_class };
    let f: unsafe extern "C" fn(*const ObjcSuper, Sel, Id) = std::mem::transmute(objc_msgSendSuper as unsafe extern "C" fn());
    f(&superclass, selector, argument)
}

/// A new `NSString` with the content of `text`, autoreleased.
pub fn ns_string(text: &str) -> Id {
    // An interior NUL ends the text, as it ends a C string.
    let text = CString::new(text.split('\0').next().unwrap_or_default()).unwrap_or_default();
    unsafe {
        let f: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(class("NSString"), sel("stringWithUTF8String:"), text.as_ptr())
    }
}

/// The content of an `NSString`; `None` for nil.
pub fn string(object: Id) -> Option<String> {
    if object.is_null() {
        return None;
    }
    let utf8 = send_id(object, "UTF8String") as *const c_char;
    if utf8.is_null() {
        return None;
    }
    // SAFETY: `UTF8String` returns a NUL-terminated buffer that lives at least as long as
    // the string it was asked of, which the caller holds.
    Some(unsafe { CStr::from_ptr(utf8) }.to_string_lossy().into_owned())
}
