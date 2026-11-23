//! The framework's types resolve with no XML prefix, in the run-time loader and in the markup
//! compiler (ground rule 7 of `docs/porting/PORTING-GUIDE.md`, #1280). One type of each namespace.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::{Border, Control};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::compile_documents;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// A type of each namespace that is mapped to the default XML namespace by this work.
const TYPES: &[&str] = &[
    // FerroUI.Collections
    "ResetBehavior",
    // FerroUI.Interactivity
    "RoutingStrategies",
    // FerroUI.Media.Fonts
    "IFontCollection",
    // FerroUI.Media.Immutable
    "ImmutableSolidColorBrush",
    // FerroUI.Media.TextFormatting
    "TabAlignment",
    // FerroUI.Platform
    "PlatformThemeVariant",
    // FerroUI.Platform.Storage
    "WellKnownFolder",
    // FerroUI.Rendering
    "RendererDebugOverlays",
    // FerroUI.Controls.Converters
    "EnumToBoolConverter",
    // FerroUI.Controls.Platform
    "X11NetWmWindowType",
    // FerroUI.Controls.Primitives.PopupPositioning
    "PopupAnchor",
    // FerroUI.Controls.Selection
    "ISelectionModel",
    // FerroUI.Navigation
    "NavigationMode",
    // FerroUI.Controls.Repeater
    "IndexPath",
];

struct Scope {
    locator: Option<Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

fn scope() -> Scope {
    crate::register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    FerroRuntimeXamlLoader::register();
    Scope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

fn document(name: &str) -> String {
    format!(
        r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
            Tag="{{x:Type {name}}}" />"#
    )
}

#[test]
fn each_namespace_resolves_in_the_run_time_loader() {
    let _scope = scope();
    for name in TYPES {
        let loaded = FerroRuntimeXamlLoader::parse_as::<Ref<Border>>(&document(name), None);
        assert!(loaded.is_ok(), "{name}: {:?}", loaded.err());
    }
}

#[test]
fn each_namespace_resolves_in_the_markup_compiler() {
    let _scope = scope();
    for name in TYPES {
        let xaml = document(name);
        let compiled = compile_documents(
            &[("Types.xaml", xaml.as_str())],
            Some("ferres://App/"),
            &RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        let source = compiled[0].source.as_ref().unwrap_or_else(|reason| panic!("{name}: {reason}"));
        assert!(source.contains(name), "{name}: {source}");
    }
}
