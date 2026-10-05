use crate::ToolTipService;
use ferroui_base::input::IInputRoot;
use ferroui_base::{Ref, Visual};
use std::rc::Rc;

/// Shows and hides the tooltips of controls as the pointer moves (unstable,
/// private API; internal upstream).
pub trait IToolTipService {
    /// Updates the control whose tooltip is shown: `candidate_tool_tip_host`
    /// is the visual under the pointer in `root`, if any.
    fn update(&self, root: &Rc<dyn IInputRoot>, candidate_tool_tip_host: Option<Ref<Visual>>);

    /// The service as the default tooltip service, if it is one.
    fn as_tool_tip_service(&self) -> Option<&ToolTipService> {
        None
    }
}
