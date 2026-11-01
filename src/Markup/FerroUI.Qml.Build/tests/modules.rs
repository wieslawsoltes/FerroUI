use ferroui_qml_build::Compiler;
use std::path::Path;

#[test]
fn qualified_reexports_use_the_requested_version_and_internal_type() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/module-data");
    let imports = fixture.join("imports");
    let result = Compiler::new()
        .import_path(imports.join("Design").join(".."))
        .compile(fixture.join("app.qml"))
        .unwrap();
    let controls = imports.join("Design/Controls.1");
    for file in [
        controls.join("qmldir"),
        controls.join("tile.qml"),
        controls.join("base.qml"),
        imports.join("Design/Theme/qmldir"),
    ] {
        assert!(
            result.dependencies.contains(&file.canonicalize().unwrap()),
            "missing {}",
            file.display()
        );
    }
    assert!(!result
        .dependencies
        .contains(&controls.join("tile-old.qml").canonicalize().unwrap()));
    assert!(!result
        .dependencies
        .contains(&fixture.join("local/other.qml").canonicalize().unwrap()));
}

#[test]
fn type_keys_are_identical_in_cargo_and_standalone_processes() {
    use std::{path::Path, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/QmlCompiled");
    let compile = |inside_cargo: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ferroui-qmlc"));
        command
            .current_dir(&root)
            .args(["-I", "singletons/imports", "singletons/Main.qml"]);
        if inside_cargo {
            command
                .env("CARGO_PKG_NAME", "qml-compiled-tests")
                .env("CARGO_MANIFEST_DIR", root.canonicalize().unwrap());
        } else {
            command
                .env_remove("CARGO_PKG_NAME")
                .env_remove("CARGO_MANIFEST_DIR");
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    };
    assert_eq!(compile(true), compile(false));
}
