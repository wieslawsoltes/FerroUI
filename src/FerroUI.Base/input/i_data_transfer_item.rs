use super::DataFormat;
use std::any::Any;
use std::rc::Rc;

/// Represent an item inside a [`IDataTransfer`](super::IDataTransfer). An
/// item may support several formats and can return the value of a given
/// format on demand.
///
/// See [`DataTransferItem`](super::DataTransferItem) for the mutable
/// implementation.
pub trait IDataTransferItem {
    /// Gets the formats supported by this item.
    fn formats(&self) -> Rc<[DataFormat]>;

    /// Tries to get a value for a given format. Returns `None` if the
    /// format is not supported.
    ///
    /// Implementations of this method are expected to return a value
    /// matching the exact data type of the underlying
    /// [`DataFormatOf`](super::DataFormatOf).
    ///
    /// To retrieve a typed value, use
    /// [`DataTransferItemExtensions::try_get_value`](super::DataTransferItemExtensions::try_get_value).
    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>>;
}
