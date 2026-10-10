use super::native_win_rt_methods::NativeWinRTMethods;
use super::TrustLevel;
use crate::interop::unmanaged_methods::co_task_mem_alloc;
use ferroui_microcom::{Guid, HResult};

/// What every object of the Windows Runtime that the backend implements
/// answers as an `IInspectable`.
///
/// The reference has a base class whose methods find the interfaces and
/// the name of the class that derives from it by reflection. An object of
/// the port names them: its implementation of the generated
/// `IInspectableImpl` calls these functions with the identifiers of the
/// interfaces it implements and with its class name.
#[allow(dead_code)]
pub(crate) struct WinRTInspectable;

#[allow(dead_code)] // Created by the composition effects of stage 2c.
impl WinRTInspectable {
    /// `GetIids`: the identifiers of the interfaces, in memory of the COM
    /// allocator the caller frees.
    ///
    /// # Safety
    /// `iid_count` and `iids` must be the out parameters of a call of
    /// `IInspectable::GetIids`: valid for a write of a count and of a
    /// pointer.
    pub(crate) unsafe fn get_iids(interfaces: &[Guid], iid_count: *mut u64, iids: *mut *mut Guid) -> Result<(), HResult> {
        let mem = co_task_mem_alloc(std::mem::size_of_val(interfaces)).cast::<Guid>();
        if mem.is_null() && !interfaces.is_empty() {
            return Err(HResult(crate::interop::unmanaged_methods::HRESULT::E_OUTOFMEMORY));
        }
        // SAFETY: the block has room for every identifier; the out
        // parameters are valid by the contract of this function. The
        // count of the system's interface is 32 bits wide (the interface
        // definition of the reference declares 64): only those are
        // written.
        unsafe {
            for (c, interface) in interfaces.iter().enumerate() {
                mem.add(c).write(*interface);
            }
            iids.write(mem);
            iid_count.cast::<u32>().write(interfaces.len() as u32);
        }
        Ok(())
    }

    /// `GetRuntimeClassName`: a new string handle with the name of the
    /// class, which the caller deletes.
    pub(crate) fn runtime_class_name(full_name: &str) -> Result<isize, HResult> {
        NativeWinRTMethods::windows_create_string(full_name)
    }

    /// `GetTrustLevel`.
    pub(crate) fn trust_level() -> TrustLevel {
        TrustLevel::BaseTrust
    }
}
