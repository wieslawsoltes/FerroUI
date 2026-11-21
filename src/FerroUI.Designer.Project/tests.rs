use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(1);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ferroui-project-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn write(&self, name: &str, text: &str) -> String {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        file_uri(path).unwrap()
    }
    fn workspace(&self) -> ProjectWorkspace {
        ProjectWorkspace::new(Solution::discover(&self.0).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rust_modules_use_target_paths_and_exact_draft_text() {
    let fixture = Fixture::new();
    let root = fixture.write("src/app.rs", "mod shell { #[path = \"colors.rs\"] mod theme; }\r\nmod views;\r\n");
    let theme = fixture.write("src/shell/colors.rs", "pub const VALUE: u32 = 20;\n");
    let views = fixture.write("src/views.rs", "mod details;\n");
    let details = fixture.write("src/views/details.rs", "fn view() {}\n");
    let authored = "fn view() { /* 🧭 */ }\r\n";
    let buffers = [PreviewSourceBuffer { uri: theme.clone(), source: Some("pub const VALUE: u32 = 42;\r\n".into()) }];
    let layout = RustModuleSources::capture(&fixture.0, &fixture.0.join("src/app.rs"), &details, authored, &buffers).unwrap();
    assert_eq!(layout.target().uri, root);
    assert_eq!(layout.active().source.as_ref(), authored);
    assert_eq!(layout.module_file(&root, &["views".into()]).unwrap().uri, views);
    assert_eq!(layout.module_file(&views, &["details".into()]).unwrap().uri, details);
    let palette = layout.module_file(&root, &["shell".into(), "theme".into()]).unwrap();
    assert_eq!(palette.uri, theme);
    assert_eq!(palette.source.as_ref(), buffers[0].source.as_deref().unwrap());
    let alternate = file_uri(fixture.0.join("src/views/mod.rs")).unwrap();
    assert!(layout.dependencies().any(|dependency| dependency == alternate));
}

#[test]
fn rust_modules_require_unique_owned_declarations_and_current_sources() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let outside_source = outside.write("excluded.rs", "pub const VALUE: u32 = 0;\n");
    let target = fixture.0.join("src/app.rs");
    let source = format!("mod colors; #[cfg(feature = \"night\")] mod night; #[path = {:?}] mod outside;", outside.0.join("excluded.rs"));
    let uri = fixture.write("src/app.rs", &source);
    let colors = fixture.write("src/colors.rs", "pub const VALUE: u32 = 20;\n");
    let night = fixture.write("src/night.rs", "pub const VALUE: u32 = 30;\n");
    assert!(RustModuleSources::capture(&fixture.0, &target, &night, "", &[]).is_none());
    assert!(RustModuleSources::capture(&fixture.0, &target, &outside_source, "", &[]).is_none());
    let buffers = [PreviewSourceBuffer { uri: colors.clone(), source: None }];
    assert!(RustModuleSources::capture(&fixture.0, &target, &uri, &source, &buffers).is_none());
    fixture.write("src/colors/mod.rs", "pub const VALUE: u32 = 40;\n");
    assert!(RustModuleSources::capture(&fixture.0, &target, &colors, "", &[]).is_none());
}

#[test]
fn rust_palette_project_selects_a_unique_cargo_target() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname='review'\n");
    let active = fixture.write("views.rs", "fn view() {}\n");
    fixture.write("design.rs", "mod views;\n");
    fixture.write("worker.rs", "fn main() {}\n");
    let mut project = RustModuleProject {
        manifest: fixture.0.join("Cargo.toml"),
        targets: vec![fixture.0.join("worker.rs"), fixture.0.join("design.rs")],
    };
    let capture = |project: &RustModuleProject| project.capture(&fixture.0, &active, "fn view() {}\n", &[]);
    let design = file_uri(fixture.0.join("design.rs")).unwrap();
    assert_eq!(capture(&project).unwrap().target().uri, design);
    project.targets.push(fixture.0.join("design.rs"));
    assert_eq!(capture(&project).unwrap().target().uri, design);
    fixture.write("worker.rs", "mod views; fn main() {}\n");
    assert!(capture(&project).is_none());
}

#[test]
fn preview_resources_resolve_transitive_files_with_owned_drafts_and_uri_aliases() {
    let fixture = Fixture::new();
    let source = r#"<Grid xmlns="https://github.com/ferroui"><ResourceInclude Source="Resources/Outer.xaml"/></Grid>"#;
    let uri = fixture.write("View.xaml", source);
    let outer = fixture.write("Resources/Outer.xaml", r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceInclude><ResourceInclude.Source>Color%20and%20size.xaml</ResourceInclude.Source></ResourceInclude></ResourceDictionary>"#);
    let values = fixture.write("Resources/Color and size.xaml", "\u{feff}<ResourceDictionary/>");
    let overrides = [PreviewSourceBuffer { uri: values.clone(), source: Some("<ResourceDictionary><!-- draft --></ResourceDictionary>".into()) }];
    let document = ferroui_designer_core::Document::new(source).unwrap();
    let graph = local_preview_sources(Some(&fixture.0), &uri, &document, &overrides);
    assert_eq!(graph.dependencies.len(), 2);
    assert!(graph.accepted.contains_key(&outer));
    assert!(graph.accepted[&values].contains("<!-- draft -->"));
    assert!(graph.unavailable.is_empty());
    let include = document.ast().elements().find(|node| node.local_name() == "ResourceInclude").unwrap();
    let (address, declared) = graph.includes.resolve(&uri, &document, include).unwrap();
    assert_eq!((address, declared.uri.as_str()), (outer.as_str(), outer.as_str()));
    let include = declared.document.ast().elements().find(|node| node.local_name() == "ResourceInclude").unwrap();
    let (_, draft) = graph.includes.resolve(address, &declared.document, include).unwrap();
    assert_eq!(draft.uri, values);
    assert_eq!(draft.document.source(), graph.accepted[&values]);
    let graph = local_preview_sources(Some(&fixture.0), &uri, &document, &[]);
    assert_eq!(graph.accepted[&values], "<ResourceDictionary/>");
}

#[test]
fn preview_resources_fence_cycles_missing_and_out_of_scope_paths() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let outside_uri = outside.write("Sensitive.xaml", "<ResourceDictionary/>");
    let source = format!(r#"<Grid xmlns="https://github.com/ferroui" xmlns:other="urn:vendor"><ResourceInclude Source="A.xaml"/><ResourceInclude Source="Missing.xaml"/><ResourceInclude Source="{outside_uri}"/><other:ResourceInclude Source="Ignored.xaml"/></Grid>"#);
    let uri = fixture.write("View.xaml", &source);
    let cyclic = fixture.write("A.xaml", r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceInclude Source="A.xaml"/></ResourceDictionary>"#);
    let graph = local_preview_sources(Some(&fixture.0), &uri, &ferroui_designer_core::Document::new(source).unwrap(), &[]);
    assert!(graph.accepted.is_empty());
    assert_eq!(graph.includes.documents().count(), 0);
    assert!(graph.unavailable[&cyclic].contains("Circular"));
    assert!(graph.unavailable[&outside_uri].contains("inside"));
    let missing = file_uri(fixture.0.join("Missing.xaml")).unwrap();
    assert!(graph.dependencies.contains(&missing));
    assert_eq!(graph.unavailable.len(), 3);
}

#[test]
fn hosted_preview_uses_runtime_addresses_and_watches_workspace_inputs() {
    let fixture = Fixture::new();
    let source = r#"<Border xmlns="https://github.com/ferroui"><ResourceInclude Source="Dictionaries/Outer.xaml"/><ResourceInclude Source="Embedded.xaml"/><ResourceInclude Source="../../Other/Private.xaml"/></Border>"#;
    let uri = fixture.write("Authoring/View.xaml", source);
    let outer = fixture.write("Authoring/Dictionaries/Outer.xaml", r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceInclude Source="../Color%20set.xaml"/></ResourceDictionary>"#);
    let colors = fixture.write("Authoring/Color set.xaml", "<ResourceDictionary/>");
    let embedded = file_uri(fixture.0.join("Authoring/Embedded.xaml")).unwrap();
    let document = ferroui_designer_core::Document::new(source).unwrap();
    let buffers = [PreviewSourceBuffer { uri: colors.clone(), source: Some("<ResourceDictionary><!-- draft --></ResourceDictionary>".into()) }];
    let base = "ferres://Test/Packaged/View.xaml";
    let graph = hosted_preview_sources(&fixture.0, &uri, base, &document, &buffers, None);
    assert_eq!(graph.dependencies, [outer, colors.clone(), embedded].into_iter().collect());
    assert_eq!(graph.accepted.len(), 2);
    assert_eq!(graph.accepted["ferres://Test/Packaged/Color%20set.xaml"], buffers[0].source.as_deref().unwrap());
    assert!(graph.uses_drafts);
    assert!(graph.unavailable.is_empty());
    let address = "ferres://Test/Packaged/Color%20set.xaml";
    let runtime = "ferres://test/Packaged/Color%20set.xaml";
    let accepted = ferroui_reload_protocol::fingerprint(graph.accepted[address].as_bytes());
    let saved = ferroui_reload_protocol::fingerprint(b"<ResourceDictionary/>");
    let (logical, input) = graph.includes.authored_revision(runtime, accepted).unwrap();
    assert_eq!(logical, address);
    assert_eq!(input.uri, colors);
    assert!(graph.includes.authored_revision(address, saved).is_none());
    assert!(graph.includes.authored_revision("ferres://test/Packaged/color%20set.xaml", accepted).is_none());
    assert!(graph.includes.authored_revision(&colors, accepted).is_none());

    let invalid = [PreviewSourceBuffer { uri: colors, source: None }];
    let graph = hosted_preview_sources(&fixture.0, &uri, base, &document, &invalid, None);
    assert!(!graph.accepted.contains_key(address));
    assert!(graph.unavailable[address].contains("draft"));
}

#[test]
fn toolbox_snippets_overlay_the_active_package_and_ignore_invalid_fragments() {
    let fixture = Fixture::new();
    fixture.write(".ferroui/toolbox/Card.xaml", "<Button Content='Shared'/>");
    fixture.write(".ferroui/toolbox/Invalid.xaml", "<Button");
    fixture.write(".ferroui/toolbox/Sibling.xaml", "<TextBlock Text='Shared label'/>");
    fixture.write("app/Cargo.toml", "[package]\nname = 'app'\nversion = '0.1.0'\n");
    fixture.write("app/.ferroui/toolbox/Card.xaml", "<?xml version='1.0'?><Button Content='App'/>");
    fixture.write("app/views/Page.xaml", "<Grid/>");
    let files = toolbox_snippets(&fixture.0, &fixture.0.join("app/views/Page.xaml")).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].title, "Card");
    assert_eq!(files[0].path, PathBuf::from("app/.ferroui/toolbox/Card.xaml"));
    assert_eq!(files[0].source, "<Button Content='App'/>");
    assert_eq!(files[1].title, "Sibling");
}
#[test]
fn successful_cargo_task_returns_its_build_script_output() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname='toolbox-fixture'\nversion='0.1.0'\nedition='2021'\n",
    );
    fixture.write("src/lib.rs", "");
    let metadata =
        ferroui_build_scan::model::AssemblyModel::new("ToolboxFixture", "toolbox_fixture");
    fixture.write(
        "build.rs",
        &format!(
            r##"
fn main() {{
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("toolbox.xamlmeta"), r#"{}"#).unwrap();
}}
"##,
            metadata.to_json()
        ),
    );
    let solution = Solution::discover(&fixture.0).unwrap();
    let task = BuildTask::cargo(&solution.projects[0], TaskKind::Check, false).unwrap();
    let mut child = task.start().unwrap();
    let limit = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while child.poll() == TaskStatus::Running && std::time::Instant::now() < limit {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child
        .take_output()
        .into_iter()
        .map(|part| part.text)
        .collect::<String>();
    assert_eq!(
        child.poll(),
        TaskStatus::Exited { code: Some(0) },
        "{output}"
    );
    let build = child.take_build_metadata().unwrap();
    assert_eq!(build.package.as_deref(), Some("toolbox-fixture"));
    assert_eq!(build.package_root().unwrap(), fixture.0);
    assert!(
        build
            .catalog()
            .unwrap()
            .models()
            .models()
            .iter()
            .any(|model| model.name == "ToolboxFixture")
    );
    assert!(child.take_build_metadata().is_none());
    assert!(!output.contains("build-script-executed"));
}
#[test]
fn paths_roundtrip_unicode_reserved_characters_and_relative_resources() {
    let fixture = Fixture::new();
    let path = fixture.0.join("Résumé #?% 🦀.xaml");
    let uri = file_uri(&path).unwrap();
    assert_eq!(file_path(&uri).unwrap(), path);
    assert!(uri.contains("%23%3F%25"));
    assert_eq!(
        resolve_uri("ferres://app/Views/Page.xaml", "../Assets/Brush.xaml").unwrap(),
        "ferres://app/Assets/Brush.xaml"
    );
    assert!(resolve_uri("ferres://app/Page.xaml", "../../secret").is_err());
    #[cfg(not(windows))]
    assert!(file_path("file://remote/a").is_err());
    assert!(file_path("file:///bad%00name").is_err());
}
#[test]
fn loose_solution_discovers_documents_and_ignores_generated_folders() {
    let fixture = Fixture::new();
    fixture.write("Views/Page.xaml", "<Grid/>");
    fixture.write("target/generated.xaml", "<Grid/>");
    fixture.write("Assets/logo.svg", "<svg/>");
    let solution = Solution::discover(&fixture.0).unwrap();
    assert_eq!(solution.projects.len(), 1);
    assert_eq!(solution.projects[0].documents.len(), 2);
    assert!(solution.projects[0]
        .documents
        .iter()
        .any(|d| d.kind == DocumentKind::Xaml));
    let descriptor = fixture.0.join("solution.json");
    solution.save_descriptor(&descriptor).unwrap();
    assert_eq!(Solution::load_descriptor(descriptor).unwrap(), solution);
}
#[test]
fn buffers_keep_invalid_drafts_and_protect_external_changes() {
    let fixture = Fixture::new();
    let uri = fixture.write("Page.xaml", "<Grid/>");
    let mut workspace = fixture.workspace();
    workspace.open(&uri).unwrap();
    workspace.update_source(&uri, 0, "<Grid").unwrap();
    assert!(!workspace.buffer(&uri).unwrap().document().is_valid());
    assert!(workspace.close(&uri, false).is_err());
    workspace.save(&uri).unwrap();
    assert!(!workspace.buffer(&uri).unwrap().is_dirty());
    fixture.write("Page.xaml", "<Canvas/>");
    workspace.update_source(&uri, 1, "<Border/>").unwrap();
    assert!(workspace.save(&uri).is_err());
    assert_eq!(
        std::fs::read_to_string(file_path(&uri).unwrap()).unwrap(),
        "<Canvas/>"
    );
    workspace
        .save_with_policy(&uri, SavePolicy::Overwrite)
        .unwrap();
    workspace.close(&uri, false).unwrap();
    assert!(workspace.buffers().next().is_none());
}
#[test]
fn multi_document_updates_are_atomic_under_version_conflicts() {
    let fixture = Fixture::new();
    let a = fixture.write("A.xaml", "<Grid/>");
    let b = fixture.write("B.xaml", "<Border/>");
    let mut workspace = fixture.workspace();
    workspace.open(&a).unwrap();
    workspace.open(&b).unwrap();
    assert!(workspace
        .apply_updates(vec![
            BufferUpdate {
                uri: a.clone(),
                expected_version: 0,
                source: "<Canvas/>".into()
            },
            BufferUpdate {
                uri: b.clone(),
                expected_version: 1,
                source: "<TextBlock/>".into()
            }
        ])
        .is_err());
    assert_eq!(workspace.buffer(&a).unwrap().source(), "<Grid/>");
    assert_eq!(workspace.buffer(&a).unwrap().version(), 0);
    workspace
        .apply_updates(vec![
            BufferUpdate {
                uri: a.clone(),
                expected_version: 0,
                source: "<Canvas/>".into(),
            },
            BufferUpdate {
                uri: b.clone(),
                expected_version: 0,
                source: "<TextBlock/>".into(),
            },
        ])
        .unwrap();
    assert_eq!(workspace.buffer(&b).unwrap().version(), 1);
}
#[test]
fn save_as_rename_reload_and_dirty_close_have_distinct_semantics() {
    let fixture = Fixture::new();
    let mut workspace = fixture.workspace();
    workspace.create("untitled:New", "<Grid/>").unwrap();
    let uri = workspace
        .save_as("untitled:New", fixture.0.join("New.xaml"))
        .unwrap();
    assert!(!workspace.buffer(&uri).unwrap().is_dirty());
    workspace.update_source(&uri, 0, "<Button/>").unwrap();
    let renamed = workspace
        .rename_document(&uri, fixture.0.join("Renamed.xaml"))
        .unwrap();
    assert_eq!(workspace.active_uri(), Some(renamed.as_str()));
    assert!(workspace.buffer(&renamed).unwrap().is_dirty());
    assert!(!fixture.0.join("New.xaml").exists());
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("Renamed.xaml")).unwrap(),
        "<Grid/>"
    );
    assert!(workspace.reload(&renamed, false).is_err());
    workspace.reload(&renamed, true).unwrap();
    assert_eq!(workspace.buffer(&renamed).unwrap().source(), "<Grid/>");
}
#[test]
fn native_document_history_survives_tab_storage() {
    let fixture = Fixture::new();
    let uri = fixture.write("View.xaml", "<Button/>");
    let mut workspace = fixture.workspace();
    workspace.open(&uri).unwrap();
    let mut document = workspace.buffer(&uri).unwrap().document().clone();
    document
        .apply(
            "Width",
            vec![ferroui_designer_core::Edit::SetAttribute {
                node: document.ast().root(),
                name: "Width".into(),
                value: "10".into(),
            }],
        )
        .unwrap();
    workspace.store_document(&uri, 0, document).unwrap();
    let mut restored = workspace.buffer(&uri).unwrap().document().clone();
    restored.undo().unwrap();
    assert_eq!(restored.source(), "<Button/>");
}
#[test]
fn cargo_discovery_uses_actual_workspace_members_and_targets() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "app/Cargo.toml",
        "[package]\nname=\"sample-app\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    );
    fixture.write("app/src/main.rs", "fn main() {}\n");
    fixture.write("app/Main.xaml", "<Grid/>");
    let solution = Solution::discover(fixture.0.join("app")).unwrap();
    assert_eq!(solution.projects[0].name, "sample-app");
    assert!(solution.projects[0]
        .targets
        .iter()
        .any(|t| t.name == "sample-app" && t.kind.contains(&"bin".into())));
    let task =
        BuildTask::cargo(&solution.projects[0], TaskKind::Run { target: None }, true).unwrap();
    assert!(task.arguments().contains(&"--release".into()));
    assert!(BuildTask::cargo(
        &solution.projects[0],
        TaskKind::Run {
            target: Some("missing".into())
        },
        false
    )
    .is_err());
    let workspace = ProjectWorkspace::new(solution);
    assert_eq!(
        workspace
            .resolve_path("ferres://sample_app/Main.xaml")
            .unwrap(),
        fixture.0.join("app/Main.xaml")
    );
    assert_eq!(
        workspace
            .resolve_path("resm:sample_app.Main.xaml?assembly=sample_app")
            .unwrap(),
        fixture.0.join("app/Main.xaml")
    );
    assert!(workspace
        .resolve_path("ferres://sample_app/../Cargo.toml")
        .is_err());
}
#[cfg(unix)]
#[test]
fn symlink_paths_cannot_escape_the_solution() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    outside.write("Secret.xaml", "<Grid/>");
    std::os::unix::fs::symlink(&outside.0, fixture.0.join("link")).unwrap();
    let mut workspace = fixture.workspace();
    assert!(workspace
        .open(&file_uri(fixture.0.join("link/Secret.xaml")).unwrap())
        .is_err());
    workspace.create("untitled:New", "<Grid/>").unwrap();
    assert!(workspace
        .save_as("untitled:New", fixture.0.join("link/New.xaml"))
        .is_err());
}

#[test]
fn buffer_uri_aliases_cannot_create_duplicate_open_documents() {
    let fixture = Fixture::new();
    let uri = fixture.write("A B.xaml", "<Grid/>");
    let mut workspace = fixture.workspace();
    workspace.open(&uri).unwrap();
    let local_alias = uri.replacen("file:///", "file://localhost/", 1);
    assert_eq!(workspace.open(&local_alias).unwrap().uri(), uri);
    assert_eq!(workspace.buffers().count(), 1);
    let created = file_uri(fixture.0.join("C D.xaml")).unwrap();
    workspace
        .create(
            created.replacen("file:///", "file://localhost/", 1),
            "<Grid/>",
        )
        .unwrap();
    assert!(workspace.create(&created, "<Button/>").is_err());
    assert_eq!(workspace.buffer(&created).unwrap().source(), "<Grid/>");
}

#[cfg(unix)]
#[test]
fn every_buffer_operation_uses_one_identity_for_symlink_uri_aliases() {
    let fixture = Fixture::new();
    let uri = fixture.write("View.xaml", "<Grid/>");
    let alias_root = fixture.0.join("alias");
    std::os::unix::fs::symlink(&fixture.0, &alias_root).unwrap();
    let alias = file_uri(alias_root.join("View.xaml")).unwrap();
    assert_ne!(alias, uri);
    assert_eq!(canonical_file_uri(&alias).unwrap(), uri);
    let mut workspace = fixture.workspace();
    workspace.open(&alias).unwrap();
    assert_eq!(workspace.buffer(&alias).unwrap().uri(), uri);
    workspace.activate(&alias).unwrap();
    assert_eq!(workspace.active_uri(), Some(uri.as_str()));
    workspace.update_source(&alias, 0, "<Button/>").unwrap();
    workspace.acknowledge_saved(&alias, 1, "<Grid/>").unwrap();
    let document = workspace.buffer(&alias).unwrap().document().clone();
    workspace.store_document(&alias, 1, document).unwrap();
    workspace.save(&alias).unwrap();
    workspace.reload(&alias, false).unwrap();
    let version = workspace.buffer(&uri).unwrap().version();
    assert!(workspace
        .apply_updates(vec![
            BufferUpdate {
                uri: uri.clone(),
                expected_version: version,
                source: "<Border/>".into()
            },
            BufferUpdate {
                uri: alias.clone(),
                expected_version: version,
                source: "<Canvas/>".into()
            },
        ])
        .is_err());
    assert_eq!(workspace.buffer(&alias).unwrap().source(), "<Button/>");
    assert_eq!(workspace.buffer(&uri).unwrap().version(), version);
    workspace.close(&alias, false).unwrap();
    assert!(workspace.buffer(&uri).is_none());

    let new_alias = file_uri(alias_root.join("New.xaml")).unwrap();
    let new_uri = file_uri(fixture.0.join("New.xaml")).unwrap();
    assert_eq!(canonical_file_uri(&new_alias).unwrap(), new_uri);
    workspace.create(&new_alias, "<Grid/>").unwrap();
    workspace.save(&new_alias).unwrap();
    assert_eq!(workspace.buffer(&new_uri).unwrap().uri(), new_uri);
    let saved = workspace
        .save_as(&new_alias, alias_root.join("Copy.xaml"))
        .unwrap();
    assert_eq!(saved, file_uri(fixture.0.join("Copy.xaml")).unwrap());
}

#[cfg(unix)]
#[test]
fn missing_parent_identity_keeps_workspace_boundaries_and_rejects_dangling_links() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    std::os::unix::fs::symlink(outside.0.join("missing"), fixture.0.join("broken")).unwrap();
    let broken = file_uri(fixture.0.join("broken")).unwrap();
    let broken_child = file_uri(fixture.0.join("broken/View.xaml")).unwrap();
    assert!(canonical_file_uri(&broken).is_err());
    assert!(canonical_file_uri(&broken_child).is_err());
    let mut workspace = fixture.workspace();
    assert!(workspace.create(&broken, "<Grid/>").is_err());
    assert!(workspace.create(&broken_child, "<Grid/>").is_err());

    let new_uri = file_uri(fixture.0.join("new/sub/View.xaml")).unwrap();
    assert_eq!(canonical_file_uri(&new_uri).unwrap(), new_uri);
    workspace.create(&new_uri, "<Grid/>").unwrap();
    std::os::unix::fs::symlink(&outside.0, fixture.0.join("new")).unwrap();
    assert!(workspace.save(&new_uri).is_err());
    assert!(workspace.resolve_path(&new_uri).is_err());
    assert!(!outside.0.join("sub/View.xaml").exists());
    assert!(workspace.buffer(&new_uri).unwrap().is_dirty());
}

#[cfg(windows)]
#[test]
fn canonical_windows_files_and_new_files_use_conventional_drive_uris() {
    let fixture = Fixture::new();
    let uri = fixture.write("Views/Água #% 🦀.xaml", "<Grid/>");
    assert!(uri.starts_with("file:///") && !uri.contains("%3F"), "{uri}");
    assert!(uri.as_bytes().get(8).is_some_and(u8::is_ascii_alphabetic));
    assert_eq!(&uri[9..11], ":/");
    let path = file_path(&uri).unwrap();
    assert_eq!(
        path.canonicalize().unwrap(),
        fixture.0.join("Views/Água #% 🦀.xaml")
    );
    assert_eq!(file_uri(&path).unwrap(), uri);
    assert_eq!(canonical_file_uri(&uri).unwrap(), uri);
    let new_uri = file_uri(fixture.0.join("New.xaml")).unwrap();
    assert_eq!(canonical_file_uri(&new_uri).unwrap(), new_uri);
    let mut workspace = fixture.workspace();
    workspace.open(&uri).unwrap();
    workspace.create(&new_uri, "<Border/>").unwrap();
    workspace.save(&new_uri).unwrap();
    assert_eq!(workspace.buffer(&new_uri).unwrap().source(), "<Border/>");
    assert!(file_path("file:///root-relative.xaml").is_err());
    assert!(file_uri(r"C:drive-relative.xaml").is_err());
    assert!(file_uri(r"\root-relative.xaml").is_err());
    assert_eq!(
        file_path("file://server/share/View.xaml").unwrap(),
        PathBuf::from(r"\\?\UNC\server\share\View.xaml")
    );
}

#[test]
fn custom_embedded_resource_mappings_still_enforce_workspace_boundaries() {
    let fixture = Fixture::new();
    fixture.write("Assets/Brush.xaml", "<Color/>");
    let outside = Fixture::new();
    outside.write("Hidden.xaml", "<Color/>");
    let mut workspace = fixture.workspace();
    workspace
        .register_resource("resm:custom.key", fixture.0.join("Assets/Brush.xaml"))
        .unwrap();
    assert_eq!(
        workspace.resolve_path("resm:custom.key").unwrap(),
        fixture.0.join("Assets/Brush.xaml")
    );
    assert!(workspace
        .register_resource("resm:escape", outside.0.join("Hidden.xaml"))
        .is_err());
}

#[test]
fn relative_uri_resolution_encodes_literal_unicode_spaces_and_canonical_escapes() {
    assert_eq!(
        resolve_uri("file:///Views/View.xaml", "../Assets/Áqua %62rush.xaml").unwrap(),
        "file:///Assets/%C3%81qua%20brush.xaml"
    );
    #[cfg(unix)]
    {
        let path = std::path::Path::new("/tmp/literal\\backslash.xaml");
        assert_eq!(file_path(&file_uri(path).unwrap()).unwrap(), path);
    }
}

#[test]
fn resource_uri_references_preserve_base_file_queries_and_directory_semantics() {
    let base = "ferres://app/Views/View.xaml?theme=dark#part";
    assert_eq!(
        resolve_uri(base, "").unwrap(),
        "ferres://app/Views/View.xaml?theme=dark"
    );
    assert_eq!(
        resolve_uri(base, "?theme=light").unwrap(),
        "ferres://app/Views/View.xaml?theme=light"
    );
    assert_eq!(
        resolve_uri(base, "#other").unwrap(),
        "ferres://app/Views/View.xaml?theme=dark#other"
    );
    assert_eq!(
        resolve_uri(base, "//shared/Assets/Brush.xaml").unwrap(),
        "ferres://shared/Assets/Brush.xaml"
    );
    let directory = resolve_uri(base, "../Assets/").unwrap();
    assert_eq!(directory, "ferres://app/Assets/");
    assert_eq!(
        resolve_uri(&directory, "Brush.xaml").unwrap(),
        "ferres://app/Assets/Brush.xaml"
    );
    assert_eq!(resolve_uri(base, "../Assets/./").unwrap(), directory);
    assert_eq!(resolve_uri(base, "../Assets/Color/..").unwrap(), directory);
    assert_eq!(
        resolve_uri("ferres://app?theme=dark", "Brush.xaml").unwrap(),
        "ferres://app/Brush.xaml"
    );
}

#[test]
fn provider_save_acknowledges_its_snapshot_without_losing_later_edits_or_history() {
    let fixture = Fixture::new();
    let uri = fixture.write("Async.xaml", "<Grid/>");
    let mut workspace = fixture.workspace();
    workspace.open(&uri).unwrap();
    workspace.update_source(&uri, 0, "<Button/>").unwrap();
    assert!(workspace.acknowledge_saved(&uri, 0, "<Button/>").is_err());
    workspace.acknowledge_saved(&uri, 1, "<Grid/>").unwrap();
    let buffer = workspace.buffer(&uri).unwrap();
    assert!(buffer.is_dirty() && buffer.document().is_dirty());
    assert_eq!(buffer.source(), "<Button/>");
    let mut retained = buffer.document().clone();
    retained.undo().unwrap();
    assert!(!retained.is_dirty());
    workspace.acknowledge_saved(&uri, 1, "<Button/>").unwrap();
    assert!(!workspace.buffer(&uri).unwrap().document().is_dirty());
    assert!(!workspace.buffer(&uri).unwrap().is_dirty());
    assert_eq!(workspace.buffer(&uri).unwrap().version(), 1);
}
