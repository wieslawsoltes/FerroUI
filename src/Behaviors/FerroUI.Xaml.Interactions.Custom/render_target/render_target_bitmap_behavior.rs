//! Port of `RenderTarget/RenderTargetBitmapBehavior.cs`.

use crate::{IRenderTargetBitmapRenderHost, IRenderTargetBitmapRenderer};
use ferroui_base::animation::TimeSpan;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::IImage;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, PixelSize, Ref, StaticType, StyledElementImpl, StyledProperty, Vector,
};
use ferroui_controls::Image;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Behavior that draws into a [`RenderTargetBitmap`] and assigns it to the associated [`Image`].
#[repr(C)]
pub struct RenderTargetBitmapBehavior {
    base: StyledElementBehaviorOfT,
    bitmap: RefCell<Option<Rc<RenderTargetBitmap>>>,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    // Deviation (DEVIATIONS.md, Behaviors): the stopwatch of the reference is the clock of the
    // dispatcher (milliseconds), started when the behavior is created.
    stopwatch_start: Cell<Option<i64>>,
}

ferro_class!(RenderTargetBitmapBehavior: StyledElementBehaviorOfT);
ferro_class_info!(RenderTargetBitmapBehavior {
    new: RenderTargetBitmapBehavior::new,
    interfaces: [
        Rc<dyn IRenderTargetBitmapRenderHost> => render_host_of,
    ],
});
ferroui_base::ferro_impl_classes!(RenderTargetBitmapBehavior: StyledElementImpl);

ferro_properties! {
    impl RenderTargetBitmapBehavior {
        /// Identifies the `PixelWidth` property.
        pub fn pixel_width_property() -> StyledProperty<i32> {
            FerroProperty::register::<RenderTargetBitmapBehavior, _>("PixelWidth", 200)
        }

        /// Identifies the `PixelHeight` property.
        pub fn pixel_height_property() -> StyledProperty<i32> {
            FerroProperty::register::<RenderTargetBitmapBehavior, _>("PixelHeight", 200)
        }

        /// Identifies the `Dpi` property.
        pub fn dpi_property() -> StyledProperty<Vector> {
            FerroProperty::register::<RenderTargetBitmapBehavior, _>("Dpi", Vector::new(96.0, 96.0))
        }

        /// Identifies the `Renderer` property.
        pub fn renderer_property() -> StyledProperty<Option<Rc<dyn IRenderTargetBitmapRenderer>>> {
            FerroProperty::register::<RenderTargetBitmapBehavior, _>("Renderer", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for RenderTargetBitmapBehavior {
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            let dispatcher = Dispatcher::ui_thread();
            if dispatcher.check_access() {
                this.stopwatch_start.set(Some(dispatcher.now()));
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for RenderTargetBitmapBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let Some(associated_object) = this.associated_object_as::<Image>() else {
                return;
            };

            let bitmap =
                Rc::new(RenderTargetBitmap::with_dpi(PixelSize::new(this.pixel_width(), this.pixel_height()), this.dpi()));
            *this.bitmap.borrow_mut() = Some(bitmap.clone());
            associated_object.set_source(Some(bitmap as Rc<dyn IImage>));

            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(16));
            let behavior = this.to_ref().downgrade();
            let tick = timer.tick(move |_timer: &DispatcherTimer| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_tick();
                }
            });
            timer.start();
            *this.timer.borrow_mut() = Some((timer, tick));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            let timer = this.timer.borrow_mut().take();
            if let Some((timer, tick)) = timer {
                timer.stop();
                tick.dispose();
            }

            let bitmap = this.bitmap.borrow_mut().take();
            if let Some(bitmap) = bitmap {
                bitmap.dispose();
            }
        }
    }
}

impl RenderTargetBitmapBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Image as StaticType>::TYPE),
            bitmap: RefCell::new(None),
            timer: RefCell::new(None),
            stopwatch_start: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the pixel width of the bitmap. This is a property of the object model.
    pub fn pixel_width(&self) -> i32 {
        self.get_value(Self::pixel_width_property())
    }

    pub fn set_pixel_width(&self, value: i32) {
        self.set_value(Self::pixel_width_property(), value)
    }

    /// Gets or sets the pixel height of the bitmap. This is a property of the object model.
    pub fn pixel_height(&self) -> i32 {
        self.get_value(Self::pixel_height_property())
    }

    pub fn set_pixel_height(&self, value: i32) {
        self.set_value(Self::pixel_height_property(), value)
    }

    /// Gets or sets the DPI vector of the bitmap. This is a property of the object model.
    pub fn dpi(&self) -> Vector {
        self.get_value(Self::dpi_property())
    }

    pub fn set_dpi(&self, value: Vector) {
        self.set_value(Self::dpi_property(), value)
    }

    /// Gets or sets the renderer used to draw the bitmap. This is a property of the object model.
    pub fn renderer(&self) -> Option<Rc<dyn IRenderTargetBitmapRenderer>> {
        self.get_value(Self::renderer_property())
    }

    pub fn set_renderer(&self, value: Option<Rc<dyn IRenderTargetBitmapRenderer>>) {
        self.set_value(Self::renderer_property(), value)
    }

    /// Requests that the behavior renders its content to the bitmap
    /// ([`IRenderTargetBitmapRenderHost::render`]).
    pub fn render(&self) {
        let Some(associated_object) = self.associated_object_as::<Image>() else {
            return;
        };

        let Some(bitmap) = self.bitmap.borrow().clone() else {
            return;
        };

        let mut context = bitmap.create_drawing_context();
        if let Some(renderer) = self.renderer() {
            renderer.render(&mut context, self.elapsed());
        }
        context.dispose();

        associated_object.invalidate_visual();
    }

    /// The behavior as the contract of render hosts.
    pub fn as_render_target_bitmap_render_host(&self) -> Rc<dyn IRenderTargetBitmapRenderHost> {
        Rc::new(RenderHostHandle(self.to_ref()))
    }

    fn elapsed(&self) -> TimeSpan {
        let now = Dispatcher::ui_thread().now();
        let start = match self.stopwatch_start.get() {
            Some(start) => start,
            None => {
                self.stopwatch_start.set(Some(now));
                now
            }
        };
        TimeSpan::from_milliseconds((now - start) as f64)
    }

    fn on_tick(&self) {
        self.render();
    }
}

impl IRenderTargetBitmapRenderHost for RenderTargetBitmapBehavior {
    fn render(&self) {
        RenderTargetBitmapBehavior::render(self)
    }

    fn reference_id(&self) -> *const () {
        let object: &FerroObject = self;
        object as *const FerroObject as *const ()
    }
}

// Deviation (DEVIATIONS.md, Behaviors): a property typed `IRenderTargetBitmapRenderHost` holds a
// shared handle of the contract; the behavior converts to it through this handle (an interface of
// its class info), which compares by the identity of the behavior.
/// The handle of a behavior as the contract of render hosts.
struct RenderHostHandle(Ref<RenderTargetBitmapBehavior>);

impl IRenderTargetBitmapRenderHost for RenderHostHandle {
    fn render(&self) {
        self.0.render()
    }

    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }
}

fn render_host_of(behavior: Ref<RenderTargetBitmapBehavior>) -> Rc<dyn IRenderTargetBitmapRenderHost> {
    behavior.as_render_target_bitmap_render_host()
}
