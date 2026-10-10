use super::win_rt_inspectable::WinRTInspectable;
use super::{IInspectable, IInspectableImpl, IPropertyValue, IPropertyValueImpl, PropertyType, TrustLevel};
use crate::interop::unmanaged_methods::{co_task_mem_alloc, HRESULT};
use ferroui_microcom::{ComPtr, Guid, HResult, Interface};
use std::ffi::c_void;

/// A value of the Windows Runtime the backend hands to the system as an
/// `IPropertyValue`: a single-precision number, an unsigned number or an
/// array of single-precision numbers.
#[allow(dead_code)] // Created by the composition effects of stage 2c.
pub(crate) struct WinRTPropertyValue {
    single_array: Option<Vec<f32>>,
    type_: PropertyType,
    u_int32: u32,
    single: f32,
}

/// "Not supported".
#[allow(dead_code)]
const NOT_IMPLEMENTED: HResult = HResult(0x8000_4001);

#[allow(dead_code)]
impl WinRTPropertyValue {
    pub(crate) fn from_single(f: f32) -> WinRTPropertyValue {
        WinRTPropertyValue { single_array: None, type_: PropertyType::Single, u_int32: 0, single: f }
    }

    pub(crate) fn from_u_int32(u: u32) -> WinRTPropertyValue {
        WinRTPropertyValue { single_array: None, type_: PropertyType::UInt32, u_int32: u, single: 0.0 }
    }

    pub(crate) fn from_single_array(ui_color: Vec<f32>) -> WinRTPropertyValue {
        WinRTPropertyValue { single_array: Some(ui_color), type_: PropertyType::SingleArray, u_int32: 0, single: 0.0 }
    }

    /// The value as an object the system can call.
    pub(crate) fn into_com(self) -> ComPtr<IPropertyValue> {
        IPropertyValue::from_impl(self)
    }
}

impl IInspectableImpl for WinRTPropertyValue {
    fn get_iids(&self, iid_count: *mut u64, iids: *mut *mut Guid) -> Result<(), HResult> {
        // SAFETY: the out parameters of the call the generated thunk
        // received.
        unsafe { WinRTInspectable::get_iids(&[IInspectable::IID, IPropertyValue::IID], iid_count, iids) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        WinRTInspectable::runtime_class_name("FerroUI.Win32.WinRT.WinRTPropertyValue")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(WinRTInspectable::trust_level())
    }
}

impl IPropertyValueImpl for WinRTPropertyValue {
    fn type_(&self) -> Result<PropertyType, HResult> {
        Ok(self.type_)
    }

    fn is_numeric_scalar(&self) -> Result<i32, HResult> {
        Ok(0)
    }

    fn get_u_int8(&self) -> Result<u8, HResult> {
        Ok(0)
    }

    fn get_int16(&self) -> Result<i16, HResult> {
        Ok(0)
    }

    fn get_u_int16(&self) -> Result<u16, HResult> {
        Ok(0)
    }

    fn get_int32(&self) -> Result<i32, HResult> {
        Ok(0)
    }

    fn get_u_int32(&self) -> Result<u32, HResult> {
        Ok(self.u_int32)
    }

    fn get_int64(&self) -> Result<i64, HResult> {
        Ok(0)
    }

    fn get_u_int64(&self) -> Result<u64, HResult> {
        Ok(0)
    }

    fn get_single(&self) -> Result<f32, HResult> {
        Ok(self.single)
    }

    fn get_double(&self) -> Result<f64, HResult> {
        Ok(0.0)
    }

    fn get_char16(&self) -> Result<u16, HResult> {
        Ok(0)
    }

    fn get_boolean(&self) -> Result<i32, HResult> {
        Ok(0)
    }

    fn get_string(&self) -> Result<isize, HResult> {
        Ok(0)
    }

    fn get_guid(&self) -> Result<Guid, HResult> {
        Ok(Guid::ZERO)
    }

    fn get_date_time(&self, _value: *mut c_void) -> Result<(), HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_time_span(&self, _value: *mut c_void) -> Result<(), HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_point(&self, _value: *mut c_void) -> Result<(), HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_size(&self, _value: *mut c_void) -> Result<(), HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_rect(&self, _value: *mut c_void) -> Result<(), HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_u_int8array(&self, __value_size: *mut u32) -> Result<*mut u8, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_int16array(&self, __value_size: *mut u32) -> Result<*mut i16, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_u_int16array(&self, __value_size: *mut u32) -> Result<*mut u16, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_int32array(&self, __value_size: *mut u32) -> Result<*mut i32, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_u_int32array(&self, __value_size: *mut u32) -> Result<*mut u32, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_int64array(&self, __value_size: *mut u32) -> Result<*mut i64, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_u_int64array(&self, __value_size: *mut u32) -> Result<*mut u64, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_single_array(&self, __value_size: *mut u32) -> Result<*mut f32, HResult> {
        let Some(single_array) = &self.single_array else {
            return Err(NOT_IMPLEMENTED);
        };
        let s = co_task_mem_alloc(std::mem::size_of_val(single_array.as_slice())).cast::<f32>();
        if s.is_null() && !single_array.is_empty() {
            return Err(HResult(HRESULT::E_OUTOFMEMORY));
        }
        // SAFETY: the size is the out parameter of the call the generated
        // thunk received; the block has room for every number, and the
        // caller frees it with the COM allocator.
        unsafe {
            __value_size.write(single_array.len() as u32);
            std::ptr::copy_nonoverlapping(single_array.as_ptr(), s, single_array.len());
        }

        Ok(s)
    }

    fn get_double_array(&self, __value_size: *mut u32) -> Result<*mut f64, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_char16array(&self, __value_size: *mut u32) -> Result<*mut u16, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_boolean_array(&self, __value_size: *mut u32) -> Result<*mut i32, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_string_array(&self, __value_size: *mut u32) -> Result<*mut isize, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_inspectable_array(&self, __value_size: *mut u32) -> Result<*mut *mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_guid_array(&self, __value_size: *mut u32) -> Result<*mut Guid, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_date_time_array(&self, __value_size: *mut u32) -> Result<*mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_time_span_array(&self, __value_size: *mut u32) -> Result<*mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_point_array(&self, __value_size: *mut u32) -> Result<*mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_size_array(&self, __value_size: *mut u32) -> Result<*mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }

    fn get_rect_array(&self, __value_size: *mut u32) -> Result<*mut c_void, HResult> {
        Err(NOT_IMPLEMENTED)
    }
}
