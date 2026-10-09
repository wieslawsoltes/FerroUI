//! The markup documents and the other assets of the sample as embedded
//! assets.
//!
//! The module of the sample, with the tables the build script of this crate
//! generates from the files of the sample: the documents and the other
//! assets as they are (no placeholder artwork is put in).

use crate::ASSEMBLY;
use ferroui_base::platform::register_assets;

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// A document that does not load yet, from `excluded.txt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExcludedDocument {
    /// The rooted asset path of the document (`/Pages/CalendarPage.xaml`).
    pub path: &'static str,
    /// Whether the document itself loads and only its class does not yet
    /// construct and show.
    pub page_only: bool,
    /// What the document waits for: `missing: <types>` for types that are
    /// not ported, `gap <id>: <text>` for a gap of the framework,
    /// `code-behind: <text>` for a class of the sample that is not ported.
    pub reason: &'static str,
}

/// The documents that do not load yet.
pub fn excluded_documents() -> &'static [ExcludedDocument] {
    EXCLUDED
}

/// What the document with the rooted asset path `path` waits for, when it
/// is listed.
pub fn excluded(path: &str) -> Option<&'static ExcludedDocument> {
    EXCLUDED.iter().find(|excluded| excluded.path.eq_ignore_ascii_case(path))
}

/// The markup documents of the sample, as `(rooted asset path, the class
/// the document names)`.
pub fn documents() -> &'static [(&'static str, Option<&'static str>)] {
    DOCUMENTS
}

/// The content of the embedded asset with the rooted path `path`.
pub(crate) fn asset(path: &str) -> Option<&'static [u8]> {
    ASSETS.iter().find(|(asset_path, _)| *asset_path == path).map(|(_, content)| *content)
}

/// Registers the embedded assets with the asset loader under the name of
/// the assembly of the sample.
pub(crate) fn register() {
    register_assets(ASSEMBLY.name, ASSETS);
}
