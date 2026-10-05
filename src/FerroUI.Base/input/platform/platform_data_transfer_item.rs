use super::ClipboardError;
use crate::input::{DataFormat, DataFormatOf, IAsyncDataTransferItem, IDataTransferItem, LocalBoxFuture};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The members a platform backend provides for a
/// [`PlatformDataTransferItem`].
pub trait PlatformDataTransferItemImpl {
    /// Provides the formats supported by the item. Called until it
    /// succeeds: a failure is reported to the caller and not kept.
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError>;

    /// Gets the value for a format the item supports.
    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError>;
}

struct SingleFormatItem {
    format: DataFormat,
    value: Rc<dyn Any>,
}

impl PlatformDataTransferItemImpl for SingleFormatItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(vec![self.format.clone()])
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        if self.format == *format {
            Ok(Some(self.value.clone()))
        } else {
            Ok(None)
        }
    }
}

/// The base of the synchronous data transfer items of the platform
/// backends: caches the formats and only asks for the values of supported
/// formats.
pub struct PlatformDataTransferItem {
    formats: RefCell<Option<Rc<[DataFormat]>>>,
    platform_impl: Box<dyn PlatformDataTransferItemImpl>,
}

impl PlatformDataTransferItem {
    /// Creates the item for a backend implementation.
    pub fn new(platform_impl: impl PlatformDataTransferItemImpl + 'static) -> Rc<PlatformDataTransferItem> {
        Rc::new(PlatformDataTransferItem { formats: RefCell::new(None), platform_impl: Box::new(platform_impl) })
    }

    /// Creates an item with a single format and its value.
    pub fn create<T: 'static>(format: &DataFormatOf<T>, value: T) -> Rc<PlatformDataTransferItem> {
        Self::new(SingleFormatItem { format: format.as_data_format().clone(), value: Rc::new(value) })
    }

    /// Gets the formats supported by this item.
    ///
    /// # Panics
    /// Panics if the backend fails to provide them (where the reference
    /// throws from the property); [`try_formats`](Self::try_formats) reports the
    /// failure instead.
    #[track_caller]
    pub fn formats(&self) -> Rc<[DataFormat]> {
        match self.try_formats() {
            Ok(formats) => formats,
            Err(error) => panic!("{error}"),
        }
    }

    /// Gets the formats supported by this item, or the failure of the
    /// backend providing them.
    pub fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        if let Some(formats) = self.formats.borrow().as_ref() {
            return Ok(formats.clone());
        }

        let formats: Rc<[DataFormat]> = Rc::from(self.platform_impl.provide_formats()?);
        *self.formats.borrow_mut() = Some(formats.clone());
        Ok(formats)
    }

    /// Gets whether the item supports a specific format.
    #[track_caller]
    pub fn contains(&self, format: &DataFormat) -> bool {
        self.formats().iter().any(|candidate| candidate == format)
    }

    /// Tries to get a value for a given format. Returns `None` if the
    /// format is not supported.
    ///
    /// # Panics
    /// Panics if the backend fails to provide the formats or the value
    /// (where the reference throws);
    /// [`try_get_raw_result`](Self::try_get_raw_result) reports the failure
    /// instead.
    #[track_caller]
    pub fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        match self.try_get_raw_result(format) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        }
    }

    /// Tries to get a value for a given format: `None` if the format is not
    /// supported, or the failure of the backend providing the formats or
    /// the value.
    pub fn try_get_raw_result(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        if self.try_formats()?.iter().any(|candidate| candidate == format) {
            self.platform_impl.try_get_raw_core(format)
        } else {
            Ok(None)
        }
    }
}

impl IDataTransferItem for PlatformDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        PlatformDataTransferItem::formats(self)
    }

    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        PlatformDataTransferItem::try_get_raw(self, format)
    }
}

impl IAsyncDataTransferItem for PlatformDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        PlatformDataTransferItem::formats(self)
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        PlatformDataTransferItem::try_formats(self)
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::ready(PlatformDataTransferItem::try_get_raw_result(self, format)))
    }
}
