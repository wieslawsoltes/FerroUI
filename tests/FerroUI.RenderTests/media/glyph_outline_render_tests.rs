//! Port of upstream's `Media/GlyphOutlineRenderTests.cs`.
//!
//! Upstream's `LoadGlyphTypeface` copies the bytes of the asset into memory
//! of the Skia library, creates a typeface of that library from them and
//! wraps it in the platform typeface of the Skia backend. The test crate
//! does not depend on the Skia library, so the port asks the font manager of
//! the backend for the platform typeface of the stream
//! (`IFontManagerImpl.TryCreateGlyphTypeface` with a stream), which does the
//! same: a copy of the bytes, a typeface from the copy, the platform
//! typeface with no font simulations.

use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, DrawingContext, FontSimulations, GlyphTypeface, IBrush};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl, IGeometryImpl};
use ferroui_base::utilities::Uri;
use ferroui_base::*;
use ferroui_controls::{Border, Control, ControlImpl};
use std::rc::Rc;
use std::sync::Arc;

// Direct asset URIs (not via FontManager) so we can address Inter-Regular and
// InterVariable separately even though they share the family name "Inter".
const INTER_REGULAR_ASSET: &str = "resm:FerroUI.Skia.RenderTests.Assets.Inter-Regular.ttf?assembly=ferroui-render-tests";

const INTER_VARIABLE_ASSET: &str = "resm:FerroUI.Skia.RenderTests.Assets.InterVariable.ttf?assembly=ferroui-render-tests";

const MI_SANS_ASSET: &str = "resm:FerroUI.Skia.RenderTests.Assets.MiSans-Normal.ttf?assembly=ferroui-render-tests";

const ADOBE_BLANK_ASSET: &str = "resm:FerroUI.Skia.RenderTests.Assets.AdobeBlank2VF.ttf?assembly=ferroui-render-tests";

// Purpose-built fixture (no production font encodes point matching): its 'P' glyph is a
// composite assembled with ARGS_ARE_XY_VALUES clear, i.e. by point matching.
const POINT_MATCH_ASSET: &str = "resm:FerroUI.Skia.RenderTests.Assets.PointMatch.ttf?assembly=ferroui-render-tests";

fn base() -> TestBase {
    TestBase::new(r"Media\GlyphOutline")
}

#[test]
fn should_render_inter_latin_glyph() {
    let t = base();
    t.render_to_file(build_target(load_glyph_typeface(INTER_REGULAR_ASSET), 'A'), "Should_Render_Inter_Latin_Glyph");
    t.compare_images("Should_Render_Inter_Latin_Glyph");
}

#[test]
fn should_render_inter_composite_glyph() {
    let t = base();
    // Á = base 'A' + combining acute accent. Inter stores it as a composite glyph,
    // which exercises GlyfTable's recursive component path.
    t.render_to_file(
        build_target(load_glyph_typeface(INTER_REGULAR_ASSET), 'Á'),
        "Should_Render_Inter_Composite_Glyph",
    );
    t.compare_images("Should_Render_Inter_Composite_Glyph");
}

#[test]
fn should_render_point_matched_composite_glyph() {
    let t = base();
    // 'P' is a composite whose second component (a small square) is placed by point
    // matching, aligning its bottom-left point onto the top-right point of the base
    // rectangle, rather than by an x/y offset. This is the only way to exercise
    // GlyfTable's point-matching build path through the full Skia pipeline, since no
    // production font uses point matching. A broken implementation would drop the square
    // at the origin instead of the rectangle's corner, which the image diff would catch.
    t.render_to_file(
        build_target(load_glyph_typeface(POINT_MATCH_ASSET), 'P'),
        "Should_Render_PointMatched_Composite_Glyph",
    );
    t.compare_images("Should_Render_PointMatched_Composite_Glyph");
}

#[test]
fn should_render_mi_sans_cjk_glyph() {
    let t = base();
    // 中, a CJK ideograph with many contours and counters.
    t.render_to_file(build_target(load_glyph_typeface(MI_SANS_ASSET), '中'), "Should_Render_MiSans_CJK_Glyph");
    t.compare_images("Should_Render_MiSans_CJK_Glyph");
}

#[test]
fn should_render_inter_variable_at_default() {
    let t = base();
    // Variable font with fvar / gvar tables present. Variation is not applied,
    // so this renders the default-instance outline: the baseline against which a
    // future variable-font render test will compare deformed outputs.
    t.render_to_file(
        build_target(load_glyph_typeface(INTER_VARIABLE_ASSET), 'R'),
        "Should_Render_InterVariable_At_Default",
    );
    t.compare_images("Should_Render_InterVariable_At_Default");
}

#[test]
fn blank_variable_font_returns_null_outline() {
    let _t = base();
    // AdobeBlank2VF is a variable font whose glyphs are intentionally empty.
    // GetGlyphOutline must return null without throwing on a font with fvar but
    // no usable contour data.
    let gt = load_glyph_typeface(ADOBE_BLANK_ASSET);
    let glyph_index = if gt.character_to_glyph_map().contains_glyph('A' as i32) {
        gt.character_to_glyph_map().get_glyph('A' as i32)
    } else {
        0u16
    };

    assert!(gt.get_glyph_outline(glyph_index).is_none());
}

fn load_glyph_typeface(asset_uri: &str) -> Rc<GlyphTypeface> {
    let loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
    let mut stream = loader.open(&Uri::absolute(asset_uri).expect("the URI of the asset is valid"), None).expect("the asset opens");
    let font_manager = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();
    let platform_typeface = font_manager
        .try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None)
        .unwrap_or_else(|| panic!("The backend failed to load the font."));
    GlyphTypeface::new(platform_typeface, FontSimulations::None).expect("the tables of the font are read")
}

fn build_target(glyph_typeface: Rc<GlyphTypeface>, ch: char) -> Ref<Border> {
    let target = Border::new();
    target.set_width(240.0);
    target.set_height(240.0);
    target.set_background(Some(Brushes::white()));
    target.set_child(GlyphOutlineControl::new(&glyph_typeface, ch));
    target
}

/// Renders a single glyph outline produced by `GlyphTypeface::get_glyph_outline`, filled in black.
/// The hosting border supplies the white background and the layout
/// size; the control itself fills the area assigned by layout.
#[repr(C)]
struct GlyphOutlineControl {
    base: Control,
    outline: Option<Arc<dyn IGeometryImpl>>,
}

ferro_class!(GlyphOutlineControl: Control);
ferro_impl_classes!(
    GlyphOutlineControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl GlyphOutlineControl {
    fn new(glyph_typeface: &GlyphTypeface, ch: char) -> Ref<Self> {
        const EM_SIZE: f64 = 200.0;
        const MARGIN: f64 = 20.0;

        if !glyph_typeface.character_to_glyph_map().contains_glyph(ch as i32) {
            return instantiate(Self { base: Control::construct(), outline: None });
        }

        let glyph_index = glyph_typeface.character_to_glyph_map().get_glyph(ch as i32);
        let scale = EM_SIZE / glyph_typeface.metrics().design_em_height as f64;

        // glyf is y-up; flip the y axis and translate so the baseline lands inside
        // the bitmap with a margin of `MARGIN` on every side.
        let transform = Matrix::create_scale(scale, -scale) * Matrix::create_translation(MARGIN, EM_SIZE + MARGIN);

        let outline = glyph_typeface.get_glyph_outline(glyph_index).map(|outline| {
            let outline: Arc<dyn IGeometryImpl> = outline.with_transform(transform);
            outline
        });

        instantiate(Self { base: Control::construct(), outline })
    }
}

impl VisualImpl for GlyphOutlineControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(outline) = &this.outline {
            let black: Rc<dyn IBrush> = Brushes::black();
            context.draw_geometry_impl(Some(&black), None, outline);
        }
    }
}
