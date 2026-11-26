use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_project::{Solution, file_uri};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{fs, path::PathBuf};

const VIEW: &str = include_str!("../../editors/vscode/test/fixtures/ResourceMove/Workspace.xaml");
const PALETTE: &str = include_str!("../../editors/vscode/test/fixtures/ResourceMove/Palette.xaml");

struct Project(PathBuf);

impl Project {
    fn write(&self, name: &str, source: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, source).unwrap();
        file_uri(path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn studio_resource_move_revalidates_unopened_destinations_and_links_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = std::env::temp_dir().join(format!(
        "ferroui-studio-resource-move-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let project = Project(root.canonicalize().unwrap());
    let view = project.write("Workspace.xaml", VIEW);
    let palette = project.write("Palette.xaml", PALETTE);
    let workspace = DesignerWorkspace::with_uri(VIEW, view.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&project.0).unwrap())
        .unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    let session = workspace.session();
    let read = Permissions::default();
    let write = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let site = json!({
        "offset": VIEW.find("x:Key=\"Accent\"").unwrap() + "x:Key=\"".len(),
        "expected_revision": session.borrow().document().revision(),
    });
    let review = || {
        let targets = provider
            .call("designer_resource_move_targets", &site, read)
            .unwrap();
        let option = targets["destinations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["uri"] == palette)
            .unwrap();
        let mut arguments = site.clone();
        arguments["destination"] = json!({"uri":option["uri"],"request":option["request"]});
        let plan = provider
            .call("designer_plan_resource_move", &arguments, read)
            .unwrap();
        assert_eq!(plan["linkedDestination"], true);
        assert_eq!(plan["documents"].as_array().unwrap().len(), 2);
        arguments["review"] = plan["review"].clone();
        (arguments, plan)
    };
    let (initial, _) = review();
    assert!(
        provider
            .call("designer_move_resource", &initial, read)
            .is_err()
    );
    let changed = format!("{PALETTE}<!-- kept external note -->\n");
    project.write("Palette.xaml", &changed);
    assert!(
        provider
            .call("designer_move_resource", &initial, write)
            .is_err()
    );
    assert_eq!(session.borrow().authored_source(), VIEW);
    let (current, plan) = review();
    provider
        .call("designer_move_resource", &current, write)
        .unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        plan["documents"][0]["after"]
    );
    assert_eq!(
        workspace.project.borrow().tabs[&palette].source(),
        plan["documents"][1]["after"]
    );
    workspace.activate_document(&palette).unwrap();
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), changed);
    assert_eq!(workspace.project.borrow().tabs[&view].source(), VIEW);
    workspace.source_history(true).unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        plan["documents"][1]["after"]
    );
    assert_eq!(
        fs::read_to_string(project.0.join("Palette.xaml")).unwrap(),
        changed
    );
}
