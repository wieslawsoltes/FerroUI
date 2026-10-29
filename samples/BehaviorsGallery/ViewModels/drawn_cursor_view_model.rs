//! Port of `ViewModels/DrawnCursorViewModel.cs`.

use super::ViewModelBase;
use ferroui_base::input::Cursor;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{Brushes, IBrush, IPen, Pen};
use ferroui_base::{ferro_markup_type, PixelPoint, PixelSize, Point};
use ferroui_reactiveui::reactive_object;
use ferroui_xaml_interactions_custom::ICursorProvider;
use std::rc::Rc;

/// The view model of the page of the drawn cursor: a provider that draws a cross.
pub struct DrawnCursorViewModel {
    base: ViewModelBase,
    cursor_provider: Rc<dyn ICursorProvider>,
}
reactive_object!(DrawnCursorViewModel, base);

impl PartialEq for DrawnCursorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DrawnCursorViewModel {
    /// `DrawnCursorViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            cursor_provider: Rc::new(CrossCursorProvider),
        })
    }

    /// `CursorProvider`.
    pub fn cursor_provider(&self) -> Rc<dyn ICursorProvider> {
        self.cursor_provider.clone()
    }
}

/// A provider of a cursor drawn as a blue cross.
struct CrossCursorProvider;

impl ICursorProvider for CrossCursorProvider {
    /// `CreateCursor()`.
    fn create_cursor(&self) -> Rc<Cursor> {
        let size = PixelSize::new(32, 32);
        let bitmap = RenderTargetBitmap::new(size);
        {
            let mut ctx = bitmap.create_drawing_context_with_clear(true);
            let brush: Rc<dyn IBrush> = Brushes::blue();
            let pen: Rc<dyn IPen> = Pen::with_brush(Some(brush), 2.0).into();
            ctx.draw_line(
                &pen,
                Point::new(0.0, 0.0),
                Point::new(size.width as f64, size.height as f64),
            );
            ctx.draw_line(
                &pen,
                Point::new(size.width as f64, 0.0),
                Point::new(0.0, size.height as f64),
            );
            ctx.dispose();
        }
        Cursor::from_bitmap(&bitmap, PixelPoint::new(16, 16))
    }
}

type Vm = Rc<DrawnCursorViewModel>;

ferro_markup_type!(class DrawnCursorViewModel {
    this: Rc<DrawnCursorViewModel>,
    handles: [DrawnCursorViewModel, Rc<DrawnCursorViewModel>, Option<Rc<DrawnCursorViewModel>>],
    constructors: [() => DrawnCursorViewModel::new],
    properties: [
        CursorProvider: Option<Rc<dyn ICursorProvider>> { get: |this: &Vm| Some(this.cursor_provider()) },
    ],
    notify_property_changed: DrawnCursorViewModel,
});
