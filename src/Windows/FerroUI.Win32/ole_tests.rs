//! Tests of the data transfer over OLE that run against the system (on
//! Windows only): blocks of global memory, a data transfer as a data
//! object and back, and the tests of virtual files of the reference
//! (`OleVirtualFileDataTests` of its integration tests: they test types
//! that are internal to the backend, which a test crate of the port cannot
//! reach, so they are tests of the crate).
//!
//! No test here touches the clipboard of the system: the tests of a crate
//! run side by side, and the clipboard is one for the session. The smoke
//! run of the example covers it.

use crate::clipboard_format_registry::ClipboardFormatRegistry;
use crate::data_transfer_to_ole_data_object_wrapper::DataTransferToOleDataObjectWrapper;
use crate::interop::unmanaged_methods::{
    self, GlobalAllocFlags, DATADIR_GET, DVASPECT, DV_E_DVASPECT, DV_E_FORMATETC, DV_E_TYMED, FORMATETC, HRESULT, STGMEDIUM,
    STG_E_MEDIUMFULL, TYMED,
};
use crate::ole_data_object_helper::{self, to_format_etc};
use crate::ole_data_object_to_data_transfer_wrapper::OleDataObjectToDataTransferWrapper;
use crate::ole_virtual_file_data::{self, tests::create_file_group_descriptor_bytes};
use crate::win32_com::{
    IDataObject, IDataObjectAsyncCapability, IDataObjectAsyncCapabilityImpl, IDataObjectImpl, IDataObjectVtbl, IEnumFORMATETC,
};
use ferroui_base::input::{
    DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, DataTransferItemExtensions, IDataTransfer,
};
use ferroui_base::platform::storage::file_io::{BclStorageItemHandle, StorageProviderHelpers};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::reactive::IDisposable;
use ferroui_microcom::{ComObject, ComPtr, Guid, HResult, IUnknownVtbl, ImplementedBy, Interface, RawHResult, S_OK};
use std::cell::Cell;
use std::ffi::c_void;
use std::future::Future;
use std::io::Read;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// The value of a future that is ready when it is first asked.
fn ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future is not ready"),
    }
}

/// COM on the thread of a test: a single-threaded apartment, as the UI
/// thread is.
fn initialize_ole() {
    let result = unmanaged_methods::ole_initialize();
    assert!(result == HRESULT::S_OK || result == HRESULT::S_FALSE, "OleInitialize: {result:#010x}");
}

#[test]
fn bytes_round_trip_through_a_block_of_global_memory() {
    let data: Vec<u8> = (0..=255).collect();
    let mut h_global = 0isize;
    // SAFETY: no block is given: one is allocated.
    assert_eq!(unsafe { ole_data_object_helper::write_bytes_to_hglobal(&mut h_global, &data) }, HRESULT::S_OK);
    assert_ne!(h_global, 0);
    assert!(unmanaged_methods::global_size(h_global) >= data.len());
    // SAFETY: the block that was just allocated; freed below.
    unsafe {
        let read = ole_data_object_helper::read_bytes_from_hglobal(h_global).unwrap();
        assert_eq!(&read[..data.len()], data);

        // A block that is given has to have room.
        let mut same = h_global;
        assert_eq!(ole_data_object_helper::write_bytes_to_hglobal(&mut same, &[7; 16]), HRESULT::S_OK);
        assert_eq!(same, h_global);
        assert_eq!(&ole_data_object_helper::read_bytes_from_hglobal(h_global).unwrap()[..16], [7; 16]);
        assert_eq!(ole_data_object_helper::write_bytes_to_hglobal(&mut same, &vec![0; 4096]), STG_E_MEDIUMFULL);
        unmanaged_methods::global_free(h_global);
    }
}

fn custom_text_format() -> ferroui_base::input::DataFormatOf<String> {
    DataFormat::create_string_application_format("ferroui-win32-tests.text")
}

fn custom_bytes_format() -> ferroui_base::input::DataFormatOf<Rc<[u8]>> {
    DataFormat::create_bytes_application_format("ferroui-win32-tests.bytes")
}

/// A data transfer with text, a string in a format of the application and
/// bytes in another.
fn sample_data_transfer() -> Rc<DataTransfer> {
    let item = DataTransferItem::new();
    item.set_text(Some("Zażółć \u{1F600}"));
    item.set(&custom_text_format(), Some("custom text".to_string()));
    item.set(&custom_bytes_format(), Some(Rc::from(vec![0u8, 1, 2, 0, 255])));
    let transfer = DataTransfer::new();
    transfer.add(item);
    transfer
}

fn enumerate(enumerator: &IEnumFORMATETC) -> Vec<u16> {
    let mut ids = Vec::new();
    loop {
        let mut format = FORMATETC::default();
        let mut fetched = 0u32;
        // SAFETY: room for one element and a count, of this frame.
        let result = unsafe { enumerator.next(1, &mut format, &mut fetched) };
        if result != HRESULT::S_OK {
            assert_eq!((result, fetched), (HRESULT::S_FALSE, 0));
            return ids;
        }
        assert_eq!(fetched, 1);
        assert_eq!(format, to_format_etc(format.cf_format));
        ids.push(format.cf_format);
    }
}

#[test]
fn a_data_transfer_is_a_data_object_of_its_formats() {
    let (wrapper, data_object) = DataTransferToOleDataObjectWrapper::new(sample_data_transfer());
    let text_id = ClipboardFormatRegistry::get_or_add_format(&custom_text_format());
    let bytes_id = ClipboardFormatRegistry::get_or_add_format(&custom_bytes_format());
    assert!(text_id >= 0xC000 && bytes_id >= 0xC000 && text_id != bytes_id, "{text_id:#x} {bytes_id:#x}");
    assert_eq!(unmanaged_methods::get_clipboard_format_name(text_id).as_deref(), Some("frn-app-fmt:ferroui-win32-tests.text"));

    // The formats, in the order of the data transfer.
    let enumerator = data_object.enum_format_etc(DATADIR_GET).unwrap().unwrap();
    assert_eq!(enumerate(&enumerator), [13, text_id, bytes_id]);
    // Reset, skip and clone.
    enumerator.reset().unwrap();
    assert_eq!(enumerator.skip(1), HRESULT::S_OK);
    let clone = IEnumFORMATETC::clone(&enumerator).unwrap().unwrap();
    assert_eq!(enumerate(&clone), [text_id, bytes_id]);
    assert_eq!(enumerate(&enumerator), [text_id, bytes_id]);
    enumerator.reset().unwrap();
    assert_eq!(enumerator.skip(3), HRESULT::S_FALSE);
    // Several at once: fewer than asked for is reported with their count.
    enumerator.reset().unwrap();
    let mut formats = [FORMATETC::default(); 5];
    let mut fetched = 99u32;
    // SAFETY: room for five elements and a count, of this frame.
    assert_eq!(unsafe { enumerator.next(5, formats.as_mut_ptr(), &mut fetched) }, HRESULT::S_FALSE);
    assert_eq!(fetched, 3);
    // The formats a data object can be given are not enumerated.
    assert!(data_object.enum_format_etc(2).is_err());

    // What is asked for and how.
    let query = |mut format: FORMATETC| {
        // SAFETY: a structure of this frame.
        unsafe { data_object.query_get_data(&mut format) }
    };
    assert_eq!(query(to_format_etc(13)), HRESULT::S_OK);
    assert_eq!(query(to_format_etc(text_id)), HRESULT::S_OK);
    assert_eq!(query(to_format_etc(15)), DV_E_FORMATETC);
    assert_eq!(query(FORMATETC { tymed: TYMED::TYMED_ISTREAM, ..to_format_etc(13) }), DV_E_TYMED);
    assert_eq!(query(FORMATETC { tymed: TYMED::TYMED_GDI, ..to_format_etc(13) }), DV_E_TYMED);
    assert_eq!(query(FORMATETC { dw_aspect: DVASPECT::DVASPECT_ICON, ..to_format_etc(13) }), DV_E_DVASPECT);

    // The data of a format, in a block the object allocates.
    let mut format = to_format_etc(13);
    let mut medium = STGMEDIUM::default();
    // SAFETY: two structures of this frame; the medium is released below.
    unsafe {
        assert_eq!(data_object.get_data(&mut format, &mut medium), HRESULT::S_OK);
        assert_eq!(medium.tymed, TYMED::TYMED_HGLOBAL);
        let bytes = ole_data_object_helper::read_bytes_from_hglobal(medium.unionmember).unwrap();
        assert_eq!(ole_data_object_helper::read_string_from_bytes(&bytes), "Zażółć \u{1F600}");

        // The data of a format, in a block of the caller.
        let mut here = STGMEDIUM { tymed: TYMED::TYMED_HGLOBAL, ..Default::default() };
        assert_eq!(data_object.get_data_here(&mut format, &mut here), STG_E_MEDIUMFULL);
        here.unionmember = unmanaged_methods::global_alloc(GlobalAllocFlags::GHND, 2);
        assert_eq!(data_object.get_data_here(&mut format, &mut here), STG_E_MEDIUMFULL);
        unmanaged_methods::global_free(here.unionmember);
        here.unionmember = unmanaged_methods::global_alloc(GlobalAllocFlags::GHND, 256);
        assert_eq!(data_object.get_data_here(&mut format, &mut here), HRESULT::S_OK);
        let bytes = ole_data_object_helper::read_bytes_from_hglobal(here.unionmember).unwrap();
        assert_eq!(ole_data_object_helper::read_string_from_bytes(&bytes), "Zażółć \u{1F600}");
        unmanaged_methods::global_free(here.unionmember);
        unmanaged_methods::release_stg_medium(&mut medium);

        assert_eq!(data_object.set_data(&mut format, &mut medium, 0), HRESULT::E_NOTIMPL);
        assert!(data_object.get_canonical_format_etc(&mut format).is_err());
    }
    assert_eq!(data_object.d_unadvise(0), Err(HResult(0x8004_0003)));

    // The object of this process is recognised, and tells when the last
    // reference is released.
    assert!(Rc::ptr_eq(&DataTransferToOleDataObjectWrapper::try_unwrap(&data_object).unwrap(), &wrapper));
    let destroyed = Rc::new(Cell::new(false));
    let flag = destroyed.clone();
    wrapper.on_destroyed(move || flag.set(true));
    let address = data_object.as_ptr();
    assert!(!wrapper.is_disposed());
    drop(enumerator);
    drop(clone);
    drop(data_object);
    assert!(destroyed.get());
    assert!(wrapper.is_disposed());
    // The address is not a data object of this process any more.
    // SAFETY: the pointer is only used as a key; nothing is read through
    // the reference.
    assert!(DataTransferToOleDataObjectWrapper::try_unwrap(unsafe { &*address }).is_none());
}

#[test]
fn a_data_object_is_a_data_transfer_of_its_formats() {
    let (_wrapper, data_object) = DataTransferToOleDataObjectWrapper::new(sample_data_transfer());
    let transfer = OleDataObjectToDataTransferWrapper::new(&data_object);
    let transfer_contract: Rc<dyn IDataTransfer> = transfer.clone();

    let formats = transfer_contract.formats();
    assert_eq!(formats.len(), 3);
    assert!(formats.contains(&DataFormat::text()));
    assert!(formats.contains(&custom_text_format()));
    assert!(formats.contains(&custom_bytes_format()));
    // One item with every format that is not a file.
    assert_eq!(transfer_contract.items().len(), 1);

    assert_eq!(transfer_contract.try_get_text().as_deref(), Some("Zażółć \u{1F600}"));
    assert_eq!(transfer_contract.try_get_value(&custom_text_format()).as_deref(), Some("custom text"));
    let bytes = transfer_contract.try_get_value(&custom_bytes_format()).unwrap();
    assert_eq!(&bytes[..5], [0u8, 1, 2, 0, 255]);
    assert!(transfer_contract.try_get_value(&DataFormat::file()).is_none());
    assert!(transfer_contract.try_get_value(&DataFormat::create_string_application_format("ferroui-win32-tests.absent")).is_none());

    transfer.dispose();
}

#[test]
fn files_travel_as_a_list_of_their_paths() {
    let directory = std::env::temp_dir().join(format!("ferroui-win32-ole-tests-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("a file ż.txt");
    std::fs::write(&path, b"contents").unwrap();
    let storage_item = |path: &std::path::Path| -> Rc<dyn IStorageItem> {
        match StorageProviderHelpers::try_create_bcl_storage_item(Some(&path.to_string_lossy())).unwrap() {
            BclStorageItemHandle::Folder(folder) => folder,
            BclStorageItemHandle::File(file) => file,
        }
    };

    let source = DataTransfer::new();
    let (file_item, directory_item) = (storage_item(&path), storage_item(&directory));
    // The paths as the storage items have them.
    let expected = [file_item.try_get_local_path().unwrap(), directory_item.try_get_local_path().unwrap()];
    source.add(DataTransferItem::create_file(Some(file_item)));
    source.add(DataTransferItem::create_file(Some(directory_item)));
    let (_wrapper, data_object) = DataTransferToOleDataObjectWrapper::new(source);

    let transfer: Rc<dyn IDataTransfer> = OleDataObjectToDataTransferWrapper::new(&data_object);
    assert_eq!(&*transfer.formats(), [DataFormat::file().as_data_format().clone()]);
    // An item for each file.
    assert_eq!(transfer.items().len(), 2);
    let files = transfer.try_get_files().unwrap();
    let paths: Vec<String> = files.iter().map(|file| file.try_get_local_path().unwrap()).collect();
    assert_eq!(paths, expected);
    assert!(files[0].clone().as_storage_file().is_some());
    assert!(files[1].clone().as_storage_folder().is_some());

    transfer.dispose();
    drop(data_object);
    let _ = std::fs::remove_dir_all(&directory);
}

/// What the data object of the test below saw of the asynchronous
/// capability.
#[derive(Default)]
struct AsyncState {
    async_mode: Cell<bool>,
    start_operation_count: Cell<i32>,
    end_operation_count: Cell<i32>,
}

struct Capability(Rc<AsyncState>);

impl IDataObjectAsyncCapabilityImpl for Capability {
    fn set_async_mode(&self, do_operation_async: i32) -> Result<(), HResult> {
        self.0.async_mode.set(do_operation_async != 0);
        Ok(())
    }

    fn get_async_mode(&self) -> Result<i32, HResult> {
        Ok(i32::from(self.0.async_mode.get()))
    }

    fn start_operation(&self, _reserved: *mut c_void) -> Result<(), HResult> {
        self.0.start_operation_count.set(self.0.start_operation_count.get() + 1);
        Ok(())
    }

    fn in_operation(&self) -> Result<i32, HResult> {
        Ok(i32::from(self.0.start_operation_count.get() > self.0.end_operation_count.get()))
    }

    fn end_operation(&self, _result: RawHResult, _reserved: *mut c_void, _effects: i32) -> Result<(), HResult> {
        self.0.end_operation_count.set(self.0.end_operation_count.get() + 1);
        Ok(())
    }
}

/// `AsyncVirtualFileDataObject` of the tests of the reference: the data
/// object of a data transfer that also has the asynchronous capability.
/// The reference derives from the wrapper and implements a second
/// interface; here the object passes the calls of `IDataObject` on to the
/// data object of the wrapper and answers the query for the capability
/// with an object of its own.
struct AsyncVirtualFileDataObject {
    inner: ComPtr<IDataObject>,
    capability: ComPtr<IDataObjectAsyncCapability>,
}

impl IDataObjectImpl for AsyncVirtualFileDataObject {
    fn get_data(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        // SAFETY (of every call passed on): the arguments of the caller.
        unsafe { self.inner.get_data(format, medium) }
    }

    fn get_data_here(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        unsafe { self.inner.get_data_here(format, medium) }
    }

    fn query_get_data(&self, format: *mut FORMATETC) -> u32 {
        unsafe { self.inner.query_get_data(format) }
    }

    fn get_canonical_format_etc(&self, format_in: *mut FORMATETC) -> Result<FORMATETC, HResult> {
        unsafe { self.inner.get_canonical_format_etc(format_in) }
    }

    fn set_data(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM, release: i32) -> u32 {
        unsafe { self.inner.set_data(format, medium, release) }
    }

    fn enum_format_etc(&self, direction: i32) -> Result<Option<ComPtr<IEnumFORMATETC>>, HResult> {
        self.inner.enum_format_etc(direction)
    }

    fn d_advise(&self, format: *mut FORMATETC, advf: i32, sink: *mut c_void) -> Result<i32, HResult> {
        unsafe { self.inner.d_advise(format, advf, sink) }
    }

    fn d_unadvise(&self, connection: i32) -> Result<(), HResult> {
        self.inner.d_unadvise(connection)
    }

    fn enum_d_advise(&self) -> Result<*mut c_void, HResult> {
        self.inner.enum_d_advise()
    }
}

/// `QueryInterface` of the object above: the capability for its
/// identifier, and what a data object answers otherwise.
unsafe extern "system" fn query_interface_with_capability(this: *mut c_void, riid: *const Guid, ppv: *mut *mut c_void) -> RawHResult {
    // SAFETY: the system calls with valid pointers; `this` is an object
    // made by `async_virtual_file_data_object`.
    unsafe {
        if !riid.is_null() && !ppv.is_null() && IDataObjectAsyncCapability::matches_iid(&*riid) && !IDataObject::matches_iid(&*riid) {
            let object = ComObject::<AsyncVirtualFileDataObject>::value(this);
            *ppv = object.capability.clone().into_raw().cast();
            return S_OK;
        }
        (<IDataObject as ImplementedBy<AsyncVirtualFileDataObject>>::VTBL.base.query_interface)(this, riid, ppv)
    }
}

static ASYNC_VIRTUAL_FILE_DATA_OBJECT_VTBL: IDataObjectVtbl = IDataObjectVtbl {
    base: IUnknownVtbl {
        query_interface: query_interface_with_capability,
        ..IUnknownVtbl::new::<IDataObject, AsyncVirtualFileDataObject>()
    },
    ..IDataObjectVtbl::new::<IDataObject, AsyncVirtualFileDataObject>()
};

fn async_virtual_file_data_object(transfer: Rc<DataTransfer>, state: Rc<AsyncState>) -> ComPtr<IDataObject> {
    let (_wrapper, inner) = DataTransferToOleDataObjectWrapper::new(transfer);
    let object = IDataObject::from_impl(AsyncVirtualFileDataObject {
        inner,
        capability: IDataObjectAsyncCapability::from_impl(Capability(state)),
    });
    // SAFETY: a COM object of the runtime starts with the pointer to its
    // vtable; the replacement is a vtable of the same interface for the
    // same type, which differs in `QueryInterface` alone. Nothing else
    // holds the object yet.
    unsafe { *(object.as_ptr() as *mut *const IDataObjectVtbl) = &ASYNC_VIRTUAL_FILE_DATA_OBJECT_VTBL };
    object
}

/// `Exposes_Virtual_File_As_StorageFile` of the tests of the reference.
/// The reference disposes the file on a worker thread; a storage item of
/// the port belongs to its thread, so it is disposed here.
#[test]
fn exposes_virtual_file_as_storage_file() {
    initialize_ole();
    let expected_contents: Rc<[u8]> = Rc::from(vec![1u8, 2, 3, 4]);
    let transfer_item = DataTransferItem::new();
    transfer_item.set(
        &ole_virtual_file_data::file_group_descriptor_format(),
        Some(Rc::from(create_file_group_descriptor_bytes(&[("archive-entry.txt", expected_contents.len() as u64)]))),
    );
    transfer_item.set(&ole_virtual_file_data::file_contents_format(), Some(expected_contents.clone()));
    let transfer = DataTransfer::new();
    transfer.add(transfer_item);

    let state = Rc::new(AsyncState::default());
    let source = async_virtual_file_data_object(transfer, state.clone());
    let target = OleDataObjectToDataTransferWrapper::new(&source);
    let target_transfer: Rc<dyn IDataTransfer> = target.clone();

    assert!(target_transfer.formats().contains(&DataFormat::file()));
    let items = target_transfer.items();
    assert_eq!(items.len(), 2);
    let item = &items[0];
    assert!(!items[1].formats().contains(&DataFormat::file()));
    let file = item.try_get_value(&DataFormat::file()).expect("the first item is a file");
    let file = file.as_storage_file().expect("the item of a virtual file is a storage file");

    assert_eq!(file.name(), "archive-entry.txt");
    assert_eq!(file.path().original_string(), "virtual-file:///archive-entry.txt");
    assert_eq!(ready(file.get_basic_properties_async()).size(), Some(expected_contents.len() as u64));
    let mut stream = ready(file.open_read_async()).unwrap();
    let mut output = Vec::new();
    stream.read_to_end(&mut output).unwrap();
    // A block of global memory may be larger than what was asked for.
    assert_eq!(&output[..expected_contents.len()], &*expected_contents);

    assert_eq!(state.end_operation_count.get(), 0);
    file.dispose();
    assert!(state.async_mode.get());
    assert_eq!(state.start_operation_count.get(), 1);
    assert_eq!(state.end_operation_count.get(), 1);
    // Disposing again ends nothing again.
    file.dispose();
    assert_eq!(state.end_operation_count.get(), 1);

    target.dispose();
}

/// `Rejects_Truncated_FileGroupDescriptorW` of the tests of the reference.
#[test]
fn rejects_truncated_file_group_descriptor_w() {
    let memory = unmanaged_methods::global_alloc(GlobalAllocFlags::GHND, 4);
    assert_ne!(memory, 0);
    // SAFETY: the block that was just allocated, freed at the end.
    unsafe {
        assert!(unmanaged_methods::write_global(memory, &1u32.to_le_bytes()));
        assert!(ole_virtual_file_data::read_descriptors(memory).is_empty());
        unmanaged_methods::global_free(memory);
    }
}

#[test]
fn reads_the_descriptors_of_a_block_of_global_memory() {
    let bytes = create_file_group_descriptor_bytes(&[("a.txt", 1), ("b.txt", 2)]);
    let mut memory = 0isize;
    // SAFETY: a block is allocated for the bytes, and freed at the end.
    unsafe {
        assert_eq!(ole_data_object_helper::write_bytes_to_hglobal(&mut memory, &bytes), HRESULT::S_OK);
        let descriptors = ole_virtual_file_data::read_descriptors(memory);
        assert_eq!(descriptors.len(), 2);
        assert_eq!((descriptors[1].name.as_str(), descriptors[1].size), ("b.txt", Some(2)));
        unmanaged_methods::global_free(memory);
    }
}
