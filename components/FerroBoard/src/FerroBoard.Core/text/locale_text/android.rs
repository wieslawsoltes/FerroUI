//! Android ICU (API 24+, within FerroUI's API 26 minimum), through the real JVM.
//! Only locale comparison/search crosses JNI; no controls or framework services enter Core.
use super::{CollationError, Ordering, length, pointer};
use jni_sys::{JNI_OK, JNI_VERSION_1_6, JNIEnv, JavaVM, jclass, jmethodID, jobject, jvalue};
use std::{
    ffi::{CStr, c_void},
    marker::PhantomData,
    ptr,
    rc::Rc,
    sync::atomic::{AtomicPtr, Ordering as AtomicOrdering},
};

static VM: AtomicPtr<JavaVM> = AtomicPtr::new(ptr::null_mut());

/// Install the actual Android VM at the composition root, normally from JNI_OnLoad.
/// Reinstalling the same VM is idempotent; replacing it is rejected.
///
/// # Safety
/// `vm` must be the process-lifetime JavaVM pointer supplied by Android. LocaleText
/// operations and destruction must run on their already JVM-attached owner thread.
pub unsafe fn install_java_vm(vm: *mut c_void) -> Result<(), CollationError> {
    let vm = vm.cast::<JavaVM>();
    if vm.is_null() {
        return Err(native("install Android JavaVM", -1));
    }
    match VM.compare_exchange(
        ptr::null_mut(),
        vm,
        AtomicOrdering::AcqRel,
        AtomicOrdering::Acquire,
    ) {
        Ok(_) => Ok(()),
        Err(previous) if previous == vm => Ok(()),
        Err(_) => Err(native("replace Android JavaVM", -1)),
    }
}
fn native(operation: &'static str, code: i32) -> CollationError {
    CollationError::Native { operation, code }
}

macro_rules! jni {
    ($env:expr, $name:ident $(, $argument:expr)* $(,)?) => {
        ((**$env).v1_6.$name)($env $(, $argument)*)
    };
}

#[derive(Clone, Copy)]
struct Env(*mut JNIEnv);
impl Env {
    fn current() -> Result<Self, CollationError> {
        let vm = VM.load(AtomicOrdering::Acquire);
        if vm.is_null() {
            return Err(native(
                "Android JavaVM not installed at composition root",
                -1,
            ));
        }
        let mut env: *mut c_void = ptr::null_mut();
        // SAFETY: install_java_vm's contract pins this process VM; GetEnv writes one pointer.
        let status = unsafe { ((**vm).v1_4.GetEnv)(vm, &mut env, JNI_VERSION_1_6) };
        if status != JNI_OK || env.is_null() {
            return Err(native(
                "JNI GetEnv (owner thread must already be attached)",
                status,
            ));
        }
        Ok(Self(env.cast()))
    }
    fn check(self, operation: &'static str) -> Result<(), CollationError> {
        // SAFETY: this is the attached calling thread's environment.
        if unsafe { jni!(self.0, ExceptionCheck) } {
            // SAFETY: clear the pending Java exception before returning its explicit Rust error.
            // Do not log user text or leave the VM in a pending-exception state.
            unsafe { jni!(self.0, ExceptionClear) };
            Err(native(operation, -1))
        } else {
            Ok(())
        }
    }
    fn local(self, raw: jobject, operation: &'static str) -> Result<Local, CollationError> {
        let local = (!raw.is_null()).then_some(Local { env: self, raw });
        self.check(operation)?;
        local.ok_or_else(|| native(operation, 0))
    }
    fn class(self, name: &CStr) -> Result<Local, CollationError> {
        // SAFETY: boot class name is a terminated constant and this environment is current.
        self.local(
            unsafe { jni!(self.0, FindClass, name.as_ptr()) },
            "JNI FindClass (Android ICU)",
        )
    }
    fn method(
        self,
        class: jclass,
        name: &CStr,
        signature: &CStr,
        static_: bool,
    ) -> Result<jmethodID, CollationError> {
        // SAFETY: live class and constant, correctly typed method names/signatures.
        let method = unsafe {
            if static_ {
                jni!(
                    self.0,
                    GetStaticMethodID,
                    class,
                    name.as_ptr(),
                    signature.as_ptr()
                )
            } else {
                jni!(
                    self.0,
                    GetMethodID,
                    class,
                    name.as_ptr(),
                    signature.as_ptr()
                )
            }
        };
        self.check("JNI method lookup (Android ICU)")?;
        if method.is_null() {
            Err(native("JNI method lookup returned null", 0))
        } else {
            Ok(method)
        }
    }
    fn string(self, text: &[u16]) -> Result<Local, CollationError> {
        let count = length(text)?;
        // SAFETY: exact checked UTF16 length. JNI copies the borrowed units, including NUL
        // and permitted unpaired surrogates. There is no UTF8/modified-UTF8 round trip.
        self.local(
            unsafe { jni!(self.0, NewString, pointer(text), count) },
            "JNI NewString",
        )
    }
    fn object(
        self,
        class: jclass,
        constructor: jmethodID,
        arguments: &[jvalue],
    ) -> Result<Local, CollationError> {
        // SAFETY: private callers use the exact constructor signatures declared below.
        self.local(
            unsafe { jni!(self.0, NewObjectA, class, constructor, arguments.as_ptr()) },
            "Android ICU constructor",
        )
    }
}

struct Local {
    env: Env,
    raw: jobject,
}
impl Local {
    fn global(&self) -> Result<Global, CollationError> {
        // SAFETY: live local on its owner thread. NewGlobalRef creates independently owned ref.
        let raw = unsafe { jni!(self.env.0, NewGlobalRef, self.raw) };
        let global = (!raw.is_null()).then_some(Global { env: self.env, raw });
        self.env.check("JNI NewGlobalRef")?;
        global.ok_or_else(|| native("JNI NewGlobalRef returned null", 0))
    }
}
impl Drop for Local {
    fn drop(&mut self) {
        // SAFETY: the private local never leaves its attached owner-thread operation.
        unsafe { jni!(self.env.0, DeleteLocalRef, self.raw) };
    }
}
struct Global {
    env: Env,
    raw: jobject,
}
impl Drop for Global {
    fn drop(&mut self) {
        // SAFETY: exactly one owning reference; LocaleText is owner-thread-only and its
        // installation contract requires that thread to remain attached until destruction.
        unsafe { jni!(self.env.0, DeleteGlobalRef, self.raw) };
    }
}

pub(super) struct Collator {
    env: Env,
    sensitive: Global,
    insensitive: Global,
    iterator_class: Global,
    search_class: Global,
    iterator_constructor: jmethodID,
    search_constructor: jmethodID,
    compare_method: jmethodID,
    first_method: jmethodID,
    _owner_thread: PhantomData<Rc<()>>,
}
impl Collator {
    pub(super) fn new(locale: &str) -> Result<Self, CollationError> {
        let env = Env::current()?;
        let locale_class = env.class(c"android/icu/util/ULocale")?;
        let locale_method =
            env.method(locale_class.raw, c"<init>", c"(Ljava/lang/String;)V", false)?;
        let identifier = env.string(&locale.encode_utf16().collect::<Vec<_>>())?;
        let locale_args = [jvalue { l: identifier.raw }];
        // ULocale accepts native ICU identifiers and BCP47 tags; it preserves locale
        // extensions instead of silently treating underscore identifiers as root.
        let locale = env.object(locale_class.raw, locale_method, &locale_args)?;
        let class = env.class(c"android/icu/text/Collator")?;
        let rule_class = env.class(c"android/icu/text/RuleBasedCollator")?;
        let instance = env.method(
            class.raw,
            c"getInstance",
            c"(Landroid/icu/util/ULocale;)Landroid/icu/text/Collator;",
            true,
        )?;
        let strength = env.method(class.raw, c"setStrength", c"(I)V", false)?;
        let compare_method = env.method(
            class.raw,
            c"compare",
            c"(Ljava/lang/String;Ljava/lang/String;)I",
            false,
        )?;
        let make = |level: i32| -> Result<Global, CollationError> {
            let args = [jvalue { l: locale.raw }];
            // SAFETY: getInstance(Locale) returns Collator; returned local is scoped below.
            let object = env.local(
                unsafe {
                    jni!(
                        env.0,
                        CallStaticObjectMethodA,
                        class.raw,
                        instance,
                        args.as_ptr()
                    )
                },
                "Collator.getInstance",
            )?;
            // SAFETY: both refs are live; StringSearch requires this exact subtype.
            let is_rule = unsafe { jni!(env.0, IsInstanceOf, object.raw, rule_class.raw) };
            env.check("Collator RuleBasedCollator type check")?;
            if !is_rule {
                return Err(native("Android ICU requires RuleBasedCollator", 0));
            }
            let args = [jvalue { i: level }];
            // SAFETY: Collator.setStrength(int) returns void. SECONDARY=1, TERTIARY=2.
            unsafe { jni!(env.0, CallVoidMethodA, object.raw, strength, args.as_ptr()) };
            env.check("Collator.setStrength")?;
            object.global()
        };
        let sensitive = make(2)?;
        let insensitive = make(1)?;
        let iterator = env.class(c"java/text/StringCharacterIterator")?;
        let iterator_constructor =
            env.method(iterator.raw, c"<init>", c"(Ljava/lang/String;)V", false)?;
        let search = env.class(c"android/icu/text/StringSearch")?;
        let search_constructor = env.method(search.raw, c"<init>", c"(Ljava/lang/String;Ljava/text/CharacterIterator;Landroid/icu/text/RuleBasedCollator;)V", false)?;
        let first_method = env.method(search.raw, c"first", c"()I", false)?;
        Ok(Self {
            env,
            sensitive,
            insensitive,
            iterator_class: iterator.global()?,
            search_class: search.global()?,
            iterator_constructor,
            search_constructor,
            compare_method,
            first_method,
            _owner_thread: PhantomData,
        })
    }
    fn handle(&self, ignore_case: bool) -> jobject {
        if ignore_case {
            self.insensitive.raw
        } else {
            self.sensitive.raw
        }
    }
    pub(super) fn compare(
        &self,
        left: &[u16],
        right: &[u16],
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        let left = self.env.string(left)?;
        let right = self.env.string(right)?;
        let arguments = [jvalue { l: left.raw }, jvalue { l: right.raw }];
        // SAFETY: Collator.compare(String,String) returns int; collator/strings are live.
        let result = unsafe {
            jni!(
                self.env.0,
                CallIntMethodA,
                self.handle(ignore_case),
                self.compare_method,
                arguments.as_ptr()
            )
        };
        self.env.check("Collator.compare")?;
        Ok(result.cmp(&0))
    }
    pub(super) fn contains(
        &self,
        text: &[u16],
        pattern: &[u16],
        ignore_case: bool,
    ) -> Result<bool, CollationError> {
        if pattern.is_empty() || self.compare(pattern, &[], ignore_case)? == Ordering::Equal {
            return Ok(true);
        }
        if text.is_empty() {
            return Ok(false);
        }
        let text = self.env.string(text)?;
        let pattern = self.env.string(pattern)?;
        let iterator = self.env.object(
            self.iterator_class.raw,
            self.iterator_constructor,
            &[jvalue { l: text.raw }],
        )?;
        let arguments = [
            jvalue { l: pattern.raw },
            jvalue { l: iterator.raw },
            jvalue {
                l: self.handle(ignore_case),
            },
        ];
        let search = self
            .env
            .object(self.search_class.raw, self.search_constructor, &arguments)?;
        // SAFETY: StringSearch.first() returns int. All Java text/search/iterator refs are
        // held until after the call; the platform performs actual ICU collation search.
        let found = unsafe {
            jni!(
                self.env.0,
                CallIntMethodA,
                search.raw,
                self.first_method,
                ptr::null()
            )
        };
        self.env.check("StringSearch.first")?;
        Ok(found >= 0)
    }
}
