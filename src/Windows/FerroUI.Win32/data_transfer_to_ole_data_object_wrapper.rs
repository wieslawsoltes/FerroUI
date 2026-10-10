//! Wraps a data transfer of the framework into a Win32 `IDataObject`.
//!
//! The reference's wrapper is one object that is both the managed wrapper
//! and, through its callback base class, the COM object. Here the COM
//! object (`DataObjectCallback`, private) holds the wrapper: the wrapper is
//! what the backend keeps and asks, the COM object is what the system
//! holds, and when the last reference of the system is released the
//! wrapper is told ([`DataTransferToOleDataObjectWrapper::on_destroyed`]
//! handlers, then the data transfer is released), as `Destroyed` of the
//! reference.
//!
//! The reference recognises one of its own data objects that comes back
//! from the system (a drag inside the process) by unwrapping the managed
//! object of a COM pointer; here the live COM objects of the thread are
//! kept by address ([`DataTransferToOleDataObjectWrapper::try_unwrap`]).

use crate::clipboard_format_registry::ClipboardFormatRegistry;
use crate::interop::unmanaged_methods::{
    ClipboardFormat, COR_E_OBJECTDISPOSED, DATADIR_GET, DVASPECT, DV_E_DVASPECT, DV_E_FORMATETC, DV_E_TYMED, FORMATETC,
    HRESULT, OLE_E_ADVISENOTSUPPORTED, STGMEDIUM, STG_E_MEDIUMFULL, TYMED,
};
use crate::ole_data_object_helper;
use crate::win32_com::{IDataObject, IDataObjectImpl, IEnumFORMATETC, IEnumFORMATETCImpl};
use crate::wnd_proc_guard::guard;
use ferroui_base::input::{DataFormat, DataFormatKind, IDataTransfer};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::{Rc, Weak};

thread_local! {
    /// The data objects of this thread that the system (or the backend)
    /// still holds, by the address of the COM object.
    static LIVE: RefCell<HashMap<usize, Weak<DataTransferToOleDataObjectWrapper>>> = RefCell::new(HashMap::new());
}

pub(crate) struct FormatEnumerator {
    formats: Rc<[FORMATETC]>,
    current: Cell<u32>,
}

impl FormatEnumerator {
    pub(crate) fn new(format_ids: &[u16]) -> FormatEnumerator {
        FormatEnumerator {
            formats: format_ids.iter().map(|&format_id| ole_data_object_helper::to_format_etc(format_id)).collect(),
            current: Cell::new(0),
        }
    }
}


impl IEnumFORMATETCImpl for FormatEnumerator {
    fn next(&self, celt: u32, rgelt: *mut FORMATETC, results: *mut u32) -> u32 {
        if rgelt.is_null() {
            return HRESULT::E_INVALIDARG;
        }

        let mut i = 0u32;
        while i < celt && (self.current.get() as usize) < self.formats.len() {
            // SAFETY: the caller gives an array of `celt` elements, and
            // `i` is less than `celt`.
            unsafe { rgelt.add(i as usize).write(self.formats[self.current.get() as usize]) };
            self.current.set(self.current.get() + 1);
            i += 1;
        }

        if i != celt {
            // The reference leaves the count unwritten here; the contract
            // of the interface is that it is written whenever it is given.
            if !results.is_null() {
                // SAFETY: a non-null count of the caller.
                unsafe { results.write(i) };
            }
            return HRESULT::S_FALSE;
        }

        // "results" parameter can be NULL if celt is 1.
        if !results.is_null() {
            // SAFETY: a non-null count of the caller.
            unsafe { results.write(i) };
        }
        HRESULT::S_OK
    }

    fn skip(&self, celt: u32) -> u32 {
        let current = self.current.get();
        self.current.set(current + celt.min((i32::MAX as u32).saturating_sub(current)));
        if self.current.get() as usize >= self.formats.len() {
            return HRESULT::S_FALSE;
        }
        HRESULT::S_OK
    }

    fn reset(&self) -> Result<(), HResult> {
        self.current.set(0);
        Ok(())
    }

    fn clone(&self) -> Result<Option<ComPtr<IEnumFORMATETC>>, HResult> {
        Ok(Some(IEnumFORMATETC::from_impl(FormatEnumerator {
            formats: self.formats.clone(),
            current: Cell::new(self.current.get()),
        })))
    }
}

/// Wraps a data transfer of the framework into a Win32 `IDataObject`.
pub(crate) struct DataTransferToOleDataObjectWrapper {
    data_transfer: RefCell<Option<Rc<dyn IDataTransfer>>>,
    format_ids: RefCell<Option<Rc<[u16]>>>,
    on_destroyed: RefCell<Vec<Box<dyn Fn()>>>,
}

/// The COM object of a wrapper.
struct DataObjectCallback {
    wrapper: Rc<DataTransferToOleDataObjectWrapper>,
    /// The address the object is kept under, once it is known.
    address: Cell<usize>,
}

impl Drop for DataObjectCallback {
    fn drop(&mut self) {
        let address = self.address.get();
        // The thread-local list is gone when the thread ends.
        let _ = LIVE.try_with(|live| live.borrow_mut().remove(&address));
        self.wrapper.destroyed();
    }
}

impl DataTransferToOleDataObjectWrapper {
    /// Wraps a data transfer: the wrapper, and the COM object the system
    /// is given. The data transfer is released when the last reference of
    /// the COM object is.
    pub(crate) fn new(data_transfer: Rc<dyn IDataTransfer>) -> (Rc<DataTransferToOleDataObjectWrapper>, ComPtr<IDataObject>) {
        let wrapper = Rc::new(DataTransferToOleDataObjectWrapper {
            data_transfer: RefCell::new(Some(data_transfer)),
            format_ids: RefCell::new(None),
            on_destroyed: RefCell::new(Vec::new()),
        });
        let data_object = IDataObject::from_impl(DataObjectCallback { wrapper: wrapper.clone(), address: Cell::new(0) });
        Self::register(&wrapper, &data_object);
        (wrapper, data_object)
    }

    fn register(wrapper: &Rc<DataTransferToOleDataObjectWrapper>, data_object: &ComPtr<IDataObject>) {
        let address = data_object.as_ptr() as usize;
        // SAFETY: the object was just made by `from_impl` over a
        // `DataObjectCallback`, and is alive.
        unsafe { ferroui_microcom::ComObject::<DataObjectCallback>::value(data_object.as_ptr().cast::<c_void>()) }
            .address
            .set(address);
        LIVE.with(|live| live.borrow_mut().insert(address, Rc::downgrade(wrapper)));
    }

    /// The wrapper of a data object, when the object is a live one of this
    /// thread (`MicroComRuntime.TryUnwrapManagedObject` of the reference).
    pub(crate) fn try_unwrap(data_object: &IDataObject) -> Option<Rc<DataTransferToOleDataObjectWrapper>> {
        let address = data_object as *const IDataObject as usize;
        LIVE.with(|live| live.borrow().get(&address).and_then(Weak::upgrade))
    }

    /// The wrapped data transfer, until it is released.
    pub(crate) fn data_transfer(&self) -> Option<Rc<dyn IDataTransfer>> {
        self.data_transfer.borrow().clone()
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.data_transfer.borrow().is_none()
    }

    fn format_ids(&self) -> Rc<[u16]> {
        if let Some(format_ids) = self.format_ids.borrow().as_ref() {
            return format_ids.clone();
        }
        let format_ids = self.calc_format_ids();
        *self.format_ids.borrow_mut() = Some(format_ids.clone());
        format_ids
    }

    /// Adds a handler that is called when the system has released the
    /// data object (`OnDestroyed` of the reference).
    pub(crate) fn on_destroyed(&self, handler: impl Fn() + 'static) {
        self.on_destroyed.borrow_mut().push(Box::new(handler));
    }

    /// Whether a format can be asked for, and which data format it is; the
    /// result code when it cannot.
    fn validate_format(&self, format: &FORMATETC) -> Result<(Rc<dyn IDataTransfer>, DataFormat), u32> {
        if !(format.tymed == TYMED::TYMED_HGLOBAL
            || (format.tymed == TYMED::TYMED_GDI && format.cf_format == ClipboardFormat::CF_BITMAP))
        {
            return Err(DV_E_TYMED);
        }

        if format.dw_aspect != DVASPECT::DVASPECT_CONTENT {
            return Err(DV_E_DVASPECT);
        }

        let Some(data_transfer) = self.data_transfer() else {
            return Err(COR_E_OBJECTDISPOSED);
        };

        if !self.format_ids().contains(&format.cf_format) {
            return Err(DV_E_FORMATETC);
        }

        Ok((data_transfer, ClipboardFormatRegistry::get_or_add_format_from_id(format.cf_format)))
    }

    fn calc_format_ids(&self) -> Rc<[u16]> {
        let Some(data_transfer) = self.data_transfer() else {
            return Rc::from([]);
        };

        let formats = data_transfer.formats();
        let mut format_ids = Vec::with_capacity(formats.len());

        for data_format in formats.iter() {
            if data_format.kind() == DataFormatKind::InProcess {
                continue;
            }

            if DataFormat::bitmap() == *data_format {
                // We add extra formats for bitmaps
                format_ids.extend(
                    ClipboardFormatRegistry::image_formats().iter().map(ClipboardFormatRegistry::get_or_add_format),
                );
            } else {
                format_ids.push(ClipboardFormatRegistry::get_or_add_format(data_format));
            }
        }

        format_ids.into()
    }

    fn destroyed(&self) {
        let handlers = std::mem::take(&mut *self.on_destroyed.borrow_mut());
        for handler in &handlers {
            handler();
        }
        self.release_data_transfer();
    }

    pub(crate) fn release_data_transfer(&self) {
        let data_transfer = self.data_transfer.borrow_mut().take();
        if let Some(data_transfer) = data_transfer {
            data_transfer.dispose();
        }
    }

    // The members of `IDataObject`.

    pub(crate) fn get_data(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        if format.is_null() || medium.is_null() {
            return HRESULT::E_INVALIDARG;
        }
        // SAFETY: a non-null format of the caller, read for the call.
        let format = unsafe { format.read() };
        let (data_transfer, data_format) = match self.validate_format(&format) {
            Ok(valid) => valid,
            Err(result) => return result,
        };

        let mut filled = STGMEDIUM::default();
        let result = if format.tymed == TYMED::TYMED_GDI {
            filled.tymed = TYMED::TYMED_GDI;
            ole_data_object_helper::write_data_to_gdi(&*data_transfer, &data_format, &mut filled.unionmember)
        } else {
            filled.tymed = TYMED::TYMED_HGLOBAL;
            // SAFETY: the handle is 0: a block is allocated.
            unsafe { ole_data_object_helper::write_data_to_hglobal(&*data_transfer, &data_format, &mut filled.unionmember) }
        };
        // SAFETY: a non-null medium of the caller, which it owns from here.
        unsafe { medium.write(filled) };
        result
    }

    pub(crate) fn get_data_here(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        if format.is_null() || medium.is_null() {
            return HRESULT::E_INVALIDARG;
        }
        // SAFETY: a non-null format of the caller, read for the call.
        let format = unsafe { format.read() };
        let (data_transfer, data_format) = match self.validate_format(&format) {
            Ok(valid) => valid,
            Err(result) => return result,
        };

        // SAFETY: a non-null medium of the caller, whose block (when it
        // has one) the caller allocated and keeps alive for the call.
        unsafe {
            if (*medium).unionmember == 0 {
                return STG_E_MEDIUMFULL;
            }

            let mut h_global = (*medium).unionmember;
            ole_data_object_helper::write_data_to_hglobal(&*data_transfer, &data_format, &mut h_global)
        }
    }

    pub(crate) fn query_get_data(&self, format: *mut FORMATETC) -> u32 {
        if format.is_null() {
            return HRESULT::E_INVALIDARG;
        }
        // SAFETY: a non-null format of the caller, read for the call.
        let format = unsafe { format.read() };
        match self.validate_format(&format) {
            Ok(_) => HRESULT::S_OK,
            Err(result) => result,
        }
    }

    pub(crate) fn enum_format_etc(&self, direction: i32) -> Result<Option<ComPtr<IEnumFORMATETC>>, HResult> {
        if self.is_disposed() {
            return Err(HResult(HRESULT::E_NOTIMPL));
        }

        if direction == DATADIR_GET {
            return Ok(Some(IEnumFORMATETC::from_impl(FormatEnumerator::new(&self.format_ids()))));
        }

        Err(HResult(HRESULT::E_NOTIMPL))
    }
}


impl IDataObjectImpl for DataObjectCallback {
    fn get_data(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        guard(HRESULT::E_UNEXPECTED, || self.wrapper.get_data(format, medium))
    }

    fn get_data_here(&self, format: *mut FORMATETC, medium: *mut STGMEDIUM) -> u32 {
        guard(HRESULT::E_UNEXPECTED, || self.wrapper.get_data_here(format, medium))
    }

    fn query_get_data(&self, format: *mut FORMATETC) -> u32 {
        guard(HRESULT::E_UNEXPECTED, || self.wrapper.query_get_data(format))
    }

    fn get_canonical_format_etc(&self, _format_in: *mut FORMATETC) -> Result<FORMATETC, HResult> {
        Err(HResult(HRESULT::E_NOTIMPL))
    }

    fn set_data(&self, _pformatetc: *mut FORMATETC, _pmedium: *mut STGMEDIUM, _f_release: i32) -> u32 {
        HRESULT::E_NOTIMPL
    }

    fn enum_format_etc(&self, direction: i32) -> Result<Option<ComPtr<IEnumFORMATETC>>, HResult> {
        guard(Err(HResult(HRESULT::E_UNEXPECTED)), || self.wrapper.enum_format_etc(direction))
    }

    fn d_advise(&self, _p_formatetc: *mut FORMATETC, _advf: i32, _advise_sink: *mut c_void) -> Result<i32, HResult> {
        Ok(0)
    }

    fn d_unadvise(&self, _connection: i32) -> Result<(), HResult> {
        Err(HResult(OLE_E_ADVISENOTSUPPORTED))
    }

    fn enum_d_advise(&self) -> Result<*mut c_void, HResult> {
        Ok(std::ptr::null_mut())
    }
}
