//! Port of `Pages/WriteableBitmapPage.cs`.

use ferroui_base::StyledElementImplExt;
use ferroui_base::VisualImplExt;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Color, Colors, DrawingContext, IBrush};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, PixelSize, Rect, Ref,
    StyledElementImpl, Vector, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct WriteableBitmapPage {
    base: Control,
    unpremul_bitmap: RefCell<Option<WriteableBitmap>>,
    premul_bitmap: RefCell<Option<WriteableBitmap>>,
    /// The time the stopwatch of the page was started at, in the
    /// milliseconds of the clock of the dispatcher.
    st: i64,
}

ferro_class!(WriteableBitmapPage: Control);
ferro_impl_classes!(WriteableBitmapPage: FerroObjectImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(WriteableBitmapPage { new: WriteableBitmapPage::new });

impl StyledElementImpl for WriteableBitmapPage {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.unpremul_bitmap.borrow_mut() = Some(WriteableBitmap::new(
            PixelSize::new(256, 256),
            Vector::new(96.0, 96.0),
            Some(PixelFormat::BGRA8888),
            Some(AlphaFormat::Unpremul),
        ));
        *this.premul_bitmap.borrow_mut() = Some(WriteableBitmap::new(
            PixelSize::new(256, 256),
            Vector::new(96.0, 96.0),
            Some(PixelFormat::BGRA8888),
            Some(AlphaFormat::Premul),
        ));

        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);

        let unpremul_bitmap = this.unpremul_bitmap.borrow_mut().take();
        if let Some(unpremul_bitmap) = unpremul_bitmap {
            unpremul_bitmap.dispose();
        }

        // As in the upstream sample, the premultiplied bitmap is disposed and stays in its field.
        if let Some(premul_bitmap) = this.premul_bitmap.borrow().as_ref() {
            premul_bitmap.dispose();
        }
        *this.unpremul_bitmap.borrow_mut() = None;
    }
}

impl VisualImpl for WriteableBitmapPage {
    fn render(this: &Self, context: &mut DrawingContext) {
        let unpremul_bitmap = this.unpremul_bitmap.borrow();
        let premul_bitmap = this.premul_bitmap.borrow();
        let (Some(unpremul_bitmap), Some(premul_bitmap)) = (unpremul_bitmap.as_ref(), premul_bitmap.as_ref()) else {
            return;
        };

        fn fill_pixels(bitmap: &WriteableBitmap, fill_alpha: u8, premul: bool) {
            let fb = bitmap.lock();
            {
                let size = fb.size();
                let mut data = vec![0i32; (size.width * size.height) as usize];

                for y in 0..size.height {
                    for x in 0..size.width {
                        let mut color = Color::new(fill_alpha, 0, 255, 0);

                        if premul {
                            let r = (i32::from(color.r) * i32::from(color.a) / 255) as u8;
                            let g = (i32::from(color.g) * i32::from(color.a) / 255) as u8;
                            let b = (i32::from(color.b) * i32::from(color.a) / 255) as u8;

                            color = Color::new(fill_alpha, r, g, b);
                        }

                        data[(y * size.width + x) as usize] = color.to_uint32() as i32;
                    }
                }

                // `Marshal.Copy(data, 0, fb.Address, fb.Size.Width * fb.Size.Height)`.
                fb.with_data(&mut |pixels| {
                    for (pixel, value) in pixels.chunks_exact_mut(4).zip(&data) {
                        pixel.copy_from_slice(&value.to_ne_bytes());
                    }
                });
            }
            fb.dispose();
        }

        Self::parent_render(this, context);

        let elapsed_milliseconds = Dispatcher::ui_thread().now() - this.st;
        let alpha = ((elapsed_milliseconds / 10) % 256) as u8;

        fill_pixels(unpremul_bitmap, alpha, false);
        fill_pixels(premul_bitmap, alpha, true);

        let red: Rc<dyn IBrush> = Brushes::red();
        context.fill_rectangle(&red, Rect::new(0.0, 0.0, 256.0 * 3.0, 256.0), 0.0);

        context.draw_image_with_rects(
            unpremul_bitmap,
            Rect::new(0.0, 0.0, 256.0, 256.0),
            Rect::new(0.0, 0.0, 256.0, 256.0),
        );

        context.draw_image_with_rects(
            premul_bitmap,
            Rect::new(0.0, 0.0, 256.0, 256.0),
            Rect::new(256.0, 0.0, 256.0, 256.0),
        );

        let lime: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::LIME, f64::from(alpha) / 255.0));
        context.fill_rectangle(&lime, Rect::new(512.0, 0.0, 256.0, 256.0), 0.0);

        let page = this.to_ref();
        Dispatcher::ui_thread().post_local(move || page.invalidate_visual(), DispatcherPriority::BACKGROUND);
    }
}

impl WriteableBitmapPage {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            unpremul_bitmap: RefCell::new(None),
            premul_bitmap: RefCell::new(None),
            st: Dispatcher::ui_thread().now(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
