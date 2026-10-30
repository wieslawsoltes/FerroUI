//! Source-preserving catalog model shared by the UI, tests and parity reports.

pub use crate::catalog_data::{CATALOG, SOURCE_COMMIT};

/// An upstream documentation link.
#[derive(Clone, Copy, Debug)]
pub struct DocumentationLink {
    pub title: &'static str,
    pub uri: &'static str,
}

/// One original code sample. Source sections retain every substitution token;
/// only line endings/trailing newlines are normalized. Runnable adaptations are separate.
#[derive(Clone, Copy, Debug)]
pub struct SampleDefinition {
    pub header: &'static str,
    pub source: &'static str,
    pub source_sha256: &'static str,
    pub xaml: &'static str,
    pub code: &'static str,
    pub namespace_imports: &'static [&'static str],
}

/// One entry of Microsoft's checked-in `catalog/windows-samples.json`.
#[derive(Clone, Copy, Debug)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    /// Exact source metadata used by the AutoSuggestBox details example.
    pub subtitle: &'static str,
    /// Embedded native resource URI, retaining the source path's spelling.
    pub image_uri: &'static str,
    pub source: &'static str,
    pub code_behind: &'static str,
    pub docs: &'static [DocumentationLink],
    pub samples: &'static [SampleDefinition],
    /// Precomputed lowercase search index, so filtering does not allocate per row.
    pub search_index: &'static str,
}

impl CatalogEntry {
    /// The exact source used for this port, rather than the moving upstream branch.
    pub fn source_url(&self) -> String {
        source_url(self.source)
    }

    /// Searches all whitespace-delimited tokens. Category, tags and the stable
    /// upstream ID are indexed as well as the title and description.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        query
            .split_whitespace()
            .all(|token| self.search_index.contains(token))
    }
}

/// URL of a file in the immutable upstream source snapshot.
pub fn source_url(path: &str) -> String {
    format!("https://github.com/microsoft/WinUI-Gallery/blob/{SOURCE_COMMIT}/{path}")
}

/// Finds a page by its stable upstream ID (ASCII case-insensitive).
pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id))
}

/// Filters the full catalog without creating controls for non-selected pages.
pub fn search(query: &str, category: Option<&str>) -> Vec<&'static CatalogEntry> {
    let folded = query.to_lowercase();
    let tokens: Vec<_> = folded.split_whitespace().collect();
    CATALOG
        .iter()
        .filter(|entry| category.is_none_or(|category| entry.category == category))
        .filter(|entry| {
            tokens
                .iter()
                .all(|token| entry.search_index.contains(token))
        })
        .collect()
}
