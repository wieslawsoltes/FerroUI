use super::*;

#[test]
fn launcher_search_matches_words_across_the_entry() {
    let catalog = ApplicationCatalog::from_applications(vec![
        DesktopApplication::new(
            "editor.desktop".into(),
            "Éditeur".into(),
            "Edit documents".into(),
            String::new(),
        ),
        DesktopApplication::new(
            "terminal.desktop".into(),
            "Terminal".into(),
            "Command line".into(),
            String::new(),
        ),
    ]);
    assert_eq!(
        catalog.search_indices("DOCUM ÉDI"),
        [catalog.index_of("editor.desktop").unwrap()]
    );
    assert_eq!(catalog.search_indices("command édit").len(), 0);
    assert_eq!(catalog.search_indices(" ").len(), 2);
}

#[cfg(target_os = "linux")]
#[test]
fn xdg_precedence_visibility_and_launch() {
    use std::{env, fs, path::PathBuf, process::Command, time::Duration};
    const FIXTURE: &str = "FERRO_DESKTOP_TEST_CATALOG";

    // GIO caches the XDG environment at its first use. Run the fixture in a
    // fresh process to leave the test harness and the user's session intact.
    if let Some(fixture) = env::var_os(FIXTURE) {
        let dir = PathBuf::from(fixture);
        let catalog = ApplicationCatalog::discover();
        assert_eq!(catalog.unavailable_reason(), None);
        let ids: Vec<_> = catalog
            .applications()
            .iter()
            .map(|app| app.id.as_str())
            .collect();
        assert_eq!(ids, ["ferroutil.desktop"]);
        let app = &catalog.applications()[0];
        assert_eq!(app.name, "User Utility");
        assert_eq!(app.description, "Catalog fixture");
        assert!(app.icon.contains("utilities-terminal"));
        app.launch().unwrap();
        for _ in 0..200 {
            if dir.join("launch marker").is_file() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("GIO accepted the launch but the process did not create its marker");
    }

    let dir = env::temp_dir().join(format!("ferro-desktop-catalog-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let user = dir.join("user/applications");
    let system = dir.join("system/applications");
    fs::create_dir_all(&user).unwrap();
    fs::create_dir_all(&system).unwrap();
    let generic = "[Desktop Entry]\nType=Application\nName=System Utility\nExec=/bin/true\n";
    fs::write(system.join("ferroutil.desktop"), generic).unwrap();
    let user_app = format!(
        "[Desktop Entry]\nType=Application\nName=User Utility\nComment=Catalog fixture\nIcon=utilities-terminal\nExec=/usr/bin/touch \"{}\" %U\nOnlyShowIn=FerroUI;\n",
        dir.join("launch marker").display(),
    );
    fs::write(user.join("ferroutil.desktop"), user_app).unwrap();
    fs::write(system.join("masked.desktop"), generic).unwrap();
    fs::write(
        user.join("masked.desktop"),
        format!("{generic}Hidden=true\n"),
    )
    .unwrap();
    fs::write(
        system.join("invisible.desktop"),
        format!("{generic}NoDisplay=true\n"),
    )
    .unwrap();
    fs::write(
        system.join("missing.desktop"),
        format!("{generic}TryExec=/no/such/program\n"),
    )
    .unwrap();
    fs::write(
        system.join("other.desktop"),
        format!("{generic}OnlyShowIn=OtherDesktop;\n"),
    )
    .unwrap();
    let status = Command::new(env::current_exe().unwrap())
        .arg("xdg_precedence_visibility_and_launch")
        .env(FIXTURE, &dir)
        .env("XDG_DATA_HOME", dir.join("user"))
        .env("XDG_DATA_DIRS", dir.join("system"))
        .env("XDG_CURRENT_DESKTOP", "FerroUI")
        .status();
    fs::remove_dir_all(dir).unwrap();
    assert!(status.unwrap().success());
}
