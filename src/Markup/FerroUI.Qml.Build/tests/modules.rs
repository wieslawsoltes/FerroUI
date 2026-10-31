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
