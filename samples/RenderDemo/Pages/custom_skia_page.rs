//! Port of `Pages/CustomSkiaPage.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    Brushes, DrawingContext, GlyphRun, IImmutableGlyphRunReference, ImmediateDrawingContext, Typeface,
};
use ferroui_base::platform::IGlyphRunImpl;
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_skia::ISkiaApiLeaseFeature;
use skia_safe::gradient::{Colors as GradientColors, Gradient, Interpolation};
use skia_safe::{BlendMode, Canvas, Color, Color4f, Paint, TileMode};
use std::any::TypeId;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[repr(C)]
pub struct CustomSkiaPage {
    base: Control,
    no_skia: Rc<GlyphRun>,
}

ferro_class!(CustomSkiaPage: Control);
ferro_impl_classes!(
    CustomSkiaPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(CustomSkiaPage { new: CustomSkiaPage::new });

/// The stopwatch of the draw operations (`static Stopwatch St = Stopwatch.StartNew()`): the
/// time it was started at. A draw operation is rendered on the render thread, where the
/// clock of the dispatcher is not available.
static ST: OnceLock<Instant> = OnceLock::new();

fn st_elapsed() -> Duration {
    ST.get_or_init(Instant::now).elapsed()
}

/// The reference to the platform glyph run of a draw operation, in the form the immediate
/// drawing context draws: an operation is shared with the render thread, so it keeps the
/// platform glyph run of the reference it was created with, which is thread-safe.
struct GlyphRunReference(Arc<dyn IGlyphRunImpl>);

impl IImmutableGlyphRunReference for GlyphRunReference {
    fn glyph_run(&self) -> Option<Arc<dyn IGlyphRunImpl>> {
        Some(self.0.clone())
    }

    fn dispose(&self) {}
}

struct CustomDrawOp {
    no_skia: GlyphRunReference,
    bounds: Rect,
}

impl CustomDrawOp {
    fn new(bounds: Rect, no_skia: &GlyphRun) -> Self {
        // The stopwatch starts with the first operation.
        st_elapsed();
        let no_skia = no_skia
            .try_create_immutable_glyph_run_reference()
            .and_then(|reference| reference.glyph_run())
            .expect("the immutable reference of the glyph run");
        Self { no_skia: GlyphRunReference(no_skia), bounds }
    }

    fn render_skia(&self, canvas: &Canvas) {
        canvas.save();
        // create the first shader
        let colors = [
            Color4f::from(Color::from_rgb(0, 255, 255)),
            Color4f::from(Color::from_rgb(255, 0, 255)),
            Color4f::from(Color::from_rgb(255, 255, 0)),
            Color4f::from(Color::from_rgb(0, 255, 255)),
        ];

        let _sx = Self::animate(100, 2, 10);
        let _sy = Self::animate(1000, 5, 15);
        let elapsed_total_seconds = st_elapsed().as_secs_f64();
        let light_position = skia_safe::Point::new(
            (self.bounds.width / 2.0 + elapsed_total_seconds.cos() * self.bounds.width / 4.0) as f32,
            (self.bounds.height / 2.0 + elapsed_total_seconds.sin() * self.bounds.height / 4.0) as f32,
        );
        {
            let sweep = skia_safe::gradient::shaders::sweep_gradient(
                skia_safe::Point::new(
                    ((self.bounds.width as i32) / 2) as f32,
                    ((self.bounds.height as i32) / 2) as f32,
                ),
                (0.0, 360.0),
                &Gradient::new(GradientColors::new(&colors, None, TileMode::Clamp, None), Interpolation::default()),
                None,
            );
            let turbulence = skia_safe::perlin_noise_shader::fractal_noise((0.05, 0.05), 4, 0.0, None);
            let blur = skia_safe::image_filters::blur(
                (Self::animate(100, 2, 10) as f32, Self::animate(100, 5, 15) as f32),
                None,
                None,
                None,
            );
            if let (Some(sweep), Some(turbulence)) = (sweep, turbulence) {
                let shader = skia_safe::shaders::blend(BlendMode::SrcATop, sweep, turbulence);
                let mut paint = Paint::default();
                paint.set_shader(shader);
                paint.set_image_filter(blur);
                canvas.draw_paint(&paint);
            }
        }

        {
            let light_colors = [
                Color4f::from(Color::from_argb(100, 255, 200, 200)),
                Color4f::from(Color::TRANSPARENT),
                Color4f::from(Color::from_argb(220, 40, 40, 40)),
                Color4f::from(Color::from_argb(Self::animate(100, 200, 220) as u8, 20, 20, 20)),
            ];
            let light_positions: [f32; 4] = [0.3, 0.3, 0.8, 1.0];
            let pseudo_light = skia_safe::gradient::shaders::radial_gradient(
                (light_position, (self.bounds.width / 3.0) as f32),
                &Gradient::new(
                    GradientColors::new(&light_colors, Some(&light_positions[..]), TileMode::Clamp, None),
                    Interpolation::default(),
                ),
                None,
            );
            if let Some(pseudo_light) = pseudo_light {
                let mut paint = Paint::default();
                paint.set_shader(pseudo_light);
                canvas.draw_paint(&paint);
            }
        }
        canvas.restore();
    }

    fn animate(d: i32, from: i32, to: i32) -> i32 {
        let ms = (st_elapsed().as_millis() as i64 / i64::from(d)) as i32;
        let diff = to - from;
        let range = diff * 2;
        let mut v = ms % range;
        if v > diff {
            v = range - v;
        }
        let rv = v + from;
        if rv < from || rv > to {
            panic!("WTF");
        }
        rv
    }
}

impl ICustomDrawOperation for CustomDrawOp {
    fn dispose(&self) {
        // No-op
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn hit_test(&self, _p: Point) -> bool {
        false
    }

    fn equals(&self, _other: &dyn ICustomDrawOperation) -> bool {
        false
    }

    fn render(&self, context: &mut ImmediateDrawingContext<'_>) {
        let lease_feature = context
            .try_get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
            .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned());
        match lease_feature {
            None => context.draw_glyph_run(&*Brushes::black(), &self.no_skia),
            Some(lease_feature) => {
                let lease = lease_feature.lease();
                lease.with_sk_canvas(&mut |canvas| self.render_skia(canvas));
                lease.dispose();
            }
        }
    }
}

impl VisualImpl for CustomSkiaPage {
    fn render(this: &Self, context: &mut DrawingContext) {
        let op: Arc<dyn ICustomDrawOperation> = Arc::new(CustomDrawOp::new(
            Rect::new(0.0, 0.0, this.bounds().width, this.bounds().height),
            &this.no_skia,
        ));
        context.custom(&op);
        let page = this.to_ref();
        // The operation is not awaited (`InvokeAsync` without an await).
        drop(
            Dispatcher::ui_thread()
                .invoke_async_local_with_priority(move || page.invalidate_visual(), DispatcherPriority::BACKGROUND),
        );
    }
}

impl CustomSkiaPage {
    pub fn construct() -> Self {
        let text = "Current rendering API is not Skia";
        let glyph_typeface = Typeface::default().glyph_typeface();
        let glyphs: Vec<u16> =
            text.encode_utf16().map(|ch| glyph_typeface.character_to_glyph_map().get_glyph(i32::from(ch))).collect();
        let no_skia = GlyphRun::from_glyph_indices(glyph_typeface, 12.0, ReadOnlyMemory::from_str(text), &glyphs, None, 0);

        Self { base: Control::construct(), no_skia }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);
        this
    }
}
