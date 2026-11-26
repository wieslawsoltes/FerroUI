use super::{decode, integer, s, state, text};
use crate::DesignerWorkspace;
use serde_json::{Value, json};
use std::rc::Rc;

const READ: &str = "designer_command_bindings";
const EDIT: &str = "designer_command_binding";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, READ | EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let path = json!({"type":"string","maxLength":4096});
    let choice = json!({"oneOf":[
        s::object(json!({"kind":{"enum":["keep", "clear"]}}), &["kind"]),
        s::object(json!({"kind":{"const":"bind"}, "path":path}), &["kind", "path"]),
    ]});
    let change = s::object(
        json!({"property": path, "command": choice, "parameter": choice}),
        &["property", "command", "parameter"],
    );
    let read = s::tool(
        READ,
        "Read command properties, current authored values, and typed XAML DataContext paths for a control. Pass prefix to browse a nested member.",
        s::object(
            json!({"node": s::id(), "prefix": path, "expected_revision": s::revision()}),
            &["node", "expected_revision"],
        ),
        false,
    );
    let mut edit = s::tool(
        EDIT,
        "Set, keep, or clear a simple typed XAML command binding and its parameter as one edit. Copy uri, version, and scopeEpoch from designer_command_bindings.",
        s::object(
            json!({"node": s::id(), "expected_revision": s::revision(), "expected_uri": path,
            "expected_version": s::revision(), "expected_scope_epoch": s::revision(), "change": change}),
            &[
                "node",
                "expected_revision",
                "expected_uri",
                "expected_version",
                "expected_scope_epoch",
                "change",
            ],
        ),
        true,
    );
    edit["annotations"]["destructiveHint"] = false.into();
    [read, edit]
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let site = workspace.command_site(super::super::node_id(args, "node")?)?;
    if name == READ {
        let mut model = json!(
            workspace.command_model(&site, args["prefix"].as_str().unwrap_or_default().into())?
        );
        model["uri"] = json!(site.uri);
        model["version"] = json!(site.request.version);
        model["scopeEpoch"] = json!(site.scope_epoch);
        return Ok(model);
    }
    if text(args, "expected_uri")? != site.uri
        || i64::try_from(integer(args, "expected_version")?).ok() != Some(site.request.version)
        || integer(args, "expected_scope_epoch")? != site.scope_epoch
    {
        return Err("The source or preview scope changed. Read the command binding again".into());
    }
    workspace.apply_command_binding(&site, &decode(args, "change")?)?;
    Ok(state(&workspace.session().borrow()))
}
