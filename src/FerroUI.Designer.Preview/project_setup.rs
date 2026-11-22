use ferroui_designer_project::{project_templates, toolchain};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateRequest {
    parent: PathBuf,
    request: project_templates::Request,
    review: Option<String>,
}

pub(super) fn template(method: &str, params: Value) -> Result<Value, String> {
    let input: TemplateRequest =
        serde_json::from_value(params).map_err(|error| error.to_string())?;
    if method == "projectTemplateCreate" {
        let review = input
            .review
            .as_deref()
            .ok_or("Review the project files before creating them")?;
        super::serialize(
            project_templates::create(&input.parent, &input.request, review)
                .map_err(|error| error.to_string()),
        )
    } else {
        super::serialize(
            project_templates::plan(&input.parent, &input.request)
                .map_err(|error| error.to_string()),
        )
    }
}

pub(super) fn toolchain(params: Value) -> Result<Value, String> {
    let request = serde_json::from_value(params).map_err(|error| error.to_string())?;
    super::serialize(toolchain::inspect(&request).map_err(|error| error.to_string()))
}
