use crate::embedding::EmbeddableControlRoot;
use crate::remote::server::RemoteServerTopLevelImpl;
use ferroui_base::{BoxedValue, Ref};
use ferroui_remote_protocol::IFerroRemoteTransportConnection;
use std::rc::Rc;
use std::sync::Arc;

/// Renders its content into frames that are sent over a connection of the
/// remote protocol. (Internal upstream; public here for the crates the
/// upstream assembly is visible to.)
pub struct RemoteServer {
    top_level: Ref<EmbeddableControlRoot>,
    // `EmbeddableRemoteServerTopLevelImpl` of the original adds nothing to
    // its base class; the base class is used as it is.
    #[cfg_attr(not(test), allow(dead_code))]
    platform_impl: Rc<RemoteServerTopLevelImpl>,
}

impl RemoteServer {
    pub fn new(transport: Arc<dyn IFerroRemoteTransportConnection>) -> RemoteServer {
        let platform_impl = RemoteServerTopLevelImpl::new(transport);
        let top_level = EmbeddableControlRoot::with_impl(platform_impl.as_top_level_impl());
        top_level.prepare();
        top_level.start_rendering();
        //TODO: Somehow react on closed connection?
        RemoteServer { top_level, platform_impl }
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.top_level.content()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.top_level.set_content(value)
    }

    pub fn dispose(&self) {
        self.top_level.stop_rendering();
        self.top_level.dispose();
    }

    /// The top-level the content is rendered in (`_topLevel`).
    #[cfg(test)]
    pub(crate) fn top_level(&self) -> &Ref<EmbeddableControlRoot> {
        &self.top_level
    }

    /// The implementation of the top-level.
    #[cfg(test)]
    pub(crate) fn platform_impl(&self) -> &Rc<RemoteServerTopLevelImpl> {
        &self.platform_impl
    }
}
