use super::schema as s;
use crate::DesignerWorkspace;
use ferroui_designer_project::{create_new_file, file_uri, ProjectWorkspace};
use ferroui_designer_tools::token_handoff::{Direction, Plan, Request};
use serde_json::{json, Value};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Component, Path},
};

pub(super) const NAME: &str = "designer_design_tokens";
pub(super) const CREATE: &str = "designer_create_design_token_file";

pub(super) fn tool() -> Value {
    s::tool(
        NAME,
        "Review colors, numbers, dimensions, durations, gradient brushes, layered shadows and key splines between DTCG and XAML. Returns generated source, mappings and issues. options.mappings is keyed by token path during import and resource key during export.",
        request_schema(),
        false,
    )
}

fn request_schema() -> Value {
    s::object(
        json!({
            "direction":{"type":"string","enum":["import","export"]}, "source":s::string(),
            "options":s::object(json!({"pixelsPerRem":s::number(), "mappings":s::dictionary(s::object(json!({
                "key":s::string(), "token":s::string(), "kind":s::string(), "xamlType":s::string()
            }), &[]))}), &[])
        }),
        &["direction", "source"],
    )
}

pub(super) fn create_tool() -> Value {
    let mut tool = s::tool(CREATE, "Create a reviewed design token file at a new path inside the open solution. Pass the conversion arguments and review ID returned by designer_design_tokens.",
        s::object(json!({"conversion":request_schema(),"review":s::string(),"destination":s::string()}), &["conversion", "review", "destination"]), true);
    tool["annotations"]["destructiveHint"] = false.into();
    tool
}

pub(super) fn call(args: &Value) -> Result<Value, String> {
    let (_, plan, review) = reviewed(args)?;
    let mut result = serde_json::to_value(plan).map_err(|error| error.to_string())?;
    result["review"] = review.into();
    Ok(result)
}

pub(super) fn create(workspace: &DesignerWorkspace, args: &Value) -> Result<Value, String> {
    let solution = workspace
        .solution()
        .ok_or("Open the solution for the new token file")?;
    let root = solution
        .root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let destination = Path::new(
        args["destination"]
            .as_str()
            .ok_or("Choose a new token file path")?,
    );
    if destination.as_os_str().is_empty()
        || !destination
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("Use a new path relative to the open solution".into());
    }
    let (direction, plan, review) = reviewed(&args["conversion"])?;
    if args["review"].as_str() != Some(&review) || plan.mappings.is_empty() {
        return Err("Review the current token conversion before creating its file".into());
    }
    let expected = match direction {
        Direction::Import => &["xaml"][..],
        Direction::Export => &["json", "tokens"],
    };
    if !destination.extension().is_some_and(|suffix| {
        expected
            .iter()
            .any(|extension| suffix.eq_ignore_ascii_case(extension))
    }) {
        return Err("Choose a file extension appropriate for the token conversion".into());
    }
    let uri = file_uri(root.join(destination)).map_err(|error| error.to_string())?;
    let path = ProjectWorkspace::new(solution)
        .resolve_path(&uri)
        .map_err(|error| error.to_string())?;
    if !path.starts_with(&root) {
        return Err("Keep the new token file inside the open solution".into());
    }
    create_new_file(&path, &plan.source).map_err(|error| error.to_string())?;
    Ok(json!({"uri":uri,"mappings":plan.mappings.len()}))
}

fn reviewed(args: &Value) -> Result<(Direction, Plan, String), String> {
    let request: Request =
        serde_json::from_value(args.clone()).map_err(|error| error.to_string())?;
    let plan = request.plan().map_err(|error| error.to_string())?;
    let mut key = DefaultHasher::new();
    (request.source.as_str(), plan.source.as_str()).hash(&mut key);
    Ok((request.direction, plan, format!("{:016x}", key.finish())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{DesignerMcp, Permissions};
    use ferroui_mcp::UiToolProvider;

    #[test]
    fn studio_serves_a_read_only_conversion_with_explicit_mapping() {
        let provider = DesignerMcp {
            session: std::rc::Weak::new(),
            workspace: None,
            spacing: Default::default(),
        };
        let args = json!({"direction":"import","source":r#"{"card":{"$type":"dimension","$value":{"value":16,"unit":"px"}}}"#,
            "options":{"mappings":{"card":{"key":"CardSize"}}}});
        let output = provider.call(NAME, &args, Permissions::default()).unwrap();
        assert_eq!(output["mappings"][0]["key"], "CardSize");
        assert!(output["source"]
            .as_str()
            .unwrap()
            .contains("<x:Double x:Key=\"CardSize\">16</x:Double>"));
        assert!(provider.call(NAME, &json!({"direction":"import","source":"{}","options":{"mappings":{"card":{"replace":true}}}}), Permissions::default()).is_err());
    }
}
