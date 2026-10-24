use crate::Chart;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};

/// Uses normal framework name, label and description resolution. Peer queries
/// never force a chart layout or decode its data.
#[repr(C)]
pub struct ChartAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(ChartAutomationPeer: ControlAutomationPeer);
ferro_class_info!(ChartAutomationPeer {});
ferro_impl_classes!(ChartAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for ChartAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Chart".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
    /// UI Automation has no chart type: a custom control announced as a chart.
    fn get_localized_control_type_core(_this: &Self) -> String {
        "chart".into()
    }
    /// `AutomationProperties.Name`, then `LabeledBy` (the framework's resolution, as WinUI's
    /// tests require), then the legend title, then the titles of the series; read from the
    /// properties alone, without laying the chart out.
    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        if name.as_deref().is_some_and(|name| !name.trim().is_empty()) {
            return name;
        }
        let chart = this.chart()?;
        let legend = chart.legend_title();
        if !legend.trim().is_empty() {
            return Some(legend);
        }
        let titles: Vec<String> = chart
            .series()
            .snapshot()
            .iter()
            .map(|series| series.title())
            .filter(|title| !title.trim().is_empty())
            .collect();
        (!titles.is_empty()).then(|| titles.join(", ")).or(name)
    }
}
impl ChartAutomationPeer {
    pub fn construct(chart: &Chart) -> Self {
        Self {
            base: ControlAutomationPeer::construct(chart),
        }
    }
    pub fn new(chart: &Chart) -> Ref<Self> {
        instantiate(Self::construct(chart))
    }
    fn chart(&self) -> Option<Ref<Chart>> {
        ControlAutomationPeer::owner(self).cast()
    }
}
