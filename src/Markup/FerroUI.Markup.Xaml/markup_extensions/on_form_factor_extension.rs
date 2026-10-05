//! Port of `MarkupExtensions/OnFormFactorExtension.cs`.

use super::on_platform_extension::option_property;
use super::On;
use ferroui_base::metadata::{IAddChild, IServiceProvider};
use crate::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_base::platform::FormFactorType;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Provides a value depending on the form factor of the device the
/// application runs on (desktop, mobile, TV).
///
/// The non-generic and the generic class of the managed original are this
/// one type: [`OnFormFactorExtension`] is the instantiation for any value,
/// [`OnFormFactorExtensionOf`] the typed one.
pub struct OnFormFactorExtensionBase<TReturn> {
    default: RefCell<Option<TReturn>>,
    desktop: RefCell<Option<TReturn>>,
    mobile: RefCell<Option<TReturn>>,
    tv: RefCell<Option<TReturn>>,
}

/// `OnFormFactorExtension`: the extension for values of any type.
pub type OnFormFactorExtension = OnFormFactorExtensionBase<BoxedValue>;

/// `OnFormFactorExtension<TReturn>`: the extension for values of one type.
pub type OnFormFactorExtensionOf<TReturn> = OnFormFactorExtensionBase<TReturn>;

impl<TReturn: Clone> OnFormFactorExtensionBase<TReturn> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            default: RefCell::new(None),
            desktop: RefCell::new(None),
            mobile: RefCell::new(None),
            tv: RefCell::new(None),
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
    option_property!(desktop, set_desktop);
    option_property!(mobile, set_mobile);
    option_property!(tv, set_tv);

    /// Required for the compiler, which replaces the call with the selected
    /// branch at compile time.
    pub fn provide_value(self: &Rc<Self>) -> Rc<Self> {
        self.clone()
    }

    /// Whether the branch for `option` is the one to provide: whether the
    /// runtime platform service of `service_provider` reports that form
    /// factor.
    ///
    /// # Panics
    /// Panics if `service_provider` is a root service provider and no
    /// runtime platform was registered (the service lookup of the managed
    /// original throws). Use
    /// [`try_should_provide_option`](Self::try_should_provide_option) to
    /// handle it.
    pub fn should_provide_option(service_provider: &Rc<dyn IServiceProvider>, option: FormFactorType) -> bool {
        crate::throw(Self::try_should_provide_option(service_provider, option))
    }

    /// [`should_provide_option`](Self::should_provide_option) without the
    /// panic.
    pub fn try_should_provide_option(
        service_provider: &Rc<dyn IServiceProvider>,
        option: FormFactorType,
    ) -> Result<bool, crate::XamlLoadException> {
        let platform = XamlIlRuntimeHelpers::get_runtime_platform(service_provider)?;
        Ok(platform.is_some_and(|platform| platform.get_runtime_info().form_factor() == option))
    }
}

impl<TReturn> IAddChild<Rc<On<TReturn>>> for OnFormFactorExtensionBase<TReturn> {
    fn add_child(&self, _child: Rc<On<TReturn>>) {}
}

impl<TReturn> PartialEq for OnFormFactorExtensionBase<TReturn> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

// The options are the values of the form factor enumeration (attribute
// arguments are literals): Desktop = 1, Mobile = 2, TV = 3.
ferro_markup_type!(class OnFormFactorExtension as "OnFormFactorExtension" {
    this: Rc<OnFormFactorExtension>,
    handles: [OnFormFactorExtension, Rc<OnFormFactorExtension>, Option<Rc<OnFormFactorExtension>>],
    interfaces: [Rc<dyn IAddChild<Rc<On>>>],
    constructors: [
        () => OnFormFactorExtension::new,
        (Option<BoxedValue>) => OnFormFactorExtension::with_default,
    ],
    properties: [
        Default: Option<BoxedValue> { get: OnFormFactorExtension::default, set: OnFormFactorExtension::set_default }
            [MarkupExtensionDefaultOption],
        Desktop: Option<BoxedValue> { get: OnFormFactorExtension::desktop, set: OnFormFactorExtension::set_desktop }
            [MarkupExtensionOption(1)],
        Mobile: Option<BoxedValue> { get: OnFormFactorExtension::mobile, set: OnFormFactorExtension::set_mobile }
            [MarkupExtensionOption(2)],
        TV: Option<BoxedValue> { get: OnFormFactorExtension::tv, set: OnFormFactorExtension::set_tv }
            [MarkupExtensionOption(3)],
    ],
    methods: [
        fn ProvideValue() -> Rc<OnFormFactorExtension> => OnFormFactorExtension::provide_value,
        fn AddChild(Rc<On>) => |this: &Rc<OnFormFactorExtension>, child: Rc<On>| IAddChild::add_child(&**this, child),
        static try fn ShouldProvideOption(Rc<dyn IServiceProvider>, FormFactorType) -> bool =>
            |service_provider: Rc<dyn IServiceProvider>, option: FormFactorType| {
                OnFormFactorExtension::try_should_provide_option(&service_provider, option)
            },
    ],
});
