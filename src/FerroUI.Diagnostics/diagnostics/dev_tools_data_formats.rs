//! Port of `Diagnostics/DevToolsDataFormats.cs`.

use ferroui_base::input::{DataFormat, DataFormatOf};

/// Contains data formats related to dev tools.
pub struct DevToolsDataFormats;

impl DevToolsDataFormats {
    // TODO (upstream): this name isn't ideal. For instance, it's not a valid UTI for macOS.
    // We currently have a converter in place in native code for backwards compatibility with IDataObject,
    // but this should ideally be removed at some point.
    // Consider using DataFormat.CreateApplicationFormat() instead (breaking change).

    /// Gets the clipboard data format representing a selector.
    /// It's used for quick format recognition in IDEs.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): the name of the format names the port.
    pub fn selector() -> DataFormatOf<String> {
        ferroui_base::ferro_thread_local! {
            static SELECTOR: DataFormatOf<String> = DataFormat::create_string_platform_format("Ferro_DevTools_Selector");
        }
        SELECTOR.with(Clone::clone)
    }
}
