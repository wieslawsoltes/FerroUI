//! Saved comparison boards and scenario references in a reviewed XAML move.
use super::{move_document, FileMoveDocument, FileMoveDraft, LanguageService};
use crate::{LanguageError, LineIndex, Result};
use ferroui_designer_project::{
    workspace_documents, ProjectWorkspace, SavedDesignFile, XamlFileMove,
};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf};

#[path = "file_move_saved_sites.rs"]
mod sites;
#[path = "file_move_json.rs"]
mod token;
use sites::{Move, Scenario, StageSet};

const SAVED_FILE_LIMIT: usize = 1024 * 1024;

struct Candidate<'a> {
    path: PathBuf,
    kind: SavedDesignFile,
    workspace: &'a ProjectWorkspace,
}

pub(super) fn plan(
    service: &LanguageService,
    movement: &XamlFileMove,
    drafts: &BTreeMap<String, &FileMoveDraft>,
) -> Result<Vec<FileMoveDocument>> {
    let mut documents = Vec::new();
    let mut stage_keys = BTreeMap::new();
    let mut scenarios = Vec::new();
    let candidates = candidates(service)?;
    let context = Move::new(movement)?;
    for (uri, file) in &candidates {
        if file.kind == SavedDesignFile::Scenario {
            scenarios.push((uri, file));
            continue;
        }
        let (version, source) = snapshot(uri, file, drafts)?;
        let Some(source) = source else { continue };
        let manifest = match serde_json::from_str::<StageSet<'_>>(json_source(&source)) {
            Ok(manifest) => manifest,
            Err(error) => {
                if file.kind == SavedDesignFile::Json && is_scenario(&source) {
                    scenarios.push((uri, file));
                    continue;
                }
                let saved = saved_for_draft(uri, file, version)?;
                let saved_stage = saved.as_deref().and_then(|text| {
                    serde_json::from_str::<StageSet<'_>>(json_source(text))
                        .ok()
                        .map(|stage| (text, stage))
                });
                if file.kind == SavedDesignFile::Stages || saved_stage.is_some() {
                    let references_move = match saved_stage {
                        Some((text, stage)) => !context.stage_patches(text, stage)?.0.is_empty(),
                        None => false,
                    };
                    if references_move || context.may_reference(&source) {
                        return Err(invalid(uri, error));
                    }
                } else if saved.as_deref().is_some_and(is_scenario) {
                    scenarios.push((uri, file));
                }
                continue;
            }
        };
        let (patches, keys) = context.stage_patches(&source, manifest)?;
        if !patches.is_empty() {
            stage_keys.insert(uri.as_str(), keys);
            documents.push(move_document(
                uri,
                version,
                &LineIndex::new(source.as_ref()),
                patches,
            )?);
        }
    }
    // Scenario keys are aliases for their selected stage set. Change a key
    // only when that set actually changes the identical authored spelling.
    for (uri, file) in scenarios {
        let (version, source) = snapshot(uri, file, drafts)?;
        let Some(source) = source else { continue };
        let base = file.path.parent().expect("an indexed file has a directory");
        let scenario = match serde_json::from_str::<Scenario<'_>>(json_source(&source)) {
            Ok(scenario) => scenario,
            Err(error) => {
                if context.may_reference(&source)
                    || saved_for_draft(uri, file, version)?.is_some_and(|saved| {
                        let Ok(case) = serde_json::from_str::<Scenario<'_>>(json_source(&saved))
                        else {
                            return false;
                        };
                        let keys = case.moved_stage_keys(base, &stage_keys);
                        match context.scenario_patches(uri, base, &saved, case, keys) {
                            Ok(patches) => !patches.is_empty(),
                            Err(_) => keys.is_some() || context.may_reference(&saved),
                        }
                    })
                {
                    return Err(invalid(uri, error));
                }
                continue;
            }
        };
        let keys = scenario.moved_stage_keys(base, &stage_keys);
        let patches = match context.scenario_patches(uri, base, &source, scenario, keys) {
            Ok(patches) => patches,
            Err(error) if keys.is_some() || context.may_reference(&source) => return Err(error),
            Err(_) => continue,
        };
        if !patches.is_empty() {
            documents.push(move_document(
                uri,
                version,
                &LineIndex::new(source.as_ref()),
                patches,
            )?);
        }
    }
    Ok(documents)
}

fn candidates(service: &LanguageService) -> Result<BTreeMap<String, Candidate<'_>>> {
    let mut files = BTreeMap::new();
    for workspace in service.workspaces.values() {
        let projects = &workspace.solution.projects;
        // A virtual Cargo root can own shared scenario and stage artifacts.
        let shared = if projects.iter().any(|p| p.root == workspace.solution.root) {
            Vec::new()
        } else {
            workspace_documents(&workspace.solution.root)?
        };
        for entry in projects
            .iter()
            .flat_map(|project| &project.documents)
            .chain(&shared)
        {
            if let Some(kind) = SavedDesignFile::for_path(&entry.path) {
                files.entry(entry.uri.clone()).or_insert_with(|| Candidate {
                    path: entry.path.clone(),
                    kind,
                    workspace,
                });
            }
        }
    }
    Ok(files)
}

fn snapshot<'a>(
    uri: &str,
    file: &Candidate<'_>,
    drafts: &BTreeMap<String, &'a FileMoveDraft>,
) -> Result<(i64, Option<std::borrow::Cow<'a, str>>)> {
    use std::borrow::Cow;
    if let Some(draft) = drafts.get(uri) {
        if draft.source.len() > SAVED_FILE_LIMIT {
            return Ok((draft.version, None));
        }
        return Ok((draft.version, Some(Cow::Borrowed(&draft.source))));
    }
    Ok((-1, saved_text(uri, file)?.map(Cow::Owned)))
}

fn saved_for_draft(uri: &str, file: &Candidate<'_>, version: i64) -> Result<Option<String>> {
    if version >= 0 {
        saved_text(uri, file)
    } else {
        Ok(None)
    }
}

fn saved_text(uri: &str, file: &Candidate<'_>) -> Result<Option<String>> {
    let path = file.workspace.resolve_path(uri)?;
    let mut text = String::new();
    let open = File::open(path).map_err(|error| LanguageError(error.to_string()))?;
    if open
        .metadata()
        .map_err(|error| LanguageError(error.to_string()))?
        .len()
        > SAVED_FILE_LIMIT as u64
    {
        return Ok(None);
    }
    match open
        .take((SAVED_FILE_LIMIT + 1) as u64)
        .read_to_string(&mut text)
    {
        Ok(size) if size <= SAVED_FILE_LIMIT => Ok(Some(text)),
        Ok(_) => Ok(None),
        Err(error) if file.kind == SavedDesignFile::Json => {
            if error.kind() == std::io::ErrorKind::InvalidData {
                return Ok(None);
            }
            Err(LanguageError(error.to_string()))
        }
        Err(error) => Err(LanguageError(error.to_string())),
    }
}

fn json_source(source: &str) -> &str {
    source.trim_start_matches('\u{feff}')
}

fn is_scenario(source: &str) -> bool {
    serde_json::from_str::<ferroui_designer_tools::scenario::Scenario>(json_source(source)).is_ok()
}

fn invalid(uri: &str, error: serde_json::Error) -> LanguageError {
    LanguageError(format!(
        "Repair the saved design that may reference this file: {uri}. {error}"
    ))
}
