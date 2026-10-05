//! Port of `MarkupExtensions/OnPlatformExtension.cs`.

use super::On;
use ferroui_base::metadata::IAddChild;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Provides a value depending on the operating system the application runs
/// on:
///
/// ```xml
/// <TextBlock Text="{OnPlatform Default='Hello', Windows='Hello Windows'}" />
/// ```
///
/// The markup compiler replaces the extension with a branch per option that
/// asks [`should_provide_option`](Self::should_provide_option); the object
/// itself only carries the declared options.
///
/// The non-generic and the generic class of the managed original are this
/// one type: [`OnPlatformExtension`] is the instantiation for any value,
/// [`OnPlatformExtensionOf`] the typed one.
pub struct OnPlatformExtensionBase<TReturn> {
    default: RefCell<Option<TReturn>>,
    windows: RefCell<Option<TReturn>>,
    mac_os: RefCell<Option<TReturn>>,
    linux: RefCell<Option<TReturn>>,
    android: RefCell<Option<TReturn>>,
    ios: RefCell<Option<TReturn>>,
    browser: RefCell<Option<TReturn>>,
}

/// `OnPlatformExtension`: the extension for values of any type.
pub type OnPlatformExtension = OnPlatformExtensionBase<BoxedValue>;

/// `OnPlatformExtension<TReturn>`: the extension for values of one type.
pub type OnPlatformExtensionOf<TReturn> = OnPlatformExtensionBase<TReturn>;

macro_rules! option_property {
    ($(#[$doc:meta])* $get:ident, $set:ident) => {
        $(#[$doc])*
        pub fn $get(&self) -> Option<TReturn> {
            self.$get.borrow().clone()
        }

        pub fn $set(&self, value: Option<TReturn>) {
            *self.$get.borrow_mut() = value;
        }
    };
}
pub(crate) use option_property;

impl<TReturn: Clone> OnPlatformExtensionBase<TReturn> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            default: RefCell::new(None),
            windows: RefCell::new(None),
            mac_os: RefCell::new(None),
            linux: RefCell::new(None),
            android: RefCell::new(None),
            ios: RefCell::new(None),
            browser: RefCell::new(None),
        })
    }

    /// Creates the extension with its default value.
    pub fn with_default(default_value: Option<TReturn>) -> Rc<Self> {
        let this = Self::new();
        this.set_default(default_value);
        this
    }

    option_property!(
        /// The value used when no option applies.
        default, set_default
    );
    option_property!(windows, set_windows);
    option_property!(mac_os, set_mac_os);
    option_property!(linux, set_linux);
    option_property!(android, set_android);
    option_property!(ios, set_ios);
    option_property!(browser, set_browser);

    /// Required for the compiler, which replaces the call with the selected
    /// branch at compile time.
    pub fn provide_value(self: &Rc<Self>) -> Rc<Self> {
        self.clone()
    }

    /// Whether the branch for `option` is the one to provide on the
    /// operating system the application runs on.
    pub fn should_provide_option(option: &str) -> bool {
        should_provide_option_internal(option)
    }
}

impl<TReturn> IAddChild<Rc<On<TReturn>>> for OnPlatformExtensionBase<TReturn> {
    fn add_child(&self, _child: Rc<On<TReturn>>) {}
}

impl<TReturn> PartialEq for OnPlatformExtensionBase<TReturn> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn should_provide_option_internal(option: &str) -> bool {
    match option {
        "WINDOWS" => cfg!(target_os = "windows"),
        "OSX" => cfg!(target_os = "macos"),
        "LINUX" => cfg!(target_os = "linux"),
        "ANDROID" => cfg!(target_os = "android"),
        "IOS" => cfg!(target_os = "ios"),
        "BROWSER" => cfg!(target_family = "wasm"),
        _ => is_os_platform(option),
    }
}

/// `OperatingSystem.IsOSPlatform(platform)`: a case-insensitive comparison
/// with the name of the platform.
fn is_os_platform(platform: &str) -> bool {
    let current: &[&str] = if cfg!(target_family = "wasm") {
        &["BROWSER"]
    } else {
        match std::env::consts::OS {
            "macos" => &["OSX", "MACOS"],
            "ios" => &["IOS"],
            "windows" => &["WINDOWS"],
            "linux" => &["LINUX"],
            "android" => &["ANDROID"],
            "freebsd" => &["FREEBSD"],
            _ => &[],
        }
    };
    current.iter().any(|name| name.eq_ignore_ascii_case(platform)) || platform.eq_ignore_ascii_case(std::env::consts::OS)
}

ferro_markup_type!(class OnPlatformExtension as "OnPlatformExtension" {
    this: Rc<OnPlatformExtension>,
    handles: [OnPlatformExtension, Rc<OnPlatformExtension>, Option<Rc<OnPlatformExtension>>],
    interfaces: [Rc<dyn IAddChild<Rc<On>>>],
    constructors: [
        () => OnPlatformExtension::new,
        (Option<BoxedValue>) => OnPlatformExtension::with_default,
    ],
    properties: [
        Default: Option<BoxedValue> { get: OnPlatformExtension::default, set: OnPlatformExtension::set_default }
            [MarkupExtensionDefaultOption],
        Windows: Option<BoxedValue> { get: OnPlatformExtension::windows, set: OnPlatformExtension::set_windows }
            [MarkupExtensionOption("WINDOWS")],
        macOS: Option<BoxedValue> { get: OnPlatformExtension::mac_os, set: OnPlatformExtension::set_mac_os }
            [MarkupExtensionOption("OSX")],
        Linux: Option<BoxedValue> { get: OnPlatformExtension::linux, set: OnPlatformExtension::set_linux }
            [MarkupExtensionOption("LINUX")],
        Android: Option<BoxedValue> { get: OnPlatformExtension::android, set: OnPlatformExtension::set_android }
            [MarkupExtensionOption("ANDROID")],
        iOS: Option<BoxedValue> { get: OnPlatformExtension::ios, set: OnPlatformExtension::set_ios }
            [MarkupExtensionOption("IOS")],
        Browser: Option<BoxedValue> { get: OnPlatformExtension::browser, set: OnPlatformExtension::set_browser }
            [MarkupExtensionOption("BROWSER")],
    ],
    methods: [
        fn ProvideValue() -> Rc<OnPlatformExtension> => OnPlatformExtension::provide_value,
        fn AddChild(Rc<On>) => |this: &Rc<OnPlatformExtension>, child: Rc<On>| IAddChild::add_child(&**this, child),
        static fn ShouldProvideOption(String) -> bool => |option: String| {
            OnPlatformExtension::should_provide_option(&option)
        },
    ],
});
