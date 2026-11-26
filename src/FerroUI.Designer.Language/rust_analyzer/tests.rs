use super::*;
use crate::Position;
use std::{fs, path::Path, sync::atomic::Ordering};

#[cfg(unix)]
#[test]
fn cancellation_interrupts_a_server_with_a_blocked_input_pipe() {
    let cancel = AtomicBool::new(false);
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "sleep 30"]);
    let mut server = Session::spawn(command, &cancel, Duration::from_secs(5)).unwrap();
    std::thread::scope(|threads| {
        threads.spawn(|| {
            std::thread::sleep(Duration::from_millis(80));
            cancel.store(true, Ordering::Release);
        });
        let error = server
            .request("textDocument/didOpen", json!("x".repeat(1024 * 1024)))
            .unwrap_err();
        assert!(error.to_string().contains("cancelled"));
    });
}

#[test]
fn semantic_responses_require_the_generated_declaration_and_the_draft_version() {
    let root = std::env::temp_dir().canonicalize().unwrap();
    let generated = root.join("views.rs");
    let input = root.join("src/lib.rs");
    let rename = GeneratedRustRename {
        uri: file_uri(&generated).unwrap(),
        position: Position {
            line: 6,
            character: 7,
        },
        old_name: "build_view_xaml".into(),
        new_name: "build_clients_xaml".into(),
    };
    let edit = |version| {
        json!({"documentChanges":[
            {"textDocument":{"uri":rename.uri,"version":null},"edits":[
                {"range":{"start":rename.position,"end":{"line":6,"character":22}},"newText":rename.new_name}]},
            {"textDocument":{"uri":file_uri(&input).unwrap(),"version":version},"edits":[
                {"range":{"start":{"line":2,"character":14},"end":{"line":2,"character":29}},"newText":rename.new_name}]}
        ]})
    };
    let generated = BTreeSet::from([root]);
    let versions = BTreeMap::from([(file_uri(&input).unwrap(), 70)]);
    let resolution = workspace_edit::resolve(&rename, edit(1), &generated, &versions).unwrap();
    assert_eq!(resolution.documents.len(), 1);
    assert_eq!(resolution.documents[0].version, 70);
    assert!(workspace_edit::resolve(&rename, edit(5), &generated, &versions).is_err());
    assert!(
        workspace_edit::resolve(&rename, json!({"changes":{}}), &generated, &versions).is_err()
    );
    assert!(workspace_edit::resolve(&rename, json!({"documentChanges":[{"kind":"rename","oldUri":rename.uri,"newUri":"file:///other.rs"}]}), &generated, &versions).is_err());
}

#[test]
#[ignore = "requires a native rust-analyzer and Cargo toolchain"]
fn resolves_the_cargo_generated_definition_and_live_authored_imports() {
    assert!(
        std::env::var_os("CARGO_TARGET_DIR").is_none(),
        "Run the test executable with CARGO_TARGET_DIR unset to isolate the fixture"
    );
    let root = std::env::temp_dir().join(format!("ferroui-rust-rename-{}", std::process::id()));
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let fixture = Fixture(root);
    fs::create_dir_all(fixture.0.join("src")).unwrap();
    let root = fixture.0.canonicalize().unwrap();
    let manifest = root.join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname='xref-fixture'\nversion='0.0.0'\nedition='2024'\n[features]\ncards=[]\n",
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        r#"
use std::{env, fs, path::PathBuf};
fn main() {
    assert!(env::var_os("CARGO_FEATURE_CARDS").is_some());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("xaml");
    fs::create_dir_all(&out).unwrap();
    let view = out.join("views.rs");
    fs::write(&view, "pub fn build_view_xaml() {}\n").unwrap();
    fs::write(out.join("mod.rs"), format!("pub mod views {{ include!({view:?}); }}")).unwrap();
}
"#,
    )
    .unwrap();
    let source = r#"include!(concat!(env!("OUT_DIR"), "/xaml/mod.rs"));
mod archive;
mod others { pub fn build_view_xaml() {} }
const _STATUS: &str = "🧭"; use views::build_view_xaml as open_card;
macro_rules! show { ($view:path) => { $view() }; }
pub fn demonstrate() { open_card(); show!(views::build_view_xaml); others::build_view_xaml(); }
"#;
    fs::write(root.join("src/lib.rs"), source).unwrap();
    let archive = "pub fn show() { super::views::build_view_xaml(); }\n";
    fs::write(root.join("src/archive.rs"), archive).unwrap();
    assert!(
        Command::new("cargo")
            .args([
                "check",
                "--offline",
                "--quiet",
                "--release",
                "--features",
                "cards"
            ])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let generated = fs::read_dir(root.join("target/release/build"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path().join(Path::new("out/xaml/views.rs")))
        .find(|path| path.is_file())
        .unwrap();
    let source = source
        .replace('\n', "\r\n")
        .replace("show!(views::", "views::build_view_xaml(); show!(views::");
    let drafts = [FileMoveDraft {
        uri: file_uri(root.join("src/lib.rs")).unwrap(),
        version: 28,
        source: source.clone(),
    }];
    let renames = [GeneratedRustRename {
        uri: file_uri(generated).unwrap(),
        position: Position {
            line: 0,
            character: 7,
        },
        old_name: "build_view_xaml".into(),
        new_name: "build_clients_xaml".into(),
    }];
    let project = RustAnalyzerProject {
        root,
        manifest,
        features: vec!["cards".into()],
        release: true,
    };
    let cancel = AtomicBool::new(false);
    let resolutions = resolve_file_move(&project, &renames, &drafts, &cancel).unwrap();
    let changes = &resolutions[0].documents;
    assert_eq!(changes.len(), 2);
    let changed = changes
        .iter()
        .find(|document| document.uri == drafts[0].uri)
        .unwrap();
    assert_eq!(changed.version, 28);
    let after = crate::apply_text_edits(&source, &changed.edits).unwrap();
    assert!(after.contains("views::build_clients_xaml as open_card"));
    assert!(after.contains("open_card(); views::build_clients_xaml(); show!(views::build_clients_xaml); others::build_view_xaml();"));
    assert_eq!(changed.edits.len(), 3);
    let closed = changes
        .iter()
        .find(|document| document.uri.ends_with("/src/archive.rs"))
        .unwrap();
    assert_eq!(closed.version, -1);
    assert_eq!(
        crate::apply_text_edits(archive, &closed.edits).unwrap(),
        archive.replace("build_view_xaml", "build_clients_xaml")
    );
    cancel.store(true, Ordering::Release);
    assert!(
        resolve_file_move(&project, &renames, &drafts, &cancel)
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
}
