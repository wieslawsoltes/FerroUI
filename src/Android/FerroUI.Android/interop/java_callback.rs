//! A call from the Java class of an optional package into Rust.
//!
//! Such a package has no native methods of its own. Its class holds the number of a
//! [`JavaCallback`] and calls `org.ferroui.android.NativeCallback.invoke`. The number is only a
//! key in a table of the thread: a stale or wrong one reaches nothing, and no pointer crosses
//! into Java. The method is registered with the first callback, so an application that has none
//! pays nothing at start-up.

use super::java::{read_string, register_natives, JavaClass, NativeMethod};
use super::natives::guard;
use jni_sys::{jclass, jint, jlong, jobject, JNIEnv};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Once;

const NATIVE_CALLBACK: &str = "org/ferroui/android/NativeCallback";

/// What the Java class passed; the meaning of each field is agreed with that class.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JavaCallbackArgs {
    pub kind: i32,
    pub number: i64,
    pub value: i32,
    pub first: bool,
    pub second: bool,
    pub first_text: Option<String>,
    pub second_text: Option<String>,
}

type Action = Rc<dyn Fn(JavaCallbackArgs) -> bool>;

ferroui_base::ferro_thread_local! {
    static CALLBACKS: RefCell<BTreeMap<i64, Action>> = const { RefCell::new(BTreeMap::new()) };
    static NEXT_HANDLE: Cell<i64> = const { Cell::new(1) };
}

/// A function Java can call on this thread for as long as this value lives.
pub struct JavaCallback {
    handle: i64,
    _thread: PhantomData<Rc<()>>,
}

impl JavaCallback {
    /// Makes a callback. Called on a thread Java code called into, such as the main thread.
    pub fn new(action: impl Fn(JavaCallbackArgs) -> bool + 'static) -> Self {
        register();
        let handle = NEXT_HANDLE.with(|next| next.replace(next.get() + 1));
        CALLBACKS.with(|callbacks| callbacks.borrow_mut().insert(handle, Rc::new(action)));
        Self { handle, _thread: PhantomData }
    }

    /// The number the Java class passes to `NativeCallback.invoke`.
    pub fn handle(&self) -> i64 {
        self.handle
    }
}

impl Drop for JavaCallback {
    fn drop(&mut self) {
        // Dropped after the borrow ends: what the action owns may drop another callback.
        let action = CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&self.handle));
        drop(action);
    }
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn invoke(
    _env: *mut JNIEnv,
    _class: jclass,
    handle: jlong,
    kind: jint,
    number: jlong,
    value: jint,
    first: u8,
    second: u8,
    first_text: jobject,
    second_text: jobject,
) -> u8 {
    guard(0, || {
        // Cloned out of the table: the action can make or drop callbacks.
        let action = CALLBACKS.with(|callbacks| callbacks.borrow().get(&handle).cloned());
        let Some(action) = action else { return 0 };
        // SAFETY: by the signature each is null or a string, valid for this call.
        let (first_text, second_text) = unsafe { (read_string(first_text), read_string(second_text)) };
        u8::from(action(JavaCallbackArgs {
            kind,
            number,
            value,
            first: first != 0,
            second: second != 0,
            first_text,
            second_text,
        }))
    })
}

fn register() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let method = NativeMethod {
            name: c"invoke",
            signature: c"(JIJIZZLjava/lang/String;Ljava/lang/String;)Z",
            function: invoke as *mut c_void,
        };
        // SAFETY: `invoke` has the parameters of the signature after the environment and the
        // class: `J` an `i64`, `I` an `i32`, `Z` one byte, and a string a reference.
        unsafe { register_natives(&JavaClass::find(NATIVE_CALLBACK), &[method]) };
    });
}
