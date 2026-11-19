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
                    r#"{"path":"crate::views::ReviewCard","properties":{"Caption":"heading"}}"#],
                    ["FerroUI.Designer.RsxAnimation.v1:crate::motion::reading_radius", "crate::StatusCard::reading_radius_property"],
                    ["FerroUI.Designer.RsxAnimation.v1:crate::StatusCard::relative_radius_property", "crate::StatusCard::relative_radius_property"]],
                "types":[{"name":"StatusCard","namespace":"Example.Views","kind":"class",
                    "rust_path":"::example::StatusCard","object_model":true,
                    "base":"::ferroui_controls::Control","default_constructor":{},
                    "properties":[{"name":"Caption","value_type":"String","setter":{}}],
                    "registered":[{"accessor":"reading_radius_property","name":"ReadingRadius","kind":"styled",
                        "value_type":"::ferroui_base::corner_radius::CornerRadius","registration":"declared"},
                        {"accessor":"relative_radius_property","name":"RelativeRadius","kind":"styled",
                        "value_type":"::ferroui_base::RelativeScalar","registration":"declared"}]}]}),
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

#[test]
fn registered_typed_keys_keep_comments_other_invocations_and_opaque_clips() {
    use ferroui_designer_animation::{KeyframeEdit, ValueKind};

    let source = r##"fn other() { rsx! { TextBlock { text: "🦀" } } }
    fn view() {
        let card = rsx! { StackPanel {
            styles: [ferroui_view::style("#Review")
                .animation(ferroui_view::animation(ferroui_view::ms(2000))
                    .key_frame(0.0, |key| key.set(crate::motion::reading_radius(),
                        ferroui_base::CornerRadius::uniform(0.0)))
                    .key_frame(0.5, |key| key.set(crate::motion::reading_radius(),
                        ferroui_base::CornerRadius::new(24.0, /* top-right */ 28.0, 20.0, 16.0)))
                    .iterations(ferroui_base::animation::IterationCount::new(2)))
                .animation(make_animation(model))],
            crate::views::ReviewCard {}
        }};
    }"##.replace('\n', "\r\n");
    let caret = source.find("let card").unwrap() + "let card = ".len();
    let catalog = Fixture::catalog();
    let projection = catalog.project(&source, caret).unwrap();
    assert_eq!(projection.omitted_count, 1);
    let markup = projection.preview_markup();
    assert!(markup.source.contains("x:SetterTargetType=\"motion:StatusCard\""));
    assert!(markup.source.contains("IterationCount=\"2\""));
    assert!(markup.source.contains("Property=\"ReadingRadius\" Value=\"24,28,20,16\""));
    let animation = markup.animations[0].1;
    assert!(animation.maps_key("ReadingRadius", 1.0, ValueKind::CornerRadius));
    let changes = animation.patch_keyframe("ReadingRadius", KeyframeEdit::Upsert {
        previous: Some(1.0), time: 1.5, value: "24,30,20,18", easing: "LinearEasing",
    }).unwrap();
    let mut updated = source.clone();
    for patch in changes.into_iter().rev() {
        updated.replace_range(patch.span.range(), &patch.replacement);
    }
    assert_eq!(updated, source.replace("key_frame(0.5,", "key_frame(0.75,")
        .replace("/* top-right */ 28.0, 20.0, 16.0", "/* top-right */ 30.0, 20.0, 18.0"));
    assert!(catalog.project(&updated, caret).unwrap().xaml().contains("Value=\"24,30,20,18\""));
    assert!(animation.patch_keyframe("ReadingRadius", KeyframeEdit::Upsert {
        previous: Some(0.0), time: 0.0, value: "10,20,30,40", easing: "LinearEasing",
    }).is_err());
    assert!(RsxCatalog::built_in().project(&source, caret).unwrap().preview_markup().animations.is_empty());
    let changes = animation.author_keyframe(&source, &catalog, "ReadingRadius", KeyframeEdit::Upsert {
        previous: None, time: 1.5, value: "24,30,20,18", easing: "LinearEasing",
    }).unwrap();
    assert_eq!(changes.len(), 1);
    assert!(changes[0].span.is_empty());
    assert_eq!(changes[0].replacement, concat!(
        "\r\n                    .key_frame(0.75, |key| key.set(crate::motion::reading_radius(), ",
        "::ferroui_base::CornerRadius::new(24.0, 30.0, 20.0, 18.0)))"
    ));
    let inserted = ferroui_designer_core::TextPatch::apply_all(&source, changes).unwrap();
    let inserted = catalog.project(&inserted, caret).unwrap();
    let markup = inserted.preview_markup();
    assert!(markup.animations[0].1.maps_key("ReadingRadius", 1.5, ValueKind::CornerRadius));
    assert_eq!(inserted.omitted_count, 1);
}

#[test]
fn registered_relative_scalar_keys_support_unit_edits_and_typed_insertion() {
    use ferroui_designer_animation::{KeyframeEdit, ValueKind};
    use ferroui_designer_core::TextPatch;

    let source = r##"fn view() { rsx! { StackPanel {
        styles: [ferroui_view::style("#Review").animation(ferroui_view::animation(ferroui_view::ms(2000))
            .key_frame(0.0, |key| key.set(crate::StatusCard::relative_radius_property(),
                ferroui_base::RelativeScalar::new(/* radius 🧭 */ 0.25f64, ferroui_base::RelativeUnit::Relative))))],
        crate::views::ReviewCard { name: "Review" }
    }}}"##.replace('\n', "\r\n");
    let caret = source.find("rsx!").unwrap();
    let catalog = Fixture::catalog();
    let projection = catalog.project(&source, caret).unwrap();
    let markup = projection.preview_markup();
    assert!(markup.source.contains("Property=\"RelativeRadius\" Value=\"25%\""));
    let clip = markup.animations[0].1;
    let edited = TextPatch::apply_all(&source, clip.patch_keyframe("RelativeRadius", KeyframeEdit::Upsert {
        previous: Some(0.0), time: 0.0, value: "12.5", easing: "LinearEasing",
    }).unwrap()).unwrap();
    assert_eq!(edited, source.replace("0.25f64, ferroui_base::RelativeUnit::Relative", "12.5f64, ferroui_base::RelativeUnit::Absolute"));
    let inserted = clip.author_keyframe(&source, &catalog, "RelativeRadius", KeyframeEdit::Upsert {
        previous: None, time: 1.0, value: "60%", easing: "LinearEasing",
    }).unwrap();
    assert!(inserted[0].replacement.contains("::ferroui_base::RelativeScalar::new(0.6, ::ferroui_base::RelativeUnit::Relative)"));
    let inserted = TextPatch::apply_all(&source, inserted).unwrap();
    let projection = catalog.project(&inserted, caret).unwrap();
    assert!(projection.preview_markup().animations[0].1.maps_key("RelativeRadius", 1.0, ValueKind::RelativeScalar));
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
