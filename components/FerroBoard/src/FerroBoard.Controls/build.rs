fn main() {
    let java =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("java");
    println!("cargo::rerun-if-changed={}", java.display());
    println!("cargo::metadata=android_java={}", java.display());

    use ferroui_build_scan::export::{Export, dependencies_from_env, write_if_changed};
    use ferroui_build_scan::model::{
        AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel,
    };
    use std::path::PathBuf;
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let result = Export::new(
        root,
        out.join("scanned"),
        "ferroboard_controls",
        dependencies_from_env(),
    )
    .execute();
    for line in result
        .lines
        .iter()
        .filter(|line| !line.starts_with("cargo::metadata=xamlmeta="))
    {
        println!("{line}");
    }
    assert!(
        result.errors.is_empty(),
        "metadata scan: {:?}",
        result.errors
    );
    let mut model = AssemblyModel::parse(
        &std::fs::read_to_string(out.join("scanned/ferroboard_controls.xamlmeta")).unwrap(),
    )
    .unwrap();
    let mut mode = TypeModel::new(
        "KeyboardVisibilityMode",
        TypeKind::Enum,
        RustType::resolved("::ferroboard_controls::KeyboardVisibilityMode"),
        "ferroboard_controls",
    );
    mode.namespace = "FerroBoard.Controls".into();
    mode.explicit_namespace = Some("FerroBoard.Controls".into());
    mode.public_path = Some("::ferroboard_controls::KeyboardVisibilityMode".into());
    mode.handles = vec![RustType::resolved(
        "::ferroboard_controls::KeyboardVisibilityMode",
    )];
    for (name, value) in [("Auto", 0), ("AlwaysVisible", 1), ("Manual", 2)] {
        mode.enum_members.push(EnumMemberModel {
            name: name.into(),
            rust_variant: Some(name.into()),
            rust_value: None,
            value: Some(value),
        });
    }
    model.types.push(mode);
    let path = out.join("ferroboard_controls.xamlmeta");
    write_if_changed(&path, &model.to_json()).unwrap();
    println!("cargo::metadata=xamlmeta={}", path.display());
}
