use super::{DocumentEdits, LanguageService, LineIndex, Result, TextEdit};
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::file_path;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReviewedDocument {
    pub edit: DocumentEdits,
    pub before: String,
    pub after: String,
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
