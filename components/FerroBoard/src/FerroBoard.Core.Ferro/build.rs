//! Ordinary native source scan plus declaration-generated foreign enum facts.
mod enum_model;
fn main() {
    use ferroui_build_scan::export::{Export, dependencies_from_env, write_if_changed};
    use ferroui_build_scan::model::AssemblyModel;
    use std::path::PathBuf;
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let scanned = out.join("scanned");
    let result = Export::new(
        manifest,
        scanned.clone(),
        "ferroboard_core_ferro",
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
    println!("cargo::rerun-if-changed=enum_model.rs");
    let mut model = AssemblyModel::parse(
        &std::fs::read_to_string(scanned.join("ferroboard_core_ferro.xamlmeta")).unwrap(),
    )
    .unwrap();
    enum_model::append(&mut model);
    let path = out.join("ferroboard_core_ferro.xamlmeta");
    write_if_changed(&path, &model.to_json()).unwrap();
    println!("cargo::metadata=xamlmeta={}", path.display());
}
