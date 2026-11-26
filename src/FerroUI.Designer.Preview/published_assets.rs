use crate::projects::Projects;
use ferroui_designer_project::{file_uri, is_raster_asset};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub(crate) struct Query {
    uri: String,
    project: u64,
    address: Option<String>,
}

pub(crate) fn query(projects: &Projects, query: Query) -> Result<Value, String> {
    let project = projects
        .document(Some(query.project), &query.uri)?
        .ok_or("Build the owning Cargo package before browsing its published assets")?;
    let catalog = project.resources.packaged_assets();
    if let Some(address) = query.address {
        if address.len() > 4096 {
            return Err("The published asset address is too long".into());
        }
        let source = catalog
            .resolve(&address)
            .map_err(|error| error.to_string())?
            .ok_or("The asset is unavailable in the current Cargo export table")?;
        let raster = is_raster_asset(&source.file);
        let uri = file_uri(source.file).map_err(|error| error.to_string())?;
        return Ok(json!({ "uri": uri, "raster": raster }));
    }
    serde_json::to_value(catalog.published().collect::<Vec<_>>()).map_err(|error| error.to_string())
}
