use super::{DataFormat, DataFormatKind};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;

/// Represents a format usable with the clipboard and drag-and-drop, with a
/// data type.
///
/// `T` is the type of the values of the format as they are handed out:
/// [`String`] for text, `Rc<[u8]>` for bytes, `Rc<Bitmap>` for bitmaps. It
/// is only used to resolve the typed accessors.
///
/// This type cannot be instantiated directly. Use universal formats such as
/// [`DataFormat::text`], or create custom formats using
/// [`DataFormat::create_bytes_application_format`],
/// [`DataFormat::create_string_application_format`],
/// [`DataFormat::create_bytes_platform_format`],
/// [`DataFormat::create_string_platform_format`] or
/// [`DataFormat::create_in_process_format`].
pub struct DataFormatOf<T> {
    format: DataFormat,
    _data_type: PhantomData<fn() -> T>,
}

impl<T> DataFormatOf<T> {
    pub(super) fn new(kind: DataFormatKind, identifier: &str) -> Self {
        Self { format: DataFormat::new(kind, identifier), _data_type: PhantomData }
    }

    /// The format without its data type.
    #[inline]
    pub fn as_data_format(&self) -> &DataFormat {
        &self.format
    }
}

impl<T> Deref for DataFormatOf<T> {
    type Target = DataFormat;

    #[inline]
    fn deref(&self) -> &DataFormat {
        &self.format
    }
}

impl<T> Clone for DataFormatOf<T> {
    fn clone(&self) -> Self {
        Self { format: self.format.clone(), _data_type: PhantomData }
    }
}

impl<T> From<DataFormatOf<T>> for DataFormat {
    fn from(value: DataFormatOf<T>) -> DataFormat {
        value.format
    }
}

impl<T, U> PartialEq<DataFormatOf<U>> for DataFormatOf<T> {
    #[inline]
    fn eq(&self, other: &DataFormatOf<U>) -> bool {
        self.format == other.format
    }
}

impl<T> Eq for DataFormatOf<T> {}

impl<T> PartialEq<DataFormat> for DataFormatOf<T> {
    #[inline]
    fn eq(&self, other: &DataFormat) -> bool {
        self.format == *other
    }
}

impl<T> PartialEq<DataFormatOf<T>> for DataFormat {
    #[inline]
    fn eq(&self, other: &DataFormatOf<T>) -> bool {
        *self == other.format
    }
}

impl<T> Hash for DataFormatOf<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.format.hash(state)
    }
}

impl<T> fmt::Display for DataFormatOf<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.format, f)
    }
}

impl<T> fmt::Debug for DataFormatOf<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.format, f)
    }
}
