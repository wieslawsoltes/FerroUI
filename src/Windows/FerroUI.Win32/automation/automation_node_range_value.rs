//! The range value pattern of a node (the port of
//! `AutomationNode.RangeValue.cs`).

use super::automation_node::AutomationNode;
use super::interop::IRangeValueProviderImpl;
use ferroui_controls::automation::provider::IRangeValueProvider;
use ferroui_microcom::HResult;
use std::rc::Rc;

impl IRangeValueProviderImpl for AutomationNode {
    fn set_value(&self, value: f64) -> Result<(), HResult> {
        self.invoke_sync_provider(move |x: &Rc<dyn IRangeValueProvider>| Self::enabled(x.set_value(value)))
    }

    fn get_value(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(x.value()))
    }

    fn get_is_read_only(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(i32::from(x.is_read_only())))
    }

    fn get_maximum(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(x.maximum()))
    }

    fn get_minimum(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(x.minimum()))
    }

    // Deviation (DEVIATIONS.md, Windows platform backend): the reference answers one for both
    // changes, whatever the peer says; the port answers the provider's values, as WinUI's
    // `RangeBaseAutomationPeer` does.
    fn get_large_change(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(x.large_change()))
    }

    fn get_small_change(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IRangeValueProvider>| Ok(x.small_change()))
    }
}
