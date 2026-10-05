//! Port of `ViewModels/CursorPageViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::input::{Cursor, StandardCursorType};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_markup_type, PixelPoint};
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::rc::Rc;

/// The values of `StandardCursorType`, in the order of their declaration
/// (`Enum.GetValues<StandardCursorType>()`).
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

/// The view model of the cursor page.
pub struct CursorPageViewModel {
    base: ViewModelBase,
    standard_cursors: Rc<BindableList<Rc<StandardCursorModel>>>,
    custom_cursor: Rc<Cursor>,
}

impl PartialEq for CursorPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for CursorPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl CursorPageViewModel {
    /// # Panics
    /// Panics if the icon of the custom cursor cannot be opened or decoded
    /// (an exception of the constructor in the managed original).
    pub fn new() -> Rc<CursorPageViewModel> {
        let standard_cursors = BindableList::new(STANDARD_CURSOR_TYPES.into_iter().map(StandardCursorModel::new));

        let uri = Uri::absolute("ferres://ControlCatalog/Assets/icon-32.png").unwrap_or_else(|e| panic!("{e}"));
        let mut s = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
        let bitmap = Bitmap::from_stream(&mut s).unwrap_or_else(|e| panic!("{e}"));
        let custom_cursor = Cursor::from_bitmap(&bitmap, PixelPoint::new(16, 16));

        Rc::new(Self { base: ViewModelBase::new(), standard_cursors, custom_cursor })
    }

    pub fn standard_cursors(&self) -> Rc<BindableList<Rc<StandardCursorModel>>> {
        self.standard_cursors.clone()
    }

    pub fn custom_cursor(&self) -> Rc<Cursor> {
        self.custom_cursor.clone()
    }
}

ferro_markup_type!(class CursorPageViewModel {
    this: Rc<CursorPageViewModel>,
    handles: [CursorPageViewModel, Rc<CursorPageViewModel>, Option<Rc<CursorPageViewModel>>],
    constructors: [() => CursorPageViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        StandardCursors: ItemsSource {
            get: |this: &Rc<CursorPageViewModel>| ItemsSource::from(this.standard_cursors())
        },
        CustomCursor: Rc<Cursor> { get: |this: &Rc<CursorPageViewModel>| this.custom_cursor() },
    ],
    notify_property_changed: CursorPageViewModel,
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
    pub fn new(type_: StandardCursorType) -> Rc<StandardCursorModel> {
        Rc::new(Self { type_, cursor: Cursor::new(type_) })
    }

    pub fn type_(&self) -> StandardCursorType {
        self.type_
    }

    pub fn cursor(&self) -> Rc<Cursor> {
        self.cursor.clone()
    }
}

ferro_markup_type!(class StandardCursorModel {
    this: Rc<StandardCursorModel>,
    handles: [StandardCursorModel, Rc<StandardCursorModel>, Option<Rc<StandardCursorModel>>],
    constructors: [(StandardCursorType) => StandardCursorModel::new],
    properties: [
        Type: StandardCursorType { get: |this: &Rc<StandardCursorModel>| this.type_() },
        Cursor: Rc<Cursor> { get: |this: &Rc<StandardCursorModel>| this.cursor() },
    ],
});


#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_standard_cursor_types_are_all_the_values_in_order() {
        for (index, type_) in STANDARD_CURSOR_TYPES.into_iter().enumerate() {
            assert_eq!(index as i32, type_ as i32);
        }
    }
}
