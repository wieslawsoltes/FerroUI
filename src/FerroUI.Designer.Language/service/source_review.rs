use super::{
    handlers::read_rust, DocumentEdits, LanguageService, LineIndex, Location, Result, State,
    TextEdit,
};
use crate::{require, LanguageError};
use ferroui_designer_core::{SourceSpan, TextPatch, XmlText};
use ferroui_designer_project::file_path;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

/// A source review starts from an exact open buffer and a UTF-16 position.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourceReviewRequest {
    pub version: i64,
    pub position: crate::Position,
}

#[derive(Debug, Deserialize)]
pub struct SourceDraft {
    pub uri: String,
    pub version: i64,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSymbolRename {
    pub uri: String,
    pub position: crate::Position,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RustRenameResolution {
    pub rename: RustSymbolRename,
    pub documents: Vec<DocumentEdits>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRenamePlan {
    pub name: String,
    pub new_name: String,
    pub review: String,
    pub documents: Vec<ReviewedDocument>,
    pub unmapped: Vec<Location>,
    /// A semantic Rust rename required before applying the review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_rename: Option<RustSymbolRename>,
}

pub use SourceRenamePlan as RegisteredRenamePlan;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReviewedDocument {
    pub edit: DocumentEdits,
    pub before: String,
    pub after: String,
}

/// An incomplete draft can conceal a use that still exists in its saved source.
pub(super) fn check_incomplete(
    uri: &str,
    state: &State,
    mentions: impl Fn(&str) -> bool,
) -> Result<()> {
    let relevant = |source: &str| {
        mentions(source)
            || source.contains('&')
                && XmlText::text_node(source, SourceSpan::new(0, source.len()))
                    .is_some_and(|text| mentions(text.value()))
    };
    let draft = relevant(state.document.source()) || relevant(state.document.valid_source());
    let path = file_path(uri)?;
    let disk = if !draft && state.opened && path.is_file() {
        use std::io::Read;
        const MAX: u64 = 16 * 1024 * 1024;
        let mut source = String::new();
        std::fs::File::open(path)
            .and_then(|file| file.take(MAX + 1).read_to_string(&mut source))
            .map_err(|error| LanguageError(error.to_string()))?;
        require(source.len() as u64 <= MAX, "Indexed file exceeds 16 MiB")?;
        relevant(&source)
    } else {
        false
    };
    require(
        !draft && !disk,
        format!("Repair the incomplete XAML that may use the name before reviewing: {uri}"),
    )
}

pub(super) fn reviewed_document(
    uri: &str,
    version: i64,
    index: &LineIndex,
    patches: Vec<TextPatch>,
) -> Result<ReviewedDocument> {
    let before = index.source();
    let edits = patches
        .iter()
        .map(|patch| {
            Ok(TextEdit {
                range: index.range(patch.span)?,
                new_text: patch.replacement.clone(),
            })
        })
        .collect::<Result<_>>()?;
    let after = TextPatch::apply_all(before, patches)?;
    Ok(ReviewedDocument {
        edit: DocumentEdits {
            uri: uri.to_owned(),
            version,
            edits,
        },
        before: before.to_owned(),
        after,
    })
}

pub(super) struct RustSnapshot<'a> {
    pub text: Cow<'a, str>,
    pub version: i64,
}

impl<'a> RustSnapshot<'a> {
    pub fn read(uri: &str, drafts: &BTreeMap<String, &'a SourceDraft>) -> Result<Self> {
        if let Some(draft) = drafts.get(uri) {
            return Ok(Self {
                text: Cow::Borrowed(&draft.source),
                version: draft.version,
            });
        }
        Ok(Self {
            text: Cow::Owned(read_rust(uri)?),
            version: -1,
        })
    }

    pub fn shared(&self) -> RustSnapshot<'_> {
        RustSnapshot {
            text: Cow::Borrowed(self.text.as_ref()),
            version: self.version,
        }
    }

    pub fn document(&self, uri: &str, patches: Vec<TextPatch>) -> Result<ReviewedDocument> {
        reviewed_document(uri, self.version, &LineIndex::new(&self.text), patches)
    }

    pub fn merge(
        &self,
        uri: &str,
        mut patches: Vec<TextPatch>,
        documents: &mut Vec<ReviewedDocument>,
    ) -> Result<ReviewedDocument> {
        let index = LineIndex::new(&self.text);
        if let Some(position) = documents.iter().position(|entry| entry.edit.uri == uri) {
            let base = documents.remove(position);
            require(
                base.before == self.text && base.edit.version == self.version,
                "The Rust source changed while preparing the review",
            )?;
            for edit in base.edit.edits {
                patches.push(TextPatch {
                    span: index.span(edit.range)?,
                    replacement: edit.new_text,
                });
            }
        }
        patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
        patches.dedup();
        reviewed_document(uri, self.version, &index, patches)
    }
}

impl LanguageService {
    pub(super) fn refresh_project_index(&mut self) -> Result<()> {
        let solutions = self
            .workspaces
            .values()
            .map(|workspace| workspace.solution.clone())
            .collect::<Vec<_>>();
        for mut solution in solutions {
            solution.refresh()?;
            self.index_solution(&solution)?;
        }
        let missing = self
            .documents
            .iter()
            .filter(|(uri, state)| {
                !state.opened
                    && uri.starts_with("file://")
                    && file_path(uri).is_ok_and(|path| !path.is_file())
            })
            .map(|(uri, _)| uri.clone())
            .collect::<Vec<_>>();
        for uri in missing {
            self.set_state(uri, None);
        }
        Ok(())
    }
}
