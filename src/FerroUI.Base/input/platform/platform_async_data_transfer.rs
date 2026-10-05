use super::ClipboardError;
use crate::input::{DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem};
use crate::reactive::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// The members a platform backend provides for a
/// [`PlatformAsyncDataTransfer`].
pub trait PlatformAsyncDataTransferImpl {
    /// Provides the formats supported by the data transfer. Called until it
    /// succeeds: a failure is reported to the caller and not kept.
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError>;

    /// Provides the items of the data transfer. Called until it succeeds: a
    /// failure is reported to the caller and not kept.
    fn provide_items(&self) -> Result<Vec<Rc<dyn IAsyncDataTransferItem>>, ClipboardError>;

    /// Releases the platform resources of the data transfer. `owner` tells
    /// whether the formats and the items have been provided.
    fn dispose(&self, owner: &PlatformAsyncDataTransfer);
}

/// The base of the asynchronous data transfers of the platform backends:
/// caches the formats and the items.
pub struct PlatformAsyncDataTransfer {
    formats: RefCell<Option<Rc<[DataFormat]>>>,
    items: RefCell<Option<Rc<[Rc<dyn IAsyncDataTransferItem>]>>>,
    platform_impl: Box<dyn PlatformAsyncDataTransferImpl>,
}

impl PlatformAsyncDataTransfer {
    /// Creates the data transfer for a backend implementation.
    pub fn new(platform_impl: impl PlatformAsyncDataTransferImpl + 'static) -> Rc<PlatformAsyncDataTransfer> {
        Rc::new(PlatformAsyncDataTransfer {
            formats: RefCell::new(None),
            items: RefCell::new(None),
            platform_impl: Box::new(platform_impl),
        })
    }

    /// Gets the formats supported by the data transfer.
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

    /// Gets the formats supported by the data transfer, or the failure of
    /// the backend providing them.
    pub fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        if let Some(formats) = self.formats.borrow().as_ref() {
            return Ok(formats.clone());
        }

        let formats: Rc<[DataFormat]> = Rc::from(self.platform_impl.provide_formats()?);
        *self.formats.borrow_mut() = Some(formats.clone());
        Ok(formats)
    }

    /// Whether the formats have been provided by the backend.
    pub fn are_formats_initialized(&self) -> bool {
        self.formats.borrow().is_some()
    }

    /// Gets the items of the data transfer.
    ///
    /// # Panics
    /// Panics if the backend fails to provide them (where the reference
    /// throws from the property); [`try_items`](Self::try_items) reports the
    /// failure instead.
    #[track_caller]
    pub fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        match self.try_items() {
            Ok(items) => items,
            Err(error) => panic!("{error}"),
        }
    }

    /// Gets the items of the data transfer, or the failure of the backend
    /// providing them.
    pub fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        if let Some(items) = self.items.borrow().as_ref() {
            return Ok(items.clone());
        }

        let items: Rc<[Rc<dyn IAsyncDataTransferItem>]> = Rc::from(self.platform_impl.provide_items()?);
        *self.items.borrow_mut() = Some(items.clone());
        Ok(items)
    }

    /// Whether the items have been provided by the backend.
    pub fn are_items_initialized(&self) -> bool {
        self.items.borrow().is_some()
    }
}

impl IDisposable for PlatformAsyncDataTransfer {
    fn dispose(&self) {
        self.platform_impl.dispose(self)
    }
}

impl IAsyncDataTransfer for PlatformAsyncDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        PlatformAsyncDataTransfer::formats(self)
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        PlatformAsyncDataTransfer::items(self)
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        PlatformAsyncDataTransfer::try_formats(self)
    }

    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        PlatformAsyncDataTransfer::try_items(self)
    }
}
