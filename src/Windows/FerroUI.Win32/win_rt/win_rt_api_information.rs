use super::native_win_rt_methods::{HStringInterop, NativeWinRTMethods};
use super::IApiInformationStatics;
use crate::win32_platform::Win32Platform;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_microcom::ComPtr;
use std::cell::OnceCell;

thread_local! {
    // The reference keeps one object for the process. An interface pointer
    // of the port belongs to the thread that got it, so every thread that
    // asks has its own.
    static STATICS: OnceCell<Option<ComPtr<IApiInformationStatics>>> = const { OnceCell::new() };
}

/// Any WinRT API might not be available even if Windows version is supposed to support them (Win PE, Xbox...).
/// Using ApiInformation is a typical solution in UWP/WinUI apps, so we should do as well.
pub(crate) struct WinRTApiInformation;

// The platform settings ask for types; the other questions are asked by
// the composition modes of stage 2c.
#[allow(dead_code)]
impl WinRTApiInformation {
    fn statics() -> Option<ComPtr<IApiInformationStatics>> {
        STATICS.with(|statics| {
            statics
                .get_or_init(|| {
                    if Win32Platform::windows_version().major < 10 {
                        return None;
                    }

                    match NativeWinRTMethods::create_activation_factory::<IApiInformationStatics>(
                        "Windows.Foundation.Metadata.ApiInformation",
                    ) {
                        Ok(api_statics) => Some(api_statics),
                        Err(ex) => {
                            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                                logger.log_with_values(None, "Unable to create ApiInformation instance: {0}", &[&ex]);
                            }
                            None
                        }
                    }
                })
                .clone()
        })
    }

    /// Asks the object with one or two names; `false` when there is no
    /// object, a name cannot be made a string handle, or the call fails.
    fn ask(names: [&str; 2], call: impl FnOnce(&IApiInformationStatics, isize, isize, *mut i32) -> i32) -> bool {
        let Some(statics) = Self::statics() else {
            return false;
        };

        let (Ok(first), Ok(second)) = (HStringInterop::new(Some(names[0])), HStringInterop::new(Some(names[1]))) else {
            return false;
        };
        let mut result = 0;
        if call(&statics, first.handle(), second.handle(), &mut result) == 0 {
            return result == 1;
        }

        false
    }

    pub(crate) fn is_type_present(type_name: &str) -> bool {
        // SAFETY (this and the calls below): the string handles live
        // through the call, and the result is a number of the caller's
        // frame.
        Self::ask([type_name, ""], |statics, type_name, _, result| unsafe { statics.is_type_present(type_name, result) })
    }

    pub(crate) fn is_method_present(type_name: &str, method_name: &str) -> bool {
        Self::ask([type_name, method_name], |statics, type_name, method_name, result| unsafe {
            statics.is_method_present(type_name, method_name, result)
        })
    }

    pub(crate) fn is_method_present_with_arity(type_name: &str, method_name: &str, input_parameter_count: u32) -> bool {
        Self::ask([type_name, method_name], |statics, type_name, method_name, result| unsafe {
            statics.is_method_present_with_arity(type_name, method_name, input_parameter_count, result)
        })
    }

    pub(crate) fn is_event_present(type_name: &str, event_name: &str) -> bool {
        Self::ask([type_name, event_name], |statics, type_name, event_name, result| unsafe {
            statics.is_event_present(type_name, event_name, result)
        })
    }

    pub(crate) fn is_property_present(type_name: &str, property_name: &str) -> bool {
        Self::ask([type_name, property_name], |statics, type_name, property_name, result| unsafe {
            statics.is_property_present(type_name, property_name, result)
        })
    }

    pub(crate) fn is_read_only_property_present(type_name: &str, property_name: &str) -> bool {
        Self::ask([type_name, property_name], |statics, type_name, property_name, result| unsafe {
            statics.is_read_only_property_present(type_name, property_name, result)
        })
    }

    pub(crate) fn is_writeable_property_present(type_name: &str, property_name: &str) -> bool {
        Self::ask([type_name, property_name], |statics, type_name, property_name, result| unsafe {
            statics.is_writeable_property_present(type_name, property_name, result)
        })
    }

    pub(crate) fn is_enum_named_value_present(enum_type_name: &str, value_name: &str) -> bool {
        Self::ask([enum_type_name, value_name], |statics, enum_type_name, value_name, result| unsafe {
            statics.is_enum_named_value_present(enum_type_name, value_name, result)
        })
    }

    pub(crate) fn is_api_contract_present_by_major(contract_name: &str, major_version: u16) -> bool {
        Self::ask([contract_name, ""], |statics, contract_name, _, result| unsafe {
            statics.is_api_contract_present_by_major(contract_name, major_version, result)
        })
    }

    pub(crate) fn is_api_contract_present_by_major_and_minor(contract_name: &str, major_version: u16, minor_version: u16) -> bool {
        Self::ask([contract_name, ""], |statics, contract_name, _, result| unsafe {
            statics.is_api_contract_present_by_major_and_minor(contract_name, major_version, minor_version, result)
        })
    }
}
