//! Workspace lifecycle and language automation use the same project/editor transaction APIs.
use super::schema as s;
use super::{decode, integer, state, text};
use crate::DesignerWorkspace;
use ferroui_designer_language::{FormattingOptions, WorkspaceEdit};
use ferroui_designer_project::{BuildTask, TaskKind};
use serde_json::{json, Value};
use std::rc::Rc;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_project"
            | "designer_project_edit"
            | "designer_build_plan"
            | "designer_project_task"
            | "designer_language"
            | "designer_language_apply"
    )
}
pub(super) fn tools() -> Vec<Value> {
    let query = |action: &str, extra: Value, required: &[&str]| {
        let mut fields = extra;
        fields["uri"] = s::string();
        s::action(action, fields, required, false)
    };
    let mut queries = Vec::new();
    for action in ["diagnostics", "symbols", "folding", "semantic_tokens"] {
        queries.push(query(action, json!({}), &[]));
    }
    for action in [
        "completion",
        "hover",
        "definitions",
        "references",
        "signature",
    ] {
        let mut fields = json!({"offset":s::revision()});
        if action == "references" {
            fields["include_declaration"] = json!({"type":"boolean"});
        }
        queries.push(query(action, fields, &["offset"]));
    }
    queries.push(query("format", json!({"options":s::formatting()}), &[]));
    queries.push(query(
        "rename",
        json!({"offset":s::revision(),"new_name":s::string()}),
        &["offset", "new_name"],
    ));
    queries.push(query(
        "resource_uri",
        json!({"reference":s::string()}),
        &["reference"],
    ));
    queries.push(s::action(
        "workspace_symbols",
        json!({"query":s::string()}),
        &["query"],
        false,
    ));
    let plans = ["check", "build", "run"]
        .into_iter()
        .map(|kind| {
            let mut fields = json!({"project":s::revision(),"release":{"type":"boolean"}});
            if kind == "run" {
                fields["target"] = s::nullable(s::string());
            }
            s::action(kind, fields, &[], false)
        })
        .collect();
    let mut tasks: Vec<_> = ["check", "build", "run"]
        .into_iter()
        .map(|kind| {
            let mut fields = json!({"release":{"type":"boolean"}});
            if kind == "run" {
                fields["target"] = s::nullable(s::string());
            }
            s::action(kind, fields, &[], true)
        })
        .collect();
    tasks.push(s::action("cancel", json!({}), &[], true));
    vec![
        s::tool("designer_project","Read the discovered solution, selected project, open document URIs, discovery status and current build task. include_output returns the bounded captured compiler output.",s::object(json!({"include_output":{"type":"boolean"}}),&[]),false),
        s::actions("designer_project_edit","Open a solution asynchronously, select a project, activate/close an editor document, or save all authored buffers. Poll designer_project for discovery completion. close requires an explicit discard choice; saves preserve external-change checks.",vec![
            s::action("open_solution",json!({"path":s::string()}),&["path"],true),
            s::action("select_project",json!({"project":s::revision()}),&["project"],true),
            s::action("activate",json!({"uri":s::string()}),&["uri"],true),
            s::action("close",json!({"uri":s::string(),"discard":{"type":"boolean"}}),&["uri","discard"],true),
            s::action("save_all",json!({}),&[],true),
        ],true),
        s::actions("designer_build_plan","Describe the exact shell-free Cargo task without starting it. Project defaults to the selected project; run validates the binary target.",plans,false),
        s::actions("designer_project_task","Save checked editor buffers and explicitly start a Cargo check/build/run task, or cancel the owned task. Starting a process additionally requires allow_input because builds execute the project's build scripts with host privileges. Poll designer_project for status/output; no shell text is accepted.",tasks,true),
        s::actions("designer_language","Query the shared XAML language service over open and indexed project files. uri defaults to the active document. Cursor offsets are UTF-8 byte offsets; returned ranges are LSP UTF-16. format returns text edits with the current version; rename returns a versioned WorkspaceEdit for designer_language_apply.",queries,false),
        s::tool("designer_language_apply","Apply a prepared multi-document language WorkspaceEdit atomically through retained editor tabs and the project version guards. The active document also requires expected_revision; no files are saved automatically.",s::object(json!({"expected_revision":s::revision(),"edit":s::workspace_edit()}),&["expected_revision","edit"]),true),
    ]
}
pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "designer_project" => project(workspace, args["include_output"] == true),
        "designer_project_edit" => {
            match text(args, "action")?.as_str() {
                "open_solution" => workspace.open_solution(text(args, "path")?)?,
                "select_project" => workspace.select_project(index(args, "project")?)?,
                "activate" => workspace.activate_document(&text(args, "uri")?)?,
                "close" => workspace.close_document(
                    &text(args, "uri")?,
                    args["discard"].as_bool().ok_or("discard must be boolean")?,
                )?,
                "save_all" => workspace.save_all_documents()?,
                _ => return Err("Unknown project action".into()),
            }
            project(workspace, false)
        }
        "designer_build_plan" => {
            let solution = workspace
                .solution()
                .ok_or("Open a solution before planning a project task")?;
            let index = args["project"]
                .as_u64()
                .map(|value| {
                    usize::try_from(value).map_err(|_| "project index is too large".to_owned())
                })
                .transpose()?
                .unwrap_or(workspace.selected_project());
            let project = solution
                .projects
                .get(index)
                .ok_or("Unknown project index")?;
            let task = BuildTask::cargo(project, task_kind(args)?, args["release"] == true)
                .map_err(|error| error.to_string())?;
            Ok(json!({"task":task,"project":index}))
        }
        "designer_project_task" => {
            if args["action"] == "cancel" {
                workspace.cancel_project_task()?;
            } else {
                workspace.start_project_task(task_kind(args)?, args["release"] == true)?;
            }
            project(workspace, true)
        }
        "designer_language" => language(workspace, args),
        "designer_language_apply" => {
            let edit: WorkspaceEdit = decode(args, "edit")?;
            workspace.apply_language_edits(edit)?;
            let session = workspace.session();
            let result =
                json!({"document":state(&session.borrow()),"documents":workspace.document_uris()});
            Ok(result)
        }
        _ => Err("Unknown workspace tool".into()),
    }
}
fn index(args: &Value, key: &str) -> Result<usize, String> {
    usize::try_from(integer(args, key)?).map_err(|_| format!("{key} is too large"))
}
fn task_kind(args: &Value) -> Result<TaskKind, String> {
    match text(args, "action")?.as_str() {
        "check" => Ok(TaskKind::Check),
        "build" => Ok(TaskKind::Build),
        "run" => Ok(TaskKind::Run {
            target: args["target"].as_str().map(str::to_owned),
        }),
        _ => Err("Unknown Cargo task".into()),
    }
}
fn project(workspace: &Rc<DesignerWorkspace>, include_output: bool) -> Result<Value, String> {
    let task = workspace.project_task_status();
    workspace.poll_project_work();
    let session = workspace.session();
    let mut result = json!({"document":state(&session.borrow()),"active_uri":session.borrow().document_uri(),"solution":workspace.solution(),"selected_project":workspace.selected_project(),"documents":workspace.document_uris(),"discovering":workspace.is_discovering_solution(),"task":task});
    if include_output {
        result["output"] = json!(workspace.project_task_output());
    }
    Ok(result)
}
fn language(workspace: &Rc<DesignerWorkspace>, args: &Value) -> Result<Value, String> {
    workspace.synchronize_language_service()?;
    let session = workspace.session();
    let active = session.borrow().document_uri().to_owned();
    let revision = session.borrow().document().revision();
    let uri = args["uri"].as_str().unwrap_or(&active);
    let action = text(args, "action")?;
    let language = workspace.try_language_service()?;
    let result: Value = match action.as_str() {
        "diagnostics" => json!(language
            .diagnostics(uri)
            .map_err(|error| error.to_string())?),
        "completion" => json!(language
            .completions(uri, index(args, "offset")?)
            .map_err(|error| error.to_string())?),
        "hover" => json!(language
            .hover(uri, index(args, "offset")?)
            .map_err(|error| error.to_string())?),
        "definitions" => json!(language
            .definitions(uri, index(args, "offset")?)
            .map_err(|error| error.to_string())?),
        "references" => json!(language
            .references_with_declarations(
                uri,
                index(args, "offset")?,
                args["include_declaration"] != false
            )
            .map_err(|error| error.to_string())?),
        "signature" => json!(language
            .signature_help(uri, index(args, "offset")?)
            .map_err(|error| error.to_string())?),
        "symbols" => json!(language.symbols(uri).map_err(|error| error.to_string())?),
        "folding" => json!(language.folding(uri).map_err(|error| error.to_string())?),
        "semantic_tokens" => json!(language
            .semantic_tokens(uri)
            .map_err(|error| error.to_string())?),
        "workspace_symbols" => json!(language
            .workspace_symbols(&text(args, "query")?)
            .map_err(|error| error.to_string())?),
        "resource_uri" => json!(language
            .resource_uri(uri, &text(args, "reference")?)
            .map_err(|error| error.to_string())?),
        "format" => {
            let options = if args.get("options").is_some() {
                decode(args, "options")?
            } else {
                FormattingOptions::default()
            };
            json!(language
                .format(uri, options)
                .map_err(|error| error.to_string())?)
        }
        "rename" => json!(language
            .rename(uri, index(args, "offset")?, &text(args, "new_name")?)
            .map_err(|error| error.to_string())?),
        _ => return Err("Unknown language query".into()),
    };
    Ok(json!({"revision":revision,"uri":uri,"version":language.version(uri),"result":result}))
}
