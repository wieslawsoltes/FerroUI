#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use crate::interop::win_rt::to_abi;
    use crate::interop::{IIRawElementProviderSimple, ISelectionItemProviderImpl, ISelectionProviderImpl};
    use ferroui_controls::automation::provider::{ISelectionItemProvider as FerroSelectionItem, ISelectionProvider as FerroSelection};
    use ferroui_microcom::{ComPtr, HResult};
    use windows_sys::Win32::System::Com::CoTaskMemAlloc;

    impl ISelectionProviderImpl for FerroToXamlPeerProxy {
        fn can_select_multiple(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroSelection>().is_some_and(|p| p.can_select_multiple())))
        }

        fn is_selection_required(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroSelection>().is_some_and(|p| p.is_selection_required())))
        }

        fn get_selection(&self, result_size: *mut u32, result: *mut *mut *mut IIRawElementProviderSimple) -> Result<(), HResult> {
            let mut list: Vec<ComPtr<IIRawElementProviderSimple>> = Vec::new();
            if let (Some(provider), Some(host)) = (self.peer.get_provider::<dyn FerroSelection>(), self.host.upgrade()) {
                for ferro_child in provider.get_selection() {
                    let proxy = FerroToXamlPeerProxy::get_or_create(&ferro_child, &host)?;
                    if let Some(simple) = self.provider_from_peer(&proxy) {
                        list.push(simple);
                    }
                }
            }

            // The array of the caller's (a receive array of the ABI), in
            // memory of the COM allocator.
            // SAFETY: an allocation of the size of the pointers.
            let array = unsafe { CoTaskMemAlloc((list.len() * std::mem::size_of::<usize>()).max(1)) }
                .cast::<*mut IIRawElementProviderSimple>();
            if array.is_null() {
                return Err(HResult(0x8007_000E)); // E_OUTOFMEMORY
            }
            let count = list.len() as u32;
            // SAFETY: the array has room for every pointer; the out
            // parameters of the call are valid for a write; the references
            // are the caller's.
            unsafe {
                for (index, simple) in list.into_iter().enumerate() {
                    array.add(index).write(simple.into_raw());
                }
                result_size.write(count);
                result.write(array);
            }
            Ok(())
        }
    }

    impl ISelectionItemProviderImpl for FerroToXamlPeerProxy {
        fn is_selected(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroSelectionItem>().is_some_and(|p| p.is_selected())))
        }

        fn selection_container(&self) -> Result<Option<ComPtr<IIRawElementProviderSimple>>, HResult> {
            let container = self.peer.get_provider::<dyn FerroSelectionItem>().and_then(|p| p.selection_container_peer());
            if let (Some(container), Some(host)) = (container, self.host.upgrade()) {
                let container_peer = container;
                let proxy = FerroToXamlPeerProxy::get_or_create(&container_peer, &host)?;
                return Ok(self.provider_from_peer(&proxy));
            }
            Ok(None)
        }

        fn add_to_selection(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroSelectionItem>() {
                super::super::enabled(provider.add_to_selection())?;
            }
            Ok(())
        }

        fn remove_from_selection(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroSelectionItem>() {
                super::super::enabled(provider.remove_from_selection())?;
            }
            Ok(())
        }

        fn select(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroSelectionItem>() {
                super::super::enabled(provider.select())?;
            }
            Ok(())
        }
    }
}
