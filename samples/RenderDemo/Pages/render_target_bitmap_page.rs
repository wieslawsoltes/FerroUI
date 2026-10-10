//! Port of `Pages/RenderTargetBitmapPage.cs`.

use ferroui_base::StyledElementImplExt;
use ferroui_base::VisualImplExt;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{Brushes, DrawingContext, IBrush};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, PixelSize, Rect, Ref,
    StyledElementImpl, Vector, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct RenderTargetBitmapPage {
    base: Control,
    bitmap: RefCell<Option<RenderTargetBitmap>>,
    /// The time the stopwatch of the page was started at, in the
    /// milliseconds of the clock of the dispatcher.
    st: i64,
}

ferro_class!(RenderTargetBitmapPage: Control);
ferro_impl_classes!(RenderTargetBitmapPage: FerroObjectImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(RenderTargetBitmapPage { new: RenderTargetBitmapPage::new });

impl StyledElementImpl for RenderTargetBitmapPage {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.bitmap.borrow_mut() = Some(RenderTargetBitmap::with_dpi(PixelSize::new(200, 200), Vector::new(96.0, 96.0)));
        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        let bitmap = this.bitmap.borrow_mut().take();
        if let Some(bitmap) = bitmap {
            bitmap.dispose();
        }
        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

impl VisualImpl for RenderTargetBitmapPage {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bitmap = this.bitmap.borrow();
        let Some(bitmap) = bitmap.as_ref() else {
            return;
        };

        {
            let elapsed_total_seconds = (Dispatcher::ui_thread().now() - this.st) as f64 / 1000.0;
            let mut ctx = bitmap.create_drawing_context();
            let transform = ctx.push_transform(
                Matrix::create_translation(-100.0, -100.0)
                    * Matrix::create_rotation(elapsed_total_seconds)
                    * Matrix::create_translation(100.0, 100.0),
            );
            let fuchsia: Rc<dyn IBrush> = Brushes::fuchsia();
            ctx.fill_rectangle(&fuchsia, Rect::new(50.0, 50.0, 100.0, 100.0), 0.0);
            ctx.pop(transform);
            ctx.dispose();
        }

        context.draw_image_with_rects(bitmap, Rect::new(0.0, 0.0, 200.0, 200.0), Rect::new(0.0, 0.0, 200.0, 200.0));
        let page = this.to_ref();
        Dispatcher::ui_thread().post_local(move || page.invalidate_visual(), DispatcherPriority::BACKGROUND);
        Self::parent_render(this, context);
    }
}

impl RenderTargetBitmapPage {
    pub fn construct() -> Self {
        Self { base: Control::construct(), bitmap: RefCell::new(None), st: Dispatcher::ui_thread().now() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
