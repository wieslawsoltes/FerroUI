use crate::RsxCatalog;
use ferroui_designer_core::Document;
use ferroui_designer_language::Schema;
use ferroui_designer_project::MarkupCatalog;
use serde_json::json;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn catalog() -> RsxCatalog {
        let fixture = Self(std::env::temp_dir().join(format!(
            "ferroui-rsx-catalog-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        )));
        std::fs::create_dir_all(&fixture.0).unwrap();
        let models = [
            (
                "app",
                json!({"format":2,"name":"Example","crate_name":"example",
                "metadata":[["FerroUI.Designer.Rsx.v1:Example.Views.StatusCard",
                    r#"{"path":"crate::views::ReviewCard","properties":{"Caption":"heading"}}"#]],
                "types":[{"name":"StatusCard","namespace":"Example.Views","kind":"class",
                    "rust_path":"::example::StatusCard","object_model":true,
                    "base":"::ferroui_controls::Control","default_constructor":{},
                    "properties":[{"name":"Caption","value_type":"String","setter":{}}]}]}),
            ),
            (
                "dependency",
                json!({"format":2,"name":"Widgets","crate_name":"widgets",
                "metadata":[["FerroUI.Designer.Rsx.v1:Widgets.Views.Meter",
                    r#"{"path":"::widgets::Meter","inherit":true}"#]],
                "types":[{"name":"Meter","namespace":"Widgets.Views","kind":"class",
                    "rust_path":"::widgets::Meter","object_model":true,
                    "base":"::ferroui_controls::TextBlock","default_constructor":{}}]}),
            ),
            (
                "controls",
                json!({"format":2,"name":"FerroUI.Controls","crate_name":"ferroui_controls",
                "types":[{"name":"TextBlock","namespace":"FerroUI.Controls","kind":"class",
                    "rust_path":"::ferroui_controls::TextBlock","object_model":true,
                    "base":"::ferroui_controls::Control"}]}),
            ),
        ];
        for (name, model) in models {
            std::fs::write(
                fixture.0.join(format!("{name}.xamlmeta")),
                model.to_string(),
            )
            .unwrap();
        }
        let project = MarkupCatalog::load(std::slice::from_ref(&fixture.0)).unwrap();
        let mut schema = Schema::ferroui();
        schema.register_project(&project);
        RsxCatalog::from_project(&project, &schema, Some("example"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn registered_literally_mapped_component_keeps_other_invocations_and_dynamic_children() {
    let catalog = Fixture::catalog();
    let source = concat!(
        "let other = rsx! { Button {} };\r\n",
        "let view = rsx! { StackPanel {\r\n",
        "    // preserve this comment\r\n",
        "    crate::views::ReviewCard { heading: \"🦀 First\" }\r\n",
        "    ::widgets::Meter { text: selected.label }\r\n",
        "    for person in people { TextBlock { text: person.name } }\r\n",
        "}};"
    );
    let caret = source.find("heading").unwrap();
    let projection = catalog.project(source, caret).unwrap();
    assert_eq!(projection.blocks.len(), 2);
    assert!(projection.xaml().contains("<ui:StatusCard xmlns:ui=\"clr-namespace:Example.Views;assembly=Example\" Caption=\"🦀 First\""));
    assert_eq!(projection.omitted_count, 2);
    assert!(
        projection
            .edit_property(2, "Text", Some("Replacement"))
            .is_err()
    );
    let patch = projection
        .edit_property(1, "Caption", Some("Second"))
        .unwrap();
    let mut edited = source.to_owned();
    edited.replace_range(patch.span.range(), &patch.replacement);
    assert_eq!(edited, source.replace("🦀 First", "Second"));
    let language = serde_json::to_value(catalog.analyze(source)).unwrap();
    assert_eq!(
        language["properties"]["crate::views::ReviewCard"][0]["setter"],
        "heading"
    );
    assert!(
        !language["nodes"][3]["fields"][0]["literal"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn registered_round_trip_uses_qualified_rust_paths_and_retains_authored_xaml_aliases() {
    let catalog = Fixture::catalog();
    let document =
        Document::new("<custom:StatusCard xmlns:custom='using:Example.Views' Caption='First' />")
            .unwrap();
    let draft = catalog.xaml_to_rsx(&document).unwrap();
    assert!(draft.contains("crate::views::ReviewCard"));
    assert!(draft.contains("heading: \"First\""));
    let patches = catalog
        .apply_rsx(&document, &draft.replace("First", "Second & Third"))
        .unwrap();
    assert_eq!(patches.len(), 1);
    let mut edited = document.source().to_owned();
    edited.replace_range(patches[0].span.range(), &patches[0].replacement);
    assert_eq!(
        edited,
        "<custom:StatusCard xmlns:custom='using:Example.Views' Caption='Second &amp; Third' />"
    );
    let meter = Document::new(
        "<m:Meter xmlns:m='clr-namespace:Widgets.Views;assembly=Widgets' Text='Measure' />",
    )
    .unwrap();
    assert!(
        catalog
            .xaml_to_rsx(&meter)
            .unwrap()
            .contains("::widgets::Meter")
    );
    assert!(
        catalog
            .project("rsx! { ReviewCard { heading: \"Unknown import\" } }", 0)
            .is_err()
    );
    assert!(RsxCatalog::built_in().xaml_to_rsx(&document).is_err());
}
