//! Wraps a Win32 `IDataObject` into a data transfer.

use crate::clipboard_format_registry::ClipboardFormatRegistry;
use crate::interop::unmanaged_methods::{self, DATADIR_GET, FORMATETC};
use crate::ole_data_object_helper::{self, StorageItems};
use crate::ole_data_object_to_data_transfer_item_wrapper::OleDataObjectToDataTransferItemWrapper;
use crate::ole_virtual_file_data;
use crate::win32_com::{IDataObject, IEnumFORMATETC};
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use ferroui_base::platform::storage::IStorageItem;
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::rc::Rc;

/// Wraps a Win32 `IDataObject` into a data transfer.
pub(crate) struct OleDataObjectToDataTransferWrapper {
    /// A reference of the wrapper's own, released when the data transfer
    /// is disposed.
    ole_data_object: RefCell<Option<ComPtr<IDataObject>>>,
}

impl OleDataObjectToDataTransferWrapper {
    /// `ole_data_object` is the wrapped OLE data object.
    pub(crate) fn new(ole_data_object: &IDataObject) -> Rc<PlatformDataTransfer> {
        PlatformDataTransfer::new(OleDataObjectToDataTransferWrapper {
            ole_data_object: RefCell::new(Some(ComPtr::from_ref(ole_data_object))),
        })
    }

    fn data_object(&self) -> Result<ComPtr<IDataObject>, ClipboardError> {
        self.ole_data_object
            .borrow()
            .clone()
            .ok_or_else(|| ClipboardError::other("The data object of the data transfer has been released."))
    }

    fn next(enum_format: &IEnumFORMATETC) -> Option<DataFormat> {
        let mut fetched = 1u32;
        let mut format_etc = FORMATETC::default();

        // SAFETY: room for one element and a count, of this frame.
        let result = unsafe { enum_format.next(1, &mut format_etc, &mut fetched) };
        if result != 0 || fetched == 0 {
            return None;
        }

        if format_etc.ptd != 0 {
            // SAFETY: the target device of a format an enumerator handed
            // out is a block of the COM allocator the receiver frees.
            unsafe { unmanaged_methods::co_task_mem_free(format_etc.ptd as *mut std::ffi::c_void) };
        }

        Some(ClipboardFormatRegistry::get_or_add_format_from_id(format_etc.cf_format))
    }
}

impl PlatformDataTransferImpl for OleDataObjectToDataTransferWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let ole_data_object = self.data_object()?;
        let Ok(Some(enum_format)) = ole_data_object.enum_format_etc(DATADIR_GET) else {
            return Ok(Vec::new());
        };

        let _ = enum_format.reset();

        let mut formats = Vec::new();

        while let Some(format) = Self::next(&enum_format) {
            formats.push(format);
        }

        let mut has_supported_image_format = false;
        let mut has_file = false;
        let mut has_file_group_descriptor = false;
        let mut has_file_contents = false;

        for format in &formats {
            if ClipboardFormatRegistry::is_image_format(format) {
                has_supported_image_format = true;
            }

            if DataFormat::file() == *format {
                has_file = true;
            } else if ole_virtual_file_data::file_group_descriptor_format() == *format {
                has_file_group_descriptor = true;
            } else if ole_virtual_file_data::file_contents_format() == *format {
                has_file_contents = true;
            }
        }

        if has_supported_image_format {
            formats.push(DataFormat::bitmap().into());
        }

        // Shell virtual files have descriptors and indexed contents, but no CF_HDROP path.
        if has_file_group_descriptor && has_file_contents && !has_file {
            formats.push(DataFormat::file().into());
        }

        Ok(formats)
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        let ole_data_object = self.data_object()?;
        let mut non_file_formats: Option<Vec<DataFormat>> = None;
        let mut items = Vec::new();
        let mut has_files = false;

        for format in self.provide_formats()? {
            if DataFormat::file() == format {
                if has_files {
                    continue;
                }

                // This is not ideal as we're reading the filenames ahead of time to generate the appropriate items.
                // However, it's unlikely to be a heavy operation.
                let storage_items = ole_data_object_helper::try_get(&ole_data_object, &format)?;
                if let Some(storage_items) = storage_items.as_ref().and_then(|value| value.downcast_ref::<StorageItems>()) {
                    has_files = true;

                    for storage_item in storage_items {
                        items.push(PlatformDataTransferItem::create(&DataFormat::file(), storage_item.clone()));
                    }
                } else if let Some(virtual_files) =
                    ole_virtual_file_data::try_create_files(&ole_data_object).filter(|files| !files.is_empty())
                {
                    has_files = true;

                    for virtual_file in virtual_files {
                        let virtual_file: Rc<dyn IStorageItem> = virtual_file;
                        items.push(PlatformDataTransferItem::create(&DataFormat::file(), virtual_file));
                    }
                }
            } else {
                non_file_formats.get_or_insert_with(Vec::new).push(format);
            }
        }

        // Single item containing all formats except for DataFormat.File.
        if let Some(non_file_formats) = non_file_formats {
            items.push(OleDataObjectToDataTransferItemWrapper::new(ole_data_object.clone(), non_file_formats));
        }

        Ok(items)
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {
        self.ole_data_object.borrow_mut().take();
    }
}
