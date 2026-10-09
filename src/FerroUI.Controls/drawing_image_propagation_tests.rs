//! Port of `Rendering/DrawingImagePropagationTests.cs` (base unit tests): the consumer of the drawing image is an
//! image control on a canvas, so the test lives with the controls.
//!
//! Upstream's `CompositorCanvas` is a top-level with a compositor whose content is a canvas, and the debug events
//! of its composition target; it is written out here over the compositor test services of this crate.

use crate::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
use crate::{Canvas, Control, Image, Window};
use ferroui_base::media::{Brushes, DrawingImage, GeometryDrawing, RectangleGeometry};
use ferroui_base::platform::LtrbRect;
use ferroui_base::rendering::composition::{CompositingRenderer, ICompositionTargetDebugEvents};
use ferroui_base::{Rect, Ref};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

#[derive(Default)]
struct DebugEvents {
    rects: RefCell<Vec<Rect>>,
    rendered_visuals: Cell<i32>,
    visited_visuals: Cell<i32>,
}

// SAFETY: the test creates and uses the receiver on one thread; the impls only satisfy the thread-safety bound of
// the contract.
unsafe impl Send for DebugEvents {}
unsafe impl Sync for DebugEvents {}

impl DebugEvents {
    fn reset(&self) {
        self.rects.borrow_mut().clear();
        self.rendered_visuals.set(0);
        self.visited_visuals.set(0);
    }
}

impl ICompositionTargetDebugEvents for DebugEvents {
    fn rendered_visuals(&self) -> i32 {
        self.rendered_visuals.get()
    }

    fn set_rendered_visuals(&self, value: i32) {
        self.rendered_visuals.set(value);
    }

    fn visited_visuals(&self) -> i32 {
        self.visited_visuals.get()
    }

    fn set_visited_visuals(&self, value: i32) {
        self.visited_visuals.set(value);
    }

    fn rect_invalidated(&self, rc: LtrbRect) {
        self.rects.borrow_mut().push(rc.to_rect());
    }
}

struct CompositorCanvas {
    // Declared first: dropped before the services.
    canvas: Ref<Canvas>,
    window: Ref<Window>,
    events: Arc<DebugEvents>,
    services: CompositorTestServices,
}

impl CompositorCanvas {
    fn new() -> CompositorCanvas {
        let services = CompositorTestServices::start(TestServices::styled_window());
        let window_impl = MockWindowingPlatform::create_window_mock_with_size(1000.0, 1000.0);
        services.setup(&window_impl);

        let window = Window::with_impl(window_impl);
        window.show();

        let events = Arc::new(DebugEvents::default());
        let renderer = window.presentation_source().typed_renderer().clone();
        let renderer = renderer.as_any().downcast_ref::<CompositingRenderer>().expect("a compositing renderer");
        renderer.composition_target().set_debug_events(Some(events.clone()));

        let canvas = Canvas::new();
        window.set_content(Some(Control::boxed(&canvas)));
        services.run_jobs();
        events.reset();

        CompositorCanvas { canvas, window, events, services }
    }

    fn run_jobs(&self) {
        self.services.run_jobs();
    }

    fn assert_rects(&self, rects: &[Rect]) {
        self.run_jobs();
        let distinct = |rects: &[Rect]| {
            let mut texts: Vec<String> = rects.iter().map(|r| r.to_string()).collect();
            texts.sort();
            texts.dedup();
            texts
        };
        assert_eq!(distinct(rects), distinct(&self.events.rects.borrow()));
        self.events.rects.borrow_mut().clear();
    }
}

impl Drop for CompositorCanvas {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn create_image(image: &Ref<DrawingImage>) -> Ref<Image> {
    let result = Image::new();
    result.set_source(Some(image.clone().into()));
    result.set_width(20.0);
    result.set_height(10.0);
    Canvas::set_left(&result, 30.0);
    Canvas::set_top(&result, 50.0);
    result
}

#[test]
fn mutating_geometry_inside_drawing_image_invalidates_consumer() {
    let services = CompositorCanvas::new();

    let geometry = RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 10.0));
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(Brushes::red()));
    drawing.set_geometry(geometry.clone());
    let image = DrawingImage::with_drawing(&drawing);
    services.canvas.children().add(&create_image(&image));
    services.run_jobs();
    services.events.rects.borrow_mut().clear();

    geometry.set_rect(Rect::new(0.0, 0.0, 30.0, 15.0));

    services.assert_rects(&[Rect::new(30.0, 50.0, 20.0, 10.0), Rect::new(30.0, 50.0, 30.0, 15.0)]);
}
