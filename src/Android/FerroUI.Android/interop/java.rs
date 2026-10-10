//! The Java Native Interface, as safe functions.
//!
//! The reference is managed code whose runtime binds the classes of the
//! framework; the port calls them through the interface of the virtual
//! machine, and this is the one module that does (the counterpart of
//! `xlib.rs` of the X11 backend and of the interop file of the Windows
//! backend). Outside of it there is no `unsafe` for Java:
//!
//! - A Java object is a [`JavaObject`], which owns a global reference
//!   (valid on every thread, released when dropped), or a [`JavaLocal`],
//!   which owns a local reference of the current thread.
//! - A method is called by name and signature. The arguments are checked
//!   against the signature before the call is made, and so is the kind of
//!   the result, because a call that disagrees with its signature is
//!   undefined behaviour in the interface.
//! - An exception a call leaves pending is described to the log of the
//!   system, cleared, and raised as a panic that names the call, which is
//!   what an exception of a Java call is in the reference.
//!
//! The environment of the calling thread is asked of the virtual machine
//! for every call; a thread the virtual machine does not know is attached
//! the first time and detached when it ends.

use super::signature::{parse_signature, JavaType};
use jni_sys::{
    jarray, jclass, jfloat, jint, jmethodID, jobject, jsize, jvalue, JNIEnv, JNINativeMethod, JavaVM, JNI_OK,
    JNI_VERSION_1_6,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

static VM: AtomicPtr<JavaVM> = AtomicPtr::new(ptr::null_mut());
static CLASSES: OnceLock<Mutex<HashMap<String, JavaObject>>> = OnceLock::new();

/// Calls a function of the table of the environment.
macro_rules! jni {
    ($env:expr, $name:ident $(, $arg:expr)* $(,)?) => {
        ((**$env).v1_6.$name)($env $(, $arg)*)
    };
}

/// Keeps the virtual machine the library was loaded into.
///
/// # Safety
/// `vm` is the pointer the virtual machine passed to `JNI_OnLoad`.
pub(crate) unsafe fn initialize(vm: *mut c_void) {
    VM.store(vm.cast(), Ordering::Release);
}

/// Whether the library was loaded by a virtual machine.
pub(crate) fn is_initialized() -> bool {
    !VM.load(Ordering::Acquire).is_null()
}

/// Detaches a thread this module attached, when the thread ends: a thread
/// that ends attached stops the virtual machine.
struct Attachment {
    attached_here: Cell<bool>,
}

impl Drop for Attachment {
    fn drop(&mut self) {
        if self.attached_here.get() {
            let vm = VM.load(Ordering::Acquire);
            if !vm.is_null() {
                // SAFETY: the pointer is the virtual machine of the process, which lives
                // as long as the process; this thread was attached by `env` and makes no
                // further calls.
                unsafe {
                    ((**vm).v1_4.DetachCurrentThread)(vm);
                }
            }
        }
    }
}

thread_local! {
    static ATTACHMENT: Attachment = const { Attachment { attached_here: Cell::new(false) } };
}

/// The environment of the calling thread.
///
/// # Panics
/// Panics when the library was not loaded by a virtual machine, and when
/// the thread cannot be attached.
fn env() -> *mut JNIEnv {
    let vm = VM.load(Ordering::Acquire);
    if vm.is_null() {
        panic!("The Android backend was not loaded by a Java virtual machine (JNI_OnLoad did not run).");
    }

    let mut env: *mut c_void = ptr::null_mut();
    // SAFETY: the pointer is the virtual machine of the process; the call writes one
    // pointer.
    let status = unsafe { ((**vm).v1_4.GetEnv)(vm, &mut env, JNI_VERSION_1_6) };
    if status == JNI_OK && !env.is_null() {
        return env.cast();
    }

    // SAFETY: as above; a daemon attachment does not keep the virtual machine from
    // ending, and the thread is detached when it ends (`Attachment`).
    let status = unsafe { ((**vm).v1_4.AttachCurrentThreadAsDaemon)(vm, &mut env, ptr::null_mut()) };
    if status != JNI_OK || env.is_null() {
        panic!("The thread could not be attached to the Java virtual machine (status {status}).");
    }
    // A thread that is already running its destructors is not detached again: it stays
    // attached, which is what it was before this module looked.
    let _ = ATTACHMENT.try_with(|attachment| attachment.attached_here.set(true));
    env.cast()
}

/// Describes, clears and raises an exception a call left pending.
fn check_exception(env: *mut JNIEnv, context: &dyn Fn() -> String) {
    // SAFETY: `env` is the environment of this thread; the three calls take nothing else.
    let pending = unsafe { jni!(env, ExceptionCheck) };
    if pending {
        // SAFETY: as above.
        unsafe {
            jni!(env, ExceptionDescribe);
            jni!(env, ExceptionClear);
        }
        panic!("A Java exception was thrown by {} (it is described in the log of the system).", context());
    }
}

/// Something that refers to a Java object.
pub trait JavaRef {
    /// The reference. Never null, and valid for as long as `self` is.
    #[doc(hidden)]
    fn raw(&self) -> jobject;
}

/// A Java object, held through a global reference.
pub struct JavaObject {
    raw: jobject,
}

// SAFETY: a global reference is valid on every thread of the virtual machine until it is
// deleted, which only dropping the value does; the value has no other state.
unsafe impl Send for JavaObject {}
// SAFETY: as above; the functions of the interface may be called with one reference from
// several threads.
unsafe impl Sync for JavaObject {}

impl JavaObject {
    /// A global reference to the object `raw` refers to; `None` for null.
    ///
    /// # Safety
    /// `raw` is null or a reference that is valid on the calling thread (an
    /// argument of the native method that is running, a local or a global
    /// reference).
    pub(crate) unsafe fn from_raw(raw: jobject) -> Option<JavaObject> {
        if raw.is_null() {
            return None;
        }
        let env = env();
        // SAFETY: `raw` is a valid reference (the contract of this function).
        let global = unsafe { jni!(env, NewGlobalRef, raw) };
        if global.is_null() {
            panic!("The Java virtual machine is out of memory (NewGlobalRef).");
        }
        Some(JavaObject { raw: global })
    }

    /// The reference as a number, for a platform handle. The number is only
    /// meaningful while this value lives.
    pub fn handle(&self) -> isize {
        self.raw as isize
    }

    /// Whether both values refer to the same object.
    pub fn is_same_object(&self, other: &dyn JavaRef) -> bool {
        let env = env();
        // SAFETY: both references are valid (the invariants of the two types).
        unsafe { jni!(env, IsSameObject, self.raw, other.raw()) }
    }
}

impl JavaRef for JavaObject {
    fn raw(&self) -> jobject {
        self.raw
    }
}

impl Clone for JavaObject {
    fn clone(&self) -> Self {
        // SAFETY: the reference of `self` is a valid global reference.
        unsafe { JavaObject::from_raw(self.raw) }.expect("a Java object is never null")
    }
}

impl Drop for JavaObject {
    fn drop(&mut self) {
        if !is_initialized() {
            return;
        }
        let env = env();
        // SAFETY: the reference is a global reference this value owns and nothing else
        // deletes.
        unsafe { jni!(env, DeleteGlobalRef, self.raw) };
    }
}

/// A Java object, held through a local reference of the thread that
/// created it; the value cannot leave that thread.
pub struct JavaLocal {
    raw: jobject,
    _thread: PhantomData<*mut ()>,
}

impl JavaLocal {
    /// # Safety
    /// `raw` is null or a local reference of the calling thread that nothing
    /// else deletes.
    unsafe fn from_raw(raw: jobject) -> Option<JavaLocal> {
        (!raw.is_null()).then_some(JavaLocal { raw, _thread: PhantomData })
    }

    /// A global reference to the same object.
    pub fn to_global(&self) -> JavaObject {
        // SAFETY: the reference is a valid local reference of this thread.
        unsafe { JavaObject::from_raw(self.raw) }.expect("a Java object is never null")
    }
}

impl JavaRef for JavaLocal {
    fn raw(&self) -> jobject {
        self.raw
    }
}

impl Drop for JavaLocal {
    fn drop(&mut self) {
        let env = env();
        // SAFETY: the reference is a local reference of this thread that this value owns.
        unsafe { jni!(env, DeleteLocalRef, self.raw) };
    }
}

impl JavaLocal {
    /// Gives the reference up, to be returned from the native method that
    /// is running: the virtual machine releases it when the method returns.
    pub(crate) fn into_raw(self) -> jobject {
        let raw = self.raw;
        std::mem::forget(self);
        raw
    }
}

/// A new `java.lang.String`.
pub fn new_string(text: &str) -> JavaLocal {
    new_string_local(env(), text)
}

/// A new `String[]` with the given elements.
pub fn new_string_array(elements: &[String]) -> JavaLocal {
    let env = env();
    let class = JavaClass::find("java/lang/String");
    // SAFETY: the class is a valid class reference and the length is not negative.
    let array = unsafe {
        JavaLocal::from_raw(jni!(env, NewObjectArray, elements.len() as jsize, class.class.raw, ptr::null_mut()))
    };
    check_exception(env, &|| "the creation of an array of strings".to_string());
    let array = array.unwrap_or_else(|| panic!("The Java virtual machine is out of memory (NewObjectArray)."));
    for (index, element) in elements.iter().enumerate() {
        let string = new_string_local(env, element);
        // SAFETY: the array has an element at the index, and the value is a string.
        unsafe { jni!(env, SetObjectArrayElement, array.raw, index as jsize, string.raw) };
    }
    array
}

/// A Java class.
#[derive(Clone)]
pub struct JavaClass {
    class: JavaObject,
    name: String,
}

impl JavaClass {
    /// The class of the given name (`android/view/View`).
    ///
    /// A class of the application is found on the thread that loaded the
    /// library and on threads Java code called into; the classes of the Java
    /// layer of this backend are resolved when the library is loaded
    /// ([`JavaClass::preload`]) and found on every thread after that.
    ///
    /// # Panics
    /// Panics when there is no such class.
    pub fn find(name: &str) -> JavaClass {
        let classes = CLASSES.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(class) = classes.lock().unwrap_or_else(PoisonError::into_inner).get(name) {
            return JavaClass { class: class.clone(), name: name.to_string() };
        }

        let env = env();
        let name_c = CString::new(name).expect("a class name has no NUL");
        // SAFETY: `env` is the environment of this thread and the name is a terminated
        // string.
        let local = unsafe { JavaLocal::from_raw(jni!(env, FindClass, name_c.as_ptr())) };
        check_exception(env, &|| format!("the lookup of the class {name}"));
        let Some(local) = local else {
            panic!("The Java class {name} was not found.");
        };
        let class = local.to_global();
        classes.lock().unwrap_or_else(PoisonError::into_inner).insert(name.to_string(), class.clone());
        JavaClass { class, name: name.to_string() }
    }

    /// Resolves classes on the thread that can find them.
    pub(crate) fn preload(names: &[&str]) {
        for name in names {
            JavaClass::find(name);
        }
    }

    /// The name the class was found by.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// An argument of a call.
#[derive(Clone, Copy)]
pub enum JavaValue<'a> {
    Boolean(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// An object; `None` is null.
    Object(Option<&'a dyn JavaRef>),
    /// A string, created for the call.
    String(&'a str),
}

impl JavaValue<'_> {
    fn java_type(&self) -> JavaType {
        match self {
            JavaValue::Boolean(_) => JavaType::Boolean,
            JavaValue::Int(_) => JavaType::Int,
            JavaValue::Long(_) => JavaType::Long,
            JavaValue::Float(_) => JavaType::Float,
            JavaValue::Double(_) => JavaType::Double,
            JavaValue::Object(_) | JavaValue::String(_) => JavaType::Object,
        }
    }
}

/// What is called.
enum Target<'a> {
    Instance(&'a dyn JavaRef),
    Static(&'a JavaClass),
    Constructor(&'a JavaClass),
}

/// What a call returned.
enum Returned {
    Void,
    Boolean(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Object(Option<JavaLocal>),
}

fn new_string_local(env: *mut JNIEnv, text: &str) -> JavaLocal {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    // SAFETY: the pointer and the length describe the vector, which lives across the call.
    let local = unsafe { JavaLocal::from_raw(jni!(env, NewString, utf16.as_ptr(), utf16.len() as jsize)) };
    check_exception(env, &|| "the creation of a string".to_string());
    local.unwrap_or_else(|| panic!("The Java virtual machine is out of memory (NewString)."))
}

fn invoke(target: Target<'_>, name: &str, signature: &str, args: &[JavaValue<'_>], returns: JavaType) -> Returned {
    let describe = || match &target {
        Target::Instance(_) => format!("the method {name}{signature}"),
        Target::Static(class) => format!("the static method {}.{name}{signature}", class.name),
        Target::Constructor(class) => format!("the constructor {}{signature}", class.name),
    };

    // The check that makes the call sound: the arguments are of the types the signature
    // states, and the function of the interface that is used below is the one for the
    // return type of the signature.
    let Some((parameters, return_type)) = parse_signature(signature) else {
        panic!("'{signature}' is not a method signature ({}).", describe());
    };
    if return_type != returns {
        panic!("{} does not return {returns:?}.", describe());
    }
    if parameters.len() != args.len()
        || parameters.iter().zip(args).any(|(parameter, arg)| *parameter != arg.java_type())
    {
        panic!("The arguments of a call do not match the signature of {}.", describe());
    }

    let env = env();
    let mut strings = Vec::new();
    let values: Vec<jvalue> = args
        .iter()
        .map(|arg| match arg {
            JavaValue::Boolean(value) => jvalue { z: *value },
            JavaValue::Int(value) => jvalue { i: *value },
            JavaValue::Long(value) => jvalue { j: *value },
            JavaValue::Float(value) => jvalue { f: *value },
            JavaValue::Double(value) => jvalue { d: *value },
            JavaValue::Object(value) => jvalue { l: value.map_or(ptr::null_mut(), JavaRef::raw) },
            JavaValue::String(value) => {
                let local = new_string_local(env, value);
                let value = jvalue { l: local.raw };
                strings.push(local);
                value
            }
        })
        .collect();

    let name_c = CString::new(name).expect("a method name has no NUL");
    let signature_c = CString::new(signature).expect("a signature has no NUL");

    // The class the method is looked up in. For an instance it is the class of the object,
    // which is found on every thread.
    let (class_local, class, object): (Option<JavaLocal>, jclass, jobject) = match &target {
        Target::Instance(object) => {
            // SAFETY: the reference of a `JavaRef` is valid and not null.
            let local =
                unsafe { JavaLocal::from_raw(jni!(env, GetObjectClass, object.raw())) }.expect("an object has a class");
            let class = local.raw;
            (Some(local), class, object.raw())
        }
        Target::Static(class) | Target::Constructor(class) => (None, class.class.raw, ptr::null_mut()),
    };

    // SAFETY: the class is a valid class reference and both strings are terminated.
    let method: jmethodID = unsafe {
        match &target {
            Target::Static(_) => jni!(env, GetStaticMethodID, class, name_c.as_ptr(), signature_c.as_ptr()),
            Target::Instance(_) => jni!(env, GetMethodID, class, name_c.as_ptr(), signature_c.as_ptr()),
            Target::Constructor(_) => jni!(env, GetMethodID, class, c"<init>".as_ptr(), signature_c.as_ptr()),
        }
    };
    if method.is_null() {
        // SAFETY: `env` is the environment of this thread.
        unsafe { jni!(env, ExceptionClear) };
        panic!("The Java layer has no {}.", describe());
    }

    let args_ptr = values.as_ptr();
    // SAFETY: the method was found in the class of the target with this signature; the
    // argument array has one value per parameter, each of the type of its parameter
    // (checked above); the function matches the return type of the signature (checked
    // above); the references among the arguments are valid for the duration of the call.
    let returned = unsafe {
        match (&target, returns) {
            (Target::Constructor(_), _) => {
                Returned::Object(JavaLocal::from_raw(jni!(env, NewObjectA, class, method, args_ptr)))
            }
            (Target::Instance(_), JavaType::Void) => {
                jni!(env, CallVoidMethodA, object, method, args_ptr);
                Returned::Void
            }
            (Target::Instance(_), JavaType::Boolean) => {
                Returned::Boolean(jni!(env, CallBooleanMethodA, object, method, args_ptr))
            }
            (Target::Instance(_), JavaType::Int) => Returned::Int(jni!(env, CallIntMethodA, object, method, args_ptr)),
            (Target::Instance(_), JavaType::Long) => {
                Returned::Long(jni!(env, CallLongMethodA, object, method, args_ptr))
            }
            (Target::Instance(_), JavaType::Float) => {
                Returned::Float(jni!(env, CallFloatMethodA, object, method, args_ptr))
            }
            (Target::Instance(_), JavaType::Object) => {
                Returned::Object(JavaLocal::from_raw(jni!(env, CallObjectMethodA, object, method, args_ptr)))
            }
            (Target::Static(_), JavaType::Void) => {
                jni!(env, CallStaticVoidMethodA, class, method, args_ptr);
                Returned::Void
            }
            (Target::Static(_), JavaType::Boolean) => {
                Returned::Boolean(jni!(env, CallStaticBooleanMethodA, class, method, args_ptr))
            }
            (Target::Static(_), JavaType::Int) => {
                Returned::Int(jni!(env, CallStaticIntMethodA, class, method, args_ptr))
            }
            (Target::Static(_), JavaType::Long) => {
                Returned::Long(jni!(env, CallStaticLongMethodA, class, method, args_ptr))
            }
            (Target::Static(_), JavaType::Float) => {
                Returned::Float(jni!(env, CallStaticFloatMethodA, class, method, args_ptr))
            }
            (Target::Static(_), JavaType::Object) => {
                Returned::Object(JavaLocal::from_raw(jni!(env, CallStaticObjectMethodA, class, method, args_ptr)))
            }
            (_, other) => panic!("Calls that return {other:?} are not used by the backend ({}).", describe()),
        }
    };
    drop(class_local);
    drop(strings);
    check_exception(env, &describe);
    returned
}

macro_rules! call_functions {
    ($( $(#[$doc:meta])* $instance:ident, $static_:ident -> $ty:ty, $java:ident, $returned:ident; )*) => {
        $(
            $(#[$doc])*
            pub fn $instance(object: &dyn JavaRef, name: &str, signature: &str, args: &[JavaValue<'_>]) -> $ty {
                match invoke(Target::Instance(object), name, signature, args, JavaType::$java) {
                    Returned::$returned(value) => value,
                    _ => unreachable!("the call returns what its signature states"),
                }
            }

            $(#[$doc])*
            pub fn $static_(class: &JavaClass, name: &str, signature: &str, args: &[JavaValue<'_>]) -> $ty {
                match invoke(Target::Static(class), name, signature, args, JavaType::$java) {
                    Returned::$returned(value) => value,
                    _ => unreachable!("the call returns what its signature states"),
                }
            }
        )*
    };
}

call_functions! {
    /// Calls a method that returns a boolean.
    call_boolean, call_static_boolean -> bool, Boolean, Boolean;
    /// Calls a method that returns an `int`.
    call_int, call_static_int -> i32, Int, Int;
    /// Calls a method that returns a `long`.
    call_long, call_static_long -> i64, Long, Long;
    /// Calls a method that returns a `float`.
    call_float, call_static_float -> f32, Float, Float;
    /// Calls a method that returns an object; `None` is null.
    call_object, call_static_object -> Option<JavaLocal>, Object, Object;
}

/// Calls a method that returns nothing.
pub fn call_void(object: &dyn JavaRef, name: &str, signature: &str, args: &[JavaValue<'_>]) {
    invoke(Target::Instance(object), name, signature, args, JavaType::Void);
}

/// Calls a static method that returns nothing.
pub fn call_static_void(class: &JavaClass, name: &str, signature: &str, args: &[JavaValue<'_>]) {
    invoke(Target::Static(class), name, signature, args, JavaType::Void);
}

/// Creates an object with the constructor of the given signature (which
/// returns `V`, as constructors are written in the interface).
pub fn new_object(class: &JavaClass, signature: &str, args: &[JavaValue<'_>]) -> JavaLocal {
    match invoke(Target::Constructor(class), "<init>", signature, args, JavaType::Void) {
        Returned::Object(Some(object)) => object,
        _ => panic!("The constructor {}{signature} returned no object.", class.name),
    }
}

/// Whether `object` is an instance of the class of the given name.
fn instance_of(env: *mut JNIEnv, object: jobject, class_name: &str) -> bool {
    let class = JavaClass::find(class_name);
    // SAFETY: both references are valid.
    unsafe { jni!(env, IsInstanceOf, object, class.class.raw) }
}

/// Whether the object is an instance of the class of the given name
/// (`android/app/Activity`).
pub fn is_instance_of(object: &dyn JavaRef, class_name: &str) -> bool {
    instance_of(env(), object.raw(), class_name)
}

/// The text of a `java.lang.String`.
///
/// # Panics
/// Panics when the object is not a string.
pub fn string_of(object: &dyn JavaRef) -> String {
    let env = env();
    if !instance_of(env, object.raw(), "java/lang/String") {
        panic!("The Java object is not a string.");
    }
    // SAFETY: the object is a string (checked above); the region read is its whole
    // length and the buffer has that many units.
    unsafe {
        let length = jni!(env, GetStringLength, object.raw());
        let mut buffer = vec![0u16; length.max(0) as usize];
        jni!(env, GetStringRegion, object.raw(), 0, length, buffer.as_mut_ptr());
        String::from_utf16_lossy(&buffer)
    }
}

/// The values of an `int[]`.
///
/// # Panics
/// Panics when the object is not an array of `int`.
pub fn int_array_of(object: &dyn JavaRef) -> Vec<i32> {
    let env = env();
    if !instance_of(env, object.raw(), "[I") {
        panic!("The Java object is not an array of int.");
    }
    // SAFETY: the object is an array of `int` (checked above).
    unsafe { read_int_array(object.raw()) }
}

/// The values of a `float[]`.
///
/// # Panics
/// Panics when the object is not an array of `float`.
pub fn float_array_of(object: &dyn JavaRef) -> Vec<f32> {
    let env = env();
    if !instance_of(env, object.raw(), "[F") {
        panic!("The Java object is not an array of float.");
    }
    // SAFETY: the object is an array of `float` (checked above).
    unsafe { read_float_array(object.raw()) }
}

/// The elements of an array of strings; a null element is an empty string.
///
/// # Panics
/// Panics when the object is not an array of strings.
pub fn string_array_of(object: &dyn JavaRef) -> Vec<String> {
    let env = env();
    if !instance_of(env, object.raw(), "[Ljava/lang/String;") {
        panic!("The Java object is not an array of strings.");
    }
    // SAFETY: the object is an array of objects (checked above), every index is below
    // its length, and an element is null or a local reference this thread owns.
    unsafe {
        let length = jni!(env, GetArrayLength, object.raw());
        (0..length)
            .map(|index| {
                JavaLocal::from_raw(jni!(env, GetObjectArrayElement, object.raw(), index))
                    .map(|element| string_of(&element))
                    .unwrap_or_default()
            })
            .collect()
    }
}

/// Copies a `String` argument of a native method; `None` for null.
///
/// # Safety
/// `string` is null or a valid reference to a `java.lang.String`.
pub(crate) unsafe fn read_string(string: jobject) -> Option<String> {
    if string.is_null() {
        return None;
    }
    let env = env();
    // SAFETY: the reference is a string (the contract); the region read is its whole
    // length and the buffer has that many units.
    unsafe {
        let length = jni!(env, GetStringLength, string);
        let mut buffer = vec![0u16; length.max(0) as usize];
        jni!(env, GetStringRegion, string, 0, length, buffer.as_mut_ptr());
        Some(String::from_utf16_lossy(&buffer))
    }
}

/// Copies an `int[]` argument of a native method.
///
/// # Safety
/// `array` is null or a valid reference to an array of `int`.
pub(crate) unsafe fn read_int_array(array: jarray) -> Vec<i32> {
    if array.is_null() {
        return Vec::new();
    }
    let env = env();
    // SAFETY: the reference is an array of `int` (the contract); the region is its whole
    // length and the buffer has that many values.
    unsafe {
        let length = jni!(env, GetArrayLength, array);
        let mut buffer: Vec<jint> = vec![0; length.max(0) as usize];
        jni!(env, GetIntArrayRegion, array, 0, length, buffer.as_mut_ptr());
        buffer
    }
}

/// Copies a `float[]` argument of a native method.
///
/// # Safety
/// `array` is null or a valid reference to an array of `float`.
pub(crate) unsafe fn read_float_array(array: jarray) -> Vec<f32> {
    if array.is_null() {
        return Vec::new();
    }
    let env = env();
    // SAFETY: as `read_int_array`, for `float`.
    unsafe {
        let length = jni!(env, GetArrayLength, array);
        let mut buffer: Vec<jfloat> = vec![0.0; length.max(0) as usize];
        jni!(env, GetFloatArrayRegion, array, 0, length, buffer.as_mut_ptr());
        buffer
    }
}

/// The address of the environment of the calling thread, for the one
/// function of the NDK that takes it (`ANativeWindow_fromSurface`).
pub(crate) fn raw_env() -> *mut c_void {
    env().cast()
}

/// A native method of a class of the Java layer.
pub(crate) struct NativeMethod {
    pub name: &'static CStr,
    pub signature: &'static CStr,
    pub function: *mut c_void,
}

/// Registers the native methods of a class.
///
/// # Safety
/// Each function is an `extern "system"` function whose parameters, after
/// the environment and the class or the object, are those of its signature.
///
/// # Panics
/// Panics when the class does not declare one of the methods.
pub(crate) unsafe fn register_natives(class: &JavaClass, methods: &[NativeMethod]) {
    let env = env();
    let table: Vec<JNINativeMethod> = methods
        .iter()
        .map(|method| JNINativeMethod {
            name: method.name.as_ptr().cast_mut(),
            signature: method.signature.as_ptr().cast_mut(),
            fnPtr: method.function,
        })
        .collect();
    // SAFETY: the table has `len` entries of terminated strings and function addresses
    // that match their signatures (the contract of this function); the virtual machine
    // copies what it keeps.
    let status = unsafe { jni!(env, RegisterNatives, class.class.raw, table.as_ptr(), table.len() as jint) };
    check_exception(env, &|| format!("the registration of the native methods of {}", class.name));
    if status != JNI_OK {
        panic!("The native methods of {} could not be registered (status {status}).", class.name);
    }
}

/// Throws a `java.lang.RuntimeException` on the calling thread, which the
/// Java caller of the native method that is running sees when the method
/// returns.
pub(crate) fn throw_runtime_exception(message: &str) {
    let env = env();
    // The message is modified UTF-8 for the interface: no NUL, and nothing outside the
    // basic plane, which that encoding writes differently.
    let message: String = message.chars().map(|c| if c == '\0' || c as u32 > 0xffff { '?' } else { c }).collect();
    let message = CString::new(message).expect("the message has no NUL");
    // SAFETY: the class name and the message are terminated strings; a class of the
    // runtime is found on every thread.
    unsafe {
        if jni!(env, ExceptionCheck) {
            // An exception is already on its way to the caller.
            return;
        }
        let class = jni!(env, FindClass, c"java/lang/RuntimeException".as_ptr());
        if !class.is_null() {
            jni!(env, ThrowNew, class, message.as_ptr());
            jni!(env, DeleteLocalRef, class);
        }
    }
}
