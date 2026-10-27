use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_project::{Solution, file_uri};

const MAIN: &str =
    r#"<UserControl xmlns="https://github.com/ferroui"><TextBlock Text="Main"/></UserControl>"#;
const CARD: &str = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.Card"><TextBlock Text="Saved card"/></UserControl>"#;
const THEME: &str = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ControlTheme x:Key="CardTheme" TargetType="Button"><Setter Property="Padding" Value="8"/></ControlTheme></ResourceDictionary>"#;

struct ProjectFiles(std::path::PathBuf);
impl ProjectFiles {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-component-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Card.xaml"), CARD).unwrap();
        std::fs::write(root.join("Theme.xaml"), THEME).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn uri(&self, file: &str) -> String {
        file_uri(self.0.join(file)).unwrap()
    }
}
impl Drop for ProjectFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn exported(workspace: &DesignerWorkspace, uri: &str) -> Vec<DesignerDefinitionAsset> {
    workspace
        .components
        .borrow()
        .definitions
        .values()
        .filter(|asset| asset.origin_uri.as_deref() == Some(uri))
        .cloned()
        .collect()
}

#[test]
fn indexed_project_definitions_appear_without_reopening_files_or_rebuilding_unchanged_snapshots() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = ProjectFiles::new();
    let workspace = DesignerWorkspace::with_uri(MAIN, "memory://main.xaml".into()).unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    let card_uri = files.uri("Card.xaml");
    let card = exported(&workspace, &card_uri)
        .pop()
        .expect("Project user control is automatically cataloged");
    assert_eq!(card.kind, ComponentKind::UserControl);
    assert!(
        exported(&workspace, &files.uri("Theme.xaml"))
            .iter()
            .any(|asset| asset.kind == ComponentKind::ControlTheme)
    );
    let accepted = workspace.components.borrow().sources[&card.id]
        .document
        .clone();
    workspace.refresh_components();
    workspace.refresh_components();
    assert!(Rc::ptr_eq(
        &accepted,
        &workspace.components.borrow().sources[&card.id].document
    ));
    // A previously indexed provider remains usable without a second disk read.
    std::fs::remove_file(files.0.join("Card.xaml")).unwrap();
    workspace.refresh_components();
    workspace.open_component_document(&card.id).unwrap();
    assert_eq!(workspace.session.borrow().document().source(), CARD);
    assert!(
        !workspace
            .component_library()
            .unwrap()
            .visible_items()
            .iter()
            .any(|item| item.id == format!("definition:{}", card.id)),
        "The active source is represented by document entries, not duplicate external entries"
    );
}

#[test]
fn retained_dirty_and_invalid_tabs_override_the_disk_index_without_losing_user_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = ProjectFiles::new();
    let workspace = DesignerWorkspace::with_uri(MAIN, "memory://main.xaml".into()).unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    let uri = files.uri("Card.xaml");
    workspace.activate_document(&uri).unwrap();
    let dirty = CARD.replace("Saved card", "Unsaved card");
    workspace.session.borrow_mut().update_source(&dirty);
    // No explicit language-service synchronization: capture_tab is authoritative.
    workspace.activate_document("memory://main.xaml").unwrap();
    let asset = exported(&workspace, &uri).pop().unwrap();
    assert!(asset.source.contains("Unsaved card"));
    assert_eq!(
        std::fs::read_to_string(files.0.join("Card.xaml")).unwrap(),
        CARD
    );
    workspace.open_component_document(&asset.id).unwrap();
    assert_eq!(workspace.session.borrow().document().source(), dirty);
    assert!(workspace.session.borrow().can_undo());
    workspace.session.borrow_mut().update_source("<UserControl");
    workspace.activate_document("memory://main.xaml").unwrap();
    assert!(
        exported(&workspace, &uri).is_empty(),
        "A broken dirty tab cannot export its stale last-valid definition"
    );
    workspace.activate_document(&uri).unwrap();
    assert_eq!(
        workspace.session.borrow().document().source(),
        "<UserControl"
    );
    workspace.session.borrow_mut().update_source(&dirty);
    workspace.activate_document("memory://main.xaml").unwrap();
    assert!(
        exported(&workspace, &uri)
            .iter()
            .any(|asset| asset.source.contains("Unsaved card"))
    );
}

#[test]
fn automatic_document_retirement_preserves_explicit_providers_and_batch_failure_is_atomic() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::with_uri(MAIN, "memory://main.xaml".into()).unwrap();
    workspace
        .register_component_document("memory://explicit.xaml", THEME)
        .unwrap();
    workspace
        .language_service()
        .open("memory://indexed.xaml", CARD, 1)
        .unwrap();
    workspace.synchronize_language_service().unwrap();
    assert!(!exported(&workspace, "memory://indexed.xaml").is_empty());
    let before = workspace.components.borrow().definitions.clone();
    let replacement = THEME.replace("Value=\"8\"", "Value=\"32\"");
    assert!(
        workspace
            .register_component_documents([
                ("memory://explicit.xaml", replacement.as_str()),
                ("memory://batch-broken.xaml", "<UserControl"),
            ])
            .is_err()
    );
    assert_eq!(workspace.components.borrow().definitions, before);
    assert!(
        workspace
            .register_component_documents([
                ("memory://explicit.xaml", replacement.as_str()),
                ("memory://explicit.xaml", THEME),
            ])
            .is_err(),
        "Duplicate document identities cannot partially replace an existing provider"
    );
    assert_eq!(workspace.components.borrow().definitions, before);
    workspace
        .language_service()
        .close("memory://indexed.xaml")
        .unwrap();
    workspace.synchronize_language_service().unwrap();
    assert!(exported(&workspace, "memory://indexed.xaml").is_empty());
    assert!(!exported(&workspace, "memory://explicit.xaml").is_empty());
    let count = workspace
        .register_component_documents([
            ("memory://first.xaml", CARD),
            ("memory://second.xaml", THEME),
        ])
        .unwrap();
    assert_eq!(count, 2);
    assert!(!exported(&workspace, "memory://first.xaml").is_empty());
    assert!(!exported(&workspace, "memory://second.xaml").is_empty());
}

#[cfg(unix)]
#[test]
fn project_component_activation_reuses_canonical_alias_tabs_and_their_dirty_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = ProjectFiles::new();
    let alias = files.0.join("alias");
    std::os::unix::fs::symlink(&files.0, &alias).unwrap();
    let workspace = DesignerWorkspace::with_uri(MAIN, "memory://main.xaml".into()).unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    let canonical = files.uri("Card.xaml");
    let alias_uri = file_uri(alias.join("Card.xaml")).unwrap();
    workspace.activate_document(&alias_uri).unwrap();
    let dirty = CARD.replace("Saved card", "Alias draft");
    workspace.session.borrow_mut().update_source(&dirty);
    workspace.activate_document("memory://main.xaml").unwrap();
    let asset = exported(&workspace, &canonical).pop().unwrap();
    workspace.open_component_document(&asset.id).unwrap();
    assert_eq!(workspace.session.borrow().document_uri(), canonical);
    assert_eq!(workspace.session.borrow().document().source(), dirty);
    assert!(workspace.session.borrow().can_undo());
    assert_eq!(
        workspace
            .document_uris()
            .iter()
            .filter(|uri| *uri == &canonical)
            .count(),
        1
    );
}
