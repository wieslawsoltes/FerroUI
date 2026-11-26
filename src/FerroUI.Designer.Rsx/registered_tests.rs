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
                    r#"{"path":"crate::views::ReviewCard","properties":{"Caption":"heading","SaveCommand":"perform_save","SaveCommandParameter":"save_argument","HeroImage":"hero_raster","BadgeImage":"badge_raster"}}"#],
                    ["FerroUI.Designer.RsxAnimation.v1:crate::motion::reading_radius", "crate::StatusCard::reading_radius_property"],
                    ["FerroUI.Designer.RsxAnimation.v1:crate::StatusCard::relative_radius_property", "crate::StatusCard::relative_radius_property"]],
                "types":[{"name":"StatusCard","namespace":"Example.Views","kind":"class",
                    "rust_path":"::example::StatusCard","object_model":true,
                    "base":"::ferroui_controls::Control","default_constructor":{},
                    "properties":[{"name":"Caption","value_type":"String","setter":{}},
                        {"name":"SaveCommand","value_type":"Option<::std::rc::Rc<dyn ::ferroui_base::input::ICommand>>","setter":{}},
                        {"name":"SaveCommandParameter","value_type":"Option<::ferroui_base::BoxedValue>","setter":{}},
                        {"name":"HeroImage","value_type":"Option<::std::rc::Rc<dyn ::ferroui_base::media::IImage>>","setter":{}},
                        {"name":"BadgeImage","value_type":"Option<::std::rc::Rc<dyn ::ferroui_base::media::IImage>>","setter":{}}],
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
fn registered_image_properties_follow_the_mapped_setter_and_exclude_expressions() {
    use ferroui_designer_core::TextPatch;

    let catalog = Fixture::catalog();
    let source = r#"rsx! { crate::views::ReviewCard {
        hero_raster(::ferroui_view::embedded_image(::core::include_bytes!(concat!("img/", "card.png")))),
    } }"#;
    let projection = catalog.project(source, 0).unwrap();
    assert_eq!(projection.omitted_count, 0);
    assert_eq!(projection.nodes[0].editable_image_properties(), ["BadgeImage", "HeroImage"]);
    assert!(!projection.nodes[0].can_set_image_source());
    assert!(projection.xaml().contains("HeroImage=\"img/card.png\""));
    let patch = projection.edit_property(0, "HeroImage", Some("img/hero.png")).unwrap();
    assert_eq!(TextPatch::apply_all(source, vec![patch]).unwrap(), source.replace("card.png", "hero.png"));
    let patch = projection.edit_property(0, "BadgeImage", Some("img/badge.png")).unwrap();
    let inserted = TextPatch::apply_all(source, vec![patch]).unwrap();
    assert!(inserted.contains("badge_raster: ::ferroui_view::embedded_image("));
    assert!(catalog.project(&inserted, 0).unwrap().xaml().contains("BadgeImage=\"img/badge.png\""));

    let dynamic = source.replace("hero_raster(", "badge_raster(load_badge()), hero_raster(");
    assert_eq!(catalog.project(&dynamic, 0).unwrap().nodes[0].editable_image_properties(), ["HeroImage"]);
}

#[test]
fn registered_command_and_parameter_keep_the_declared_rust_setter_names() {
    use crate::{RsxCommandEdit, RsxCommandSelection};
    use ferroui_designer_core::TextPatch;

    let catalog = Fixture::catalog();
    let source = "rsx! { crate::views::ReviewCard { heading: \"Review\" } }";
    let projection = catalog.project(source, 0).unwrap();
    let document = Document::new(projection.xaml()).unwrap();
    let node = document.ast().root();
    let slots = projection.command_slots(&document, node);
    assert_eq!(slots[0].binding.property, "SaveCommand");
    assert_eq!(slots[0].parameter.as_ref().unwrap().property, "SaveCommandParameter");
    let edits = projection.plan_command(&document, node, &RsxCommandEdit {
        property: "SaveCommand".into(),
        command: RsxCommandSelection::Set { expression: "save()".into() },
        parameter: RsxCommandSelection::Set { expression: "account_id".into() },
    }).unwrap();
    let changed = TextPatch::apply_all(source, edits).unwrap();
    assert!(changed.contains("perform_save: save(), save_argument: account_id,"));
    let updated = catalog.project(&changed, 0).unwrap();
    assert!(updated.xaml().contains("Caption=\"Review\""));
    assert!(!updated.xaml().contains("SaveCommand"));
}
#[test]
fn registered_style_property_projects_typed_custom_control_and_keeps_constructor_comments() {
    use crate::RsxStyleEdit;
    use ferroui_designer_core::TextPatch;

    let catalog = Fixture::catalog();
    let source = r#"rsx! { StackPanel {
        styles: [ferroui_view::style("StatusCard")
            .set(crate::StatusCard::reading_radius_property(),
                ferroui_base::CornerRadius::new(10.0, /* outer */ 20.0, 30.0, 40.0))],
        crate::views::ReviewCard {}
    }}"#;
    let projection = catalog.project(source, 0).unwrap();
    let style = projection.style_definitions()[0].1;
    let setter = &style.setters[0];
    assert_eq!(setter.value().as_deref(), Some("10,20,30,40"));
    assert!(projection.xaml().contains("Selector=\"rsxStyle0|StatusCard\""));
    assert!(projection.xaml().contains("Property=\"rsxOwner:StatusCard.ReadingRadius\""));
    let plan = projection.plan_style(&RsxStyleEdit::Setter {
        style: style.declaration.start, setter: setter.declaration.start, value: Some("10,25,30,40".into()),
    }).unwrap();
    assert_eq!(TextPatch::apply_all(source, plan).unwrap(), source.replace("20.0", "25.0"));
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
    let changes = animation.patch_keyframe("ReadingRadius", KeyframeEdit::Upsert {
        previous: Some(0.0), time: 0.0, value: "10,20,30,40", easing: "LinearEasing",
    }).unwrap();
    assert_eq!(
        ferroui_designer_core::TextPatch::apply_all(&source, changes).unwrap(),
        source.replace("CornerRadius::uniform(0.0)", "CornerRadius::new(10.0, 20.0, 30.0, 40.0)"),
    );
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
    assert!(language["properties"]["crate::views::ReviewCard"]
        .as_array().unwrap().iter().any(|property| property["setter"] == "heading" && property["xamlName"] == "Caption"));
    assert!(
        !language["nodes"][3]["fields"][0]["literal"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn registered_builder_call_round_trips_the_argument_and_reveals_computed_values() {
    use ferroui_designer_core::TextPatch;

    let catalog = Fixture::catalog();
    let source = concat!(
        "fn other() { rsx! { Button { width(42.0) } }; }\r\n",
        "fn view() { rsx! { StackPanel {\r\n",
        "    crate::views::ReviewCard { heading /* 🧭 retain */ (/* caption */ \"Ready\",) }\r\n",
        "    ::widgets::Meter { text(customer.label()) }\r\n",
        "}}; }\r\n",
    );
    let caret = source.find("heading").unwrap();
    let projection = catalog.project(source, caret).unwrap();
    assert_eq!(projection.omitted_count, 1);
    assert!(projection.xaml().contains("Caption=\"Ready\""));
    assert_eq!(&source[projection.nodes[2].expressions["Text"].range()], "customer.label()");
    assert!(projection.edit_property(2, "Text", Some("Assigned")).is_err());
    let change = projection.edit_property(1, "Caption", Some("Reviewed")).unwrap();
    assert_eq!(
        TextPatch::apply_all(source, vec![change]).unwrap(),
        source.replace("\"Ready\"", "\"Reviewed\""),
    );
    let static_source = source.replace("text(customer.label())", "text(\"Loaded\")");
    let projection = catalog.project(&static_source, caret).unwrap();
    assert_eq!(projection.blocks.len(), 2);
    let draft = projection.xaml().replace("Caption=\"Ready\"", "Caption=\"Reviewed\"");
    assert_eq!(
        TextPatch::apply_all(&static_source, projection.apply_xaml(&draft).unwrap()).unwrap(),
        static_source.replace("\"Ready\"", "\"Reviewed\""),
    );
    let language = serde_json::to_value(catalog.analyze(&static_source)).unwrap();
    assert_eq!(language["nodes"][2]["fields"][0]["property"]["xamlName"], "Caption");
    assert_eq!(language["nodes"][2]["fields"][0]["literal"], true);
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
