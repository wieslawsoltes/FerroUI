//! Exports source-scanned metadata plus original foreign enum descriptors.
mod foreign_enum_build;
mod foreign_flag_build;
fn main() {
    use ferroui_build_scan::export::{dependencies_from_env, write_if_changed, Export};
    use ferroui_build_scan::model::AssemblyModel;
    use std::path::PathBuf;
    let manifest =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("build output directory"));
    let crate_name = "ferrodock_model_ferro";
    // Keep the normal scanner output separate, so an unchanged final export
    // is never temporarily overwritten and does not spuriously rebuild consumers.
    let scanned = out.join("scanned");
    let outcome = Export::new(
        manifest.clone(),
        scanned.clone(),
        crate_name,
        dependencies_from_env(),
    )
    .execute();
    for line in outcome
        .lines
        .iter()
        .filter(|l| !l.starts_with("cargo::metadata=xamlmeta="))
    {
        println!("{line}");
    }
    assert!(
        outcome.errors.is_empty(),
        "shared metadata scan failed: {:?}",
        outcome.errors
    );
    println!("cargo::rerun-if-changed=foreign_enum_build.rs");
    let text = std::fs::read_to_string(scanned.join(format!("{crate_name}.xamlmeta")))
        .expect("shared scanner export");
    let mut model = AssemblyModel::parse(&text).expect("shared assembly model");
    foreign_enum_build::dock_operation(&out, &mut model);
    let path = out.join(format!("{crate_name}.xamlmeta"));
    write_if_changed(&path, &model.to_json()).expect("write standard enum-augmented export");
    println!("cargo::metadata=xamlmeta={}", path.display());
}
