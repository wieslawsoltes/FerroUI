use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_project::{
    BuildMetadata, CargoTarget, DocumentEntry, DocumentKind, ProjectDescriptor, Solution, file_uri,
};
use ferroui_mcp::{Permissions, UiToolProvider};
use std::fs;
use std::path::PathBuf;

const MARKUP: &str = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Audit.Settings"><Button x:Name="Apply" Content="Apply settings" /></UserControl>"#;
const RUST: &str = "// Keep this comment\nstruct Settings;\nferro_class!(Settings: UserControl);\nferro_class_info!(Settings { markup: { namespace: \"Audit\" } });\n";
const REGISTRY: &str = r#"{
    "format":2,"name":"Audit","crate_name":"audit",
    "xmlns_definitions":[["https://github.com/ferroui","FerroUI.Controls"]],
    "types":[
        {"name":"Settings","namespace":"Audit","explicit_namespace":"Audit","kind":"class","object_model":true,"module":"audit","rust_path":"::audit::Settings"},
        {"name":"Interactive","namespace":"FerroUI.Interactivity","kind":"class","rust_path":"::ferroui_base::interactivity::Interactive"},
        {"name":"Button","namespace":"FerroUI.Controls","kind":"class","rust_path":"::ferroui_controls::Button","base":"::ferroui_base::interactivity::Interactive",
            "fields":[{"name":"ClickEvent","is_static":true,"return_type":"::ferroui_base::interactivity::RoutedEvent<::ferroui_base::interactivity::RoutedEventArgs>"}]}
    ]
}"#;

struct Files {
    root: PathBuf,
    xaml: String,
    rust: String,
}

impl Files {
    fn open(source: &str) -> (Self, Rc<DesignerWorkspace>) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-studio-handler-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("out")).unwrap();
        let root = root.canonicalize().unwrap();
        let xaml = root.join("Settings.xaml");
        let rust = root.join("lib.rs");
        let manifest = root.join("Cargo.toml");
        fs::write(&xaml, source).unwrap();
        fs::write(&rust, RUST).unwrap();
        fs::write(&manifest, "[package]\nname='audit'\n").unwrap();
        fs::write(root.join("out/audit.xamlmeta"), REGISTRY).unwrap();
        let files = Self {
            root: root.clone(),
            xaml: file_uri(&xaml).unwrap(),
            rust: file_uri(&rust).unwrap(),
        };
        let workspace = DesignerWorkspace::with_uri(source, files.xaml.clone()).unwrap();
        workspace
            .set_solution(Solution {
                format_version: 1,
                root: root.clone(),
                projects: vec![ProjectDescriptor {
                    id: "audit".into(),
                    name: "audit".into(),
                    root: root.clone(),
                    manifest_path: manifest.clone(),
                    documents: vec![DocumentEntry {
                        uri: files.xaml.clone(),
                        path: xaml,
                        kind: DocumentKind::Xaml,
                    }],
                    targets: vec![CargoTarget {
                        name: "audit".into(),
                        kind: vec!["lib".into()],
                        source: rust,
                    }],
                }],
            })
            .unwrap();
        workspace
            .language_service()
            .set_build_metadata(&BuildMetadata {
                manifest,
                directories: vec![root.join("out")],
                package: Some("audit".into()),
            })
            .unwrap();
        (files, workspace)
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn studio_mcp_checks_both_revisions_and_undoes_the_paired_buffers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (files, workspace) = Files::open(MARKUP);
    let session = workspace.session();
    let node = session
        .borrow()
        .document()
        .ast()
        .find_name("Apply")
        .unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    let permitted = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let mut input = json!({"node":node,"event":"Click","handler":"OnApplyClick","expected_revision":session.borrow().document().revision()});
    let inspection = provider
        .call("designer_event_handler", &input, Permissions::default())
        .unwrap();
    assert_eq!(inspection["suggestedName"], "OnApplyClick");
    assert_eq!(
        inspection["plan"]["edit"]["document_changes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(session.borrow().authored_source(), MARKUP);
    input["expected_rust_version"] = json!(inspection["rustVersion"].as_i64().unwrap() + 1);
    assert!(
        provider
            .call("designer_scaffold_handler", &input, permitted)
            .unwrap_err()
            .contains("Rust buffer changed")
    );
    input["expected_rust_version"] = inspection["rustVersion"].clone();
    provider
        .call("designer_scaffold_handler", &input, permitted)
        .unwrap();
    assert!(
        session
            .borrow()
            .authored_source()
            .contains("Click=\"OnApplyClick\"")
    );
    workspace.activate_document(&files.rust).unwrap();
    assert!(
        session
            .borrow()
            .authored_source()
            .contains("fn OnApplyClick(")
    );
    let draft = format!(
        "// A newer Rust edit\n{}",
        session.borrow().authored_source()
    );
    session.borrow_mut().update_source(draft);
    workspace.activate_document(&files.xaml).unwrap();
    assert!(
        workspace
            .source_history(false)
            .unwrap_err()
            .contains("newer")
    );
    workspace.activate_document(&files.rust).unwrap();
    workspace.source_history(false).unwrap();
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":session.borrow().document().revision()}),
            permitted,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), RUST);
    workspace.activate_document(&files.xaml).unwrap();
    assert_eq!(session.borrow().authored_source(), MARKUP);
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":session.borrow().document().revision()}),
            permitted,
        )
        .unwrap();
    assert!(
        session
            .borrow()
            .authored_source()
            .contains("Click=\"OnApplyClick\"")
    );
    assert!(
        workspace.project.borrow().tabs[&files.rust]
            .source()
            .contains("fn OnApplyClick(")
    );
    assert_eq!(fs::read_to_string(files.root.join("lib.rs")).unwrap(), RUST);
}

#[test]
fn studio_source_action_reveals_the_registered_handler_for_an_authored_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let markup = MARKUP.replace("Content=", "Click=\"OnApply\" Content=");
    let (files, workspace) = Files::open(&markup);
    let caret = markup.find("OnApply").unwrap();
    workspace.select_source_span(SourceSpan::new(caret, caret));
    let button = workspace
        .control()
        .get_control::<ferroui_controls::Button>("CreateEventHandler");
    button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
        ferroui_controls::Button::click_event(),
    ));
    assert_eq!(workspace.session().borrow().document_uri(), files.rust);
    assert!(
        workspace
            .session()
            .borrow()
            .authored_source()
            .contains("fn OnApply(")
    );
    let source = workspace.source_editor();
    let source_text = source.text().unwrap();
    assert_eq!(
        &source_text[source.selection_start() as usize..source.selection_end() as usize],
        "OnApply"
    );
    workspace.source_history(false).unwrap();
    assert_eq!(workspace.session().borrow().authored_source(), RUST);
    workspace.activate_document(&files.xaml).unwrap();
    assert_eq!(workspace.session().borrow().authored_source(), markup);
}
