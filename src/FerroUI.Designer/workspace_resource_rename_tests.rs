use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_project::{file_uri, Solution};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{fs, path::PathBuf};

const ACCOUNT: &str =
    include_str!("../../editors/vscode/test/fixtures/ResourceRename/AccountCard.xaml");
const QUEUE: &str = include_str!("../../editors/vscode/test/fixtures/ResourceRename/Queue.xaml");
const PALETTE: &str =
    include_str!("../../editors/vscode/test/fixtures/ResourceRename/Palette.xaml");

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
fn studio_resource_review_revalidates_and_undoes_all_linked_sources() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = std::env::temp_dir().join(format!(
        "ferroui-studio-resource-review-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let project = Project(root.canonicalize().unwrap());
    let account = project.write("AccountCard.xaml", ACCOUNT);
    let queue = project.write("Queue.xaml", QUEUE);
    let palette = project.write("Palette.xaml", PALETTE);
    let workspace = DesignerWorkspace::with_uri(ACCOUNT, account.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&project.0).unwrap())
        .unwrap();
    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let writes = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let query = json!({
        "offset": ACCOUNT.find("BrandInk").unwrap() + 1, "new_name": "AccountInk",
        "expected_revision": session.borrow().document().revision(),
    });
    let plan = || {
        provider
            .call(
                "designer_plan_resource_rename",
                &query,
                Permissions::default(),
            )
            .unwrap()
    };
    let apply = |review: &serde_json::Value, permission| {
        let mut arguments = query.clone();
        arguments["review"] = review["review"].clone();
        provider.call("designer_rename_resource", &arguments, permission)
    };
    let original = plan();
    assert_eq!(original["documents"].as_array().unwrap().len(), 3);
    assert!(apply(&original, Permissions::default()).is_err());
    let changed = QUEUE.replace("Review queue", "Unprocessed accounts");
    project.write("Queue.xaml", &changed);
    assert!(apply(&original, writes)
        .unwrap_err()
        .contains("resource rename changed"));
    assert_eq!(session.borrow().authored_source(), ACCOUNT);
    apply(&plan(), writes).unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        ACCOUNT.replace("BrandInk", "AccountInk")
    );
    assert_eq!(
        workspace.project.borrow().tabs[&palette].source(),
        PALETTE.replace("BrandInk", "AccountInk")
    );
    let updated = changed.replace("DynamicResource BrandInk", "DynamicResource AccountInk");
    workspace.activate_document(&queue).unwrap();
    assert_eq!(session.borrow().authored_source(), updated);
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), changed);
    assert_eq!(workspace.project.borrow().tabs[&palette].source(), PALETTE);
    assert_eq!(workspace.project.borrow().tabs[&account].source(), ACCOUNT);
    workspace.source_history(true).unwrap();
    assert_eq!(session.borrow().authored_source(), updated);
    assert_eq!(
        fs::read_to_string(project.0.join("Palette.xaml")).unwrap(),
        PALETTE
    );
}
