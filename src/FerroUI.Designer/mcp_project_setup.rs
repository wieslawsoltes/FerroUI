use super::s;
use crate::DesignerWorkspace;
use ferroui_designer_project::{file_uri, project_templates, toolchain, Solution};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const CATALOG: &str = "ferroui_project_templates";
const PLAN: &str = "ferroui_project_plan_new";
const CREATE: &str = "ferroui_project_create";
const TOOLCHAIN: &str = "ferroui_project_check_toolchain";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, CATALOG | PLAN | CREATE | TOOLCHAIN)
}

pub(super) fn tools() -> [Value; 4] {
    let fields = json!({
        "template":{"enum":["desktopRsx", "desktopXaml"]}, "name":{"type":"string","minLength":1,"maxLength":64},
        "checkout": s::string(), "folder": s::string()
    });
    let mut create = fields.clone();
    create["review"] = json!({"type":"string","minLength":16,"maxLength":16});
    let mut create = s::tool(
        CREATE,
        "Create the reviewed Cargo app in its new unoccupied folder.",
        s::object(create, &["template", "name", "review"]),
        true,
    );
    create["annotations"]["destructiveHint"] = false.into();
    [
        s::tool(CATALOG, "List the runnable FerroUI project templates and supported framework and Rust versions.",
            s::object(json!({}), &[]), false),
        s::tool(PLAN, "Review the new standalone Cargo project files under the open solution. Optional checkout is relative to the solution root.",
            s::object(fields, &["template", "name"]), false),
        create,
        s::tool(TOOLCHAIN, "Check Cargo, the active Rust compiler and installed target libraries from the open solution.",
            s::object(json!({"folder":s::string(), "manifest_path":s::string(), "package":s::string(),
                "binary":s::string(), "target":{"type":"string","maxLength":100}}), &[]), false),
    ]
}

pub(super) fn call(
    workspace: &DesignerWorkspace,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == CATALOG {
        return Ok(project_templates::catalog());
    }
    let solution = workspace
        .solution()
        .ok_or("Open a solution or a folder in Studio")?;
    let root = solution
        .root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if let Some(folder) = args["folder"].as_str() {
        let uri = file_uri(&root).map_err(|error| error.to_string())?;
        if folder != uri && root.file_name().and_then(|name| name.to_str()) != Some(folder) {
            return Err("Choose the open Studio solution folder".into());
        }
    }
    if name == TOOLCHAIN {
        return check_toolchain(&solution, workspace.selected_project(), &root, args);
    }
    let template =
        serde_json::from_value(args["template"].clone()).map_err(|error| error.to_string())?;
    let package = args["name"].as_str().ok_or("Pass a project name")?;
    let checkout = args["checkout"]
        .as_str()
        .map(|path| scoped(&root, path))
        .transpose()?;
    let request = project_templates::Request {
        template,
        name: package.into(),
        checkout,
    };
    if name == CREATE {
        let review = args["review"]
            .as_str()
            .ok_or("Review the project files before creating them")?;
        project_templates::create(&root, &request, review)
            .map(|created| json!(created))
            .map_err(|error| error.to_string())
    } else {
        project_templates::plan(&root, &request)
            .map(|plan| json!(plan))
            .map_err(|error| error.to_string())
    }
}

fn check_toolchain(
    solution: &Solution,
    selected: usize,
    root: &Path,
    args: &Value,
) -> Result<Value, String> {
    let manifest = args["manifest_path"]
        .as_str()
        .map(|path| scoped(root, path))
        .transpose()?;
    let package = args["package"].as_str();
    let binary = args["binary"].as_str();
    let project = if manifest.is_some() || package.is_some() || binary.is_some() {
        let projects = solution
            .projects
            .iter()
            .filter(|project| {
                manifest
                    .as_ref()
                    .is_none_or(|manifest| &project.manifest_path == manifest)
                    && package.is_none_or(|name| project.name == name)
                    && binary.is_none_or(|name| {
                        project.targets.iter().any(|target| {
                            target.name == name && target.kind.iter().any(|kind| kind == "bin")
                        })
                    })
            })
            .collect::<Vec<_>>();
        let [project] = projects.as_slice() else {
            return Err("Choose one Cargo project and binary".into());
        };
        Some(*project)
    } else {
        solution.projects.get(selected)
    };
    let request = toolchain::Request {
        directory: project.map_or_else(|| root.to_owned(), |project| project.root.clone()),
        target: args["target"].as_str().map(str::to_owned),
        ..Default::default()
    };
    let report = toolchain::inspect(&request).map_err(|error| error.to_string())?;
    let mut value = json!(report);
    if let Some(project) = project {
        value["manifest"] = json!(project.manifest_path);
        value["package"] = json!(project.name);
    }
    if let Some(binary) = binary {
        value["binary"] = json!(binary);
    }
    Ok(value)
}

fn scoped(root: &Path, entry: &str) -> Result<PathBuf, String> {
    let path = Path::new(entry);
    if path.is_absolute() {
        return Err("Use a path relative to the open solution".into());
    }
    let resolved = root
        .join(path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !resolved.starts_with(root) {
        return Err("The path must resolve inside the open solution".into());
    }
    Ok(resolved)
}
