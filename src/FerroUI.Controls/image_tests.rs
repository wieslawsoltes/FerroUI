use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use crate::Image;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{
    DrawingImage, GeometryDrawing, IImage, ImageLoadHandler, ImageLoadResult, ImageScaleVariant,
    RectangleGeometry, Stretch, StretchDirection,
};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An image which only has a size.
struct TestBitmap {
    size: Size,
}

impl IImage for TestBitmap {
    fn size(&self) -> Size {
        self.size
    }

    fn draw(&self, _context: &mut ferroui_base::media::DrawingContext<'_>, _source_rect: Rect, _dest_rect: Rect) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_bitmap(width: i32, height: i32) -> Option<Rc<dyn IImage>> {
    Some(Rc::new(TestBitmap {
        size: Size::new(f64::from(width), f64::from(height)),
    }))
}

#[derive(Default)]
struct PendingImage {
    callbacks: Rc<ferroui_base::utilities::HandlerList<dyn Fn(&ImageLoadResult)>>,
    completed: Cell<bool>,
    size: Option<Size>,
}

impl PendingImage {
    fn complete(&self, result: ImageLoadResult) {
        self.completed.set(true);
        let callbacks = self.callbacks.snapshot();
        for (token, _) in callbacks.iter() {
            self.callbacks.remove(*token);
        }
        for (_, callback) in callbacks.iter() {
            callback(&result);
        }
    }
}

impl IImage for PendingImage {
    fn size(&self) -> Size {
        self.size.unwrap_or(Size::new(16., 8.))
    }

    fn draw(&self, _context: &mut ferroui_base::media::DrawingContext<'_>, _src: Rect, _dst: Rect) {
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn subscribe_load(&self, handler: ImageLoadHandler) -> Option<Subscription> {
        if self.completed.get() {
            return None;
        }
        let token = self.callbacks.add(handler);
        let callbacks = Rc::downgrade(&self.callbacks);
        Some(Subscription::new(Disposable::create(move || {
            if let Some(callbacks) = callbacks.upgrade() {
                callbacks.remove(token);
            }
        })))
    }
}

#[test]
fn image_load_reports_only_current_source_on_attached_control() {
    let _scope = test_scope();
    let image = Image::new();
    let root = TestRoot::with_child(image.clone());
    let events = Rc::new(RefCell::new(Vec::new()));
    let opened = events.clone();
    image.image_opened(move |_, _| opened.borrow_mut().push("opened".to_owned()));
    let failed = events.clone();
    image.image_failed(move |_, args| {
        let error = args.error().downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        failed.borrow_mut().push(args.error_message());
    });

    let retired = Rc::new(PendingImage::default());
    let active = Rc::new(PendingImage::default());
    image.set_source(Some(retired.clone()));
    retired.complete(Ok(()));
    image.set_source(Some(active.clone()));
    active.complete(Ok(()));
    assert!(events.borrow().is_empty());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened"]);

    let missing = Rc::new(PendingImage::default());
    image.set_source(Some(missing.clone()));
    missing.complete(Err(Rc::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "missing image",
    ))));
    assert_eq!(&*events.borrow(), &["opened"]);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened", "missing image"]);
    image.set_source(create_bitmap(16, 8));
    image.set_source(Some(missing));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened", "missing image"]);
    root.set_child(None);
    let detached = Rc::new(PendingImage::default());
    image.set_source(Some(detached.clone()));
    detached.complete(Ok(()));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened", "missing image"]);
    root.set_child(Some(image.clone().upcast()));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened", "missing image", "opened"]);
    root.set_child(None);
    root.set_child(Some(image.clone().upcast()));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(&*events.borrow(), &["opened", "missing image", "opened"]);
}

struct TwoScaleSource;

impl IImage for TwoScaleSource {
    fn size(&self) -> Size {
        Size::new(4., 2.)
    }

    fn draw(&self, _: &mut ferroui_base::media::DrawingContext<'_>, _: Rect, _: Rect) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn resolve_for_scaling(&self, scaling: f64, current: Option<usize>) -> Option<ImageScaleVariant> {
        let id = usize::from(scaling > 1.);
        (current != Some(id)).then(|| ImageScaleVariant {
            id,
            image: Rc::new(PendingImage {
                size: Some(Size::new(if id == 0 { 4. } else { 6. }, 2.)),
                ..Default::default()
            }),
        })
    }
}

#[test]
fn display_scale_selects_each_controls_variant_and_refreshes_load_events() {
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::{Control, Window};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source: Rc<dyn IImage> = Rc::new(TwoScaleSource);
    let first = Image::new();
    let second = Image::new();
    for image in [&first, &second] {
        image.set_source(Some(source.clone()));
    }
    let first_window = Window::new();
    first_window.set_content(Some(Control::boxed(first.clone())));
    first_window.show();
    let second_window = Window::new();
    second_window.set_content(Some(Control::boxed(second.clone())));
    second_window.show();

    let loads = Rc::new(Cell::new(0));
    let count = loads.clone();
    first.image_opened(move |_, _| count.set(count.get() + 1));
    let complete = |image: &Image| {
        let displayed = image.displayed_source().unwrap();
        displayed.as_any().downcast_ref::<PendingImage>().unwrap().complete(Ok(()));
        Dispatcher::ui_thread().run_jobs(None);
    };
    complete(&first);
    assert_eq!(loads.get(), 1);

    let change_scale = first_window.platform_impl().unwrap().scaling_changed().unwrap();
    change_scale(2.);
    assert!(Rc::ptr_eq(&first.source().unwrap(), &source));
    assert!(Rc::ptr_eq(&second.source().unwrap(), &source));
    first.measure(Size::INFINITY);
    second.measure(Size::INFINITY);
    assert_eq!(first.desired_size(), Size::new(6., 2.));
    assert_eq!(second.desired_size(), Size::new(4., 2.));
    let first_choice = first.displayed_source().unwrap();
    change_scale(1.5);
    assert!(Rc::ptr_eq(&first.displayed_source().unwrap(), &first_choice));
    complete(&first);
    assert_eq!(loads.get(), 2);

    first_window.set_content(None);
    change_scale(1.);
    assert!(Rc::ptr_eq(&first.displayed_source().unwrap(), &first_choice));
    first_window.set_content(Some(Control::boxed(first.clone())));
    assert_eq!(first.displayed_source().unwrap().size(), Size::new(4., 2.));
    complete(&first);
    assert_eq!(loads.get(), 3);
    first_window.close();
    second_window.close();
}

struct AppearanceSource;

impl IImage for AppearanceSource {
    fn size(&self) -> Size {
        Size::new(4., 2.)
    }

    fn draw(&self, _: &mut ferroui_base::media::DrawingContext<'_>, _: Rect, _: Rect) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn resolve_for_context(
        &self,
        context: &ferroui_base::media::ImageVariantContext,
        current: Option<usize>,
    ) -> Option<ImageScaleVariant> {
        let dark = context.theme.as_ref() == Some(&ferroui_base::styling::ThemeVariant::dark());
        let high = context.platform_colors.is_high_contrast();
        let id = usize::from(dark) + usize::from(high) * 2;
        (current != Some(id)).then(|| ImageScaleVariant {
            id,
            image: Rc::new(TestBitmap { size: Size::new(4. + id as f64, 2.) }),
        })
    }
}

#[test]
fn adaptive_image_refreshes_when_its_theme_or_platform_palette_changes() {
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::{Control, Window};
    use ferroui_base::platform::{ColorContrastPreference, DefaultPlatformSettings, PlatformColorValues};
    use ferroui_base::styling::ThemeVariant;

    let settings = Rc::new(DefaultPlatformSettings::new());
    let _app = UnitTestApplication::start(TestServices::styled_window().with_platform_settings(settings.clone()));
    let source: Rc<dyn IImage> = Rc::new(AppearanceSource);
    let image = Image::new();
    image.set_source(Some(source.clone()));
    let window = Window::new();
    window.set_content(Some(Control::boxed(image.clone())));
    window.show();
    let displayed = || image.displayed_source().unwrap().size().width;
    assert_eq!(displayed(), 4.);

    image.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(displayed(), 5.);

    let high = PlatformColorValues::new().with_contrast_preference(ColorContrastPreference::High);
    settings.set_color_values(high);
    assert_eq!(displayed(), 7.);
    assert!(Rc::ptr_eq(&image.source().unwrap(), &source));

    window.set_content(None);
    settings.set_color_values(PlatformColorValues::new());
    assert_eq!(displayed(), 7.);
    window.set_content(Some(Control::boxed(image.clone())));
    assert_eq!(displayed(), 5.);
    window.close();
}

#[test]
fn measure_should_return_correct_size_for_no_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Fill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_uniform_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Uniform);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(25.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_uniform_to_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::UniformToFill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_with_stretch_direction_down_only() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch_direction(StretchDirection::DownOnly);
    target.set_source(bitmap);

    target.measure(Size::new(150.0, 150.0));

    assert_eq!(Size::new(50.0, 100.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_height() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(200.0, f64::INFINITY));

    assert_eq!(Size::new(200.0, 400.0), image.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_width() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(f64::INFINITY, 400.0));

    assert_eq!(Size::new(200.0, 400.0), image.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_width_height() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(f64::INFINITY, f64::INFINITY));

    assert_eq!(Size::new(50.0, 100.0), image.desired_size());
}

#[test]
fn arrange_should_return_correct_size_for_no_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 400.0));

    assert_eq!(Size::new(50.0, 100.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Fill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 100.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_uniform_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Uniform);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 50.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_uniform_to_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::UniformToFill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 100.0), target.bounds().size());
}

#[test]
fn drawing_image_source_invalidates_measure() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 500.0, 500.0)));
    let image = DrawingImage::with_drawing(&drawing);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_source(Some(image.into()));

    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    assert_eq!(Size::new(500.0, 500.0), target.desired_size());

    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 600.0, 600.0)));

    Dispatcher::ui_thread().run_jobs(None);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Size::new(600.0, 600.0), target.desired_size());
}

#[test]
fn drawing_image_viewbox_and_empty_drawing_update_measure() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 40.0, 40.0)));
    let source = DrawingImage::with_drawing(&drawing);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_source(Some(source.clone().into()));
    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();
    assert_eq!(target.desired_size(), Size::new(40.0, 40.0));

    let update_layout = || {
        Dispatcher::ui_thread().run_jobs(None);
        root.layout_manager().execute_layout_pass();
    };
    source.set_viewbox(Some(Rect::new(0.0, 0.0, 40.0, 20.0)));
    update_layout();
    assert_eq!(target.desired_size(), Size::new(40.0, 20.0));

    source.set_viewbox(Some(Rect::new(0.0, 0.0, 20.0, 40.0)));
    update_layout();
    assert_eq!(target.desired_size(), Size::new(20.0, 40.0));

    source.set_viewbox(None);
    update_layout();
    assert_eq!(target.desired_size(), Size::new(40.0, 40.0));

    source.set_drawing(None);
    update_layout();
    assert_eq!(target.desired_size(), Size::default());
}
