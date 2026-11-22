//! Resolve portable handoff mappings through the designer's existing catalogs.
mod resolve;

use crate::DesignerAsset;
use ferroui_designer_language::ProjectLibraryEntry;
use ferroui_designer_project::{
    design_handoff::{fingerprint, AssetCopy, ManifestFile},
    file_path, file_uri, PackagedAssets,
};
use ferroui_designer_rsx::RsxCatalog;
use serde::Serialize;
use std::collections::BTreeMap;

pub struct ReviewContext<'a> {
    pub manifest: &'a ManifestFile,
    pub document: &'a str,
    /// Fingerprint of authored source, project registration and definition scope.
    pub revision: &'a str,
    pub toolbox: &'a [DesignerAsset],
    pub rsx: Option<&'a RsxCatalog>,
    pub packaged: Option<&'a PackagedAssets>,
    /// Current XAML drafts matching a canonical file URI. An unavailable draft
    /// must return an error so it cannot be replaced with an older saved file.
    pub draft: &'a dyn Fn(&str) -> Result<Option<String>, String>,
}

#[derive(Serialize)]
pub struct Review {
    pub manifest: String,
    pub document: String,
    pub rows: Vec<Row>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub target: String,
    pub reference: Option<String>,
    pub definition: Option<ProjectLibraryEntry>,
    pub asset_uri: Option<String>,
    pub insertion: Option<Insertion>,
    pub copy: Option<CopySummary>,
    pub review: Option<String>,
    pub issue: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Insertion {
    Asset { control: String },
    Image { source: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopySummary {
    pub direction: &'static str,
    pub source: String,
    pub destination: String,
    pub source_label: String,
    pub destination_label: String,
    pub bytes: usize,
}

pub struct Resolved {
    pub row: Row,
    copy: Option<AssetCopy>,
}

impl ReviewContext<'_> {
    pub fn review(&self) -> Result<Review, String> {
        self.validate_document()?;
        let mut copies = BTreeMap::new();
        let mut rows = self
            .manifest
            .manifest
            .mappings
            .iter()
            .map(|mapping| {
                let mut row = Row::new(mapping);
                match self.resolve_mapping(mapping) {
                    Ok(resolved) => {
                        row = resolved.row;
                        if let Some(copy) = &row.copy {
                            *copies.entry(copy.destination.clone()).or_insert(0usize) += 1;
                        }
                    }
                    Err(error) => row.issue = Some(error),
                }
                row
            })
            .collect::<Vec<_>>();
        for row in &mut rows {
            if row
                .copy
                .as_ref()
                .is_some_and(|copy| copies[&copy.destination] > 1)
            {
                row.issue = Some("More than one mapping writes to this destination".into());
                row.review = None;
            }
        }
        Ok(Review {
            manifest: file_uri(self.manifest.path()).map_err(|error| error.to_string())?,
            document: self.document.into(),
            rows,
        })
    }

    pub fn checked(&self, id: &str, review: &str) -> Result<Resolved, String> {
        self.validate_document()?;
        let mapping = self
            .manifest
            .manifest
            .mappings
            .iter()
            .find(|mapping| mapping.id == id)
            .ok_or("The mapping was removed. Refresh the review")?;
        let resolved = self.resolve_mapping(mapping)?;
        if resolved.row.review.as_deref() != Some(review) {
            return Err("The mapping changed. Refresh the review before applying it".into());
        }
        self.manifest.verify().map_err(|error| error.to_string())?;
        Ok(resolved)
    }

    pub fn copy(&self, id: &str, review: &str) -> Result<String, String> {
        let resolved = self.checked(id, review)?;
        let copy = resolved
            .copy
            .ok_or("There is no pending copy for this mapping")?;
        copy.apply(self.manifest)
            .map_err(|error| error.to_string())?;
        file_uri(copy.destination).map_err(|error| error.to_string())
    }

    fn validate_document(&self) -> Result<(), String> {
        let path = file_path(self.document).map_err(|error| error.to_string())?;
        let actual = path.canonicalize().map_err(|error| error.to_string())?;
        if !actual.is_file() || !actual.starts_with(self.manifest.workspace()) {
            return Err("Choose a design inside the selected workspace".into());
        }
        Ok(())
    }

    fn review_id(&self, id: &str, evidence: &str) -> String {
        fingerprint(&(
            self.manifest.workspace(),
            self.manifest.path(),
            self.manifest.fingerprint(),
            self.document,
            self.revision,
            id,
            evidence,
        ))
    }
}
