//! A window drawn by the ProGPU backend: an application set up through the application builder
//! with the macOS platform and `use_progpu()`, whose main window holds shapes and no text.
//!
//! ```text
//! cargo run -p ferroui-progpu --example progpu_window
//! FERROUI_SMOKE_EXIT_MS=4000 cargo run -p ferroui-progpu --example progpu_window
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the window is made larger half way and closed after `n`
//! milliseconds, which ends the main loop.
//!
//! The backend has no text services yet (step P4 of docs/porting/progpu-backend.md), so the
//! example registers a text shaping subsystem that provides nothing, gives the window a
//! template of its own instead of a theme, and draws no text.

#[cfg(target_os = "macos")]
mod example {
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::logging::LogEventLevel;
    use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
    use ferroui_base::media::{BoxShadow, BoxShadows, Color, Geometry, GradientSpreadMethod, IBrush, ITransform, RotateTransform};
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        ferro_class, ferro_impl_classes, instantiate, CornerRadius, FerroObject, FerroObjectImpl, PixelPoint, Ref, RelativePoint,
        Thickness,
    };
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::shapes::{Ellipse, Path, Rectangle};
    use ferroui_controls::templates::FuncControlTemplate;
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, Border, Canvas, ContentControl, Control, NewApplication, Window,
    };
    use ferroui_native::FerroNativePlatformExtensions;
    use ferroui_progpu::ProGpuApplicationExtensions;
    use std::rc::Rc;
    use std::time::Duration;

    #[repr(C)]
    pub struct App {
        base: Application,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self { base: Application::construct() })
        }
    }

    impl ApplicationImpl for App {}

    fn solid(r: u8, g: u8, b: u8) -> Option<Rc<dyn IBrush>> {
        Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(r, g, b))))
    }

    fn gradient(from: Color, to: Color) -> Option<Rc<dyn IBrush>> {
        Some(Rc::new(ImmutableLinearGradientBrush::new(
            &[ImmutableGradientStop::new(0.0, from), ImmutableGradientStop::new(1.0, to)],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            None,
            None,
            None,
        )))
    }

    fn place(canvas: &Canvas, control: impl ferroui_base::IntoRef<Control>, left: f64, top: f64) {
        let control = control.into_ref();
        let element: &FerroObject = &control;
        Canvas::set_left(element, left);
        Canvas::set_top(element, top);
        canvas.children().add(control);
    }

    fn content() -> Ref<Border> {
        let canvas = Canvas::new();

        // A rounded rectangle with a line around it.
        let rectangle = Rectangle::new();
        rectangle.set_width(140.0);
        rectangle.set_height(90.0);
        rectangle.set_radius_x(16.0);
        rectangle.set_radius_y(16.0);
        rectangle.set_fill(solid(0x25, 0x63, 0xeb));
        rectangle.set_stroke(solid(0x1e, 0x29, 0x3b));
        rectangle.set_stroke_thickness(3.0);
        place(&canvas, rectangle, 30.0, 30.0);

        // An ellipse filled with a gradient.
        let ellipse = Ellipse::new();
        ellipse.set_width(110.0);
        ellipse.set_height(110.0);
        ellipse.set_fill(gradient(Color::from_rgb(0xef, 0x44, 0x44), Color::from_rgb(0xfa, 0xcc, 0x15)));
        ellipse.set_stroke(solid(0x1e, 0x29, 0x3b));
        ellipse.set_stroke_thickness(2.0);
        place(&canvas, ellipse, 200.0, 22.0);

        // A path: a star of lines and a heart of curves.
        let star = Path::new();
        star.set_data(Geometry::parse("M 50,0 L 61,35 L 98,35 L 68,57 L 79,91 L 50,70 L 21,91 L 32,57 L 2,35 L 39,35 Z").ok());
        star.set_fill(solid(0x16, 0xa3, 0x4a));
        star.set_stroke(solid(0x14, 0x53, 0x2d));
        star.set_stroke_thickness(2.0);
        place(&canvas, star, 340.0, 26.0);
        let heart = Path::new();
        heart.set_data(Geometry::parse("M 50,30 C 50,10 20,0 10,25 C 0,50 30,70 50,90 C 70,70 100,50 90,25 C 80,0 50,10 50,30 Z").ok());
        heart.set_fill(solid(0xdb, 0x27, 0x77));
        place(&canvas, heart, 470.0, 26.0);

        // What is outside a clipping border is not drawn.
        let clipped = Ellipse::new();
        clipped.set_width(200.0);
        clipped.set_height(200.0);
        clipped.set_margin(Thickness::new(-60.0, -70.0, 0.0, 0.0));
        clipped.set_fill(solid(0xf9, 0x73, 0x16));
        let clip = Border::new();
        clip.set_width(160.0);
        clip.set_height(100.0);
        clip.set_clip_to_bounds(true);
        clip.set_background(solid(0xcb, 0xd5, 0xe1));
        clip.set_child(&clipped);
        place(&canvas, clip, 30.0, 180.0);

        // A rounded card with a shadow.
        let card = Border::new();
        card.set_width(150.0);
        card.set_height(90.0);
        card.set_corner_radius(CornerRadius::uniform(12.0));
        card.set_background(solid(0xff, 0xff, 0xff));
        card.set_box_shadow(BoxShadows::new(BoxShadow {
            offset_x: 0.0,
            offset_y: 6.0,
            blur: 18.0,
            spread: 0.0,
            color: Color::from_argb(0x66, 0x0f, 0x17, 0x2a),
            is_inset: false,
        }));
        place(&canvas, card, 230.0, 185.0);

        // Translucent shapes over each other.
        for (index, color) in [(0xef, 0x44, 0x44), (0x22, 0xc5, 0x5e), (0x3b, 0x82, 0xf6)].into_iter().enumerate() {
            let circle = Ellipse::new();
            circle.set_width(90.0);
            circle.set_height(90.0);
            circle.set_fill(solid(color.0, color.1, color.2));
            circle.set_opacity(0.6);
            let (left, top) = [(420.0, 170.0), (470.0, 170.0), (445.0, 212.0)][index];
            place(&canvas, circle, left, top);
        }

        // A rotated square.
        let square = Rectangle::new();
        square.set_width(70.0);
        square.set_height(70.0);
        square.set_fill(solid(0x7c, 0x3a, 0xed));
        square.set_render_transform_origin(RelativePoint::CENTER);
        let rotation: Rc<dyn ITransform> = RotateTransform::with_angle(30.0).into();
        square.set_render_transform(Some(rotation));
        place(&canvas, square, 60.0, 310.0);

        let border = Border::new();
        border.set_background(gradient(Color::from_rgb(0xf8, 0xfa, 0xfc), Color::from_rgb(0xdb, 0xea, 0xfe)));
        border.set_child(&canvas);
        border
    }

    fn main_window() -> Ref<Window> {
        let window = Window::new();
        // No theme is loaded: the template of the window is its content.
        window.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(ContentControl::content_property().as_property()),
            );
            scope.register("PART_ContentPresenter", presenter.clone().upcast::<FerroObject>());
            presenter.upcast()
        })));
        window.set_title(Some("FerroUI on ProGPU".to_string()));
        window.set_width(620.0);
        window.set_height(420.0);
        window.set_position(PixelPoint::new(120, 120));
        let content = content();
        let control: &Control = &content;
        window.set_content(Some(Control::boxed(control.to_ref())));
        window
    }

    fn smoke_run(window: &Ref<Window>) {
        let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) else {
            return;
        };
        println!("Will resize the window after {} ms and close it after {ms} ms", ms / 2);
        window.set_topmost(true);
        let resized = window.clone();
        let _resize = DispatcherTimer::run_once(
            move || {
                resized.set_width(760.0);
                resized.set_height(500.0);
                println!("Window resized to {:?}", resized.client_size());
            },
            Duration::from_millis(ms / 2),
            DispatcherPriority::NORMAL,
        )
        .detach();
        let closed = window.clone();
        let _close = DispatcherTimer::run_once(
            move || {
                println!("Closing the window at {:?}", closed.client_size());
                closed.close();
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        )
        .detach();
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        AppBuilder::configure::<App>()
            .use_ferro_native()
            // Text is step P4 of the backend: no shaper and no font manager until then.
            .use_text_shaping_subsystem(|| {}, "None")
            .use_progpu()
            .log_to_text_writer(std::io::stderr(), LogEventLevel::Warning, &[])
            .start(
                |app, _| {
                    let window = main_window();
                    smoke_run(&window);
                    window.opened(|| println!("Window opened")).detach();
                    app.run_window(window);
                    println!("The main loop ended");
                },
                &args,
            );
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    example::main();
    #[cfg(not(target_os = "macos"))]
    println!("The window of the ProGPU backend is implemented on macOS only.");
}
