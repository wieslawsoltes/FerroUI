//! FerroUI in one dependency.
//!
//! The facade crate of FerroUI: it has no code of its own. It re-exports the
//! crates of the framework, each under the name of its crate without the
//! `ferroui-` prefix (`ferroui-base` is [`base`], `ferroui-themes-fluent` is
//! `themes_fluent`), and gathers the types of a minimal application in a
//! [`prelude`]. Features choose the parts, so that an application names one
//! dependency:
//!
//! ```toml
//! [dependencies]
//! ferroui = "=0.1.0-preview.2"
//! ```
//!
//! The crates of one release depend on one another at exactly that version,
//! so the facade selects one consistent set of them.
//!
//! # Modules and features
//!
//! | Module | Crate | Feature | Default |
//! |---|---|---|---|
//! | [`base`] | `ferroui-base` | always | |
//! | [`controls`] | `ferroui-controls` | always | |
//! | [`markup`] | `ferroui-markup` | always | |
//! | `desktop`, `skia`, `harfbuzz`; `native` on macOS, `win32` on Windows, `x11` on Linux | `ferroui-desktop` and the crates it selects among | `desktop` | yes |
//! | `themes_fluent` | `ferroui-themes-fluent` | `fluent` | yes |
//! | `themes_simple` | `ferroui-themes-simple` | `simple` | no |
//! | `themes_winui` | `ferroui-themes-winui` | `winui-theme` | no |
//! | `markup_xaml`, `markup_xaml_loader` | `ferroui-markup-xaml`, `ferroui-markup-xaml-loader` (run-time loading) | `xaml` | no |
//! | `controls_color_picker` | `ferroui-controls-color-picker` | `color-picker` | no |
//! | `controls_charts` | `ferroui-controls-charts` | `charts` | no |
//! | `controls_inking` | `ferroui-controls-inking` | `inking` | no |
//! | `controls_treedatagrid`, `treedatagrid_core` | `ferroui-controls-treedatagrid`, `ferroui-treedatagrid-core` | `treedatagrid` | no |
//! | `dialogs` | `ferroui-dialogs` | `dialogs` | no |
//! | `diagnostics` | `ferroui-diagnostics` (the developer tools) | `diagnostics` | no |
//! | `system`, `reactive` | `ferroui-system`, `ferroui-reactive` (Rx) | `reactive` | no |
//! | `dynamic_data` | `ferroui-dynamic-data` (DynamicData) | `dynamic-data` | no |
//! | `splat`, `reactiveui`, `reactiveui_controls` | `ferroui-splat`, `ferroui-reactiveui`, `ferroui-reactiveui-controls` (ReactiveUI and its integration) | `reactiveui` | no |
//! | `browser`, `skia`, `harfbuzz` | `ferroui-browser` | `browser` | no |
//! | `vello` | `ferroui-vello`; its GPU modes with `vello-hybrid` and `vello-gpu` | `vello` | no |
//! | `headless` | `ferroui-headless` | `headless` | no |
//! | `opengl`, `metal`, `vulkan` | the platform graphics contracts | `opengl`, `metal`, `vulkan` | no |
//!
//! An application for the browser turns the default features off and names
//! `browser` and a theme. The build-time crates (`ferroui-build`,
//! `ferroui-build-scan`, `xamlx`) are not part of the facade: a crate that
//! compiles its markup in its build script names `ferroui-build` as a build
//! dependency, and the code the compiler writes names the crates it calls by
//! their own names, so such a crate depends on those crates directly.
//!
//! # A minimal application
//!
//! An application is a class derived from [`Application`](controls::Application)
//! that adds a theme to its styles and creates its main window when the
//! framework has finished initialising; `main` configures it with the
//! application builder and starts it with the classic desktop lifetime:
//!
//! ```no_run
//! use ferroui::prelude::*;
//!
//! #[repr(C)]
//! pub struct App {
//!     base: Application,
//! }
//!
//! ferro_class!(App: Application);
//! ferro_impl_classes!(App: FerroObjectImpl);
//!
//! impl NewApplication for App {
//!     fn new_application() -> Ref<Self> {
//!         instantiate(Self { base: Application::construct() })
//!     }
//! }
//!
//! impl ApplicationImpl for App {
//!     fn initialize(this: &Self) {
//!         Self::parent_initialize(this);
//!         this.styles().add(FluentTheme::new().as_style());
//!     }
//!
//!     fn on_framework_initialization_completed(this: &Self) {
//!         let lifetime = this.application_lifetime();
//!         if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
//!             let button = Button::new();
//!             button.set_content(Some(std::rc::Rc::new("Click me".to_string())));
//!             button.click(|_, _| println!("Button clicked"));
//!
//!             let window = Window::new();
//!             window.set_title(Some("Hello".to_string()));
//!             window.set_content(Some(Control::boxed(button)));
//!             desktop.set_main_window(Some(window));
//!         }
//!         Self::parent_on_framework_initialization_completed(this);
//!     }
//! }
//!
//! fn main() -> std::process::ExitCode {
//!     let args: Vec<String> = std::env::args().skip(1).collect();
//!     let exit_code = AppBuilder::configure::<App>()
//!         .use_platform_detect()
//!         .start_with_classic_desktop_lifetime(&args);
//!     std::process::ExitCode::from(exit_code as u8)
//! }
//! ```
//!
//! The same program with a few more controls is the example `hello` of this
//! crate (`cargo run -p ferroui --example hello`).
//!
//! The application builder of the facade is the one of the crates it
//! re-exports: on the desktop, platform detection selects the platform of
//! the operating system, Skia and HarfBuzz, and nothing is initialised until
//! the application is set up.
//!
//! ```
//! # #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
//! # {
//! use ferroui::prelude::*;
//!
//! let builder = AppBuilder::configure::<Application>().use_platform_detect();
//! assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
//! assert_eq!(builder.text_shaping_subsystem_name().as_deref(), Some("HarfBuzz"));
//! assert!(builder.instance().is_none());
//! # }
//! ```

#![forbid(unsafe_code)]

pub use ferroui_base as base;
pub use ferroui_controls as controls;
pub use ferroui_markup as markup;

#[cfg(feature = "desktop")]
pub use ferroui_desktop as desktop;
#[cfg(all(feature = "desktop", target_os = "macos"))]
pub use ferroui_native as native;
#[cfg(all(feature = "desktop", windows))]
pub use ferroui_win32 as win32;
#[cfg(all(feature = "desktop", target_os = "linux"))]
pub use ferroui_x11 as x11;
#[cfg(any(feature = "desktop", feature = "browser"))]
pub use ferroui_harfbuzz as harfbuzz;
#[cfg(any(feature = "desktop", feature = "browser"))]
pub use ferroui_skia as skia;

#[cfg(feature = "fluent")]
pub use ferroui_themes_fluent as themes_fluent;
#[cfg(feature = "simple")]
pub use ferroui_themes_simple as themes_simple;
#[cfg(feature = "winui-theme")]
pub use ferroui_themes_winui as themes_winui;

#[cfg(feature = "xaml")]
pub use ferroui_markup_xaml as markup_xaml;
#[cfg(feature = "xaml")]
pub use ferroui_markup_xaml_loader as markup_xaml_loader;

#[cfg(feature = "color-picker")]
pub use ferroui_controls_color_picker as controls_color_picker;
#[cfg(feature = "charts")]
pub use ferroui_controls_charts as controls_charts;
#[cfg(feature = "inking")]
pub use ferroui_controls_inking as controls_inking;
#[cfg(feature = "treedatagrid")]
pub use ferroui_controls_treedatagrid as controls_treedatagrid;
#[cfg(feature = "treedatagrid")]
pub use ferroui_treedatagrid_core as treedatagrid_core;
#[cfg(feature = "dialogs")]
pub use ferroui_dialogs as dialogs;
#[cfg(feature = "diagnostics")]
pub use ferroui_diagnostics as diagnostics;

#[cfg(feature = "reactive")]
pub use ferroui_system as system;
#[cfg(feature = "reactive")]
pub use ferroui_reactive as reactive;
#[cfg(feature = "dynamic-data")]
pub use ferroui_dynamic_data as dynamic_data;
#[cfg(feature = "reactiveui")]
pub use ferroui_splat as splat;
#[cfg(feature = "reactiveui")]
pub use ferroui_reactiveui as reactiveui;
#[cfg(feature = "reactiveui")]
pub use ferroui_reactiveui_controls as reactiveui_controls;

#[cfg(feature = "browser")]
pub use ferroui_browser as browser;
#[cfg(feature = "headless")]
pub use ferroui_headless as headless;
#[cfg(feature = "vello")]
pub use ferroui_vello as vello;

#[cfg(feature = "metal")]
pub use ferroui_metal as metal;
#[cfg(feature = "opengl")]
pub use ferroui_opengl as opengl;
#[cfg(feature = "vulkan")]
pub use ferroui_vulkan as vulkan;

pub mod prelude {
    //! The types of a minimal application, for a glob import
    //! (`use ferroui::prelude::*`).
    //!
    //! - The declaration of an application class: the macros [`ferro_class!`]
    //!   and [`ferro_impl_classes!`], [`instantiate`], [`Ref`], the traits
    //!   [`FerroObjectImpl`], [`ApplicationImpl`], [`ApplicationImplExt`] and
    //!   [`NewApplication`], and [`Application`] itself.
    //! - Starting it: [`AppBuilder`], and with the feature `desktop` the trait
    //!   that gives it `use_platform_detect` (`AppBuilderDesktopExtensions`).
    //! - The window and the common controls and panels: [`Window`],
    //!   [`Control`], [`Border`], [`Button`], [`CheckBox`], [`TextBlock`],
    //!   [`TextBox`], [`StackPanel`], [`Grid`].
    //! - The values a layout is written with: [`Thickness`], [`CornerRadius`],
    //!   [`HorizontalAlignment`], [`VerticalAlignment`], [`Orientation`] and
    //!   [`Color`].
    //! - The theme of the features that are on: `FluentTheme` (`fluent`),
    //!   `SimpleTheme` (`simple`) and `WinUITheme` (`winui-theme`).
    //!
    //! Everything else is reached through the modules of the crate
    //! ([`crate::base`], [`crate::controls`], ...).

    pub use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
    pub use ferroui_base::media::Color;
    pub use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, CornerRadius, FerroObjectImpl, Ref, Thickness};
    pub use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Button, CheckBox, Control, Grid,
        NewApplication, StackPanel, TextBlock, TextBox, Window,
    };

    #[cfg(feature = "desktop")]
    pub use ferroui_desktop::AppBuilderDesktopExtensions;
    #[cfg(feature = "fluent")]
    pub use ferroui_themes_fluent::FluentTheme;
    #[cfg(feature = "simple")]
    pub use ferroui_themes_simple::SimpleTheme;
    #[cfg(feature = "winui-theme")]
    pub use ferroui_themes_winui::WinUITheme;
}

/// Chrome DevTools Protocol inspection and automation.
#[cfg(feature = "devtools")]
pub use ferroui_devtools as devtools;
