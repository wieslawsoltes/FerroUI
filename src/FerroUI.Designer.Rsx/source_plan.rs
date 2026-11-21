//! Source coordinates and checked drafts for declarations used by an RSX view.
use crate::RsxProjection;
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::RustModuleSources;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RsxSourceLocation {
    /// A companion Cargo module URI. An empty option refers to the active Rust buffer.
    pub uri: Option<String>,
    pub span: SourceSpan,
}

pub struct RsxSourceEdit {
    pub source: RsxSourceLocation,
    pub patches: Vec<TextPatch>,
}

impl RsxSourceLocation {
    pub(crate) fn local(span: SourceSpan) -> Self {
        Self { uri: None, span }
    }
}

impl RsxProjection<'_> {
    pub fn declaration_source(&self, location: &RsxSourceLocation) -> Option<&str> {
        match &location.uri {
            Some(uri) => self.modules?.source(uri).map(|file| file.source.as_ref()),
            None => Some(self.source),
        }
    }

    /// Keep the active invocation and its Cargo module context while checking a declaration edit.
    pub(crate) fn declaration_draft(
        &self,
        edit: &RsxSourceEdit,
    ) -> Result<(String, usize, Option<RustModuleSources>), String> {
        if edit.patches.iter().any(|patch| {
            patch.span.start < edit.source.span.start || patch.span.end > edit.source.span.end
        }) {
            return Err("The edit crosses the authored Rust declaration".into());
        }
        let before = self
            .declaration_source(&edit.source)
            .ok_or("The Rust declaration is outside the accepted module sources")?;
        let after = TextPatch::apply_all(before, edit.patches.clone())
            .map_err(|error| error.to_string())?;
        if let Some(uri) = &edit.source.uri {
            let modules = self
                .modules
                .and_then(|modules| modules.with_file_text(uri, &after))
                .ok_or("The edit changes the accepted Rust module layout")?;
            Ok((self.source.to_owned(), self.invocation.start, Some(modules)))
        } else {
            let start = self.invocation_start_after(&edit.patches)?;
            let modules = self.draft_module_sources(&after)?;
            Ok((after, start, modules))
        }
    }
}
