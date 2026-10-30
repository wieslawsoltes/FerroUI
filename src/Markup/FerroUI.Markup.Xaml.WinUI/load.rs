//! The run-time entry points of the importer: the WinUI counterparts of the run-time loader's
//! `load`, `parse` and `parse_as` (`FerroRuntimeXamlLoader`), which migrate the document first.

use crate::{migrate_winui_xaml, WinUIXamlMigration, WinUIXamlOptions};
use ferroui_base::metadata::{from_markup_value, MarkupAssembly};
use ferroui_base::utilities::Uri;
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// Loads an explicitly opted-in WinUI document after the shared migration pass. Unresolved
/// semantics fail with their original-source diagnostic codes and locations. The normal compiler
/// then checks real registered types and members; migration never hides a loader error.
/// Referenced asset documents must be migrated separately by their owning project. Use
/// [`migrate_winui_xaml`] directly to inspect adaptation evidence or to feed the exact same
/// document into the build-time compiler.
pub fn load_winui(
    xaml: &str,
    local_assembly: Option<&'static MarkupAssembly>,
    root_instance: Option<BoxedValue>,
    uri: Option<Uri>,
    design_mode: bool,
    options: &WinUIXamlOptions,
) -> Result<BoxedValue, XamlLoadException> {
    // The migrated document names the x:Bind types of this crate.
    crate::register_types();
    let migrated = migrate_winui_xaml(xaml, options)
        .and_then(WinUIXamlMigration::into_xaml)
        .map_err(|error| XamlLoadException::with_inner(error.to_string(), error))?;
    // A fresh-root preview can convert a typed, deferred XAML resource error to Result. Loading an
    // existing instance remains outside this boundary: it can be partly populated when a callback
    // fails, and this function does not promise rollback.
    if root_instance.is_none() {
        ferroui_markup_xaml::xaml_error_scope::with_xaml_errors(|| {
            FerroRuntimeXamlLoader::load(&migrated, local_assembly, None, uri, design_mode)
        })
    } else {
        FerroRuntimeXamlLoader::load(&migrated, local_assembly, root_instance, uri, design_mode)
    }
}

/// The WinUI counterpart of `FerroRuntimeXamlLoader::parse`; the ordinary `parse` keeps its
/// dialect.
pub fn parse_winui(
    xaml: &str,
    local_assembly: Option<&'static MarkupAssembly>,
    options: &WinUIXamlOptions,
) -> Result<BoxedValue, XamlLoadException> {
    load_winui(xaml, local_assembly, None, None, false, options)
}

/// The WinUI counterpart of `FerroRuntimeXamlLoader::parse_as`, with the same checked handle
/// conversion.
pub fn parse_winui_as<T: Clone + 'static>(
    xaml: &str,
    local_assembly: Option<&'static MarkupAssembly>,
    options: &WinUIXamlOptions,
) -> Result<T, XamlLoadException> {
    let loaded = parse_winui(xaml, local_assembly, options)?;
    let type_name = loaded.type_name();
    from_markup_value::<T>(&Some(loaded)).ok_or_else(|| {
        XamlLoadException::with_message(format!(
            "Unable to cast object of type '{type_name}' to type '{}'.",
            std::any::type_name::<T>()
        ))
    })
}
