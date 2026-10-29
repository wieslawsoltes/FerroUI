//! Port of `WriteableBitmap/WriteableBitmapRenderBehavior.cs`.

use crate::{IWriteableBitmapRenderer, WriteableBitmapRef};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectImpl, FerroProperty,
    PixelSize, Ref, StaticType, StyledElementImpl, StyledProperty, Vector,
};
use ferroui_controls::Image;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Creates a [`WriteableBitmap`] and updates it using a renderer on a timer.
#[repr(C)]
pub struct WriteableBitmapRenderBehavior {
    base: StyledElementBehaviorOfT,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    bitmap: RefCell<Option<WriteableBitmapRef>>,
}

ferro_class!(WriteableBitmapRenderBehavior: StyledElementBehaviorOfT);
ferro_class_info!(WriteableBitmapRenderBehavior { new: WriteableBitmapRenderBehavior::new });
ferroui_base::ferro_impl_classes!(WriteableBitmapRenderBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl WriteableBitmapRenderBehavior {
        /// Identifies the `PixelWidth` property.
        pub fn pixel_width_property() -> StyledProperty<i32> {
            FerroProperty::register::<WriteableBitmapRenderBehavior, _>("PixelWidth", 256)
        }

        /// Identifies the `PixelHeight` property.
        pub fn pixel_height_property() -> StyledProperty<i32> {
            FerroProperty::register::<WriteableBitmapRenderBehavior, _>("PixelHeight", 256)
        }

        /// Identifies the `Renderer` property.
        pub fn renderer_property() -> StyledProperty<Option<Rc<dyn IWriteableBitmapRenderer>>> {
            FerroProperty::register::<WriteableBitmapRenderBehavior, _>("Renderer", None)
        }

        /// Identifies the `Bitmap` property.
        pub fn bitmap_property() -> DirectProperty<WriteableBitmapRenderBehavior, Option<WriteableBitmapRef>> {
            FerroProperty::register_direct::<WriteableBitmapRenderBehavior, _>(
                "Bitmap",
                |behavior| behavior.bitmap(),
                None,
                None,
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for WriteableBitmapRenderBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let bitmap = WriteableBitmapRef::new(WriteableBitmap::new(
                PixelSize::new(this.pixel_width(), this.pixel_height()),
                Vector::new(96.0, 96.0),
                Some(PixelFormat::BGRA8888),
                Some(AlphaFormat::Unpremul),
            ));
            this.set_bitmap(Some(bitmap.clone()));

            if let Some(associated_object) = this.associated_object_as::<Image>() {
                associated_object.set_source(Some(bitmap.as_image()));
            }

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
            let timer = this.timer.borrow_mut().take();
            if let Some((timer, tick)) = timer {
                tick.dispose();
                timer.stop();
            }

            if let Some(associated_object) = this.associated_object_as::<Image>() {
                associated_object.set_source(None);
            }

            if let Some(bitmap) = this.bitmap() {
                bitmap.dispose();
            }
            this.set_bitmap(None);
        }
    }
}

impl WriteableBitmapRenderBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Image as StaticType>::TYPE),
            timer: RefCell::new(None),
            bitmap: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the width of the bitmap in pixels. This is a property of the object model.
    pub fn pixel_width(&self) -> i32 {
        self.get_value(Self::pixel_width_property())
    }

    pub fn set_pixel_width(&self, value: i32) {
        self.set_value(Self::pixel_width_property(), value)
    }

    /// Gets or sets the height of the bitmap in pixels. This is a property of the object model.
    pub fn pixel_height(&self) -> i32 {
        self.get_value(Self::pixel_height_property())
    }

    pub fn set_pixel_height(&self, value: i32) {
        self.set_value(Self::pixel_height_property(), value)
    }

    /// Gets or sets the renderer used to update the bitmap. This is a property of the object
    /// model.
    pub fn renderer(&self) -> Option<Rc<dyn IWriteableBitmapRenderer>> {
        self.get_value(Self::renderer_property())
    }

    pub fn set_renderer(&self, value: Option<Rc<dyn IWriteableBitmapRenderer>>) {
        self.set_value(Self::renderer_property(), value)
    }

    /// Gets the created bitmap.
    pub fn bitmap(&self) -> Option<WriteableBitmapRef> {
        self.bitmap.borrow().clone()
    }

    fn set_bitmap(&self, value: Option<WriteableBitmapRef>) {
        self.set_and_raise(Self::bitmap_property(), &self.bitmap, value);
    }

    fn on_tick(&self) {
        let (Some(bitmap), Some(renderer)) = (self.bitmap(), self.renderer()) else {
            return;
        };

        renderer.render(&bitmap);
        if let Some(associated_object) = self.associated_object_as::<Image>() {
            associated_object.invalidate_visual();
        }
    }
}
