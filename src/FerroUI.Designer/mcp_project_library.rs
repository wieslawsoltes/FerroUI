use super::{decode, s, state};
use crate::DesignerWorkspace;
use ferroui_designer_language::ProjectLibraryRequest;
use ferroui_designer_project::file_uri;
use serde_json::{Value, json};
use std::rc::Rc;

const LIST: &str = "ferroui_project_library";
const OPEN: &str = "designer_open_project_definition";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, LIST | OPEN)
}

pub(super) fn tools() -> [Value; 2] {
    let position = s::object(
        json!({"line": s::revision(), "character": s::revision()}),
        &["line", "character"],
    );
    let range = s::object(
        json!({"start": position, "end": position}),
        &["start", "end"],
    );
    let definition = s::object(
        json!({
            "kind": {"enum": ["UserControl", "CustomControl", "ControlTemplate", "DataTemplate", "ControlTheme", "Style"]},
            "name": s::string(), "range": range, "key": s::string(), "targetType": s::string(),
        }),
        &["kind", "name", "range"],
    );
    let entry = s::object(
        json!({"folder": s::string(), "path": s::string(), "definition": definition}),
        &["folder", "path", "definition"],
    );
    let mut open = s::tool(
        OPEN,
        "Open a current project Library entry in its source or scoped native preview. Pass an entry returned by ferroui_project_library.",
        s::object(
            json!({"entry": entry, "destination": {"enum": ["preview", "source"]}}),
            &["entry"],
        ),
        true,
    );
    open["annotations"]["destructiveHint"] = false.into();
    [
        s::tool(
            LIST,
            "Find reusable XAML components, styles, themes, and templates in the current project Library. Results reflect unsaved drafts.",
            s::object(
                json!({"folder": s::string(), "query": {"type":"string", "maxLength":4096}, "offset": s::revision(),
            "limit": {"type":"integer", "minimum":1, "maximum":200}}),
                &[],
            ),
            false,
        ),
        open,
    ]
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == OPEN {
        let uri = workspace.open_project_library_definition(
            decode(args, "entry")?,
            args["destination"] != "source",
        )?;
        return Ok(
            json!({"uri": uri, "range": args["entry"]["definition"]["range"], "document": state(&workspace.session().borrow())}),
        );
    }
    workspace.synchronize_language_service()?;
    let folder = if let Some(folder) = args["folder"].as_str() {
        folder.to_owned()
    } else {
        let solution = workspace
            .solution()
            .ok_or("Open a solution to browse its Library")?;
        file_uri(&solution.root).map_err(|error| error.to_string())?
    };
    let offset = usize::try_from(args["offset"].as_u64().unwrap_or_default())
        .map_err(|_| "The Library offset is too large")?;
    let request = ProjectLibraryRequest {
        folder,
        query: args["query"].as_str().unwrap_or_default().into(),
        offset,
        limit: args["limit"].as_u64().unwrap_or(100) as usize,
    };
    workspace
        .try_language_service()?
        .project_library(&request)
        .map(|page| json!(page))
        .map_err(|error| error.to_string())
}
