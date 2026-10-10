use super::i_node_info_provider::{INodeInfoProvider, NodeActionArguments, NodeActionError};
use super::node_info::{NodeInfo, ACTION_SELECT};
use super::node_info_provider::NodeInfoProvider;
use crate::explore_by_touch_helper::IVirtualViewOwner;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::ISelectionItemProvider;
use std::rc::Weak;

pub(crate) struct SelectionItemNodeInfoProvider {
    base: NodeInfoProvider<dyn ISelectionItemProvider>,
}

impl SelectionItemNodeInfoProvider {
    pub(crate) fn new(owner: Weak<dyn IVirtualViewOwner>, peer: Ref<AutomationPeer>, virtual_view_id: i32) -> Self {
        Self { base: NodeInfoProvider::new(owner, peer, virtual_view_id, None) }
    }
}

impl INodeInfoProvider for SelectionItemNodeInfoProvider {
    fn virtual_view_id(&self) -> i32 {
        self.base.virtual_view_id()
    }

    fn perform_node_action(&self, action: i32, _arguments: Option<&NodeActionArguments>) -> Result<bool, NodeActionError> {
        let provider = self.base.get_provider()?;
        match action {
            ACTION_SELECT => {
                provider.select()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn populate_node_info(&self, node_info: &mut NodeInfo) -> Result<(), NodeActionError> {
        node_info.add_action(ACTION_SELECT);

        let provider = self.base.get_provider()?;
        node_info.selected = provider.is_selected();
        Ok(())
    }
}
