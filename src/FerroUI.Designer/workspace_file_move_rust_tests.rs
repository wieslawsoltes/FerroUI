use super::*;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_language::{DocumentEdits, FileMoveRustResolution, LineIndex, TextEdit};
use ferroui_designer_project::BuildMetadata;

#[test]
fn studio_mcp_reviews_pending_generated_rust_references_and_applies_one_history_step() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let xaml = format!("<Border {NS}/>");
    let (files, workspace) = Files::open("View.xaml", &xaml);
    let destination = "Archive/Clients.xaml";
    let source = "const PATH: &str = include_str!(\"View.xaml\");\r\nconst NAME: &str = \"🎨\"; use views::build_view_xaml as view;\r\n";
    let manifest = files.0.join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname='move-studio'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n",
    )
    .unwrap();
    fs::write(files.0.join("lib.rs"), source).unwrap();
    fs::write(files.0.join("build.rs"), r#"use ferroui_build::{Build, XamlGroup};
fn main() { Build::from_env().compile_group(XamlGroup::new("views").documents(&[("View.xaml", include_str!("View.xaml"))])).run(); }
"#).unwrap();
    let output = files.0.join("target/schema");
    fs::create_dir_all(output.join("xaml")).unwrap();
    fs::write(
        output.join("xaml/views.rs"),
        "pub fn build_view_xaml() {}\n",
    )
    .unwrap();
    fs::write(
        output.join("move_studio.xamlmeta"),
        json!({
            "name":"move_studio", "crate_name":"move_studio", "documents":[], "dependencies":[],
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        output.join("xaml-symbols.json"),
        json!({
            "version":1, "manifest":manifest,
            "groups":[{"module":"views","kind":"documents","discovery_root":null,
                "members":[{"key":"View.xaml","symbol":"build_view_xaml","public":true}]}]
        })
        .to_string(),
    )
    .unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    workspace
        .try_language_service()
        .unwrap()
        .set_build_metadata(&BuildMetadata {
            manifest,
            directories: vec![output],
            package: Some("move-studio".into()),
        })
        .unwrap();

    let session = workspace.session();
    let revision = || session.borrow().document().revision();
    let provider = DesignerMcp::with_workspace(&workspace);
    let write = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    let args = json!({"destination": destination, "expected_revision": revision()});
    assert!(
        provider
            .call("designer_plan_xaml_move", &args, Permissions::default())
            .unwrap_err()
            .contains("allow_input")
    );

    let pending = || {
        let target = workspace.move_target(destination).unwrap();
        let drafts = workspace.refresh_move_drafts(&target.from).unwrap();
        let base = workspace
            .try_language_service()
            .unwrap()
            .plan_file_move_with_drafts(&target.from, &target.to, &drafts)
            .unwrap();
        let (review, sender) = RustMoveReview::pending(target, base.review);
        workspace.project.borrow_mut().rust_move = Some(review);
        (base.generated_rust, sender)
    };
    let (_, abandoned) = pending();
    assert_eq!(
        provider
            .call("designer_plan_xaml_move", &args, Permissions::default())
            .unwrap()["status"],
        "resolving"
    );
    assert!(provider.call("designer_move_xaml", &json!({"destination":destination,"review":"pending","expected_revision":revision()}), write).is_err());
    let mut cancel = args.clone();
    cancel["cancel"] = true.into();
    assert_eq!(
        provider
            .call("designer_plan_xaml_move", &cancel, Permissions::default())
            .unwrap()["status"],
        "cancelled"
    );
    assert!(abandoned.send(Ok(Vec::new())).is_err());

    let (declared, analyzed) = pending();
    let [rename] = declared.as_slice() else {
        panic!("expected the emitted function")
    };
    let start = source.find(&rename.old_name).unwrap();
    let range = LineIndex::new(source)
        .range(SourceSpan::new(start, start + rename.old_name.len()))
        .unwrap();
    analyzed
        .send(Ok(vec![FileMoveRustResolution {
            rename: rename.clone(),
            documents: vec![DocumentEdits {
                uri: files.uri("lib.rs"),
                version: -1,
                edits: vec![TextEdit {
                    range,
                    new_text: rename.new_name.clone(),
                }],
            }],
        }]))
        .unwrap();
    let plan = provider
        .call("designer_plan_xaml_move", &args, Permissions::default())
        .unwrap();
    let expected = source
        .replace("View.xaml", destination)
        .replace("build_view_xaml", "build_archive_clients_xaml");
    let changed = plan["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["edit"]["uri"] == files.uri("lib.rs"))
        .unwrap();
    assert_eq!(changed["after"], expected);
    let mut apply = args;
    apply["review"] = plan["review"].clone();
    provider.call("designer_move_xaml", &apply, write).unwrap();
    assert!(files.0.join(destination).is_file());
    workspace.activate_document(&files.uri("lib.rs")).unwrap();
    assert_eq!(session.borrow().authored_source(), expected);
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert!(files.0.join("View.xaml").is_file());
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), expected);
    assert!(files.0.join(destination).is_file());
}
