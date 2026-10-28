//! Designer commands hosted by FerroUI's existing MCP transport and UI dispatcher.
//! No transport starts until the application explicitly calls `serve_mcp`.
use crate::{DesignerSession, DesignerWorkspace, Edit, PreviewStatus};
use ferroui_designer_animation::{AnimationValue, Easing};
use ferroui_designer_collaboration::{Presence, Snapshot, SyncMessage, VersionVector};
use ferroui_inspect::TreeKind;
use ferroui_mcp::UiToolProvider;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[path = "mcp_authoring.rs"]
mod authoring;
#[cfg(test)]
#[path = "mcp_automation_tests.rs"]
mod automation_tests;
#[path = "mcp_components.rs"]
mod components;
#[cfg(test)]
#[path = "mcp_components_tests.rs"]
mod components_tests;
#[path = "mcp_design_workflows.rs"]
mod design_workflows;
#[cfg(test)]
#[path = "mcp_design_workflows_tests.rs"]
mod design_workflows_tests;
#[path = "mcp_schema.rs"]
mod schema;
#[path = "mcp_selection_workflows.rs"]
mod selection_workflows;
#[path = "mcp_workspace.rs"]
mod workspace_tools;

pub use ferroui_mcp::{
    Options, Permissions, Transport, UiServerHandle as ServerHandle, reserve_stdout,
};

/// A server-local, UI-thread-owned designer tool provider. It does not keep a closed workspace
/// alive. Read-only inspector tools remain available beside these document authoring tools.
pub struct DesignerMcp {
    session: Weak<RefCell<DesignerSession>>,
    workspace: Option<Weak<DesignerWorkspace>>,
}
impl DesignerMcp {
    pub fn new(session: &Rc<RefCell<DesignerSession>>) -> Self {
        Self {
            session: Rc::downgrade(session),
            workspace: None,
        }
    }
    /// Bind project/editor tools without keeping the workspace or its session alive.
    pub fn with_workspace(workspace: &Rc<DesignerWorkspace>) -> Self {
        Self {
            session: Rc::downgrade(&workspace.session()),
            workspace: Some(Rc::downgrade(workspace)),
        }
    }
    fn session(&self) -> Result<Rc<RefCell<DesignerSession>>, String> {
        self.session
            .upgrade()
            .ok_or_else(|| "Designer session has been closed".into())
    }
}
impl DesignerWorkspace {
    /// Retain the handle for as long as tools should be reachable. Mutating tools require the
    /// existing `allow_property_writes` permission; HTTP remains authenticated and loopback-only.
    pub fn serve_mcp(&self, options: Options) -> std::io::Result<ServerHandle> {
        ferroui_mcp::serve_with_tools(options, Rc::new(DesignerMcp::new(&self.session())))
    }
    /// Host all session tools plus project lifecycle and shared language-service operations.
    pub fn serve_workspace_mcp(self: &Rc<Self>, options: Options) -> std::io::Result<ServerHandle> {
        ferroui_mcp::serve_with_tools(options, Rc::new(DesignerMcp::with_workspace(self)))
    }
}

fn definition(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    write: bool,
) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":!write,"destructiveHint":write,"openWorldHint":false}})
}
fn text(args: &Value, key: &str) -> Result<String, String> {
    args[key]
        .as_str()
        .filter(|value| value.len() <= 1_048_576)
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} must be a string no larger than 1 MiB"))
}
fn integer(args: &Value, key: &str) -> Result<u64, String> {
    args[key]
        .as_u64()
        .ok_or_else(|| format!("{key} must be a nonnegative integer"))
}
fn node_id(args: &Value, key: &str) -> Result<u64, String> {
    let id = integer(args, key)?;
    if id == 0 {
        return Err(format!("{key} must be a positive node id"));
    }
    Ok(id)
}
fn number(args: &Value, key: &str) -> Result<f64, String> {
    args[key]
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{key} must be finite"))
}
fn decode<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args[key].clone()).map_err(|error| format!("Invalid {key}: {error}"))
}
fn action_arguments(args: &Value, action: &str, allowed: &[&str]) -> Result<(), String> {
    for key in args
        .as_object()
        .ok_or("arguments must be an object")?
        .keys()
    {
        if key != "expected_revision" && key != "action" && !allowed.contains(&key.as_str()) {
            return Err(format!(
                "Argument {key} is not supported for action {action}"
            ));
        }
    }
    Ok(())
}
fn ensure_revision(session: &DesignerSession, args: &Value) -> Result<(), String> {
    let expected = integer(args, "expected_revision")?;
    let actual = session.document().revision();
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "Document revision changed: expected {expected}, actual {actual}"
        ))
    }
}
fn state(session: &DesignerSession) -> Value {
    let preview = match session.preview_status() {
        PreviewStatus::Ready { revision } => json!({"state":"ready","revision":revision}),
        PreviewStatus::InvalidSource { revision } => {
            json!({"state":"invalid_source","revision":revision})
        }
        PreviewStatus::Failed { revision, message } => {
            json!({"state":"failed","revision":revision,"message":message})
        }
    };
    json!({"revision":session.document().revision(),"valid":session.document().is_valid(),"dirty":session.document().is_dirty(),"selection":session.selection(),"can_undo":session.can_undo(),"can_redo":session.can_redo(),"preview":preview,"diagnostics":session.document().diagnostics(),"collaboration_error":session.collaboration_error()})
}

impl UiToolProvider for DesignerMcp {
    fn tools(&self) -> Vec<Value> {
        let rev = json!({"type":"integer","minimum":0});
        let id = json!({"type":"integer","minimum":1});
        let string = json!({"type":"string","maxLength":1048576});
        let edits = json!({"type":"array","minItems":1,"maxItems":4096,"items":{"type":"object"}});
        let mut tools = vec![
            definition(
                "designer_document",
                "Read authored XAML, revision, diagnostics, selection and live-preview acceptance. Invalid authored XAML remains available.",
                json!({"include_source":{"type":"boolean"}}),
                &[],
                false,
            ),
            definition(
                "designer_ast",
                "Read source-preserving AST nodes and UTF-8 spans. AST refers to last valid source when a draft is invalid.",
                json!({"node":id,"depth":{"type":"integer","minimum":0,"maximum":128}}),
                &[],
                false,
            ),
            definition(
                "designer_apply",
                "Apply one atomic, undoable source transaction. Edits use externally tagged Rust enum JSON: {\"SetAttribute\":{\"node\":1,\"name\":\"Width\",\"value\":\"100\"}}; also RemoveAttribute, InsertChild, RemoveNode, MoveNode, ReplaceSource and Text. Node ids are document-local.",
                json!({"expected_revision":rev,"label":string,"edits":edits}),
                &["expected_revision", "label", "edits"],
                true,
            ),
            definition(
                "designer_source",
                "Replace authored source with a revision guard; invalid XAML is retained for repair and never replaces the last good preview.",
                json!({"expected_revision":rev,"source":string}),
                &["expected_revision", "source"],
                true,
            ),
            definition(
                "designer_select",
                "Select source AST elements in the workspace.",
                json!({"expected_revision":rev,"nodes":{"type":"array","items":id,"maxItems":4096}}),
                &["expected_revision", "nodes"],
                true,
            ),
            definition(
                "designer_undo",
                "Undo one local authoring transaction. Shared sessions refuse to overwrite a later remote writer.",
                json!({"expected_revision":rev}),
                &["expected_revision"],
                true,
            ),
            definition(
                "designer_redo",
                "Redo one authoring transaction through the same preview and collaboration boundary.",
                json!({"expected_revision":rev}),
                &["expected_revision"],
                true,
            ),
            definition(
                "designer_tree",
                "Read the live preview visual or logical tree with inspector IDs, source AST IDs and artboard bounds.",
                json!({"tree":{"enum":["visual","logical"]}}),
                &[],
                false,
            ),
            definition(
                "designer_timeline",
                "Read typed keyframe tracks, playback state and import diagnostics.",
                json!({}),
                &[],
                false,
            ),
            definition(
                "designer_animation",
                "Control timeline playback, duration, recording and keyframes. Times are seconds. seek/duration use time; recording uses enabled; record uses target/property/value at the current playhead (seek first); move uses target/property/time/to; remove uses target/property/time; easing uses target/property/time/easing. play/pause/stop take no action arguments. Values are tagged, e.g. {\"kind\":\"Number\",\"value\":120}. Seek changes preview only. Keyframe edits serialize native FerroUI animation XAML.",
                json!({"expected_revision":rev,"action":{"enum":["seek","play","pause","stop","recording","duration","record","move","remove","easing"]},"time":{"type":"number"},"to":{"type":"number"},"target":id,"property":string,"enabled":{"type":"boolean"},"value":{"type":"object"},"easing":{"type":"object"}}),
                &["expected_revision", "action"],
                true,
            ),
            definition(
                "designer_collaboration_state",
                "Read collaboration version, conflicts, presence, optional snapshot and changes since a version vector. This does not consume outgoing operations.",
                json!({"snapshot":{"type":"boolean"},"since":{"type":"object"}}),
                &[],
                false,
            ),
            definition(
                "designer_collaboration",
                "Start or join a collaboration session, merge an authenticated peer envelope, or explicitly discard/resolve a conflicting local draft. The host is responsible for peer identity and authorization.",
                json!({"expected_revision":rev,"action":{"enum":["start","join","merge","discard_draft","resolve_draft"]},"actor":string,"document_id":string,"snapshot":{"type":"object"},"message":{"type":"object"},"version":{"type":"object"},"source":string}),
                &["expected_revision", "action"],
                true,
            ),
            definition(
                "designer_presence",
                "Receive ephemeral peer presence, using the receiver's monotonic expiry clock. This changes no authored source.",
                json!({"presence":{"type":"object"}}),
                &["presence"],
                true,
            ),
        ];
        tools.extend(authoring::tools());
        tools.extend(components::tools());
        tools.extend(design_workflows::tools());
        tools.extend(selection_workflows::tools());
        if self.workspace.is_some() {
            tools.extend(workspace_tools::tools());
        }
        tools
    }

    fn validate(&self, name: &str, args: &Value) -> Result<(), String> {
        let object = args.as_object().ok_or("arguments must be an object")?;
        let definition = self
            .tools()
            .into_iter()
            .find(|tool| tool["name"] == name)
            .ok_or("Unknown designer tool")?;
        if authoring::handles(name)
            || components::handles(name)
            || workspace_tools::handles(name)
            || design_workflows::handles(name)
            || selection_workflows::handles(name)
        {
            return schema::validate(&definition["inputSchema"], args);
        }
        for key in object.keys() {
            if definition["inputSchema"]["properties"].get(key).is_none() {
                return Err(format!("Unknown argument: {key}"));
            }
        }
        for key in definition["inputSchema"]["required"].as_array().unwrap() {
            let key = key.as_str().unwrap();
            if !object.contains_key(key) {
                return Err(format!("Missing argument: {key}"));
            }
        }
        if object.contains_key("expected_revision") {
            integer(args, "expected_revision")?;
        }
        for key in ["include_source", "snapshot"] {
            if name != "designer_collaboration"
                && object.contains_key(key)
                && !args[key].is_boolean()
            {
                return Err(format!("{key} must be boolean"));
            }
        }
        match name {
            "designer_ast" => {
                if object.contains_key("node") {
                    node_id(args, "node")?;
                }
                if object.contains_key("depth") && integer(args, "depth")? > 128 {
                    return Err("depth exceeds 128".into());
                }
            }
            "designer_apply" => {
                text(args, "label")?;
                if args["edits"].to_string().len() > 1_048_576 {
                    return Err("Transaction exceeds 1 MiB".into());
                }
                let edits: Vec<Edit> = decode(args, "edits")?;
                if edits.is_empty() || edits.len() > 4096 {
                    return Err("Use 1..4096 edits".into());
                }
            }
            "designer_source" => {
                text(args, "source")?;
            }
            "designer_select" => {
                let ids: Vec<u64> = decode(args, "nodes")?;
                if ids.len() > 4096 || ids.contains(&0) {
                    return Err("Invalid selection".into());
                }
            }
            "designer_tree" => {
                if object.contains_key("tree")
                    && !matches!(args["tree"].as_str(), Some("visual" | "logical"))
                {
                    return Err("tree must be visual or logical".into());
                }
            }
            "designer_animation" => match text(args, "action")?.as_str() {
                action @ ("seek" | "duration") => {
                    action_arguments(args, action, &["time"])?;
                    number(args, "time")?;
                }
                action @ ("play" | "pause" | "stop") => {
                    action_arguments(args, action, &[])?;
                }
                "recording" => {
                    action_arguments(args, "recording", &["enabled"])?;
                    if !args["enabled"].is_boolean() {
                        return Err("enabled must be boolean".into());
                    }
                }
                action @ ("record" | "move" | "remove" | "easing") => {
                    let allowed: &[&str] = match action {
                        "record" => &["target", "property", "value"],
                        "move" => &["target", "property", "time", "to"],
                        "remove" => &["target", "property", "time"],
                        "easing" => &["target", "property", "time", "easing"],
                        _ => unreachable!(),
                    };
                    action_arguments(args, action, allowed)?;
                    node_id(args, "target")?;
                    text(args, "property")?;
                    match action {
                        "record" => {
                            decode::<AnimationValue>(args, "value")?
                                .validate()
                                .map_err(|e| e.to_string())?;
                        }
                        "move" => {
                            number(args, "time")?;
                            number(args, "to")?;
                        }
                        "remove" => {
                            number(args, "time")?;
                        }
                        "easing" => {
                            number(args, "time")?;
                            decode::<Easing>(args, "easing")?
                                .validate()
                                .map_err(|e| e.to_string())?;
                        }
                        _ => unreachable!(),
                    }
                }
                _ => return Err("Unknown animation action".into()),
            },
            "designer_collaboration_state" => {
                if object.contains_key("since") {
                    let _: VersionVector = decode(args, "since")?;
                }
            }
            "designer_collaboration" => match text(args, "action")?.as_str() {
                "start" => {
                    action_arguments(args, "start", &["actor", "document_id"])?;
                    text(args, "actor")?;
                    text(args, "document_id")?;
                }
                "join" => {
                    action_arguments(args, "join", &["actor", "snapshot"])?;
                    text(args, "actor")?;
                    let _: Snapshot = decode(args, "snapshot")?;
                }
                "merge" => {
                    action_arguments(args, "merge", &["message"])?;
                    let _: SyncMessage = decode(args, "message")?;
                }
                "discard_draft" => {
                    action_arguments(args, "discard_draft", &[])?;
                }
                "resolve_draft" => {
                    action_arguments(args, "resolve_draft", &["source", "version"])?;
                    text(args, "source")?;
                    let _: VersionVector = decode(args, "version")?;
                }
                _ => return Err("Unknown collaboration action".into()),
            },
            "designer_presence" => {
                let _: Presence = decode(args, "presence")?;
            }
            _ => {}
        }
        Ok(())
    }

    fn call(&self, name: &str, args: &Value, permissions: Permissions) -> Result<Value, String> {
        self.validate(name, args)?;
        let definition = self
            .tools()
            .into_iter()
            .find(|tool| tool["name"] == name)
            .unwrap();
        if definition["annotations"]["readOnlyHint"] != true && !permissions.allow_property_writes {
            return Err("Designer authoring requires allow_property_writes".into());
        }
        let owner = self.session()?;
        if selection_workflows::handles(name)
            || (name == "designer_layer" && args["action"] == "select_matching_type")
        {
            if let Some(workspace) = &self.workspace {
                workspace
                    .upgrade()
                    .ok_or("Designer workspace has been closed")?
                    .ensure_selection_workflow_context()?;
            }
        }
        if workspace_tools::handles(name) {
            let workspace = self
                .workspace
                .as_ref()
                .and_then(Weak::upgrade)
                .ok_or("Designer workspace has been closed")?;
            {
                let session = owner
                    .try_borrow_mut()
                    .map_err(|_| "Designer is already processing a transaction")?;
                if args.get("expected_revision").is_some() {
                    ensure_revision(&session, args)?;
                }
                if definition["annotations"]["readOnlyHint"] != true {
                    session
                        .ensure_writable()
                        .map_err(|error| error.to_string())?;
                }
            }
            drop(workspace.try_language_service()?);
            if name == "designer_project_task"
                && args["action"] != "cancel"
                && !permissions.allow_input
            {
                return Err("Starting project processes requires allow_input".into());
            }
            return workspace_tools::call(&workspace, name, args);
        }
        let mut session = owner
            .try_borrow_mut()
            .map_err(|_| "Designer is already processing a transaction")?;
        if args.get("expected_revision").is_some() {
            ensure_revision(&session, args)?;
        }
        if authoring::handles(name) {
            return authoring::call(&mut session, name, args, permissions);
        }
        if components::handles(name) {
            return components::call(&mut session, name, args);
        }
        if design_workflows::handles(name) {
            return design_workflows::call(&mut session, name, args);
        }
        if selection_workflows::handles(name) {
            return selection_workflows::call(&mut session, name, args);
        }
        match name {
            "designer_document" => {
                let mut result = state(&session);
                if args["include_source"] != false {
                    result["source"] = json!(session.document().source());
                }
                Ok(result)
            }
            "designer_ast" => {
                let ast = session.document().ast();
                let root = args["node"].as_u64().unwrap_or(ast.root());
                let depth = args["depth"].as_u64().unwrap_or(64) as usize;
                let mut pending = vec![(root, 0usize)];
                let mut nodes = Vec::new();
                let mut truncated = false;
                while let Some((id, level)) = pending.pop() {
                    let node = ast.node(id).ok_or("Unknown AST node")?;
                    nodes.push(node);
                    if level < depth {
                        pending.extend(node.children.iter().rev().map(|id| (*id, level + 1)));
                    } else {
                        truncated |= !node.children.is_empty();
                    }
                    if nodes.len() >= 10000 {
                        truncated |= !pending.is_empty();
                        break;
                    }
                }
                Ok(
                    json!({"revision":session.document().revision(),"authored_valid":session.document().is_valid(),"nodes":nodes,"truncated":truncated}),
                )
            }
            "designer_apply" => {
                let change = session
                    .apply_at(
                        integer(args, "expected_revision")?,
                        text(args, "label")?,
                        decode(args, "edits")?,
                    )
                    .map_err(|e| e.to_string())?;
                Ok(json!({"change":change,"document":state(&session)}))
            }
            "designer_source" => {
                let result = session.update_source(text(args, "source")?);
                Ok(json!({"change":result.change,"document":state(&session)}))
            }
            "designer_select" => {
                let ids: Vec<u64> = decode(args, "nodes")?;
                if ids.iter().any(|id| {
                    !session
                        .document()
                        .ast()
                        .node(*id)
                        .is_some_and(|node| node.is_element())
                }) {
                    return Err("Selection contains an unknown element".into());
                }
                session.select(ids);
                Ok(state(&session))
            }
            "designer_undo" => {
                session.undo().map_err(|e| e.to_string())?;
                Ok(state(&session))
            }
            "designer_redo" => {
                session.redo().map_err(|e| e.to_string())?;
                Ok(state(&session))
            }
            "designer_tree" => {
                let kind = if args["tree"] == "logical" {
                    TreeKind::Logical
                } else {
                    TreeKind::Visual
                };
                let rows: Vec<Value> = session.runtime_tree(kind).iter().map(|row| json!({"inspector_id":row.inspector_id,"source_id":row.source_id,"parent":row.parent,"depth":row.depth,"type":row.type_name,"name":row.name,"bounds":row.bounds.map(|b| json!({"x":b.x,"y":b.y,"width":b.width,"height":b.height}))})).collect();
                Ok(json!({"document":state(&session),"nodes":rows}))
            }
            "designer_timeline" => Ok(
                json!({"timeline":session.timeline(),"time":session.time(),"playing":session.is_playing(),"recording":session.recording(),"warnings":session.animation_warnings()}),
            ),
            "designer_animation" => {
                match text(args, "action")?.as_str() {
                    "seek" => session.seek(number(args, "time")?)?,
                    "duration" => session.set_duration(number(args, "time")?)?,
                    "play" => session.play(),
                    "pause" => session.pause(),
                    "stop" => session.stop(),
                    "recording" => session.set_recording(args["enabled"].as_bool().unwrap()),
                    "record" => session.record_keyframe(
                        integer(args, "target")?,
                        &text(args, "property")?,
                        decode(args, "value")?,
                    )?,
                    "move" => session.move_keyframe(
                        integer(args, "target")?,
                        &text(args, "property")?,
                        number(args, "time")?,
                        number(args, "to")?,
                    )?,
                    "remove" => session.remove_keyframe(
                        integer(args, "target")?,
                        &text(args, "property")?,
                        number(args, "time")?,
                    )?,
                    "easing" => session.set_keyframe_easing(
                        integer(args, "target")?,
                        &text(args, "property")?,
                        number(args, "time")?,
                        decode(args, "easing")?,
                    )?,
                    _ => unreachable!(),
                }
                Ok(
                    json!({"document":state(&session),"time":session.time(),"playing":session.is_playing()}),
                )
            }
            "designer_collaboration_state" => {
                let mut result = json!({"version":session.collaboration_version(),"conflicts":session.collaboration_conflicts(),"error":session.collaboration_error(),"presence":session.current_presence(30000)});
                if args["snapshot"] == true {
                    result["snapshot"] = json!(session.collaboration_snapshot()?);
                }
                if args.get("since").is_some() {
                    result["changes"] =
                        json!(session.collaboration_changes_since(&decode(args, "since")?)?);
                }
                Ok(result)
            }
            "designer_collaboration" => {
                let report = match text(args, "action")?.as_str() {
                    "start" => {
                        session.enable_collaboration(
                            text(args, "actor")?,
                            text(args, "document_id")?,
                        )?;
                        Value::Null
                    }
                    "join" => {
                        session
                            .join_collaboration(text(args, "actor")?, decode(args, "snapshot")?)?;
                        Value::Null
                    }
                    "merge" => json!(session.merge_collaboration(decode(args, "message")?)?),
                    "discard_draft" => json!(session.discard_collaboration_draft()?),
                    "resolve_draft" => json!(session.resolve_collaboration_draft(
                        &decode(args, "version")?,
                        text(args, "source")?
                    )?),
                    _ => unreachable!(),
                };
                Ok(
                    json!({"document":state(&session),"version":session.collaboration_version(),"report":report}),
                )
            }
            "designer_presence" => {
                Ok(json!({"changed":session.receive_presence_now(decode(args,"presence")?)?}))
            }
            _ => Err("Unknown designer tool".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::ContentControl;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};

    fn source() -> &'static str {
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" Height="40" /></Canvas>"#
    }
    fn fixture() -> (Rc<RefCell<DesignerSession>>, DesignerMcp) {
        let session = Rc::new(RefCell::new(
            DesignerSession::new(source(), ContentControl::new()).unwrap(),
        ));
        let provider = DesignerMcp::new(&session);
        (session, provider)
    }
    fn write() -> Permissions {
        Permissions {
            allow_property_writes: true,
            allow_input: false,
        }
    }

    #[test]
    fn automation_uses_revision_guards_atomic_edits_history_and_preview_retention() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (session, provider) = fixture();
        let target = session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        let edit = json!({"expected_revision":0,"label":"Resize from agent","edits":[{"SetAttribute":{"node":target,"name":"Width","value":"180"}}]});
        assert!(
            provider
                .call("designer_apply", &edit, Permissions::default())
                .is_err()
        );
        provider.call("designer_apply", &edit, write()).unwrap();
        assert_eq!(
            session
                .borrow()
                .document()
                .ast()
                .node(target)
                .unwrap()
                .attribute("Width"),
            Some("180")
        );
        assert!(
            provider
                .call("designer_apply", &edit, write())
                .unwrap_err()
                .contains("revision")
        );
        let revision = session.borrow().document().revision();
        let failure = json!({"expected_revision":revision,"label":"Invalid batch","edits":[{"SetAttribute":{"node":target,"name":"Width","value":"999"}},{"RemoveNode":{"node":u64::MAX}}]});
        assert!(provider.call("designer_apply", &failure, write()).is_err());
        assert_eq!(session.borrow().document().revision(), revision);
        assert_eq!(
            session
                .borrow()
                .document()
                .ast()
                .node(target)
                .unwrap()
                .attribute("Width"),
            Some("180")
        );
        provider
            .call(
                "designer_undo",
                &json!({"expected_revision":revision}),
                write(),
            )
            .unwrap();
        assert_eq!(
            session
                .borrow()
                .document()
                .ast()
                .node(target)
                .unwrap()
                .attribute("Width"),
            Some("100")
        );
        let revision = session.borrow().document().revision();
        let result = provider
            .call(
                "designer_source",
                &json!({"expected_revision":revision,"source":"<Canvas"}),
                write(),
            )
            .unwrap();
        assert_eq!(result["document"]["valid"], false);
        assert_eq!(result["document"]["preview"]["state"], "invalid_source");
        assert_eq!(
            provider
                .call("designer_document", &json!({}), Permissions::default())
                .unwrap()["source"],
            "<Canvas"
        );
        assert!(
            provider
                .call("designer_ast", &json!({}), Permissions::default())
                .unwrap()["nodes"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }

    #[test]
    fn collaboration_tools_merge_real_transactions_and_reject_other_baselines() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (a, pa) = fixture();
        let (b, pb) = fixture();
        for (actor, p) in [("alice", &pa), ("bob", &pb)] {
            p.call("designer_collaboration",&json!({"expected_revision":0,"action":"start","actor":actor,"document_id":"test-doc"}),write()).unwrap();
        }
        let target = a.borrow().document().ast().find_name("Target").unwrap();
        pa.call("designer_apply",&json!({"expected_revision":0,"label":"Resize","edits":[{"SetAttribute":{"node":target,"name":"Width","value":"225"}}]}),write()).unwrap();
        let changes = pa
            .call(
                "designer_collaboration_state",
                &json!({"since":{}}),
                Permissions::default(),
            )
            .unwrap()["changes"]
            .clone();
        pb.call(
            "designer_collaboration",
            &json!({"expected_revision":0,"action":"merge","message":changes}),
            write(),
        )
        .unwrap();
        assert_eq!(
            a.borrow().document().source(),
            b.borrow().document().source()
        );
        let mut wrong = changes;
        wrong["baseline_hash"] = json!("another-document");
        let revision = b.borrow().document().revision();
        assert!(
            pb.call(
                "designer_collaboration",
                &json!({"expected_revision":revision,"action":"merge","message":wrong}),
                write()
            )
            .is_err()
        );
        assert_eq!(b.borrow().document().revision(), revision);
    }

    #[test]
    fn designer_schema_checks_unsupported_commands_before_side_effects() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (session, provider) = fixture();
        assert!(
            provider
                .call(
                    "designer_animation",
                    &json!({"expected_revision":0,"action":"seek","time":"bad"}),
                    write()
                )
                .is_err()
        );
        assert!(
            provider
                .call(
                    "designer_animation",
                    &json!({"expected_revision":0,"action":"delete_everything"}),
                    write()
                )
                .is_err()
        );
        assert!(
            provider
                .call(
                    "designer_select",
                    &json!({"expected_revision":0,"nodes":[99999]}),
                    write()
                )
                .is_err()
        );
        assert_eq!(session.borrow().document().revision(), 0);
        assert!(session.borrow().selection().is_empty());
        drop(session);
        assert!(
            provider
                .call("designer_document", &json!({}), Permissions::default())
                .unwrap_err()
                .contains("closed")
        );
    }

    #[test]
    fn ast_reports_pruned_descendants_but_complete_subtrees_are_not_truncated() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (session, provider) = fixture();
        let shallow = provider
            .call("designer_ast", &json!({"depth":0}), Permissions::default())
            .unwrap();
        assert_eq!(shallow["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(shallow["truncated"], true);
        let complete = provider
            .call("designer_ast", &json!({"depth":1}), Permissions::default())
            .unwrap();
        assert_eq!(complete["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(complete["truncated"], false);
        let target = session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        let leaf = provider
            .call(
                "designer_ast",
                &json!({"node":target,"depth":0}),
                Permissions::default(),
            )
            .unwrap();
        assert_eq!(leaf["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(leaf["truncated"], false);
        assert!(
            provider
                .validate("designer_ast", &json!({"node":0}))
                .is_err()
        );
    }

    #[test]
    fn action_specific_arguments_never_silently_change_a_different_state() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (session, provider) = fixture();
        let target = session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        let ignored_time = json!({"expected_revision":0,"action":"record","target":target,"property":"Width","value":{"kind":"Number","value":180},"time":0.25});
        assert!(
            provider
                .call("designer_animation", &ignored_time, write())
                .unwrap_err()
                .contains("time")
        );
        assert!(
            provider
                .call(
                    "designer_animation",
                    &json!({"expected_revision":0,"action":"play","enabled":false}),
                    write()
                )
                .is_err()
        );
        assert!(
            provider
                .validate(
                    "designer_collaboration",
                    &json!({"expected_revision":0,"action":"discard_draft","source":"replacement"})
                )
                .is_err()
        );
        assert!(provider.validate("designer_animation", &json!({"expected_revision":0,"action":"remove","target":0,"property":"Width","time":0})).is_err());
        assert_eq!(session.borrow().document().revision(), 0);
        assert!(session.borrow().timeline().tracks.is_empty());
        assert!(!session.borrow().is_playing());

        provider
            .call(
                "designer_animation",
                &json!({"expected_revision":0,"action":"seek","time":0.25}),
                write(),
            )
            .unwrap();
        provider.call("designer_animation", &json!({"expected_revision":0,"action":"record","target":target,"property":"Width","value":{"kind":"Number","value":180}}), write()).unwrap();
        assert!(
            session
                .borrow()
                .timeline()
                .track(target, "Width")
                .unwrap()
                .keyframes
                .iter()
                .any(|key| key.time == 0.25 && key.value == AnimationValue::Number(180.0))
        );
        assert!(
            provider
                .call(
                    "designer_animation",
                    &json!({"expected_revision":0,"action":"play"}),
                    write()
                )
                .unwrap_err()
                .contains("revision")
        );
        assert!(!session.borrow().is_playing());
    }

    #[test]
    fn readonly_access_and_reentrant_calls_do_not_mutate_session_state() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (session, provider) = fixture();
        for (name, args) in [
            ("designer_select", json!({"expected_revision":0,"nodes":[]})),
            (
                "designer_animation",
                json!({"expected_revision":0,"action":"play"}),
            ),
            (
                "designer_collaboration",
                json!({"expected_revision":0,"action":"start","actor":"alice","document_id":"read-only"}),
            ),
        ] {
            assert!(
                provider
                    .call(name, &args, Permissions::default())
                    .unwrap_err()
                    .contains("allow_property_writes")
            );
        }
        assert_eq!(session.borrow().document().revision(), 0);
        assert!(!session.borrow().is_playing());
        assert!(session.borrow().collaboration_version().is_none());
        let borrowed = session.borrow_mut();
        assert!(
            provider
                .call("designer_document", &json!({}), Permissions::default())
                .unwrap_err()
                .contains("processing a transaction")
        );
        drop(borrowed);
        assert!(
            provider
                .call(
                    "designer_document",
                    &json!({"include_source":false}),
                    Permissions::default()
                )
                .unwrap()
                .get("source")
                .is_none()
        );
    }
}
