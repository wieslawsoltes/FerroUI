//! Workspace-local appearance for the main Studio preview.
use super::{decode, ensure_revision, schema as s};
use crate::{DesignerWorkspace, PreviewEnvironment, PreviewTheme};
use serde_json::{json, Value};

const READ: &str = "designer_preview_appearance";
const SET: &str = "designer_preview_set_theme";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, READ | SET)
}

pub(super) fn tools() -> [Value; 2] {
    [
        s::tool(
            READ,
            "Read the current Studio canvas environment, supported themes, and authored document identity.",
            s::object(json!({}), &[]),
            false,
        ),
        s::tool(
            SET,
            "Set the current Studio canvas theme without editing source. Pass the document URI and revision from designer_preview_appearance.",
            s::object(
                json!({"theme":{"enum":["light","dark","inherit","high_contrast"]},
                    "expected_uri":s::string(), "expected_revision":s::revision()}),
                &["theme", "expected_uri", "expected_revision"],
            ),
            true,
        ),
    ]
}

pub(super) fn call(
    workspace: &DesignerWorkspace,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let owner = workspace.session();
    if name == SET {
        {
            let session = owner.borrow();
            ensure_revision(&session, args)?;
            if args["expected_uri"].as_str() != Some(session.document_uri()) {
                return Err("Read the appearance of the current Studio document first".into());
            }
        }
        workspace.set_preview_environment(PreviewEnvironment {
            theme: decode(args, "theme")?,
            ..workspace.preview_environment()
        })?;
    }
    let session = owner.borrow();
    Ok(json!({
        "environment": workspace.preview_environment(), "available_themes": PreviewTheme::available(),
        "uri": session.document_uri(), "revision": session.document().revision(),
    }))
}
