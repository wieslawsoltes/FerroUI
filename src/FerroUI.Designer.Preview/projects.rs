use crate::protocol::{Asset, Source};
use ferroui_designer::{DesignerAsset, registered_designer_assets};
use ferroui_designer_language::Schema;
use ferroui_designer_project::{BuildMetadata, canonical_file_uri, file_path};
use ferroui_designer_rsx::RsxCatalog;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct Projects(BTreeMap<u64, Project>);

pub(crate) struct Project {
    root: PathBuf,
    pub(crate) schema: Schema,
    pub(crate) rsx: RsxCatalog,
    pub(crate) assets: Vec<DesignerAsset>,
    pub(crate) toolbox: Vec<Asset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Registration {
    id: u64,
    workspace_root: PathBuf,
    #[serde(flatten)]
    metadata: BuildMetadata,
}

impl Projects {
    pub(crate) fn register(&mut self, request: Registration) -> Result<Option<u64>, String> {
        let workspace = request
            .workspace_root
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let root = request
            .metadata
            .package_root()
            .map_err(|error| error.to_string())?;
        if !root.starts_with(&workspace) || request.id == 0 || self.0.contains_key(&request.id) {
            return Err(
                "Project metadata must belong to the open workspace and have a new identity".into(),
            );
        }
        let previous = self
            .0
            .iter()
            .find_map(|(id, project)| (project.root == root).then_some(*id));
        if previous.is_some_and(|id| id >= request.id) {
            return Err("A newer project catalog is already active".into());
        }
        let catalog = request
            .metadata
            .catalog()
            .map_err(|error| error.to_string())?;
        let assets = registered_designer_assets(&catalog, request.metadata.package.as_deref());
        let mut schema = Schema::ferroui();
        schema.register_project(&catalog);
        let rsx = RsxCatalog::from_project(&catalog, &schema, request.metadata.package.as_deref());
        let toolbox = assets
            .iter()
            .map(|asset| crate::toolbox::describe(asset, &rsx))
            .collect();
        if let Some(id) = previous {
            self.0.remove(&id);
        }
        self.0.insert(
            request.id,
            Project {
                root,
                schema,
                rsx,
                assets,
                toolbox,
            },
        );
        Ok(previous)
    }

    pub(crate) fn source(&self, source: &Source) -> Result<Option<&Project>, String> {
        self.document(source.project, &source.uri)
    }

    pub(crate) fn document(&self, id: Option<u64>, uri: &str) -> Result<Option<&Project>, String> {
        let Some(id) = id else { return Ok(None) };
        let project = self
            .0
            .get(&id)
            .ok_or("The project catalog changed. Render the current source again.")?;
        let path = canonical_file_uri(uri)
            .and_then(|uri| file_path(&uri))
            .map_err(|error| error.to_string())?;
        if !path.starts_with(&project.root) {
            return Err("The source is outside the registered Cargo package".into());
        }
        // An outer workspace package does not own an unopened nested Cargo project.
        let nearest = path
            .ancestors()
            .skip(1)
            .take_while(|root| *root != project.root)
            .any(|root| root.join("Cargo.toml").is_file());
        Ok((!nearest).then_some(project))
    }
}
