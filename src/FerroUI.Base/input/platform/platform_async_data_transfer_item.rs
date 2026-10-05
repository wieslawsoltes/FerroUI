use super::ClipboardError;
use crate::input::{DataFormat, IAsyncDataTransferItem, LocalBoxFuture};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The members a platform backend provides for a
/// [`PlatformAsyncDataTransferItem`].
pub trait PlatformAsyncDataTransferItemImpl {
    /// Provides the formats supported by the item. Called until it
    /// succeeds: a failure is reported to the caller and not kept.
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError>;

    /// Gets the value for a format the item supports.
    fn try_get_raw_core_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>>;
}

/// The base of the asynchronous data transfer items of the platform
/// backends: caches the formats and only asks for the values of supported
/// formats.
pub struct PlatformAsyncDataTransferItem {
    formats: RefCell<Option<Rc<[DataFormat]>>>,
    platform_impl: Box<dyn PlatformAsyncDataTransferItemImpl>,
}

impl PlatformAsyncDataTransferItem {
    /// Creates the item for a backend implementation.
    pub fn new(platform_impl: impl PlatformAsyncDataTransferItemImpl + 'static) -> Rc<PlatformAsyncDataTransferItem> {
        Rc::new(PlatformAsyncDataTransferItem { formats: RefCell::new(None), platform_impl: Box::new(platform_impl) })
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

    /// Tries to get a value for a given format. The future resolves to
    /// `None` if the format is not supported, and to an error if the
    /// backend fails to provide the formats or the value.
    pub fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        match self.try_formats() {
            Ok(formats) if formats.iter().any(|candidate| candidate == format) => {
                self.platform_impl.try_get_raw_core_async(format)
            }
            Ok(_) => Box::pin(std::future::ready(Ok(None))),
            Err(error) => Box::pin(std::future::ready(Err(error))),
        }
    }
}

impl IAsyncDataTransferItem for PlatformAsyncDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        PlatformAsyncDataTransferItem::formats(self)
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        PlatformAsyncDataTransferItem::try_formats(self)
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        PlatformAsyncDataTransferItem::try_get_raw_async(self, format)
    }
}
