//! Automation: the peer of the panel and the proxies through which the
//! peers of the framework's content are part of XAML's automation tree.

mod control_type_map;
mod ferro_swap_chain_panel_automation_peer;
mod ferro_to_xaml_peer_proxy;
mod ferro_to_xaml_peer_proxy_expand_collapse;
mod ferro_to_xaml_peer_proxy_invoke;
mod ferro_to_xaml_peer_proxy_range_value;
mod ferro_to_xaml_peer_proxy_scroll;
mod ferro_to_xaml_peer_proxy_selection;
mod ferro_to_xaml_peer_proxy_toggle;
mod ferro_to_xaml_peer_proxy_value;

#[cfg(windows)]
pub(crate) use ferro_swap_chain_panel_automation_peer::FerroSwapChainPanelAutomationPeer;

/// The element is not enabled (`UIA_E_ELEMENTNOTENABLED`): what a pattern
/// answers when the framework's provider refuses with
/// `ElementNotEnabledError`, as the exception of the reference reaches
/// the client.
#[cfg(windows)]
pub(crate) const ELEMENT_NOT_ENABLED: ferroui_microcom::HResult = ferroui_microcom::HResult(0x8004_0200);

/// A line on the standard error stream for each step of the automation
/// objects XAML calls, when `FERROUI_WINUI_TRACE` is set: what a run that
/// ends in the system's frames names last (a diagnostic of the port; the
/// smoke run of the sample sets the variable).
#[cfg(windows)]
pub(crate) fn trace(what: &str) {
    use std::io::Write;
    if std::env::var_os("FERROUI_WINUI_TRACE").is_some() {
        let mut error = std::io::stderr().lock();
        let _ = writeln!(error, "[trace] automation: {what}");
        let _ = error.flush();
    }
}

/// The result code of a call of a provider of the framework that the state
/// of its element refuses (#567): `UIA_E_ELEMENTNOTENABLED` for an element
/// that is not enabled, and for the other refusals the result code of the
/// exception the reference throws (`UIA_E_INVALIDOPERATION`,
/// `COR_E_ARGUMENTOUTOFRANGE`, `COR_E_NOTSUPPORTED`), as the Windows backend
/// answers.
#[cfg(windows)]
pub(crate) fn automation_error_code(error: &ferroui_controls::automation::AutomationError) -> ferroui_microcom::HResult {
    use ferroui_controls::automation::AutomationError;
    use ferroui_microcom::{HResult, COR_E_INVALIDOPERATION, E_FAIL};
    match error {
        AutomationError::ElementNotEnabled(_) => ELEMENT_NOT_ENABLED,
        AutomationError::InvalidOperation(_) => HResult(COR_E_INVALIDOPERATION),
        AutomationError::ArgumentOutOfRange(_) => HResult(0x8013_1502),
        AutomationError::NotSupported(_) => HResult(0x8013_1515),
        _ => HResult(E_FAIL),
    }
}

/// The result of a call of a provider of the framework as a result of COM.
#[cfg(windows)]
pub(crate) fn enabled<T, E: Into<ferroui_controls::automation::AutomationError>>(
    result: Result<T, E>,
) -> Result<T, ferroui_microcom::HResult> {
    result.map_err(|error| automation_error_code(&error.into()))
}
