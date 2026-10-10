use super::i_node_info_provider::{INodeInfoProvider, NodeActionArguments, NodeActionError};
use super::node_info::{NodeInfo, ACCESSIBILITY_LIVE_REGION_POLITE, ACTION_SET_TEXT, CONTENT_CHANGE_TYPE_TEXT};
use super::node_info_provider::NodeInfoProvider;
use crate::explore_by_touch_helper::IVirtualViewOwner;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::IValueProvider;
use ferroui_controls::automation::{AutomationPropertyChangedEventArgs, ValuePatternIdentifiers};
use std::rc::Weak;

pub(crate) struct ValueNodeInfoProvider {
    base: NodeInfoProvider<dyn IValueProvider>,
}

impl ValueNodeInfoProvider {
    pub(crate) fn new(owner: Weak<dyn IVirtualViewOwner>, peer: Ref<AutomationPeer>, virtual_view_id: i32) -> Self {
        Self { base: NodeInfoProvider::new(owner, peer, virtual_view_id, Some(Self::peer_property_changed)) }
    }

    fn peer_property_changed(owner: &dyn IVirtualViewOwner, virtual_view_id: i32, e: &AutomationPropertyChangedEventArgs) {
        if e.property() == ValuePatternIdentifiers::value_property() {
            owner.invalidate_virtual_view_with(virtual_view_id, CONTENT_CHANGE_TYPE_TEXT);
        }
    }
}

/// The length of a text as the system counts it: in UTF-16 code units.
fn length_of(text: Option<&str>) -> i32 {
    text.map_or(0, |text| text.encode_utf16().count() as i32)
}

impl INodeInfoProvider for ValueNodeInfoProvider {
    fn virtual_view_id(&self) -> i32 {
        self.base.virtual_view_id()
    }

    fn perform_node_action(&self, action: i32, arguments: Option<&NodeActionArguments>) -> Result<bool, NodeActionError> {
        let provider = self.base.get_provider()?;
        match action {
            ACTION_SET_TEXT => {
                let text = arguments.and_then(|arguments| arguments.set_text_char_sequence.as_deref());
                let value = format!("{}{}", provider.value().unwrap_or_default(), text.unwrap_or_default());
                provider
                    .set_value(Some(&value))
                    .map_err(|error| NodeActionError::InvalidOperation(error.to_string()))?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn populate_node_info(&self, node_info: &mut NodeInfo) -> Result<(), NodeActionError> {
        node_info.add_action(ACTION_SET_TEXT);

        let provider = self.base.get_provider()?;
        let value = provider.value();
        node_info.editable = !provider.is_read_only();

        node_info.text_selection = Some((length_of(value.as_deref()), length_of(value.as_deref())));
        node_info.text = value;
        node_info.live_region = ACCESSIBILITY_LIVE_REGION_POLITE;
        Ok(())
    }
}
