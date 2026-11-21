use super::RsxStyleEdit;
use crate::{project_rust, RsxProjection};
use ferroui_designer_core::TextPatch;

const VIEW: &str = r##"fn logo() { rsx! { TextBlock { text: "🧭" } } }
fn customer() {
    rsx! {
        StackPanel {
            styles: [
                build_style(model),
                ferroui_view::style(r"#Customer")
                    .set(ferroui_controls::TextBlock::font_size_property(), /* title */ 22.0f64)
                    .set(ferroui_controls::TextBlock::foreground_property(), color_from(model))
                    .nested(ferroui_view::style("^.featured")
                        .set(ferroui_base::Visual::opacity_property(), 0.82))
                    .animation(ferroui_view::animation(ferroui_view::ms(1000))
                        .key_frame(0.5, |key| key.set(ferroui_base::Visual::opacity_property(), 0.5)))
            ],
            TextBlock { name: "Customer", text: "Customer review" }
        }
    }
}"##;

fn project(source: &str) -> RsxProjection<'_> {
    project_rust(
        source,
        source.find("fn customer").unwrap() + "fn customer() {".len() + 6,
    )
    .unwrap()
}

#[test]
fn mixed_styles_project_setters_nested_rules_and_animation_with_exact_edit_ranges() {
    let source = VIEW.replace('\n', "\r\n");
    let projection = project(&source);
    let styles = projection.style_definitions();
    assert_eq!(styles.len(), 2);
    let style = styles[0].1;
    let nested = styles[1].1;
    assert_eq!(style.selector, "#Customer");
    assert!(nested.is_nested());
    assert_eq!(style.setters[0].value().as_deref(), Some("22"));
    assert_eq!(style.setters[1].value(), None);
    assert_eq!(projection.style_issues.len(), 2);
    assert_eq!(projection.preview_markup().animations.len(), 1);
    assert!(projection
        .xaml()
        .contains("Property=\"TextBlock.FontSize\" Value=\"22\""));
    assert!(projection
        .xaml()
        .contains("Property=\"Visual.Opacity\" Value=\"0.82\""));
    assert!(!projection.can_replace_structure());
    assert!(projection.apply_xaml(&projection.xaml()).is_err());

    let patches = projection
        .plan_style(&RsxStyleEdit::Setter {
            style: style.declaration.start,
            setter: style.setters[0].declaration.start,
            value: Some("24.5".into()),
        })
        .unwrap();
    let updated = TextPatch::apply_all(&source, patches).unwrap();
    assert_eq!(updated, source.replace("22.0f64", "24.5f64"));
    assert_eq!(project(&updated).preview_markup().animations.len(), 1);
    assert!(projection
        .plan_style(&RsxStyleEdit::Setter {
            style: style.declaration.start,
            setter: style.setters[1].declaration.start,
            value: None,
        })
        .is_err());
    assert!(projection
        .plan_style(&RsxStyleEdit::Remove {
            style: style.declaration.start
        })
        .is_err());
}

#[test]
fn resolves_named_and_parsed_brush_values_with_the_native_color_parser() {
    let imports = "use ferroui_base::media::{Color as Paint, Colors as Palette};\n";
    for (authored, before, after) in [
        ("\"Teal\"", "#FF008080", "\"#80124568\""),
        (
            r##"Paint::try_parse(/* typed 🧭 */ r#"rgba(16,32,64,0.5)"#).expect("ink")"##,
            "#80102040",
            r##"Paint::try_parse(/* typed 🧭 */ r#"#80124568"#).expect("ink")"##,
        ),
        (
            "Palette:: /* swatch 🧭 */ BLUE_VIOLET",
            "#FF8A2BE2",
            " /* swatch 🧭 */ ::ferroui_base::media::Color::from_uint32(0x80124568)",
        ),
    ] {
        let source = format!("{imports}{}", VIEW.replace("color_from(model)", authored))
            .replace('\n', "\r\n");
        let projection = project(&source);
        let rule = projection.style_definitions()[0].1;
        assert_eq!(rule.setters[1].value().as_deref(), Some(before));
        let patches = projection
            .plan_style(&RsxStyleEdit::Setter {
                style: rule.declaration.start,
                setter: rule.setters[1].declaration.start,
                value: Some("#80124568".into()),
            })
            .unwrap();
        let updated = TextPatch::apply_all(&source, patches).unwrap();
        assert_eq!(updated, source.replace(authored, after));
        assert!(project(&updated).xaml().contains("#80124568"));
    }

    let shadowed = VIEW.replace(
        "fn customer() {",
        "fn customer() {\n    use other_module::Colors as Palette;",
    );
    let shadowed = shadowed.replace("color_from(model)", "Palette::TEAL");
    assert!(project(&shadowed).style_definitions()[0].1.setters[1]
        .value()
        .is_none());
}

#[test]
fn scoped_style_structure_preserves_other_arrays_and_literal_spelling() {
    let projection = project(VIEW);
    let style = projection.style_definitions()[0].1;
    let patches = projection
        .plan_style(&RsxStyleEdit::Selector {
            style: style.declaration.start,
            selector: "#Customer.featured".into(),
        })
        .unwrap();
    let source = TextPatch::apply_all(VIEW, patches).unwrap();
    assert_eq!(
        source,
        VIEW.replace("r\"#Customer\"", "r\"#Customer.featured\"")
    );
    assert!(projection
        .plan_style(&RsxStyleEdit::Selector {
            style: style.declaration.start,
            selector: "^ >".into(),
        })
        .is_err());
    let patches = projection
        .plan_style(&RsxStyleEdit::AddSetter {
            style: style.declaration.start,
            getter: "ferroui_controls::TextBlock::text_property".into(),
            value: "Reviewed 🧭".into(),
        })
        .unwrap();
    let source = TextPatch::apply_all(VIEW, patches).unwrap();
    assert!(
        source.contains(".set(::ferroui_controls::TextBlock::text_property(), \"Reviewed 🧭\")")
    );
    let projected = project(&source);
    let rule = projected.style_definitions()[0].1;
    let patches = projected
        .plan_style(&RsxStyleEdit::Setter {
            style: rule.declaration.start,
            setter: rule.setters[0].declaration.start,
            value: None,
        })
        .unwrap();
    let removed = TextPatch::apply_all(&source, patches).unwrap();
    assert!(removed.contains("/* title */"));
    assert!(!removed.contains("font_size_property"));
    assert!(removed.contains("color_from(model)"));
    assert!(removed.starts_with("fn logo() { rsx! { TextBlock { text: \"🧭\" } } }"));

    let projection = project(&removed);
    let style = projection.style_definitions()[0].1;
    let nested = projection
        .plan_style(&RsxStyleEdit::CreateNested {
            style: style.declaration.start,
            selector: "^.active".into(),
        })
        .unwrap();
    let source = TextPatch::apply_all(&removed, nested).unwrap();
    let projection = project(&source);
    let added = projection.style_definitions()[2].1;
    assert_eq!(added.selector, "^.active");
    let remove = projection
        .plan_style(&RsxStyleEdit::Remove {
            style: added.declaration.start,
        })
        .unwrap();
    assert_eq!(TextPatch::apply_all(&source, remove).unwrap(), removed);
}
