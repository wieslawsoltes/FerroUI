//! Hands the type models of the dependencies (`DEP_<LINKS>_XAMLMETA`: the base crate, the controls
//! and the Fluent theme with its units) to the test that writes the class entries
//! (`tests/types_tests.rs`), as `FERROUI_CAPI_XAMLMETA`, a list of paths in the form of `PATH`.
//! Nothing the library compiles is generated here.

fn main() {
    let mut paths: Vec<(String, std::ffi::OsString)> = std::env::vars_os()
        .filter_map(|(name, path)| {
            let name = name.into_string().ok()?;
            (name.starts_with("DEP_") && name.ends_with("_XAMLMETA")).then_some((name, path))
        })
        .collect();
    paths.sort();
    let joined = std::env::join_paths(paths.into_iter().map(|(_, path)| path)).unwrap_or_default();
    println!("cargo::rustc-env=FERROUI_CAPI_XAMLMETA={}", joined.to_string_lossy());
    println!("cargo::rerun-if-changed=build.rs");
}
