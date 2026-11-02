//! Native scroll input and QML-driven navigation share one physical viewport.
//! The canvas describes the model extent but contains only realized delegates.
use super::*;
use ferroui_base::{reactive::Disposable, Vector};
use ferroui_controls::{primitives::ScrollBarVisibility, Canvas, ScrollViewer};

pub(super) struct Viewport {
    pub control: Ref<ScrollViewer>,
    pub canvas: Ref<Canvas>,
    extent: Ref<Canvas>,
    origin: Rc<Cell<Vector>>,
    requested: Rc<Cell<Vector>>,
}

impl Viewport {
    pub fn new() -> Self {
        let control = ScrollViewer::new();
        control.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        control.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        let canvas = Canvas::new();
        let extent = Canvas::new();
        extent.children().add(&canvas);
        control.set_content(Some(Control::boxed(extent.clone().upcast::<Control>())));
        Self {
            control,
            canvas,
            extent,
            origin: Default::default(),
            requested: Default::default(),
        }
    }
    pub fn apply(&self, name: &str, value: f64) {
        let previous = self.requested.get();
        match name {
            "contentWidth" => {
                self.canvas.set_width(value.max(0.));
                self.extent.set_width(value.max(0.));
            }
            "contentHeight" => {
                self.canvas.set_height(value.max(0.));
                self.extent.set_height(value.max(0.));
            }
            "originX" | "originY" => {
                let origin = if name == "originX" {
                    Canvas::set_left(&self.canvas, -value);
                    self.origin.get().with_x(value)
                } else {
                    Canvas::set_top(&self.canvas, -value);
                    self.origin.get().with_y(value)
                };
                self.origin.set(origin);
                self.control.set_offset(previous - origin);
            }
            _ => {
                let offset = if name == "contentX" {
                    previous.with_x(value)
                } else {
                    previous.with_y(value)
                };
                self.requested.set(offset);
                self.control.set_offset(offset - self.origin.get());
            }
        }
    }
    pub fn connect(&self, endpoint: NativeEndpoint) -> Rc<dyn IDisposable> {
        let weak = self.control.downgrade();
        let requested = self.requested.clone();
        let origin = self.origin.clone();
        let token = self.control.scroll_changed(move |_, args| {
            let Some(viewer) = weak.upgrade() else { return };
            if !args.is_source(&viewer) {
                return;
            }
            if args.viewport_delta() != Vector::default() {
                let size = viewer.viewport();
                endpoint.dispatch(|view| {
                    view.0.instance.borrow_mut().set_list_view_viewport(
                        endpoint.object,
                        size.width,
                        size.height,
                    )?;
                    view.flush()
                });
            }
            if args.extent_delta() != Vector::default()
                || args.viewport_delta() != Vector::default()
            {
                // The first layout can coerce a requested offset while extent is still zero.
                // Reapply it once the presenter knows the canvas dimensions.
                viewer.set_offset(requested.get() - origin.get());
            }
            let offset = viewer.offset() + origin.get();
            requested.set(offset);
            endpoint.write("contentX", Value::Number(offset.x));
            endpoint.write("contentY", Value::Number(offset.y));
        });
        let weak = self.control.downgrade();
        Disposable::create(move || {
            if let Some(viewer) = weak.upgrade() {
                viewer.remove_handler(ScrollViewer::scroll_changed_event(), token);
            }
        })
    }
}
