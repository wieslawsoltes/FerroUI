//! Port of `ViewModels/CursorViewModel.cs`.

use super::items::{item_list, ItemList};
use super::ViewModelBase;
use ferroui_base::input::{Cursor, StandardCursorType};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{Brushes, IBrush, IPen, Pen};
use ferroui_base::{ferro_markup_type, PixelPoint, PixelSize, Point};
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The standard cursor types (`Enum.GetValues<StandardCursorType>()`).
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the enumeration of the framework has no public
/// list of its values; the view model lists them in the order of their declaration.
const STANDARD_CURSOR_TYPES: [StandardCursorType; 24] = [
    StandardCursorType::Arrow,
    StandardCursorType::Ibeam,
    StandardCursorType::Wait,
    StandardCursorType::Cross,
    StandardCursorType::UpArrow,
    StandardCursorType::SizeWestEast,
    StandardCursorType::SizeNorthSouth,
    StandardCursorType::SizeAll,
    StandardCursorType::No,
    StandardCursorType::Hand,
    StandardCursorType::AppStarting,
    StandardCursorType::Help,
    StandardCursorType::TopSide,
    StandardCursorType::BottomSide,
    StandardCursorType::LeftSide,
    StandardCursorType::RightSide,
    StandardCursorType::TopLeftCorner,
    StandardCursorType::TopRightCorner,
    StandardCursorType::BottomLeftCorner,
    StandardCursorType::BottomRightCorner,
    StandardCursorType::DragMove,
    StandardCursorType::DragCopy,
    StandardCursorType::DragLink,
    StandardCursorType::None,
];

/// The view model of the page of the cursors: the standard cursors and a cursor drawn in a bitmap.
pub struct CursorViewModel {
    base: ViewModelBase,
    standard_cursors: ItemList,
    custom_cursor: Rc<Cursor>,
}
reactive_object!(CursorViewModel, base);

impl PartialEq for CursorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CursorViewModel {
    /// `CursorViewModel()`.
    pub fn new() -> Rc<Self> {
        let standard_cursors = item_list(
            STANDARD_CURSOR_TYPES
                .into_iter()
                .map(StandardCursorModel::new),
        );

        let size = PixelSize::new(32, 32);
        let bitmap = RenderTargetBitmap::new(size);
        {
            let mut ctx = bitmap.create_drawing_context_with_clear(true);
            let brush: Rc<dyn IBrush> = Brushes::red();
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
        let custom_cursor = Cursor::from_bitmap(&bitmap, PixelPoint::new(16, 16));

        Rc::new(Self {
            base: ViewModelBase::new(),
            standard_cursors,
            custom_cursor,
        })
    }

    /// `StandardCursors`.
    pub fn standard_cursors(&self) -> ItemList {
        self.standard_cursors.clone()
    }

    /// `CustomCursor`.
    pub fn custom_cursor(&self) -> Rc<Cursor> {
        self.custom_cursor.clone()
    }
}

type Vm = Rc<CursorViewModel>;

ferro_markup_type!(class CursorViewModel {
    this: Rc<CursorViewModel>,
    handles: [CursorViewModel, Rc<CursorViewModel>, Option<Rc<CursorViewModel>>],
    constructors: [() => CursorViewModel::new],
    properties: [
        StandardCursors: ItemsSource { get: |this: &Vm| ItemsSource::from(this.standard_cursors()) },
        CustomCursor: Option<Rc<Cursor>> { get: |this: &Vm| Some(this.custom_cursor()) },
    ],
    notify_property_changed: CursorViewModel,
});

/// A standard cursor and its type.
pub struct StandardCursorModel {
    type_: StandardCursorType,
    cursor: Rc<Cursor>,
}

impl PartialEq for StandardCursorModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl StandardCursorModel {
    /// `StandardCursorModel(StandardCursorType)`.
    pub fn new(type_: StandardCursorType) -> Rc<Self> {
        Rc::new(Self {
            type_,
            cursor: Cursor::new(type_),
        })
    }

    /// `Type`.
    pub fn type_(&self) -> StandardCursorType {
        self.type_
    }

    /// `Cursor`.
    pub fn cursor(&self) -> Rc<Cursor> {
        self.cursor.clone()
    }
}

type Model = Rc<StandardCursorModel>;

ferro_markup_type!(class StandardCursorModel {
    this: Rc<StandardCursorModel>,
    handles: [StandardCursorModel, Rc<StandardCursorModel>, Option<Rc<StandardCursorModel>>],
    properties: [
        Type: StandardCursorType { get: |this: &Model| this.type_() },
        Cursor: Option<Rc<Cursor>> { get: |this: &Model| Some(this.cursor()) },
    ],
});
