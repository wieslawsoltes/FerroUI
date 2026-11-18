use crate::renderer::Stage;
use ferroui_designer::mcp::DesignerMcp;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub(crate) struct Query {
    pub(crate) frame: u64,
    pub(crate) name: String,
    pub(crate) arguments: Value,
    pub(crate) selected: Option<u64>,
}

pub(crate) fn catalog() -> Vec<Value> {
    DesignerMcp::session_tool_definitions()
        .into_iter()
        .filter(|tool| tool["annotations"]["readOnlyHint"] == true)
        .filter(|tool| {
            !matches!(
                tool["name"].as_str(),
                Some("designer_document" | "designer_collaboration_state")
            )
        })
        .collect()
}

pub(crate) fn query(stage: &Stage, query: &Query) -> Result<Value, String> {
    let provider = DesignerMcp::new(&stage.session);
    if !catalog()
        .iter()
        .any(|definition| definition["name"] == query.name)
    {
        return Err(
            "This Designer Studio tool requires a Studio workspace or edits editor-owned source"
                .into(),
        );
    }
    stage.session.borrow_mut().select(query.selected);
    provider.validate(&query.name, &query.arguments)?;
    provider.call(&query.name, &query.arguments, Permissions::default())
}
