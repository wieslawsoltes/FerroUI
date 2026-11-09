//! Backend-neutral drawing and text contracts exposed as an interactive catalog page.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{
        immutable::{ImmutablePen, ImmutableSolidColorBrush},
        Colors, DrawingContext, ImmediateDrawingContext, IntersectionResult, TextWrapping,
    },
    platform::{
        AlphaFormat, IBitmapImpl, IGeometryImpl, IPlatformRenderInterface, PixelFormat,
        SharedBitmapImpl,
    },
    rendering::scene_graph::ICustomDrawOperation,
    threading::{DispatcherPriority, DispatcherTimer},
    BoxedValue, FerroLocator, FerroObjectImpl, LocatorExtensions, Matrix, PixelSize, Point, Rect,
    Ref, RoundedRect, StyledElementImpl, Thickness, Vector, Visual, VisualImpl,
};
use ferroui_controls::{
    presenters::TextPresenter, Button, ContentPage, Control, ControlImpl, ScrollViewer, StackPanel,
    TextBlock, TextBox,
};
use std::{rc::Rc, sync::Arc, time::Duration};

#[repr(C)]
pub struct ParityDrawingControl {
    base: Control,
    operation: Arc<dyn ICustomDrawOperation>,
}
ferro_class!(ParityDrawingControl: Control);
ferro_impl_classes!(ParityDrawingControl: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(ParityDrawingControl {
    new: ParityDrawingControl::new
});
impl ParityDrawingControl {
    pub fn new() -> Ref<Self> {
        let operation = Arc::new(ParityDrawing::new());
        let this = instantiate(Self {
            base: Control::construct(),
            operation,
        });
        this.set_width(680.);
        this.set_height(230.);
        this
    }
}
impl VisualImpl for ParityDrawingControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        context.custom(&this.operation);
    }
}
struct ParityDrawing {
    curve: Arc<dyn IGeometryImpl>,
    broken: Arc<dyn IGeometryImpl>,
    bitmaps: Vec<Arc<SharedBitmapImpl>>,
}
impl ParityDrawing {
    fn new() -> Self {
        let factory =
            FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
        let mut builder = factory.create_stream_geometry_builder(None);
        builder.begin_figure(Point::new(0., 0.), true);
        builder.line_to(Point::new(20., 0.), true);
        builder.quadratic_bezier_to(Point::new(150., 100.), Point::new(20., 40.), false);
        builder.end_figure(true);
        let curve = builder.to_geometry();
        let mut builder = factory.create_stream_geometry_builder(None);
        builder.begin_figure(Point::new(0., 0.), true);
        builder.line_to(Point::new(80., 0.), true);
        builder.line_to(Point::new(80., 80.), false);
        builder.line_to(Point::new(0., 80.), true);
        builder.end_figure(true);
        let broken = builder.to_geometry();
        let outer = factory.create_rectangle_geometry(Rect::new(0., 0., 40., 40.));
        let inner = factory.create_rectangle_geometry(Rect::new(10., 10., 10., 10.));
        assert_eq!(
            outer.get_fill_intersection_result(&*inner),
            IntersectionResult::FullyInside
        );
        assert_eq!(
            inner.get_fill_intersection_result(&*outer),
            IntersectionResult::FullyContains
        );
        assert!(curve.get_render_bounds(None).right() > 70.);
        let mut bitmaps = Vec::new();
        for format in [
            PixelFormat::RGBA8888,
            PixelFormat::BGRA8888,
            PixelFormat::RGB565,
        ] {
            if !factory.is_supported_bitmap_pixel_format(format) {
                continue;
            }
            let bitmap = factory.create_writeable_bitmap(
                PixelSize::new(2, 1),
                Vector::new(96., 96.),
                format,
                if format.has_alpha() {
                    AlphaFormat::Unpremul
                } else {
                    AlphaFormat::Opaque
                },
            );
            let lock = bitmap.lock();
            let expected = if format == PixelFormat::RGB565 {
                vec![0, 248, 224, 7]
            } else {
                vec![0, 0, 0, 128, 0, 0, 0, 255]
            };
            lock.with_data(&mut |p| {
                p[..expected.len()].copy_from_slice(&expected);
                if format != PixelFormat::RGB565 {
                    let red = if format == PixelFormat::RGBA8888 {
                        0
                    } else {
                        2
                    };
                    p[red] = 255;
                    p[5] = 255;
                }
            });
            lock.dispose();
            bitmaps.push(bitmap as Arc<SharedBitmapImpl>);
        }
        Self {
            curve,
            broken,
            bitmaps,
        }
    }
}
impl ICustomDrawOperation for ParityDrawing {
    fn bounds(&self) -> Rect {
        Rect::new(0., 0., 680., 230.)
    }
    fn hit_test(&self, _: Point) -> bool {
        false
    }
    fn equals(&self, _: &dyn ICustomDrawOperation) -> bool {
        false
    }
    fn dispose(&self) {}
    fn render(&self, context: &mut ImmediateDrawingContext<'_>) {
        let draw = context.platform_impl();
        let original = draw.transform();
        let pen = ImmutablePen::from_uint32(0xffffffff, 2.);
        draw.set_transform(Matrix::create_translation(16., 16.) * original);
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::CORNFLOWER_BLUE)),
            Some(&pen),
            RoundedRect::new(
                Rect::new(0., 0., 120., 100.),
                Vector::new(90., 24.),
                Vector::new(12., 24.),
                Vector::ZERO,
                Vector::ZERO,
            ),
            &Default::default(),
        );
        draw.set_transform(Matrix::create_translation(180., 16.) * original);
        draw.draw_geometry(
            Some(&ImmutableSolidColorBrush::new(Colors::GOLD)),
            Some(&pen),
            &*self.curve,
        );
        draw.set_transform(Matrix::create_translation(310., 16.) * original);
        draw.draw_geometry(
            Some(&ImmutableSolidColorBrush::new(Colors::TEAL)),
            Some(&pen),
            &*self.broken,
        );
        draw.set_transform(original);
        for (i, bitmap) in self.bitmaps.iter().enumerate() {
            draw.draw_bitmap(
                &**bitmap,
                1.,
                Rect::new(0., 0., 2., 1.),
                Rect::new(16.6 + i as f64 * 180., 150.2, 150., 50.4),
            );
        }
    }
}
fn label(text: &str) -> Ref<TextBlock> {
    let t = TextBlock::new();
    t.set_text(Some(text));
    t.set_text_wrapping(TextWrapping::Wrap);
    t
}
const TEXT: &str = "office العربية עברית 👩‍💻 中文";
pub fn page() -> Ref<ContentPage> {
    let panel = StackPanel::new();
    panel.set_margin(Thickness::uniform(16.));
    panel.set_spacing(12.);
    panel.children().add(label("Asymmetric corners · fill-only curve bounds · broken stroke closure. Below: RGBA, BGRA and RGB565 writable inputs (red/green; transparency on the first two)."));
    panel.children().add(ParityDrawingControl::new());
    panel.children().add(label("Native text quality target: Latin ligatures, Arabic joining, Hebrew, emoji and CJK. Select, replace, undo and hit testing must preserve UTF-16 boundaries."));
    let text = TextBox::new();
    text.set_text(Some(TEXT));
    text.set_font_size(24.);
    panel.children().add(&text);
    let status = label("Run the text checks, or edit the text manually.");
    panel.children().add(&status);
    let action: Rc<dyn Fn()> = Rc::new(move || {
        text.set_text(Some(TEXT));
        text.set_selection_start(7);
        text.set_selection_end(14);
        assert_eq!(text.selected_text(), "العربية");
        let presenter = text
            .clone()
            .upcast::<Visual>()
            .get_visual_descendants()
            .find_map(|v| v.cast::<TextPresenter>())
            .expect("text presenter attached");
        let layout = presenter.text_layout();
        let selection = layout.hit_test_text_range(7, 7);
        assert!(!selection.is_empty());
        for rect in selection {
            assert!(rect.width.is_finite() && rect.width > 0. && rect.height > 0.);
            let hit = layout.hit_test_point(Point::new(
                rect.x + rect.width / 2.,
                rect.y + rect.height / 2.,
            ));
            assert!((0..=TEXT.encode_utf16().count() as i32).contains(&hit.text_position()));
        }
        text.set_selected_text(Some("مرحبا"));
        assert!(text.text().unwrap().contains("مرحبا"));
        text.undo();
        assert_eq!(text.text().as_deref(), Some(TEXT));
        text.set_selection_start(7);
        text.set_selection_end(14);
        status.set_text(Some("PASS: geometry containment, writable inputs, RTL selection, text hit testing, replacement and undo."));
        println!("CATALOG_RENDERER_PARITY_PASS text_quality=native selection=rtl replacement=true undo=true");
    });
    let button = Button::new();
    button.set_content(Some(Rc::new("Run text checks".to_owned()) as BoxedValue));
    let click = action.clone();
    button.click(move |_, _| click());
    panel.children().add(button);
    if std::env::var_os("FERROUI_SMOKE_RENDERER_PARITY").is_some() {
        DispatcherTimer::run_once(
            move || action(),
            Duration::from_millis(700),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(panel)));
    let page = ContentPage::new();
    page.set_header(Some(Rc::new("Renderer parity".to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(scroller)));
    page
}
