use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_project::{file_uri, BuildMetadata, Solution};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{fs, path::PathBuf};

const VIEW: &str = r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:a="using:Audit" x:DataType="a:Customer" Text="{Binding Street}"/>"#;
const RUST: &str = r#"// Keep the getter 🪴
pub struct Customer;
ferro_markup_type!(struct Customer as "Customer" {
    namespace: "Audit",
    properties: [Street: String { get: address_street }],
});
"#;

struct TestProject(PathBuf);

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn studio_applies_and_undoes_reviewed_registered_names_across_three_sources() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = std::env::temp_dir().join(format!(
        "ferroui-studio-registered-property-{}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("out")).unwrap();
    let files = TestProject(root.canonicalize().unwrap());
    let manifest = files.0.join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname='audit'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
    )
    .unwrap();
    fs::write(files.0.join("lib.rs"), RUST).unwrap();
    fs::write(files.0.join("Primary.xaml"), VIEW).unwrap();
    let other = files.0.join("Unopened.xaml");
    fs::write(&other, VIEW).unwrap();
    fs::write(files.0.join("out/audit.xamlmeta"), json!({
        "format":2,"name":"Audit","crate_name":"audit","types":[{
            "name":"Customer","namespace":"Audit","kind":"struct","rust_path":"::audit::Customer",
            "properties":[{"name":"Street","value_type":"String","getter":{}}]
        }]
    }).to_string()).unwrap();
    fs::write(files.0.join("out/xaml-rust-sources.json"), json!({
        "version":1,"manifest":manifest,"assembly":"Audit","crate_name":"audit","definitions":[
            {"rust_type":"::audit::Customer","type":"Customer","kind":"markup_type","file":"lib.rs","owner":"Customer"},
            {"rust_type":"::audit::Customer","type":"Customer","kind":"markup_type","file":"lib.rs","owner":"Customer","member":"Street"}
        ]
    }).to_string()).unwrap();
    let primary = file_uri(files.0.join("Primary.xaml")).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, primary.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    workspace
        .language_service()
        .set_build_metadata(&BuildMetadata {
            manifest,
            directories: vec![files.0.join("out")],
            package: Some("audit".into()),
        })
        .unwrap();
    let session = workspace.session();
    let revision = || session.borrow().document().revision();
    let provider = DesignerMcp::with_workspace(&workspace);
    let writes = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let mut request = json!({"offset": VIEW.find("Street").unwrap() + 1, "new_name": "AddressLine",
        "expected_revision": revision()});
    let plan = provider
        .call(
            "designer_plan_property_rename",
            &request,
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(plan["documents"].as_array().unwrap().len(), 3);
    request["review"] = plan["review"].clone();
    assert!(provider
        .call("designer_rename_property", &request, Permissions::default())
        .is_err());
    let saved_consumer = format!("<!-- 🧭 revised -->\n{VIEW}");
    fs::write(&other, &saved_consumer).unwrap();
    assert!(provider
        .call("designer_rename_property", &request, writes)
        .unwrap_err()
        .contains("rename changed"));
    assert_eq!(session.borrow().authored_source(), VIEW);
    request.as_object_mut().unwrap().remove("review");
    let plan = provider
        .call(
            "designer_plan_property_rename",
            &request,
            Permissions::default(),
        )
        .unwrap();
    request["review"] = plan["review"].clone();
    provider
        .call("designer_rename_property", &request, writes)
        .unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        VIEW.replace("Street", "AddressLine")
    );
    let rust = file_uri(files.0.join("lib.rs")).unwrap();
    let consumer = file_uri(&other).unwrap();
    assert_eq!(
        workspace.project.borrow().tabs[&rust].source(),
        RUST.replace("Street:", "AddressLine:")
    );
    assert_eq!(
        workspace.project.borrow().tabs[&consumer].source(),
        saved_consumer.replace("Street", "AddressLine")
    );
    workspace.activate_document(&consumer).unwrap();
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), saved_consumer);
    assert_eq!(workspace.project.borrow().tabs[&primary].source(), VIEW);
    assert_eq!(workspace.project.borrow().tabs[&rust].source(), RUST);
    workspace.source_history(true).unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        saved_consumer.replace("Street", "AddressLine")
    );
    assert_eq!(fs::read_to_string(other).unwrap(), saved_consumer);
    workspace.source_history(false).unwrap();
    workspace.activate_document(&primary).unwrap();
    let mut rename = json!({
        "offset": VIEW.find("Customer").unwrap() + 1, "new_name": "ClientRecord", "expected_revision": revision()
    });
    let plan = provider
        .call("designer_plan_type_rename", &rename, Permissions::default())
        .unwrap();
    assert_eq!(plan["documents"].as_array().unwrap().len(), 3);
    rename["review"] = plan["review"].clone();
    provider
        .call("designer_rename_type", &rename, writes)
        .unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        VIEW.replace("a:Customer", "a:ClientRecord")
    );
    workspace.activate_document(&rust).unwrap();
    let changed = RUST.replace("as \"Customer\"", "as \"ClientRecord\"");
    assert_eq!(session.borrow().authored_source(), changed);
    workspace.source_history(false).unwrap();
    assert_eq!(session.borrow().authored_source(), RUST);
    assert_eq!(workspace.project.borrow().tabs[&primary].source(), VIEW);
}
