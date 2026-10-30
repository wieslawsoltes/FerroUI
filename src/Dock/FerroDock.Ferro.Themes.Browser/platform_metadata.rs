//! Missing closed metadata for the actual shared typed OnPlatform implementations.
use ferroui_base::{ferro_markup_type, Thickness};
use ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf;
use std::rc::Rc;
type NativePlatform = OnPlatformExtensionOf<Thickness>;
/// Metadata carrier only; values and every operation belong to the native extension.
#[doc(hidden)]
pub struct OnPlatformThicknessMetadata;
ferro_markup_type!(class OnPlatformThicknessMetadata as "OnPlatformExtension`1" {
    namespace: "FerroUI.Markup.Xaml.MarkupExtensions",
    generic: "OnPlatformExtension`1" [Thickness],
    handles: [Rc<NativePlatform>, Option<Rc<NativePlatform>>],
    this: Rc<NativePlatform>,
    constructors: [() => NativePlatform::new, (Option<Thickness>) => NativePlatform::with_default],
    properties: [
        Default: Option<Thickness> {get: NativePlatform::default, set: NativePlatform::set_default} [MarkupExtensionDefaultOption],
        Windows: Option<Thickness> {get: NativePlatform::windows, set: NativePlatform::set_windows} [MarkupExtensionOption("WINDOWS")],
        macOS: Option<Thickness> {get: NativePlatform::mac_os, set: NativePlatform::set_mac_os} [MarkupExtensionOption("OSX")],
        Linux: Option<Thickness> {get: NativePlatform::linux, set: NativePlatform::set_linux} [MarkupExtensionOption("LINUX")],
        Android: Option<Thickness> {get: NativePlatform::android, set: NativePlatform::set_android} [MarkupExtensionOption("ANDROID")],
        iOS: Option<Thickness> {get: NativePlatform::ios, set: NativePlatform::set_ios} [MarkupExtensionOption("IOS")],
        Browser: Option<Thickness> {get: NativePlatform::browser, set: NativePlatform::set_browser} [MarkupExtensionOption("BROWSER")],
    ],
    methods: [
        fn ProvideValue() -> Rc<NativePlatform> => NativePlatform::provide_value,
        static fn ShouldProvideOption(String) -> bool => |option:String| NativePlatform::should_provide_option(&option),
    ],
});

type NativePlatformDouble = OnPlatformExtensionOf<f64>;
/// Metadata carrier for the actual native double specialization.
#[doc(hidden)]
pub struct OnPlatformDoubleMetadata;
ferro_markup_type!(class OnPlatformDoubleMetadata as "OnPlatformExtension`1" {
    namespace: "FerroUI.Markup.Xaml.MarkupExtensions",
    generic: "OnPlatformExtension`1" [f64],
    handles: [Rc<NativePlatformDouble>, Option<Rc<NativePlatformDouble>>],
    this: Rc<NativePlatformDouble>,
    constructors: [() => NativePlatformDouble::new, (Option<f64>) => NativePlatformDouble::with_default],
    properties: [
        Default: Option<f64> {get: NativePlatformDouble::default, set: NativePlatformDouble::set_default} [MarkupExtensionDefaultOption],
        Windows: Option<f64> {get: NativePlatformDouble::windows, set: NativePlatformDouble::set_windows} [MarkupExtensionOption("WINDOWS")],
        macOS: Option<f64> {get: NativePlatformDouble::mac_os, set: NativePlatformDouble::set_mac_os} [MarkupExtensionOption("OSX")],
        Linux: Option<f64> {get: NativePlatformDouble::linux, set: NativePlatformDouble::set_linux} [MarkupExtensionOption("LINUX")],
        Android: Option<f64> {get: NativePlatformDouble::android, set: NativePlatformDouble::set_android} [MarkupExtensionOption("ANDROID")],
        iOS: Option<f64> {get: NativePlatformDouble::ios, set: NativePlatformDouble::set_ios} [MarkupExtensionOption("IOS")],
        Browser: Option<f64> {get: NativePlatformDouble::browser, set: NativePlatformDouble::set_browser} [MarkupExtensionOption("BROWSER")],
    ],
    methods: [
        fn ProvideValue() -> Rc<NativePlatformDouble> => NativePlatformDouble::provide_value,
        static fn ShouldProvideOption(String) -> bool => |option:String| NativePlatformDouble::should_provide_option(&option),
    ],
});
