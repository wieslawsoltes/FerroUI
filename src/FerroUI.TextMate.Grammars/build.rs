use std::{
    env, fs,
    path::{Path, PathBuf},
};

// The demo offers these four languages. HTML also includes the CSS grammar.
const EDITOR_DEMO_PACKAGES: &[&str] = &["csharp", "css", "html", "javascript", "rust"];

fn editor_demo_resource(name: &str) -> bool {
    let Some((package, _)) = name
        .strip_prefix("grammars/")
        .and_then(|path| path.split_once('/'))
    else {
        return true;
    };
    EDITOR_DEMO_PACKAGES.contains(&package)
}

fn visit(root: &Path, path: &Path, output: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = fs::read_dir(path)
        .expect("resource directory")
        .map(|e| e.expect("resource entry").path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            visit(root, &entry, output);
        } else if entry
            .extension()
            .is_some_and(|e| e == "json" || e == "code-snippets" || e == "tmLanguage")
        {
            output.push((
                entry
                    .strip_prefix(root)
                    .expect("resource path")
                    .to_string_lossy()
                    .replace('\\', "/"),
                entry,
            ));
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=resources");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("output directory"));
    let root = manifest.join("resources");
    let mut resources = Vec::new();
    visit(&root, &root, &mut resources);
    // Cargo unifies features, so a host asking for the full bundle still gets it.
    if env::var_os("CARGO_FEATURE_EDITOR_DEMO").is_some()
        && env::var_os("CARGO_FEATURE_FULL_CATALOG").is_none()
    {
        resources.retain(|(name, _)| editor_demo_resource(name));
    }
    let mut generated = String::from("pub static RESOURCES: &[(&str, &str)] = &[\n");
    for (name, path) in resources {
        generated.push_str(&format!(
            "({name:?}, include_str!({:?})),\n",
            path.to_string_lossy()
        ));
    }
    generated.push_str("];\n");
    fs::write(out_dir.join("resources.rs"), generated).expect("write resource table");
}
