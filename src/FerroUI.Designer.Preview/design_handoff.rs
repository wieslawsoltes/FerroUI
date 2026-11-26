use crate::{projects::Projects, protocol::Source, toolbox};
use ferroui_designer::{design_handoff::ReviewContext, project_designer_assets};
use ferroui_designer_project::{
    canonical_file_uri,
    design_handoff::{fingerprint, Access, ManifestFile},
    file_path, PreviewSourceBuffer,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) fn tokens(params: Value) -> Result<Value, String> {
    let request: ferroui_designer_tools::token_handoff::Request =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    ferroui_controls::register_types();
    ferroui_controls_treedatagrid::register_types();
    let plan = ferroui_designer_animation::native::plan_design_tokens(
        request.direction, &request.source, &request.options,
    ).map_err(|error| error.to_string())?;
    serde_json::to_value(plan).map_err(|error| error.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Request {
    document: Source,
    workspace_root: PathBuf,
    manifest: PathBuf,
    #[serde(default)]
    selected_file: bool,
    revision: String,
    #[serde(default)]
    drafts: Vec<PreviewSourceBuffer>,
    #[serde(flatten)]
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Review,
    Check { id: String, review: String },
    Copy { id: String, review: String },
}

pub(crate) fn run(projects: &Projects, request: Request) -> Result<Value, String> {
    let access = if request.selected_file {
        Access::SelectedFile
    } else {
        Access::Workspace
    };
    let manifest = ManifestFile::read(&request.workspace_root, &request.manifest, access)
        .map_err(|error| error.to_string())?;
    let document = request.document;
    let project = projects.source(&document)?;
    let path = file_path(&document.uri).map_err(|error| error.to_string())?;
    let mut controls = toolbox::assets().to_vec();
    if let Some(project) = project {
        controls.extend_from_slice(&project.assets);
    }
    controls.extend(project_designer_assets(manifest.workspace(), &path)?);
    let drafts = request
        .drafts
        .into_iter()
        .filter_map(|draft| Some((canonical_file_uri(&draft.uri).ok()?, draft.source)))
        .collect::<BTreeMap<_, _>>();
    let draft = |uri: &str| match drafts.get(uri) {
        Some(Some(source)) => Ok(Some(source.clone())),
        Some(None) => Err("The mapped definition has an unavailable draft".into()),
        None => Ok(None),
    };
    let revision = fingerprint(&(&request.revision, document.project, &document.source));
    let rsx = (document.language == crate::protocol::Language::Rust).then(|| match project {
        Some(project) => &project.rsx,
        None => ferroui_designer_rsx::RsxCatalog::built_in(),
    });
    let context = ReviewContext {
        manifest: &manifest,
        document: &document.uri,
        revision: &revision,
        toolbox: &controls,
        rsx,
        packaged: project.map(|project| project.resources.packaged_assets().as_ref()),
        draft: &draft,
    };
    let result = match request.action {
        Action::Review => serde_json::to_value(context.review()?),
        Action::Check { id, review } => serde_json::to_value(context.checked(&id, &review)?.row),
        Action::Copy { id, review } => return Ok(json!({"uri": context.copy(&id, &review)?})),
    };
    result.map_err(|error| error.to_string())
}
