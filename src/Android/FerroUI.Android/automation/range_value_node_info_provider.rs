use super::i_node_info_provider::{INodeInfoProvider, NodeActionArguments, NodeActionError};
use super::node_info::{NodeInfo, RangeInfo, RANGE_TYPE_FLOAT};
use super::node_info_provider::NodeInfoProvider;
use crate::explore_by_touch_helper::IVirtualViewOwner;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::IRangeValueProvider;
use std::rc::Weak;

pub(crate) struct RangeValueNodeInfoProvider {
    base: NodeInfoProvider<dyn IRangeValueProvider>,
}

impl RangeValueNodeInfoProvider {
    pub(crate) fn new(owner: Weak<dyn IVirtualViewOwner>, peer: Ref<AutomationPeer>, virtual_view_id: i32) -> Self {
        Self { base: NodeInfoProvider::new(owner, peer, virtual_view_id, None) }
    }
}

impl INodeInfoProvider for RangeValueNodeInfoProvider {
    fn virtual_view_id(&self) -> i32 {
        self.base.virtual_view_id()
    }

    fn perform_node_action(&self, _action: i32, _arguments: Option<&NodeActionArguments>) -> Result<bool, NodeActionError> {
        Ok(false)
    }

    fn populate_node_info(&self, node_info: &mut NodeInfo) -> Result<(), NodeActionError> {
        let provider = self.base.get_provider()?;
        node_info.range_info = Some(RangeInfo {
            range_type: RANGE_TYPE_FLOAT,
            min: provider.minimum() as f32,
            max: provider.maximum() as f32,
            current: provider.value() as f32,
        });
        Ok(())
    }
}
