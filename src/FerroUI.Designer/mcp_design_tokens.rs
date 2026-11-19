use super::schema as s;
use ferroui_designer_tools::token_handoff::Request;
use serde_json::{json, Value};

pub(super) const NAME: &str = "designer_design_tokens";

pub(super) fn tool() -> Value {
    s::tool(
        NAME,
        "Plan a DTCG JSON import to a XAML dictionary or export authored literal dictionary resources as DTCG JSON. Returns candidate source, token-to-resource mappings, and issues without changing the document. options.mappings is keyed by token path during import and resource key during export.",
        s::object(json!({
            "direction":{"type":"string","enum":["import","export"]}, "source":s::string(),
            "options":s::object(json!({"pixelsPerRem":s::number(), "mappings":s::dictionary(s::object(json!({
                "key":s::string(), "token":s::string(), "kind":s::string(), "xamlType":s::string()
            }), &[]))}), &[])
        }), &["direction", "source"]),
        false,
    )
}

pub(super) fn call(args: &Value) -> Result<Value, String> {
    let request: Request =
        serde_json::from_value(args.clone()).map_err(|error| error.to_string())?;
    let plan = request.plan().map_err(|error| error.to_string())?;
    serde_json::to_value(plan).map_err(|error| error.to_string())
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
        assert!(output["source"].as_str().unwrap().contains("<x:Double x:Key=\"CardSize\">16</x:Double>"));
        assert!(provider.call(NAME, &json!({"direction":"import","source":"{}","options":{"mappings":{"card":{"replace":true}}}}), Permissions::default()).is_err());
    }
}
