//! Port of `ManagedDragDrop/ManagedDataTransferExtensions.cs`.

use super::managed_data_transfer::ManagedDataFormatHelper;
use ferroui_base::input::{IDataTransfer, IDataTransferItem};
use std::any::Any;
use std::rc::Rc;

/// Provides helpers to retrieve managed drag-drop payloads from data transfer objects.
///
/// Deviation (DEVIATIONS.md, Behaviors): the static class of extension methods of the reference
/// is two extension traits, this one for data transfers and
/// [`ManagedDataTransferItemExtensions`] for their items (the overloads of the reference); the
/// typed forms (`TryGetManagedValue<T>`) are `try_get_managed_value_as`.
pub trait ManagedDataTransferExtensions: IDataTransfer {
    /// Tries to get a managed payload for the specified application format: the stored payload,
    /// or `None` when the format is not present.
    fn try_get_managed_value(&self, format: &str) -> Option<Rc<dyn Any>> {
        for item in self.items().iter() {
            for managed_format in ManagedDataFormatHelper::create_lookup_formats(format) {
                if let Some(value) = item.try_get_raw(&managed_format) {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Tries to get a managed payload for the specified application format as a `T`: the stored
    /// payload, or `None` when it is unavailable or of another type.
    fn try_get_managed_value_as<T: 'static>(&self, format: &str) -> Option<Rc<T>> {
        self.try_get_managed_value(format).and_then(|value| value.downcast::<T>().ok())
    }
}

impl<I: IDataTransfer + ?Sized> ManagedDataTransferExtensions for I {}

/// Provides helpers to retrieve managed drag-drop payloads from the items of data transfer
/// objects (see [`ManagedDataTransferExtensions`]).
pub trait ManagedDataTransferItemExtensions: IDataTransferItem {
    /// Tries to get a managed payload for the specified application format from a single data
    /// transfer item: the stored payload, or `None` when the format is not present.
    fn try_get_managed_value(&self, format: &str) -> Option<Rc<dyn Any>> {
        for managed_format in ManagedDataFormatHelper::create_lookup_formats(format) {
            if let Some(value) = self.try_get_raw(&managed_format) {
                return Some(value);
            }
        }

        None
    }

    /// Tries to get a managed payload for the specified application format from a single data
    /// transfer item as a `T`: the stored payload, or `None` when it is unavailable or of another
    /// type.
    fn try_get_managed_value_as<T: 'static>(&self, format: &str) -> Option<Rc<T>> {
        self.try_get_managed_value(format).and_then(|value| value.downcast::<T>().ok())
    }
}

impl<I: IDataTransferItem + ?Sized> ManagedDataTransferItemExtensions for I {}
