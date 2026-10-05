use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::TabbedPage;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`TabbedPage`].
#[repr(C)]
pub struct TabbedPageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TabbedPageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TabbedPageAutomationPeer {});

impl FerroObjectImpl for TabbedPageAutomationPeer {}
impl ControlAutomationPeerImpl for TabbedPageAutomationPeer {}

impl AutomationPeerImpl for TabbedPageAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_name_core(this);

        if result.as_deref().is_none_or(str::is_empty) {
            result = this.owner().header().map(|header| ValueTypes::to_display_string(Some(&header)));
        }

        let index = this.owner().selected_index();
        let tab_count = this.get_tab_count();

        if index >= 0 && tab_count > 0 {
            let header = this
                .owner()
                .selected_page()
                .and_then(|selected_page| selected_page.header())
                .map(|header| ValueTypes::to_display_string(Some(&header)));
            let position = format!("Tab {} of {}", index + 1, tab_count);
            let tab_name = match header.as_deref() {
                None | Some("") => position,
                Some(header) => format!("{position}: {header}"),
            };
            return Some(match result.as_deref() {
                None | Some("") => tab_name,
                Some(result) => format!("{result} {tab_name}"),
            });
        }

        result
    }
}

impl TabbedPageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TabbedPage) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TabbedPage) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning tabbed page.
    pub fn owner(&self) -> Ref<TabbedPage> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a tabbed page.")
    }

    fn get_tab_count(&self) -> i32 {
        self.owner().get_tab_count()
    }
}
