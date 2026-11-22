use super::{CopySummary, Insertion, Resolved, ReviewContext, Row};
use crate::{raster_asset_fragment, DesignerAsset};
use ferroui_designer_core::Document;
use ferroui_designer_language::{exact_library_definition, ProjectLibraryEntry};
use ferroui_designer_project::{
    design_handoff::{fingerprint, Asset, AssetCopy, Mapping, Target, Transfer},
    file_uri, relative_file_path, relative_file_reference,
};
use std::path::Path;

impl Row {
    pub(super) fn new(mapping: &Mapping) -> Self {
        let (kind, target) = match &mapping.target {
            Target::Control { toolbox } => ("Control", toolbox.clone()),
            Target::Definition { path, key, scope } => {
                let target = [Some(path.as_str()), key.as_deref(), scope.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                ("Definition", target)
            }
            Target::Asset { path, .. } => ("Asset", path.clone()),
            Target::PublishedAsset { address, .. } => ("Published asset", address.clone()),
        };
        Self {
            id: mapping.id.clone(),
            name: mapping.name.clone().unwrap_or_else(|| mapping.id.clone()),
            kind,
            target,
            reference: None,
            definition: None,
            asset_uri: None,
            insertion: None,
            copy: None,
            review: None,
            issue: None,
        }
    }
}

impl ReviewContext<'_> {
    pub(super) fn resolve_mapping(&self, mapping: &Mapping) -> Result<Resolved, String> {
        let mut row = Row::new(mapping);
        let mut copy: Option<AssetCopy> = None;
        let evidence = match &mapping.target {
            Target::Control { toolbox } => {
                let asset = self.control(toolbox)?;
                row.reference = Some(self.control_reference(&asset.source, &mut row.issue));
                if row.issue.is_none() {
                    row.insertion = Some(Insertion::Asset {
                        control: toolbox.clone(),
                    });
                }
                fingerprint(&asset.source)
            }
            Target::Definition { path, key, scope } => {
                let source_path = self
                    .manifest
                    .project_path(path)
                    .map_err(|error| error.to_string())?;
                let uri = file_uri(&source_path).map_err(|error| error.to_string())?;
                let source = match (self.draft)(&uri)? {
                    Some(source) => source,
                    None => self
                        .manifest
                        .project_text(path)
                        .map_err(|error| error.to_string())?,
                };
                let definition =
                    exact_library_definition(&source, key.as_deref(), scope.as_deref())
                        .map_err(|error| error.to_string())?;
                row.reference = Some(definition.name.clone());
                row.definition = Some(ProjectLibraryEntry {
                    folder: file_uri(self.manifest.workspace())
                        .map_err(|error| error.to_string())?,
                    path: relative_file_path(self.manifest.workspace(), &source_path)
                        .map_err(|error| error.to_string())?,
                    definition,
                });
                fingerprint(&(uri, source))
            }
            Target::Asset { path, transfer } => {
                let asset = self
                    .manifest
                    .project_asset(path, transfer.as_ref())
                    .map_err(|error| error.to_string())?;
                let importing = matches!(transfer, Some(Transfer::Import { .. }));
                self.asset(&mut row, &asset, None, importing)?;
                copy = asset.copy;
                fingerprint(&(asset.file, copy.as_ref().map(AssetCopy::fingerprint)))
            }
            Target::PublishedAsset { address, export } => {
                let asset = self
                    .manifest
                    .published_asset(self.packaged, address, export.as_deref())
                    .map_err(|error| error.to_string())?;
                self.asset(&mut row, &asset, Some(address), false)?;
                copy = asset.copy;
                fingerprint(&(
                    address,
                    asset.file,
                    copy.as_ref().map(AssetCopy::fingerprint),
                ))
            }
        };
        row.review = Some(self.review_id(&mapping.id, &evidence));
        Ok(Resolved { row, copy })
    }

    fn control(&self, toolbox: &str) -> Result<&DesignerAsset, String> {
        let mut matches = self.toolbox.iter().filter(|asset| asset.id == toolbox);
        let asset = matches
            .next()
            .ok_or("Choose an exact Toolbox ID from the active project")?;
        if matches.next().is_some() {
            return Err("More than one Toolbox entry uses that ID".into());
        }
        Ok(asset)
    }

    fn control_reference(&self, source: &str, issue: &mut Option<String>) -> String {
        let reference = match self.rsx {
            Some(catalog) => Document::new(source)
                .map_err(|error| error.to_string())
                .and_then(|document| catalog.xaml_to_rsx(&document)),
            None => Ok(source.into()),
        };
        match reference {
            Ok(reference) => excerpt(&reference),
            Err(error) => {
                *issue = Some(format!(
                    "The control has no supported RSX placement. {error}"
                ));
                excerpt(source)
            }
        }
    }

    fn asset(
        &self,
        row: &mut Row,
        asset: &Asset,
        address: Option<&str>,
        importing: bool,
    ) -> Result<(), String> {
        let uri = file_uri(&asset.file).map_err(|error| error.to_string())?;
        let reference = match address {
            Some(address) => address.into(),
            None => {
                relative_file_reference(self.document, &uri).map_err(|error| error.to_string())?
            }
        };
        if asset.available {
            row.asset_uri = Some(uri);
            if asset.raster {
                let fragment = raster_asset_fragment(&reference)?;
                self.control_reference(&fragment, &mut row.issue);
                if row.issue.is_none() {
                    row.insertion = Some(Insertion::Image {
                        source: reference.clone(),
                    });
                }
            }
        }
        row.reference = Some(reference);
        row.copy = asset.copy.as_ref().map(|copy| CopySummary {
            direction: if importing { "import" } else { "export" },
            source: display_path(&copy.source),
            destination: display_path(&copy.destination),
            bytes: copy.bytes,
        });
        Ok(())
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn excerpt(source: &str) -> String {
    let limit = source.char_indices().nth(480).map(|(byte, _)| byte);
    match limit {
        Some(end) => format!("{}…", &source[..end]),
        None => source.into(),
    }
}
