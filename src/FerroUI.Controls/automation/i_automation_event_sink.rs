//! Platform delivery of automation events that an application can raise before
//! an accessibility client has requested a native provider for the peer.

use super::peers::AutomationPeer;
use super::provider::TextEditTextChangedEventArgs;
use ferroui_base::Ref;

/// Platform automation event transport, registered in the framework locator.
///
/// Calls run on the peer's UI thread. The source has already been substituted
/// through `AutomationPeer::events_source` exactly once; an implementation must
/// not follow that property again. A transport may create its provider lazily.
pub trait IAutomationEventSink: 'static {
    fn raise_text_edit_text_changed(
        &self,
        source: &Ref<AutomationPeer>,
        event: &TextEditTextChangedEventArgs,
    );
}
