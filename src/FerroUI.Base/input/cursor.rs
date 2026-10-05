use crate::media::imaging::Bitmap;
use crate::platform::{ICursorFactory, ICursorImpl};
use crate::{FerroLocator, LocatorExtensions, PixelPoint};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Defines the standard cursor types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StandardCursorType {
    #[default]
    Arrow,
    Ibeam,
    Wait,
    Cross,
    UpArrow,
    SizeWestEast,
    SizeNorthSouth,
    SizeAll,
    No,
    Hand,
    AppStarting,
    Help,
    TopSide,
    BottomSide,
    LeftSide,
    RightSide,
    TopLeftCorner,
    TopRightCorner,
    BottomLeftCorner,
    BottomRightCorner,
    DragMove,
    DragCopy,
    DragLink,
    None,
}

impl StandardCursorType {
    const ALL: [StandardCursorType; 24] = [
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

    /// The name of the cursor type.
    pub fn name(self) -> &'static str {
        match self {
            StandardCursorType::Arrow => "Arrow",
            StandardCursorType::Ibeam => "Ibeam",
            StandardCursorType::Wait => "Wait",
            StandardCursorType::Cross => "Cross",
            StandardCursorType::UpArrow => "UpArrow",
            StandardCursorType::SizeWestEast => "SizeWestEast",
            StandardCursorType::SizeNorthSouth => "SizeNorthSouth",
            StandardCursorType::SizeAll => "SizeAll",
            StandardCursorType::No => "No",
            StandardCursorType::Hand => "Hand",
            StandardCursorType::AppStarting => "AppStarting",
            StandardCursorType::Help => "Help",
            StandardCursorType::TopSide => "TopSide",
            StandardCursorType::BottomSide => "BottomSide",
            StandardCursorType::LeftSide => "LeftSide",
            StandardCursorType::RightSide => "RightSide",
            StandardCursorType::TopLeftCorner => "TopLeftCorner",
            StandardCursorType::TopRightCorner => "TopRightCorner",
            StandardCursorType::BottomLeftCorner => "BottomLeftCorner",
            StandardCursorType::BottomRightCorner => "BottomRightCorner",
            StandardCursorType::DragMove => "DragMove",
            StandardCursorType::DragCopy => "DragCopy",
            StandardCursorType::DragLink => "DragLink",
            StandardCursorType::None => "None",
        }
    }

    /// Parses the name of a cursor type, ignoring case.
    pub fn parse(s: &str) -> Option<StandardCursorType> {
        let s = s.trim();
        Self::ALL.iter().copied().find(|t| t.name().eq_ignore_ascii_case(s))
    }
}

impl fmt::Display for StandardCursorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The error returned when a string does not name a standard cursor type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseCursorError(pub String);

impl fmt::Display for ParseCursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unrecognized cursor type '{}'.", self.0)
    }
}

impl std::error::Error for ParseCursorError {}

thread_local! {
    static DEFAULT_CURSOR: RefCell<Option<Rc<Cursor>>> = const { RefCell::new(None) };
}

/// Represents a mouse cursor.
///
/// Cursors have reference semantics: two cursors are equal only if they are
/// the same object.
pub struct Cursor {
    platform_impl: Rc<dyn ICursorImpl>,
    name: String,
}

impl Cursor {
    fn get_cursor_factory() -> Rc<dyn ICursorFactory> {
        FerroLocator::current().get_required_service::<dyn ICursorFactory>()
    }

    /// The default cursor: the standard arrow.
    ///
    /// Panics if no cursor factory is registered with the locator.
    pub fn default_cursor() -> Rc<Cursor> {
        if let Some(cursor) = DEFAULT_CURSOR.with(|cursor| cursor.borrow().clone()) {
            return cursor;
        }
        let cursor = Cursor::new(StandardCursorType::Arrow);
        DEFAULT_CURSOR.with(|slot| *slot.borrow_mut() = Some(cursor.clone()));
        cursor
    }

    /// Creates a standard cursor.
    ///
    /// Panics if no cursor factory is registered with the locator.
    pub fn new(cursor_type: StandardCursorType) -> Rc<Cursor> {
        Rc::new(Cursor {
            platform_impl: Self::get_cursor_factory().get_cursor(cursor_type),
            name: cursor_type.name().to_string(),
        })
    }

    /// Creates a cursor from a bitmap. `hot_spot` is the position within
    /// the bitmap that is the position of the pointer.
    ///
    /// Panics if no cursor factory is registered with the locator.
    pub fn from_bitmap(cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<Cursor> {
        Rc::new(Cursor {
            platform_impl: Self::get_cursor_factory().create_cursor(cursor, hot_spot),
            name: "BitmapCursor".to_string(),
        })
    }

    /// The platform implementation of the cursor.
    pub fn platform_impl(&self) -> &Rc<dyn ICursorImpl> {
        &self.platform_impl
    }

    /// Releases the platform resources of the cursor.
    pub fn dispose(&self) {
        self.platform_impl.dispose()
    }

    /// Creates a standard cursor from the name of its type.
    ///
    /// Panics if no cursor factory is registered with the locator.
    pub fn parse(s: &str) -> Result<Rc<Cursor>, ParseCursorError> {
        match StandardCursorType::parse(s) {
            Some(cursor_type) => Ok(Cursor::new(cursor_type)),
            None => Err(ParseCursorError(s.to_string())),
        }
    }
}

impl PartialEq for Cursor {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Cursor {}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Debug for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
