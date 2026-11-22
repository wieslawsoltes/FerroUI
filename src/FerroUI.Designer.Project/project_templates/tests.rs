use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "ferroui-project-template-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(template: Template, name: &str) -> Request {
    Request {
        template,
        name: name.into(),
        checkout: None,
    }
}

#[test]
fn plans_both_starters_and_publishes_only_the_reviewed_files() {
    let parent = Fixture::new();
    for (template, name) in [
        (Template::DesktopRsx, "task-counter"),
        (Template::DesktopXaml, "task-editor"),
    ] {
        let request = request(template, name);
        let draft = plan(&parent.0, &request).unwrap();
        assert!(!draft.root.exists());
        assert!(draft.files.iter().any(|file| file.path == template.entry()));
        assert_eq!(draft.files.last().unwrap().path, "Cargo.toml");
        assert!(draft
            .files
            .last()
            .unwrap()
            .source
            .contains(&format!("version = \"={PUBLISHED_VERSION}\"")));
        for file in draft.files.iter().filter(|file| file.path.ends_with(".rs")) {
            syn::parse_file(&file.source).unwrap();
        }
        assert_eq!(plan(&parent.0, &request).unwrap().review, draft.review);
        assert!(create(&parent.0, &request, "unreviewed").is_err());
        assert!(!draft.root.exists());
        let created = create(&parent.0, &request, &draft.review).unwrap();
        assert_eq!(created.entry, draft.root.join(template.entry()));
        for file in draft.files {
            assert_eq!(
                fs::read_to_string(created.root.join(file.path)).unwrap(),
                file.source
            );
        }
        assert!(create(&parent.0, &request, &draft.review).is_err());
    }
}

#[test]
fn rejects_unsafe_names_and_existing_destinations_before_writing() {
    let parent = Fixture::new();
    for name in [
        "",
        "../escape",
        "unsafe/project",
        "CON",
        "aux",
        "lpt9",
        "fn",
        "9bad",
        "name\n",
    ] {
        assert!(
            plan(&parent.0, &request(Template::DesktopXaml, name)).is_err(),
            "{name:?}"
        );
    }
    let request = request(Template::DesktopRsx, "task-counter");
    let draft = plan(&parent.0, &request).unwrap();
    fs::create_dir(&draft.root).unwrap();
    fs::write(draft.root.join("README.md"), "existing").unwrap();
    assert!(create(&parent.0, &request, &draft.review).is_err());
    assert_eq!(
        fs::read_to_string(draft.root.join("README.md")).unwrap(),
        "existing"
    );
    assert!(!draft.root.join("Cargo.toml").exists());
}

#[test]
fn checkout_paths_and_destination_participate_in_the_review() {
    let parent = Fixture::new();
    let checkout = parent.0.join("FerroUI checkout");
    let mut request = request(Template::DesktopXaml, "xaml-app");
    request.checkout = Some(checkout.clone());
    assert!(plan(&parent.0, &request).is_err());
    for directory in [
        "",
        "src/FerroUI.Base",
        "src/FerroUI.View",
        "src/FerroUI.Themes.WinUI",
        "src/FerroUI.Controls",
        "src/Markup/FerroUI.Markup.Xaml",
        "src/FerroUI.Build.Tasks",
    ] {
        let directory = checkout.join(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("Cargo.toml"), "").unwrap();
    }
    let draft = plan(&parent.0, &request).unwrap();
    let manifest = &draft.files.last().unwrap().source;
    assert!(manifest.contains("FerroUI checkout"));
    assert!(manifest.contains("path = "));
    assert!(create(&checkout, &request, &draft.review).is_err());
    for dependency in [
        "src/FerroUI.ReloadProtocol",
        "src/FerroUI.HotReload",
        "external/rust-skia-bindings",
    ] {
        let directory = checkout.join(dependency);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("Cargo.toml"), "").unwrap();
    }
    let managed = plan(&parent.0, &request).unwrap();
    assert_ne!(managed.review, draft.review);
    assert!(managed
        .files
        .iter()
        .any(|file| file.path == "src/reload.rs" && syn::parse_file(&file.source).is_ok()));
    assert!(managed
        .files
        .last()
        .unwrap()
        .source
        .contains("[patch.crates-io]\nskia-bindings = { path = "));
    assert!(create(&parent.0, &request, &draft.review).is_err());
    let same_destination_other_source = Request {
        checkout: None,
        ..request
    };
    assert!(create(&parent.0, &same_destination_other_source, &draft.review).is_err());
    assert!(!draft.root.exists());
}

#[test]
fn racing_creations_and_failed_publication_keep_consistent_state() {
    let parent = Fixture::new();
    let request = request(Template::DesktopXaml, "project");
    let draft = plan(&parent.0, &request).unwrap();
    let broken = parent.0.join("broken");
    assert!(publish::create(
        &broken,
        &[
            File {
                path: "src/main.rs",
                source: String::from("fn main() {}")
            },
            File {
                path: "missing/parent/file",
                source: String::new()
            },
        ]
    )
    .is_err());
    assert!(!broken.exists());
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| create(&parent.0, &request, &draft.review).is_ok());
        let second = scope.spawn(|| create(&parent.0, &request, &draft.review).is_ok());
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_ne!(first, second);
    assert!(draft.root.join("Cargo.toml").is_file());
    assert!(draft.root.join(draft.entry).is_file());
}
