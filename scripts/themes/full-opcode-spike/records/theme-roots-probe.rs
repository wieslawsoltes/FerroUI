//! Reproduce the consumer, multi-target and external resource cases in the November size review.
//! Run at the repository root with an unused output directory and the built WinUI .xamlmeta.
use ferroui_build::theme_roots::{kept_units, Roots, TypeGraph};
use ferroui_build::{Build, TypeSystem, XamlGroup};
use ferroui_build_scan::xaml_metadata::XamlMetadata;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn ordinary(root: &Path, name: &str, inputs: Vec<PathBuf>, document: &str, lib: &str, main: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), lib).unwrap();
    fs::write(dir.join("src/main.rs"), main).unwrap();
    let out = dir.join("out");
    let result = Build::new(dir, out.clone(), name, inputs)
        .type_system(TypeSystem::Model)
        .compile_group(XamlGroup::new("view").documents(&[("View.xaml", document)]))
        .execute();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let listing = fs::read_to_string(out.join("xaml/theme_parts.txt")).unwrap();
    let generated = fs::read_to_string(out.join("xaml/view.rs")).unwrap();
    let table = listing.lines().next().unwrap_or("empty table");
    let listbox = listing.lines().any(|s| s == "Controls/ListBox.xaml");
    println!("{name}: {table}, ListBox unit={listbox}, with_parts={}", generated.contains("::with_parts("));
    out.join(format!("{name}.xamlmeta"))
}

fn resource_graph(root: &Path, meta: &Path, name: &str, entry: &str) {
    let out = root.join(name);
    let app = r#"<Application xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="HelloWorld.App">
        <Application.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries>
            <MergeResourceInclude Source="/Part.xaml" />
        </ResourceDictionary.MergedDictionaries></ResourceDictionary></Application.Resources>
    </Application>"#;
    let part = format!(r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">{entry}</ResourceDictionary>"#);
    let result = Build::new(PathBuf::from("samples/HelloWorld"), out.clone(), "hello-world", vec![meta.to_owned()])
        .type_system(TypeSystem::Model)
        .compile_group(XamlGroup::new("view").class_document("App.xaml").documents(&[("App.xaml", app), ("Part.xaml", &part)]))
        .execute();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let models = XamlMetadata::read(out.join("hello_world.xamlmeta")).unwrap();
    let graph = models.iter().flat_map(|m| &m.theme_units).find(|t| t.class == "HelloWorld.App").unwrap();
    let mut roots = Roots::default();
    roots.keys.insert("UsedA".to_string());
    let kept = kept_units(graph, &roots, &TypeGraph::default(), false);
    let unit = graph.units.iter().find(|u| u.name == "Part.xaml").unwrap();
    println!("{name}: keys={:?}, references={:?}, kept={kept:?}", unit.keys, unit.references);
}

fn main() {
    let mut args = env::args_os().skip(1);
    let root = PathBuf::from(args.next().expect("output directory"));
    let meta = PathBuf::from(args.next().expect("WinUI .xamlmeta"));
    fs::create_dir(&root).expect("create a new directory for the fixture sources and build outputs");
    let theme = r#"<Styles xmlns="https://github.com/ferroui"><WinUITheme /></Styles>"#;
    let listbox = r#"<ListBox xmlns="https://github.com/ferroui" />"#;
    let code = "use ferroui_controls::ListBox; pub fn make() { let _ = ListBox::new(); }";
    let library = ordinary(&root, "producer", vec![meta.clone()], theme, "", "fn main() {}");
    ordinary(&root, "consumer", vec![meta.clone(), library], listbox, "", "fn main() {}");
    ordinary(&root, "binroot", vec![meta.clone()], theme, "", &format!("{code} fn main() {{make();}}"));
    ordinary(&root, "libroot", vec![meta.clone()], theme, code, "fn main() {}");
    resource_graph(&root, &meta, "theme_a", r#"<SolidColorBrush x:Key="UsedA" Color="{DynamicResource BOnly}" />"#);
    resource_graph(&root, &meta, "theme_b", r##"<Color x:Key="BOnly">#ff0000</Color>"##);
}
