use super::{ResourceColorMode, RsxStyleEdit};
use crate::{RsxProjection, project_rust};
use ferroui_designer_core::TextPatch;
use ferroui_designer_tools::component_palette::PaletteLayer;

const SOURCE: &str = r##"use ferroui_controls::TextBlock as Label;
use ferroui_view::style as rule;
use ferroui_base::styling::ThemeVariant as Appearance;
mod palette {
    use ferroui_base::media::Color as Rgb;
    pub const NAME: &str = "Accent";
    pub const INK: Rgb = PRIMARY;
    const PRIMARY: Rgb = Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135);
}
fn other() { rsx! { TextBlock { text: "Other invocation" } } }
fn view() {
    use self::palette::{INK as COLOR, NAME};
    rsx! { StackPanel {
        styles: [rule("TextBlock")
            .theme_resource(Appearance::default(), NAME, (/* keep reference */ COLOR))
            .set_resource(Label::foreground_property(), NAME)],
        TextBlock { text: "Selected invocation" }
    } }
}"##;

fn project(source: &str) -> RsxProjection<'_> {
    project_rust(source, source.find("    rsx! { StackPanel").unwrap() + 4).unwrap()
}

#[test]
fn resolves_named_imports_and_edits_the_shared_or_local_color_explicitly() {
    let source = SOURCE.replace('\n', "\r\n");
    let projection = project(&source);
    assert!(projection.style_issues.is_empty());
    let rule = projection.style_definitions()[0].1;
    let color = &rule.resources[0];
    assert_eq!(color.key, "Accent");
    assert_eq!(color.layer, Some(PaletteLayer::Default));
    assert_eq!(color.shared.as_ref().unwrap().name, "PRIMARY");
    assert_eq!(
        &source[color.color_source().range()],
        "Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135)"
    );
    assert_eq!(color.color().as_deref(), Some("#FF055B87"));
    assert_eq!(rule.setters[0].resource_key(), Some("Accent"));
    let edit = |mode| RsxStyleEdit::ColorResource {
        style: rule.declaration.start,
        resource: color.declaration.start,
        value: "#EF175C88".into(),
        mode,
    };
    assert!(
        projection
            .plan_style(&edit(ResourceColorMode::Literal))
            .is_err()
    );
    let shared = projection
        .plan_style_source(&edit(ResourceColorMode::Declaration))
        .unwrap();
    assert_eq!(shared.shared_color, Some(color.color_source()));
    let start = projection.invocation_start_after(&shared.patches).unwrap();
    let edited = TextPatch::apply_all(&source, shared.patches).unwrap();
    assert_eq!(
        edited,
        source.replace(
            "Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135)",
            "Rgb::from_argb(239, 23u8, /* brand 🧭 */ 92, 136)"
        )
    );
    let draft = project_rust(&edited, start).unwrap();
    assert!(draft.xaml().contains("Selected invocation"));
    assert!(draft.xaml().contains("#EF175C88"));
    let local = projection
        .plan_style_source(&edit(ResourceColorMode::Local))
        .unwrap();
    assert!(local.shared_color.is_none());
    let edited = TextPatch::apply_all(&source, local.patches).unwrap();
    assert!(edited.contains("Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135)"));
    assert!(edited.contains("/* keep reference */"));
    let draft = project(&edited);
    let color = &draft.style_definitions()[0].1.resources[0];
    assert!(color.shared.is_none());
    assert_eq!(color.color().as_deref(), Some("#EF175C88"));
}

#[test]
fn edits_explicit_color_fields_without_reordering_or_reformatting_them() {
    let literal = "Rgb { r: 0x0C_u8, b: 0b0010_0000, a: 0xFFu8, g: /* 🧭 */ 0o025u8 }";
    let source = SOURCE
        .replace("Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135)", literal)
        .replace('\n', "\r\n");
    let projection = project(&source);
    let style = projection.style_definitions()[0].1;
    let color = &style.resources[0];
    assert_eq!(color.color().as_deref(), Some("#FF0C1520"));
    let plan = projection
        .plan_style_source(&RsxStyleEdit::ColorResource {
            style: style.declaration.start,
            resource: color.declaration.start,
            value: "#A0185060".into(),
            mode: ResourceColorMode::Declaration,
        })
        .unwrap();
    let updated = TextPatch::apply_all(&source, plan.patches).unwrap();
    assert_eq!(
        updated,
        source.replace(
            literal,
            "Rgb { r: 0x18_u8, b: 0b0110_0000, a: 0xA0u8, g: /* 🧭 */ 0o120u8 }"
        )
    );
    assert!(project(&updated).xaml().contains("#A0185060"));
    let computed = source.replace("a: 0xFFu8", "a: brand_alpha()");
    assert!(project(&computed).style_definitions()[0].1.resources[0]
        .color()
        .is_none());
}

#[test]
fn variant_and_constant_aliases_respect_their_declaring_scopes() {
    let source = SOURCE.replace(
        "fn view() {",
        "fn view() {\n    type Appearance = CustomVariant;",
    );
    let projection = project(&source);
    assert_eq!(projection.style_definitions()[0].1.resources[0].layer, None);

    let source = SOURCE.replace(
        "const PRIMARY: Rgb",
        "#[cfg(feature = \"brand\")] const PRIMARY: Rgb",
    );
    let projection = project(&source);
    assert!(
        projection.style_definitions()[0].1.resources[0]
            .color()
            .is_none()
    );

    let source = SOURCE.replace("Rgb::from_rgb(5u8, /* brand 🧭 */ 91, 135)", "INK");
    let projection = project(&source);
    assert!(
        projection.style_definitions()[0].1.resources[0]
            .color()
            .is_none()
    );
}
