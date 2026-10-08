//! A measurement, not a check: the eligibility of the markup documents of the
//! two themes for the emitter, each document transformed on its own (without
//! the group of the documents it includes) against the type system of this
//! test crate. Not a test of upstream.
//!
//! ```text
//! cargo test -p ferroui-markup-xaml-tests --lib emitter::repository_documents::measure_theme_documents -- --ignored --exact --nocapture
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::compile_documents;

use crate::support::app::xaml_test_base;

fn documents_under(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            documents_under(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "xaml") {
            found.push(path);
        }
    }
}

#[test]
#[ignore = "a measurement: prints the eligibility of the theme documents"]
fn measure_theme_documents() {
    let _base = xaml_test_base();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // The documents name types of the theme crates.
    ferroui_themes_simple::register_types();
    ferroui_themes_fluent::register_types();
    // Each theme is an assembly: its documents are compiled as one group, by their URIs.
    let mut compiled = Vec::new();
    for (theme, assembly) in [("src/FerroUI.Themes.Simple", "FerroUI.Themes.Simple"), ("src/FerroUI.Themes.Fluent", "FerroUI.Themes.Fluent")] {
        let theme_root = root.join(theme);
        let mut paths = Vec::new();
        documents_under(&theme_root, &mut paths);
        let texts: Vec<(String, String)> = paths
            .iter()
            .map(|path| {
                let name = path.strip_prefix(&theme_root).unwrap_or(path).display().to_string();
                (name, std::fs::read_to_string(path).unwrap_or_default())
            })
            .collect();
        // A document with `x:Class` is the markup of a class (code-behind documents are stage
        // E5): it is counted, not compiled.
        let class_documents = texts.iter().filter(|(_, text)| text.contains("x:Class=")).count();
        // The documents the theme itself leaves out (`Controls/excluded.txt`: they name types
        // that are not ported).
        let excluded_list = std::fs::read_to_string(theme_root.join("Controls/excluded.txt")).unwrap_or_default();
        let excluded: Vec<String> = excluded_list
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter_map(|line| line.split('|').next())
            .map(|file| format!("Controls/{}", file.trim()))
            .filter(|file| file != "Controls/")
            .collect();
        println!(
            "{assembly}: {class_documents} document(s) with x:Class and {} excluded by the theme left out",
            excluded.len()
        );
        let documents: Vec<(&str, &str)> = texts
            .iter()
            .filter(|(name, text)| !text.contains("x:Class=") && !excluded.contains(name))
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .collect();
        let root_uri = format!("ferres://{assembly}/");
        compiled.extend(compile_documents(&documents, Some(&root_uri), &RuntimeXamlLoaderConfiguration::new(), &[]));
    }
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut eligible = 0;
    for document in &compiled {
        match &document.source {
            Ok(_) => eligible += 1,
            Err(reason) => {
                // `FERROUI_EMITTER_REASONS=1` prints the reason of every document (diagnostic).
                if std::env::var("FERROUI_EMITTER_REASONS").is_ok_and(|value| value == "1") {
                    println!("not eligible  {}: {reason}", document.name);
                }
                // The kind of the first unsupported node and the reason, without positions and names.
                let reason = reason.split(" (line").next().unwrap_or(reason);
                let reason = reason.split(" Line ").next().unwrap_or(reason);
                *reasons.entry(reason.chars().take(110).collect()).or_default() += 1;
            }
        }
    }
    println!("{eligible} of {} theme documents are eligible", compiled.len());
    let mut sorted: Vec<_> = reasons.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (reason, count) in sorted {
        println!("{count:5}  {reason}");
    }
}
