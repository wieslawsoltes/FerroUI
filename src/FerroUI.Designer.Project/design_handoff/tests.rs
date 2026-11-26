use super::*;
use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Export(PathBuf);

impl Export {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(1);
        let base = std::env::temp_dir().join(format!(
            "ferroui-handoff-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        Self(base.canonicalize().unwrap())
    }

    fn write(&self, path: &str, contents: &[u8]) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for Export {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn local_exports_need_explicit_selection_and_follow_portable_paths() {
    let workspace = Export::new();
    let package = Export::new();
    let file = package.write("Design.ferroui-handoff.json", br#"{"version":1,"mappings":[{"id":"design/button","target":{"kind":"control","toolbox":"Button"}}]}"#);
    assert!(ManifestFile::read(&workspace.0, &file, Access::Workspace).is_err());
    let manifest = ManifestFile::read(&workspace.0, &file, Access::SelectedFile).unwrap();
    assert_eq!(manifest.manifest.mappings[0].id, "design/button");
    for path in [
        "../button.png",
        "Assets/../button.png",
        "/root.png",
        r"X:\Mark.png",
        "Assets//Mark.png",
    ] {
        assert!(manifest.project_path(path).is_err(), "{path}");
    }
    let duplicate = package.write("Duplicate.json", br#"{"version":1,"mappings":[{"id":"button","target":{"kind":"control","toolbox":"Button"}},{"id":"button","target":{"kind":"control","toolbox":"TextBox"}}]}"#);
    assert!(ManifestFile::read(&workspace.0, &duplicate, Access::SelectedFile).is_err());
}

#[test]
fn transfers_review_exact_bytes_and_never_replace_an_occupied_path() {
    let workspace = Export::new();
    let package = Export::new();
    let file = package.write("Design.json", br#"{"version":1,"mappings":[]}"#);
    let input = package.write("Export/Brand.png", b"first");
    let transfer = Transfer::Import {
        file: "Export/Brand.png".into(),
    };
    let manifest = ManifestFile::read(&workspace.0, &file, Access::SelectedFile).unwrap();
    let prepare = || manifest.project_asset("Assets/Brand.png", Some(&transfer));
    let pending = prepare().unwrap();
    assert!(!pending.available && pending.raster);
    let copy = pending.copy.unwrap();
    fs::write(&input, b"newer").unwrap();
    assert!(copy.apply(&manifest).is_err());
    assert!(!workspace.0.join("Assets").exists());
    let copy = prepare().unwrap().copy.unwrap();
    copy.apply(&manifest).unwrap();
    assert_eq!(
        fs::read(workspace.0.join("Assets/Brand.png")).unwrap(),
        b"newer"
    );
    assert!(prepare().unwrap().copy.is_none());
    workspace.write("Assets/Brand.png", b"other");
    assert!(prepare().is_err());
    assert!(copy.apply(&manifest).is_err());
    let transfer = Transfer::Export {
        file: "Review/Brand.png".into(),
    };
    let pending = manifest
        .project_asset("Assets/Brand.png", Some(&transfer))
        .unwrap();
    assert!(pending.available);
    let copy = pending.copy.unwrap();
    fs::write(&file, br#"{"version":1, "mappings":[]}"#).unwrap();
    assert!(copy.apply(&manifest).is_err());
    let refreshed = ManifestFile::read(&workspace.0, &file, Access::SelectedFile).unwrap();
    assert!(copy.apply(&refreshed).is_err());
    refreshed
        .project_asset("Assets/Brand.png", Some(&transfer))
        .unwrap()
        .copy
        .unwrap()
        .apply(&refreshed)
        .unwrap();
    assert_eq!(
        fs::read(package.0.join("Review/Brand.png")).unwrap(),
        b"other"
    );
}

#[cfg(unix)]
#[test]
fn imported_and_exported_paths_cannot_follow_symlinks_outside_the_review_folders() {
    let workspace = Export::new();
    let package = Export::new();
    let outside = Export::new();
    outside.write("Private.png", b"private");
    std::os::unix::fs::symlink(&outside.0, workspace.0.join("Assets")).unwrap();
    std::os::unix::fs::symlink(&outside.0, package.0.join("Links")).unwrap();
    let file = package.write("Design.json", br#"{"version":1,"mappings":[]}"#);
    let manifest = ManifestFile::read(&workspace.0, &file, Access::SelectedFile).unwrap();
    let transfer = Transfer::Import {
        file: "Links/Private.png".into(),
    };
    assert!(manifest
        .project_asset("Brand.png", Some(&transfer))
        .is_err());
    assert!(manifest.project_asset("Assets/Private.png", None).is_err());
    let transfer = Transfer::Export {
        file: "Links/Copy.png".into(),
    };
    workspace.write("Brand.png", b"shareable");
    assert!(manifest
        .project_asset("Brand.png", Some(&transfer))
        .is_err());
    assert!(!outside.0.join("Copy.png").exists());
}
