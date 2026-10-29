//! Port of `ManagedDragDrop/ManagedDataTransfer.cs`.

use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{
    DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem, IDataTransfer, IDataTransferItem, LocalBoxFuture,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// A data transfer that carries one managed payload as it is (not a serialized surrogate), so that
/// the drop target of a managed drag recovers the original object.
pub(crate) struct ManagedPayloadDataTransfer {
    item: Rc<ManagedPayloadDataTransferItem>,
    items: Rc<[Rc<dyn IDataTransferItem>]>,
    async_items: Rc<[Rc<dyn IAsyncDataTransferItem>]>,
}

impl ManagedPayloadDataTransfer {
    fn new(item: ManagedPayloadDataTransferItem) -> Self {
        let item = Rc::new(item);
        let items: Rc<[Rc<dyn IDataTransferItem>]> = Rc::from([item.clone() as Rc<dyn IDataTransferItem>]);
        let async_items: Rc<[Rc<dyn IAsyncDataTransferItem>]> =
            Rc::from([item.clone() as Rc<dyn IAsyncDataTransferItem>]);
        Self { item, items, async_items }
    }

    /// A data transfer of `payload` in the format `format`.
    ///
    /// # Panics
    /// Panics if `format` is empty or white space (the `ArgumentException` of the reference).
    pub(crate) fn create(format: &str, payload: BoxedValue) -> Rc<dyn IDataTransfer> {
        Rc::new(Self::new(ManagedPayloadDataTransferItem::new(format, payload)))
    }
}

impl IDisposable for ManagedPayloadDataTransfer {
    fn dispose(&self) {}
}

impl IDataTransfer for ManagedPayloadDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.item.formats.clone()
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        self.items.clone()
    }

    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        Some(self)
    }
}

impl IAsyncDataTransfer for ManagedPayloadDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.item.formats.clone()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.async_items.clone()
    }

    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        Some(self)
    }
}

/// The item of a [`ManagedPayloadDataTransfer`]: the payload in its format, and a text payload as
/// text too.
pub(crate) struct ManagedPayloadDataTransferItem {
    formats: Rc<[DataFormat]>,
    payload_format: DataFormat,
    payload: BoxedValue,
}

impl ManagedPayloadDataTransferItem {
    /// # Panics
    /// Panics if `format` is empty or white space (the `ArgumentException` of the reference).
    pub(crate) fn new(format: &str, payload: BoxedValue) -> Self {
        assert!(!format.trim().is_empty(), "The value cannot be an empty string or composed entirely of whitespace. (Parameter 'format')");

        let payload_format = ManagedDataFormatHelper::create_payload_format(format, &payload);
        let formats: Rc<[DataFormat]> = if payload.is::<String>() {
            Rc::from([payload_format.clone(), DataFormat::text().into()])
        } else {
            Rc::from([payload_format.clone()])
        };
        Self { formats, payload_format, payload }
    }

    /// The payload as the raw value of a format: the object the untyped value holds.
    fn raw_payload(&self) -> Rc<dyn Any> {
        // The untyped value is an `Rc<dyn AnyValue>` whose object is the payload itself: viewed
        // as `dyn Any` it downcasts to the type of the payload.
        let any: Rc<dyn Any> = self.payload.clone();
        any
    }
}

impl IDataTransferItem for ManagedPayloadDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }

    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        if *format == self.payload_format {
            return Some(self.raw_payload());
        }

        if self.payload.is::<String>() && *format == *DataFormat::text() {
            return Some(self.raw_payload());
        }

        None
    }
}

impl IAsyncDataTransferItem for ManagedPayloadDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }

    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(IDataTransferItem::try_get_raw(self, format))))
    }
}

/// The data formats of the managed payloads.
pub(crate) struct ManagedDataFormatHelper;

impl ManagedDataFormatHelper {
    /// The formats a payload stored under `identifier` can be found by: the application formats
    /// of bytes and of text for an identifier an application format accepts, the platform
    /// formats otherwise.
    ///
    /// # Panics
    /// Panics if `identifier` is empty or white space (the `ArgumentException` of the reference).
    pub(crate) fn create_lookup_formats(identifier: &str) -> [DataFormat; 2] {
        assert!(
            !identifier.trim().is_empty(),
            "The value cannot be an empty string or composed entirely of whitespace. (Parameter 'identifier')"
        );

        if Self::is_valid_application_identifier(identifier) {
            return [
                DataFormat::create_bytes_application_format(identifier).into(),
                DataFormat::create_string_application_format(identifier).into(),
            ];
        }

        [
            DataFormat::create_bytes_platform_format(identifier).into(),
            DataFormat::create_string_platform_format(identifier).into(),
        ]
    }

    /// The format of a payload stored under `identifier`: of bytes for a payload of bytes, of
    /// text otherwise; an application format for an identifier an application format accepts, a
    /// platform format otherwise.
    ///
    /// # Panics
    /// Panics if `identifier` is empty or white space (the `ArgumentException` of the reference).
    pub(crate) fn create_payload_format(identifier: &str, payload: &BoxedValue) -> DataFormat {
        assert!(
            !identifier.trim().is_empty(),
            "The value cannot be an empty string or composed entirely of whitespace. (Parameter 'identifier')"
        );

        let is_application_identifier = Self::is_valid_application_identifier(identifier);

        // `byte[]` of the reference: the shared bytes of the byte formats of the framework.
        if payload.is::<Rc<[u8]>>() {
            if is_application_identifier {
                DataFormat::create_bytes_application_format(identifier).into()
            } else {
                DataFormat::create_bytes_platform_format(identifier).into()
            }
        } else if is_application_identifier {
            DataFormat::create_string_application_format(identifier).into()
        } else {
            DataFormat::create_string_platform_format(identifier).into()
        }
    }

    fn is_valid_application_identifier(identifier: &str) -> bool {
        identifier.chars().all(|character| character.is_ascii_alphanumeric() || character == '.' || character == '-')
    }
}
