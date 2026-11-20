use crate::{apply_text_edits, lsp::LspServer, FileMoveDraft, LanguageService};
use ferroui_designer_project::{file_uri, Solution};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ferroui-file-move-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("Themes/Archive")).unwrap();
        fs::create_dir(root.join("Assets")).unwrap();
        Self(root.canonicalize().unwrap())
    }

    fn uri(&self, name: &str) -> String {
        file_uri(self.0.join(name)).unwrap()
    }
    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).unwrap();
    }
    fn index(&self, service: &mut LanguageService) {
        service
            .index_solution(&Solution::discover(&self.0).unwrap())
            .unwrap();
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn moving_a_view_rebases_typed_uri_and_include_paths_with_exact_source_ranges() {
    let project = Project::new();
    let from = project.uri("View.xaml");
    let to = project.uri("Themes/Archive/Account cards & notes.xaml");
    let source = format!(
        "<Grid {NS}>\r\n  <!-- 🧭 Keep the path and suffix --> <Image Source='Assets/Brand%20lockup.png#large' />\r\n  <Grid.Styles><StyleInclude><StyleInclude.Source><!-- author --><![CDATA[ Themes/Styles.xaml ]]></StyleInclude.Source></StyleInclude></Grid.Styles>\r\n  <TextBlock Text='Assets/Brand%20lockup.png'/><Image Source='{{Binding Logo}}'/>\r\n</Grid>\r\n"
    );
    project.write("View.xaml", &source);
    let mut service = LanguageService::default();
    project.index(&mut service);
    service.open(&from, &source, 7).unwrap();
    let plan = service.plan_file_move(&from, &to).unwrap();
    let [moved] = plan.documents.as_slice() else {
        panic!("only the moved view is affected")
    };
    assert_eq!(plan.to, to);
    assert_eq!(moved.edit.version, 7);
    assert_eq!(moved.before, source);
    assert_eq!(moved.edit.edits.len(), 2);
    assert_eq!(
        apply_text_edits(&source, &moved.edit.edits).unwrap(),
        moved.after
    );
    assert!(moved.after.contains(
        "<!-- 🧭 Keep the path and suffix --> <Image Source='../../Assets/Brand%20lockup.png#large'"
    ));
    assert!(moved
        .after
        .contains("<!-- author --><![CDATA[ ../Styles.xaml ]]>"));
    assert!(moved
        .after
        .contains("<TextBlock Text='Assets/Brand%20lockup.png'/><Image Source='{Binding Logo}'/>"));
    assert!(!moved.after.replace("\r\n", "").contains('\n'));
    assert!(project.0.join("View.xaml").exists());
    assert!(!project
        .0
        .join("Themes/Archive/Account cards & notes.xaml")
        .exists());
}

#[test]
fn review_tracks_unopened_and_open_cross_workspace_consumers_by_resolved_identity() {
    let project = Project::new();
    let other = Project::new();
    let empty = format!("<ResourceDictionary {NS}/>");
    let from = project.uri("Palette.xaml");
    let to = project.uri("Themes/Archive/Brand & 🎨.xaml");
    project.write("Palette.xaml", &empty);
    other.write("Palette.xaml", &empty);
    let direct = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='./Palette.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    project.write("Consumer.xaml", &direct);
    other.write("Unrelated.xaml", &direct);
    let package = project.0.file_name().unwrap().to_str().unwrap();
    let packaged = direct.replace(
        "./Palette.xaml",
        &format!("ferres://{package}/Palette.xaml"),
    );
    other.write("Other consumer.xaml", &packaged);
    let mut service = LanguageService::default();
    project.index(&mut service);
    other.index(&mut service);
    service
        .open(other.uri("Other consumer.xaml"), &packaged, 14)
        .unwrap();
    let first = service.plan_file_move(&from, &to).unwrap();
    assert_eq!(first.documents.len(), 3);
    assert!(first
        .documents
        .iter()
        .all(|doc| doc.edit.uri != other.uri("Unrelated.xaml")));
    let closed = first
        .documents
        .iter()
        .find(|doc| doc.edit.uri == project.uri("Consumer.xaml"))
        .unwrap();
    assert!(closed.edit.version < 0);
    assert!(closed
        .after
        .contains("'./Themes/Archive/Brand%20%26%20%F0%9F%8E%A8.xaml'"));
    let open = first
        .documents
        .iter()
        .find(|doc| doc.edit.uri == other.uri("Other consumer.xaml"))
        .unwrap();
    assert_eq!(open.edit.version, 14);
    assert!(open.after.contains(&format!(
        "ferres://{package}/Themes/Archive/Brand%20%26%20%F0%9F%8E%A8.xaml"
    )));
    assert_eq!(
        service.plan_file_move(&from, &to).unwrap().review,
        first.review
    );
    project.write("New consumer.xaml", &direct);
    let second = service.plan_file_move(&from, &to).unwrap();
    assert_ne!(first.review, second.review);
    assert_eq!(second.documents.len(), 4);
    project.write("Themes/Archive/Brand & 🎨.xaml", &empty);
    assert!(service.plan_file_move(&from, &to).is_err());
}

#[test]
fn file_move_lsp_rejects_a_referencing_draft_and_preserves_other_workspaces() {
    let project = Project::new();
    let other = Project::new();
    let from = project.uri("Palette.xaml");
    let to = project.uri("Themes/Palette.xaml");
    project.write("Palette.xaml", &format!("<ResourceDictionary {NS}/>"));
    let draft = project.uri("Draft.xaml");
    let mut lsp = LspServer::default();
    assert_eq!(lsp.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":file_uri(&project.0).unwrap()}}))[0]["result"]["capabilities"]["experimental"]["ferrouiFileMove"], true);
    assert!(lsp
        .service
        .plan_file_move(&from, &other.uri("Palette.xaml"))
        .is_err());
    let request = json!({"jsonrpc":"2.0","id":2,"method":"ferroui/planFileMove","params":{"from":from,"to":to}});
    lsp.service
        .open(&draft, "<ResourceInclude Source='Palette.xaml'", 1)
        .unwrap();
    assert!(lsp.handle(request.clone())[0]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Repair the draft"));
    lsp.service.replace(&draft, "<Grid", 2).unwrap();
    let result = lsp.handle(request);
    assert_eq!(result[0]["result"]["to"], to);
    assert_eq!(
        result[0]["result"]["documents"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn reviewed_move_updates_a_custom_cargo_target_and_live_rust_in_a_loose_workspace() {
    let workspace = Project::new();
    let src = |path: &str| format!("app/{path}");
    fs::create_dir_all(workspace.0.join("app/Views/Archive")).unwrap();
    fs::create_dir_all(workspace.0.join("app/tools")).unwrap();
    workspace.write(&src("Cargo.toml"), "[package]\nname='crm-fixture'\nversion='0.1.0'\nedition='2021'\nbuild='tools/register.rs'\n[lib]\npath='app.rs'\n");
    workspace.write(&src("Views/Card.xaml"), &format!("<Border {NS}/>"));
    workspace.write(&src("app.rs"), "pub const LABEL: &str = \"Card.xaml\";\n");
    let build = r#"use ferroui_build::{Build, XamlGroup};
const VIEWS: &[(&str, &str)] = &[("Card.xaml", include_str!("../Views/Card.xaml"))];
fn main() {
    println!("cargo::rerun-if-changed=Views/Card.xaml");
    Build::from_env().xaml_root("Views").input("./Views/Card.xaml")
        .compile_group(XamlGroup::new("cards").documents(VIEWS).class_document("Card.xaml"))
        .run();
    another.input("Views/Card.xaml");
}
"#;
    workspace.write(&src("tools/register.rs"), build);
    let from = workspace.uri(&src("Views/Card.xaml"));
    let to = workspace.uri(&src("Views/Archive/Customer 🎨.xaml"));
    let app_uri = workspace.uri(&src("app.rs"));
    let draft = FileMoveDraft {
        uri: app_uri.clone(),
        version: 14,
        source: "pub const VIEW: &str = { /* 🎨 */ include_str!(r#\"Views/Card.xaml\"#) };\r\npub const BYTES: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/Views/Card.xaml\"));\r\n".into(),
    };
    let mut service = LanguageService::default();
    workspace.index(&mut service);
    let disk = service.plan_file_move(&from, &to).unwrap();
    let live = service
        .plan_file_move_with_drafts(&from, &to, &[draft])
        .unwrap();
    assert_ne!(disk.review, live.review);
    assert_eq!(live.documents.len(), 3);
    let changed = live
        .documents
        .iter()
        .find(|item| item.edit.uri == app_uri)
        .unwrap();
    assert_eq!(changed.edit.version, 14);
    assert_eq!(
        changed.after,
        changed
            .before
            .replace("Views/Card.xaml", "Views/Archive/Customer 🎨.xaml")
    );
    assert_eq!(
        apply_text_edits(&changed.before, &changed.edit.edits).unwrap(),
        changed.after
    );
    let script = live
        .documents
        .iter()
        .find(|item| item.edit.uri == workspace.uri(&src("tools/register.rs")))
        .unwrap();
    assert_eq!(script.edit.edits.len(), 5);
    assert!(script.after.contains(
        r#"("Archive/Customer 🎨.xaml", include_str!("../Views/Archive/Customer 🎨.xaml"))"#
    ));
    assert!(script
        .after
        .contains(r#".class_document("Archive/Customer 🎨.xaml")"#));
    assert!(script
        .after
        .contains(r#".input("./Views/Archive/Customer 🎨.xaml")"#));
    assert!(script
        .after
        .contains(r#"another.input("Views/Card.xaml");"#));
    assert!(script
        .after
        .contains("rerun-if-changed=Views/Archive/Customer 🎨.xaml"));
    fs::create_dir(workspace.0.join("other-app")).unwrap();
    workspace.write(
        "other-app/Cargo.toml",
        "[package]\nname='Other'\nversion='0.1.0'\n",
    );
    assert!(service
        .plan_file_move(&from, &workspace.uri("other-app/Card.xaml"))
        .unwrap_err()
        .to_string()
        .contains("same Cargo package"));
}

#[test]
fn checked_rust_resource_moves_preserve_fragments_and_untouched_same_named_files() {
    let first = Project::new();
    let other = Project::new();
    let file = "Card.xaml";
    for project in [&first, &other] {
        project.write(file, &format!("<Border {NS}/>"));
    }
    let assembly = first.0.file_name().unwrap().to_str().unwrap();
    let consumer = format!("pub const URI: &str = \"ferres://{assembly}/{file}?theme=dark#card\";\npub const CARD: &str = include_str!(\"{file}\");\n");
    other.write("consumer.rs", &consumer);
    let mut service = LanguageService::default();
    first.index(&mut service);
    other.index(&mut service);
    let from = first.uri(file);
    let to = first.uri("Themes/Card & notes.xaml");
    let review = service.plan_file_move(&from, &to).unwrap();
    let code = review
        .documents
        .iter()
        .find(|item| item.edit.uri == other.uri("consumer.rs"))
        .unwrap();
    assert_eq!(code.edit.edits.len(), 1);
    assert_eq!(
        code.after,
        consumer.replace("/Card.xaml?", "/Themes/Card%20%26%20notes.xaml?")
    );
    let broken = FileMoveDraft {
        uri: other.uri("consumer.rs"),
        version: 2,
        source: format!("const URI: &str = \"ferres://{assembly}/{file}\"; fn incomplete("),
    };
    assert!(service
        .plan_file_move_with_drafts(&from, &to, &[broken])
        .unwrap_err()
        .to_string()
        .contains("Repair the Rust draft"));
}

#[test]
fn rejects_leaving_the_discovery_root_of_compiled_documents() {
    let project = Project::new();
    project.write(
        "Cargo.toml",
        "[package]\nname='autodiscovery'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
    );
    project.write("lib.rs", "");
    project.write("build.rs", r#"fn main() { ferroui_build::Build::from_env().xaml_root("Themes").compile_xaml().run(); }"#);
    project.write("Themes/View.xaml", &format!("<Border {NS}/>"));
    let mut service = LanguageService::default();
    project.index(&mut service);
    let from = project.uri("Themes/View.xaml");
    assert!(service
        .plan_file_move(&from, &project.uri("Themes/Archive/View.xaml"))
        .is_ok());
    let error = service
        .plan_file_move(&from, &project.uri("View.xaml"))
        .unwrap_err();
    assert!(error.to_string().contains("XAML discovery root"));
}
