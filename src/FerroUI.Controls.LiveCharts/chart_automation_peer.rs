use crate::Chart;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};

/// Native automation entry point. The owner is held weakly by the framework peer.
#[repr(C)]
pub struct ChartAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(ChartAutomationPeer:ControlAutomationPeer);
ferro_class_info!(ChartAutomationPeer {});
impl FerroObjectImpl for ChartAutomationPeer {}
impl ControlAutomationPeerImpl for ChartAutomationPeer {}
impl AutomationPeerImpl for ChartAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Chart".to_owned()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
    fn get_name_core(this: &Self) -> Option<String> {
        if let Some(name) = Self::parent_get_name_core(this).filter(|name| !name.trim().is_empty())
        {
            return Some(name);
        }
        this.try_owner()
            .and_then(|owner| owner.cast::<Chart>())
            .map(|chart| {
                let title = chart.title();
                if title.is_empty() {
                    format!("{:?} chart", chart.chart_kind())
                } else {
                    title
                }
            })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        if let Some(help) =
            Self::parent_get_help_text_core(this).filter(|help| !help.trim().is_empty())
        {
            return Some(help);
        }
        this.try_owner()
            .and_then(|owner| owner.cast::<Chart>())
            .map(|chart| {
                chart
                    .series()
                    .to_vec()
                    .iter()
                    .filter(|s| s.visible())
                    .map(|series| {
                        let values = series
                            .values()
                            .to_vec()
                            .iter()
                            .map(|p| {
                                if p.is_empty() {
                                    "missing".to_owned()
                                } else {
                                    ferroui_livecharts::axis::format_number(p.y())
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{}: {values}", series.name())
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
    }
}
impl ChartAutomationPeer {
    pub fn construct(owner: &Chart) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &Chart) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
