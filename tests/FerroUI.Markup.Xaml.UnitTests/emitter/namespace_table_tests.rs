//! The namespace table of a compiled document against the namespaces the run-time loader gives
//! the same document: the compact table has to expand to the same list, in the same order.

use ferroui_markup_xaml::xaml_il::runtime::compiled::expand_xml_namespace_entries;
use ferroui_markup_xaml::xaml_il::runtime::XmlNamespaceTypes;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{compile_documents, transformed_namespace_table};

const DOCUMENT: &str = r#"<Border xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:other="https://github.com/ferroui"
    xmlns:local="using:FerroUI.Controls"
    xmlns:clr="clr-namespace:FerroUI.Media;assembly=FerroUI.Base" />"#;

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

#[test]
fn the_compact_table_expands_to_what_the_loader_resolves() {
    ferroui_controls::register_types();
    let table = transformed_namespace_table("Table.xaml", DOCUMENT, &RuntimeXamlLoaderConfiguration::default())
        .expect("the document has a table");
    let resolved = table.resolved();
    let written = table.written();
    let prefixes = |rows: &[(String, Vec<(String, String)>)]| rows.iter().map(|row| row.0.clone()).collect::<Vec<_>>();
    assert_eq!(prefixes(&resolved), prefixes(&written));
    for ((prefix, expected), (_, entries)) in resolved.iter().zip(&written) {
        let entries: Vec<(&'static str, &'static str, XmlNamespaceTypes)> =
            entries.iter().map(|(first, assembly)| (leak(first), leak(assembly), &[][..])).collect();
        let expanded: Vec<(String, String)> = expand_xml_namespace_entries(&entries)
            .iter()
            .map(|info| (info.clr_namespace(), info.clr_assembly_name()))
            .collect();
        assert_eq!(&expanded, expected, "prefix {prefix:?}");
    }
    let row = |prefix: &str, rows: &[(String, Vec<(String, String)>)]| {
        rows.iter().find(|row| row.0 == prefix).expect("the prefix").1.len()
    };
    assert!(row("", &written) < row("", &resolved), "one reference per assembly");
    assert_eq!(row("local", &written), row("local", &resolved), "a using namespace is written as it is");
    assert_eq!(row("clr", &written), 1);
}

#[test]
fn a_lexical_scope_is_written_compactly_too() {
    ferroui_controls::register_types();
    let document = r#"<Border xmlns="https://github.com/ferroui">
        <Border xmlns:p="https://github.com/ferroui" Tag="inner" />
    </Border>"#;
    let compiled = compile_documents(
        &[("Scope.xaml", document)],
        Some("ferres://App/"),
        &RuntimeXamlLoaderConfiguration::default(),
        &[],
    );
    let source = compiled[0].source.as_ref().expect("the document compiles");
    assert!(!source.contains(r#"("FerroUI.Media", "FerroUI.Base", &[])"#), "{source}");
}
