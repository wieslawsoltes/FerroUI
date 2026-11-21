//! Revalidate an edit without substituting disk contents for the accepted drafts.
use super::{MAX_FILE_BYTES, RustModuleSources};
use std::sync::Arc;
use syn::spanned::Spanned;

impl RustModuleSources {
    /// Use the same captured sources to project a companion module.
    pub fn for_module(&self, uri: &str) -> Option<Self> {
        let source = self.source(uri)?;
        let mut snapshot = self.clone();
        snapshot.active.clone_from(&source.uri);
        Some(snapshot)
    }

    /// Make a draft of the accepted snapshot for an edit which preserves its
    /// authored module declarations. The original frame remains immutable.
    pub fn with_file_text(&self, uri: &str, source: &str) -> Option<Self> {
        let previous = self.source(uri)?;
        if source.len() > MAX_FILE_BYTES || layout(&previous.source)? != layout(source)? {
            return None;
        }
        let mut draft = self.clone();
        draft.files.get_mut(uri)?.source = Arc::from(source);
        Some(draft)
    }
}

fn layout(source: &str) -> Option<Vec<&str>> {
    let file = syn::parse_str::<syn::File>(source).ok()?;
    let mut layout = file
        .attrs
        .iter()
        .map(|attribute| source.get(attribute.span().byte_range()))
        .collect::<Option<Vec<_>>>()?;
    collect(source, &file.items, &mut layout)?;
    Some(layout)
}

fn collect<'a>(source: &'a str, items: &[syn::Item], layout: &mut Vec<&'a str>) -> Option<()> {
    for item in items {
        let syn::Item::Mod(item) = item else { continue };
        let range = item.span().byte_range();
        if let Some((braces, items)) = &item.content {
            layout.push(source.get(range.start..braces.span.open().byte_range().end)?);
            collect(source, items, layout)?;
            layout.push("}");
        } else {
            layout.push(source.get(range)?);
        }
    }
    Some(())
}
