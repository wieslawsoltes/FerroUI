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
