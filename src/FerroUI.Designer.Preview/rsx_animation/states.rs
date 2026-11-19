use crate::document::DesignDocument;
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::TextPatch;
use serde_json::Value;

pub(crate) fn state_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_state" && matches!(action, Some("upsert" | "remove"))
}

pub(crate) fn state_patches(
    design: &DesignDocument<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    let rsx = design.rsx.as_ref().ok_or("Open the current Rust view")?;
    let owner = |id| {
        design
            .rsx_node(id)
            .map(|(ordinal, _)| ordinal)
            .ok_or("Choose a mapped Rust control")
    };
    if args["action"] == "upsert" {
        let model: VisualStateGroup =
            serde_json::from_value(args["model"].clone()).map_err(|error| error.to_string())?;
        return rsx.upsert_state_group(owner(model.owner)?, &model);
    }
    let id = args["owner"].as_u64().ok_or("Choose a state group owner")?;
    let name = args["group"]
        .as_str()
        .ok_or("Choose an authored state group")?;
    rsx.remove_state_group(owner(id)?, name)
}
