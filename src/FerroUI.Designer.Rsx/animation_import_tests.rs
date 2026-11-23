use crate::RsxCatalog;
use ferroui_designer_animation::KeyframeEdit;
use ferroui_designer_core::TextPatch;

const VIEW: &str = r###"use ferroui_view::{prelude::*, animation as clip};
use ferroui_controls::Border as Card;
use ferroui_base::{RelativePoint as Pivot, RelativeUnit::Relative as Percent, Visual as Paintable};
use ferroui_base::animation::FillMode::Both as Hold;
use ferroui_base::media::{
    Color as Paint, IBrush as Brush, IEffect as Effect, ImmutableDropShadowDirectionEffect as Shadow,
    GradientSpreadMethod::Pad as Edge,
    immutable::{ImmutableLinearGradientBrush as Ramp, ImmutableGradientStop as Stop},
};
use std::rc::Rc as Shared;

fn caption() -> Element { rsx! { TextBlock { text: "Separate invocation 🧭" } } }
fn view() -> Element {
    rsx! { StackPanel {
        styles: [style("#Review").animation(clip(ms(2000))
            .key_frame(0.0, |key| key
                .set(Card::background_property(), Some(Shared::new(Ramp::new(
                    &[Stop::new(/* stop 🧭 */ 0.25f64, Paint::from_uint32(0xFF223344))],
                    1.0, None, None, Edge, None, None, None,
                )) as Shared<dyn Brush>))
                .set(Paintable::render_transform_origin_property(), Pivot::new(0.2, 0.3, Percent))
                .set(Paintable::effect_property(), Some(Shared::new(Shadow::new(
                    /* direction */ 24.0f64, 4.0, 2.0, Paint::from_rgb(32, 54, 87), 0.6,
                )) as Shared<dyn Effect>)))
            .fill_mode(Hold))],
        Border { name: "Review", background: "#223344" }
        TextBlock { text: String::from("computed") }
    } }
}
"###;

fn edit(previous: Option<f64>, time: f64, value: &str) -> KeyframeEdit<'_> {
    KeyframeEdit::Upsert {
        previous,
        time,
        value,
        easing: "LinearEasing",
    }
}

#[test]
fn resolves_imported_factories_getters_and_constructors_for_key_edits() {
    let source = VIEW.replace('\n', "\r\n");
    let catalog = RsxCatalog::built_in();
    let caret = source.rfind("rsx!").unwrap();
    let view = catalog.project(&source, caret).unwrap();
    let markup = view.preview_markup();
    let clip = markup.animations[0].1;
    assert!(clip.xaml.contains("SetterTargetType=\"Border\""));
    assert!(
        clip.xaml
            .contains("Property=\"RenderTransformOrigin\" Value=\"20%,30%\"")
    );
    let patch = clip
        .patch_keyframe("RenderTransformOrigin", edit(Some(0.0), 0.0, "20,30"))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patch).unwrap(),
        source.replace(
            "Pivot::new(0.2, 0.3, Percent)",
            "Pivot::new(20.0, 30.0, ::ferroui_base::RelativeUnit::Absolute)"
        )
    );

    let patch = clip
        .patch_keyframe(
            "Background",
            edit(
                Some(0.0),
                0.0,
                r##"
        <LinearGradientBrush SpreadMethod="Reflect" TransformOrigin="0%,0%">
            <GradientStop Offset="0.42" Color="#FF223344" />
        </LinearGradientBrush>"##,
            ),
        )
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patch).unwrap(),
        source.replace("0.25f64", "0.42f64").replace(
            "None, Edge, None",
            "None, ::ferroui_base::media::GradientSpreadMethod::Reflect, None"
        )
    );

    let value = r##"<DropShadowDirectionEffect Direction="58" ShadowDepth="4" BlurRadius="2" Color="#FF203657" Opacity="0.6"/>"##;
    let patch = clip
        .patch_keyframe("Effect", edit(Some(0.0), 0.0, value))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patch).unwrap(),
        source.replace("24.0f64", "58.0f64")
    );

    let inserted = clip
        .author_keyframe(&source, catalog, "Effect", edit(None, 1.0, value))
        .unwrap();
    let with_key = TextPatch::apply_all(&source, inserted).unwrap();
    assert!(with_key.contains("key.set(Paintable::effect_property(), "));
    let next = catalog.project(&with_key, caret).unwrap();
    let removed = next.preview_markup().animations[0]
        .1
        .author_keyframe(&with_key, catalog, "Effect", KeyframeEdit::Remove(1.0))
        .unwrap();
    assert_eq!(TextPatch::apply_all(&with_key, removed).unwrap(), source);

    let border = view
        .nodes
        .iter()
        .position(|node| node.properties.contains_key("Name"))
        .unwrap();
    let renamed = TextPatch::apply_all(
        &source,
        view.edit_control_names(&[border], Some("Approval"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(renamed, source.replace("Review", "Approval"));
}

#[test]
fn a_shadowed_factory_keeps_the_authored_clip_in_source() {
    let source = VIEW.replace(
        "fn view() -> Element {",
        "fn view() -> Element { fn clip(_: ()) {}",
    );
    let rejected = RsxCatalog::built_in()
        .project(&source, source.rfind("rsx!").unwrap())
        .unwrap();
    assert!(rejected.preview_markup().animations.is_empty());
    assert_eq!(rejected.animation_issues.len(), 1);
    let source = VIEW.replace(
        "rsx! { StackPanel",
        "{ use another_module::*; rsx! { StackPanel",
    ) + "}";
    let ambiguous = RsxCatalog::built_in()
        .project(&source, source.rfind("rsx!").unwrap())
        .unwrap();
    assert!(ambiguous.preview_markup().animations.is_empty());
}

#[test]
fn solid_brush_opacity_promotes_only_the_edited_constructor() {
    let source = r###"use ferroui_view::prelude::*;
use ferroui_base::media::{IBrush, Color, immutable::ImmutableSolidColorBrush as Solid};
use ferroui_controls::Border as Card;
use std::rc::Rc;
fn view() -> Element { rsx! { Border {
    name: "Review",
    styles: [style("Border#Review").animation(animation(ms(1000))
        .key_frame(0.0, |key| key.set(Card::background_property(),
            Some(Rc::new(Solid::from_uint32(/* keep 🧭 */ 0xFF22_3344_u32,)) as Rc<dyn IBrush>)))
        .key_frame(1.0, |key| key.set(Card::background_property(),
            Some(Rc::new(Solid::new(Color::from_rgb(34, 51, 68))) as Rc<dyn IBrush>))))],
} } }
"###
    .replace('\n', "\r\n");
    let caret = source.find("rsx!").unwrap();
    let catalog = RsxCatalog::built_in();
    let view = catalog.project(&source, caret).unwrap();
    let markup = view.preview_markup();
    let clip = markup.animations[0].1;
    let value = r##"<SolidColorBrush Color="#FF663344" Opacity="0.6"/>"##;
    let patch = clip
        .patch_keyframe("Background", edit(Some(0.0), 0.0, value))
        .unwrap();
    let edited = TextPatch::apply_all(&source, patch).unwrap();
    assert_eq!(edited, source.replace(
        "Solid::from_uint32(/* keep 🧭 */ 0xFF22_3344_u32,)",
        "Solid::with_opacity(/* keep 🧭 */ ::ferroui_base::media::Color::from_uint32(0xFF66_3344_u32), 0.6,)"
    ));
    assert!(
        catalog
            .project(&edited, caret)
            .unwrap()
            .preview_markup()
            .animations[0]
            .1
            .xaml
            .contains("Opacity=\"0.6\"")
    );
    let patch = clip
        .patch_keyframe("Background", edit(Some(1.0), 1.0, value))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patch).unwrap(),
        source.replace(
            "Solid::new(Color::from_rgb(34, 51, 68))",
            "Solid::with_opacity(Color::from_rgb(102, 51, 68), 0.6)"
        )
    );
}

#[test]
fn asymmetric_geometry_edits_expand_only_the_shorthand_that_needs_it() {
    use crate::animation::ValueShape;

    let cases = [
        (
            "CornerRadius",
            "ferroui_base::CornerRadius::uniform(/* keep 🧭 */ 8.0f64,)",
            "8,24,8,8",
            "ferroui_base::CornerRadius::new(/* keep 🧭 */ 8.0f64, 24.0, 8.0, 8.0,)",
        ),
        (
            "CornerRadius",
            "ferroui_base::CornerRadius::top_bottom(6.0f64, /* bottom */ 8.0)",
            "6,6,8,10",
            "ferroui_base::CornerRadius::new(6.0f64, 6.0, /* bottom */ 8.0, 10.0)",
        ),
        (
            "CornerRadius",
            "ferroui_base::CornerRadius::top_bottom(6.0, /* bottom */ 8.0)",
            "6,6,10,10",
            "ferroui_base::CornerRadius::top_bottom(6.0, /* bottom */ 10.0)",
        ),
        (
            "Thickness",
            "ferroui_base::Thickness::symmetric(4.0, /* vertical */ -2.0f64)",
            "4,-2,9,-2",
            "ferroui_base::Thickness::new(4.0, /* vertical */ -2.0f64, 9.0, -2.0)",
        ),
        (
            "Thickness",
            "8.0f64",
            "1,8,8,8",
            "::ferroui_base::Thickness::new(1.0f64, 8.0, 8.0, 8.0)",
        ),
    ];
    for (kind, original, value, expected) in cases {
        let shape = ValueShape::from_type(&format!("::ferroui_base::{kind}")).unwrap();
        let expression = syn::parse_str(original).unwrap();
        let source = shape.read(&expression).unwrap();
        let mut patches = Vec::new();
        source.plan(value, &mut patches).unwrap();
        let revised = TextPatch::apply_all(original, patches).unwrap();
        assert_eq!(revised, expected);
        let revised = syn::parse_str(&revised).unwrap();
        assert_eq!(shape.read(&revised).unwrap().value.to_xaml(), value);
    }
}
