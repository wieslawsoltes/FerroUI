use super::RsxStyleEdit;
use crate::{RsxProjection, project_rust};
use ferroui_designer_core::{Document, TextPatch};

const VIEW: &str = r##"fn earlier() { rsx! { TextBlock { text: "🧭" } } }
fn customer() { rsx! {
    StackPanel {
        styles: [
            ferroui_view::style("TextBlock")
                .resource("Ink", ferroui_base::media::Color::from_rgb(10u8, /* channel */ 31, 55))
                .nested(ferroui_view::style("^.selected")
                    .set(ferroui_controls::TextBlock::foreground_property(), "#345678")
                    .set(ferroui_controls::TextBlock::font_size_property(), other_size())),
        ],
        TextBlock { classes: "selected", text: owner_name() }
    }
} }"##;

fn project(source: &str) -> RsxProjection<'_> {
    project_rust(
        source,
        source.find("fn customer").unwrap() + "fn customer() { rsx".len(),
    )
    .unwrap()
}

#[test]
fn assigns_a_scoped_color_and_maps_the_original_setter() {
    let source = VIEW.replace('\n', "\r\n");
    let projection = project(&source);
    let definitions = projection.style_definitions();
    let nested = definitions[1].1;
    let style = nested.declaration.start;
    assert_eq!(projection.style_resource_choices(style)[0].key, "Ink");
    let setter = nested.setters[0].declaration.start;
    let plan = |key: &str| {
        projection.plan_style(&RsxStyleEdit::SetResource {
            style,
            key: key.into(),
            setter: Some(setter),
            getter: None,
        })
    };
    assert!(plan("Missing").is_err());
    let linked = TextPatch::apply_all(&source, plan("Ink").unwrap()).unwrap();
    assert_eq!(
        linked,
        source.replace(
            ".set(ferroui_controls::TextBlock::foreground_property(), \"#345678\")",
            ".set_resource(ferroui_controls::TextBlock::foreground_property(), \"Ink\")"
        )
    );
    let projection = project(&linked);
    let (xaml, map) = projection.mapped_markup();
    assert!(xaml.contains("<Style.Resources>"));
    assert!(xaml.contains("<DynamicResource ResourceKey=\"Ink\"/>"));
    let document = Document::new(xaml).unwrap();
    let setter = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "Setter")
        .unwrap();
    assert_eq!(
        &linked[map.style_setter(setter.span.start).unwrap().range()],
        "\"Ink\""
    );
    let style = projection.style_definitions()[0].1;
    let change = projection
        .plan_style(&RsxStyleEdit::ColorResource {
            style: style.declaration.start,
            resource: style.resources[0].declaration.start,
            value: "#FF0A2037".into(),
        })
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&linked, change).unwrap(),
        linked.replace("/* channel */ 31", "/* channel */ 32")
    );
}

#[test]
fn creates_and_projects_keys_with_xaml_punctuation() {
    let projection = project(VIEW);
    let style = projection.style_definitions()[0].1.declaration.start;
    let plan = |key: &str| {
        projection.plan_style(&RsxStyleEdit::AddColorResource {
            style,
            key: key.into(),
            value: "#DB135189".into(),
        })
    };
    assert!(plan("Ink").is_err());
    assert!(plan("\n").is_err());
    let key = "{Ink & \"Review\"}";
    let source = TextPatch::apply_all(VIEW, plan(key).unwrap()).unwrap();
    let projection = project(&source);
    assert!(
        projection
            .xaml()
            .contains("x:Key=\"{}{Ink &amp; &quot;Review&quot;}\"")
    );
    let plan = projection
        .plan_style(&RsxStyleEdit::SetResource {
            style,
            key: key.into(),
            setter: None,
            getter: Some("ferroui_controls::TextBlock::foreground_property".into()),
        })
        .unwrap();
    let linked = TextPatch::apply_all(&source, plan).unwrap();
    assert!(
        project(&linked)
            .xaml()
            .contains("ResourceKey=\"{}{Ink &amp; &quot;Review&quot;}\"")
    );
}
