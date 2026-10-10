//! Port of upstream's `Media/GlyphRunTests.cs`.

use crate::test_base::{test_font_family, TestBase};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{
    Colors, DrawingContext, Geometry, GlyphInfoList, GlyphRun, GradientStop, IBrush, LinearGradientBrush,
    SolidColorBrush, TextOptions, TextRenderingMode, Typeface,
};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::*;
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Control, ControlImpl, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\GlyphRun")
}

fn foreground_gradient() -> Rc<dyn IBrush> {
    let brush = LinearGradientBrush::new();
    brush.set_start_point(RelativePoint::new(0.0, 0.5, RelativeUnit::Relative));
    brush.set_end_point(RelativePoint::new(1.0, 0.5, RelativeUnit::Relative));
    let first = GradientStop::new();
    first.set_color(Colors::RED);
    first.set_offset(0.0);
    brush.gradient_stops().add(first);
    let second = GradientStop::new();
    second.set_color(Colors::BLUE);
    second.set_offset(1.0);
    brush.gradient_stops().add(second);
    brush.into()
}

#[test]
fn should_render_glyph_run_geometry() {
    let t = base();
    let control = GlyphRunGeometryControl::new();
    control.set_value(TextElement::foreground_property(), Some(foreground_gradient()));

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(190.0);
    target.set_height(120.0);
    target.set_child(control);

    t.render_to_file(&target, "Should_Render_GlyphRun_Geometry");

    t.compare_images("Should_Render_GlyphRun_Geometry");
}

#[test]
#[cfg_attr(not(windows), ignore = "For consistent results")]
fn should_render_glyph_run_un_positioned() {
    let t = base();
    let control = UnPositionedGlyphRunControl::new();
    control.set_value(TextElement::foreground_property(), Some(foreground_gradient()));

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(190.0);
    target.set_height(120.0);
    target.set_child(control);

    t.render_to_file(&target, "Should_Render_GlyphRun_UnPositioned");

    t.compare_images("Should_Render_GlyphRun_UnPositioned");
}

#[test]
#[cfg_attr(not(windows), ignore = "For consistent results")]
fn should_render_glyph_run_positioned() {
    let t = base();
    let control = PositionedGlyphRunControl::new();
    control.set_value(TextElement::foreground_property(), Some(foreground_gradient()));

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(190.0);
    target.set_height(120.0);
    target.set_child(control);

    t.render_to_file(&target, "Should_Render_GlyphRun_Positioned");

    t.compare_images("Should_Render_GlyphRun_Positioned");
}

#[test]
#[cfg_attr(not(windows), ignore = "For consistent results")]
fn should_render_glyph_run_aliased() {
    let t = base();
    let control = PositionedGlyphRunControl::new();
    let foreground = SolidColorBrush::new();
    foreground.set_color(Colors::BLACK);
    let foreground: Rc<dyn IBrush> = foreground.into();
    control.set_value(TextElement::foreground_property(), Some(foreground));

    TextOptions::set_text_rendering_mode(&control, TextRenderingMode::Alias);

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(190.0);
    target.set_height(120.0);
    target.set_child(control);

    t.render_to_file(&target, "Should_Render_GlyphRun_Aliased");

    t.compare_images("Should_Render_GlyphRun_Aliased");
}

fn characters() -> ReadOnlyMemory<u16> {
    ReadOnlyMemory::from(vec!['A' as u16, 'B' as u16, 'C' as u16])
}

#[repr(C)]
struct GlyphRunGeometryControl {
    base: Control,
    geometry: Ref<Geometry>,
}

ferro_class!(GlyphRunGeometryControl: Control);
ferro_impl_classes!(
    GlyphRunGeometryControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl GlyphRunGeometryControl {
    fn new() -> Ref<Self> {
        let glyph_typeface = Typeface::new(test_font_family()).glyph_typeface();

        let glyph_indices = [
            glyph_typeface.character_to_glyph_map().get_glyph('A' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('B' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('C' as i32),
        ];

        let characters = characters();

        let glyph_run = GlyphRun::from_glyph_indices(glyph_typeface, 100.0, characters, &glyph_indices, None, 0);

        let geometry = glyph_run.build_geometry();

        instantiate(Self { base: Control::construct(), geometry })
    }
}

impl VisualImpl for GlyphRunGeometryControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        let foreground = TextElement::get_foreground(this);

        context.draw_geometry(foreground.as_ref(), None, &this.geometry);
    }
}

#[repr(C)]
struct UnPositionedGlyphRunControl {
    base: Control,
    glyph_run: Rc<GlyphRun>,
}

ferro_class!(UnPositionedGlyphRunControl: Control);
ferro_impl_classes!(
    UnPositionedGlyphRunControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl UnPositionedGlyphRunControl {
    fn new() -> Ref<Self> {
        let glyph_typeface = Typeface::new(test_font_family()).glyph_typeface();

        let glyph_indices = [
            glyph_typeface.character_to_glyph_map().get_glyph('A' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('B' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('C' as i32),
        ];

        let characters = characters();

        let glyph_run = GlyphRun::from_glyph_indices(glyph_typeface, 100.0, characters, &glyph_indices, None, 0);

        instantiate(Self { base: Control::construct(), glyph_run })
    }
}

impl VisualImpl for UnPositionedGlyphRunControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        let foreground = TextElement::get_foreground(this);

        context.draw_glyph_run(foreground.as_ref(), &this.glyph_run);
    }
}

#[repr(C)]
struct PositionedGlyphRunControl {
    base: Control,
    glyph_run: Rc<GlyphRun>,
}

ferro_class!(PositionedGlyphRunControl: Control);
ferro_impl_classes!(
    PositionedGlyphRunControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl PositionedGlyphRunControl {
    fn new() -> Ref<Self> {
        let glyph_typeface = Typeface::new(test_font_family()).glyph_typeface();

        let glyph_indices = [
            glyph_typeface.character_to_glyph_map().get_glyph('A' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('B' as i32),
            glyph_typeface.character_to_glyph_map().get_glyph('C' as i32),
        ];

        let scale = 100.0 / glyph_typeface.metrics().design_em_height as f64;

        let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph_indices[0]).unwrap_or_default();

        let glyph_advance = advance as f64 * scale;

        let glyph_infos = vec![
            GlyphInfo::new(glyph_indices[0], 0, glyph_advance),
            GlyphInfo::new(glyph_indices[1], 1, glyph_advance),
            GlyphInfo::new(glyph_indices[2], 2, glyph_advance),
        ];

        let characters = characters();

        let glyph_run =
            GlyphRun::new(glyph_typeface, 100.0, characters, GlyphInfoList::from(glyph_infos), None, 0);

        instantiate(Self { base: Control::construct(), glyph_run })
    }
}

impl VisualImpl for PositionedGlyphRunControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        let foreground = TextElement::get_foreground(this);

        context.draw_glyph_run(foreground.as_ref(), &this.glyph_run);
    }
}
