#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use crate::interop::win_rt::{borrowed_string, new_string, to_abi};
    use crate::interop::IValueProviderImpl;
    use ferroui_controls::automation::provider::IValueProvider as FerroValue;
    use ferroui_controls::automation::ElementNotEnabledError;
    use ferroui_microcom::{HResult, COR_E_INVALIDOPERATION};

    impl IValueProviderImpl for FerroToXamlPeerProxy {
        fn is_read_only(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroValue>().is_none_or(|p| p.is_read_only())))
        }

        fn value(&self) -> Result<isize, HResult> {
            new_string(&self.peer.get_provider::<dyn FerroValue>().and_then(|p| p.value()).unwrap_or_default())
        }

        fn set_value(&self, value: isize) -> Result<(), HResult> {
            let text = borrowed_string(value);
            if let Some(provider) = self.peer.get_provider::<dyn FerroValue>() {
                provider.set_value(text.as_deref()).map_err(|error| {
                    if error.downcast_ref::<ElementNotEnabledError>().is_some() {
                        super::super::ELEMENT_NOT_ENABLED
                    } else {
                        HResult(COR_E_INVALIDOPERATION)
                    }
                })?;
            }
            Ok(())
        }
    }
}
