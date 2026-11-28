//! Source RibbonAutomationPeer Selection and conditional ExpandCollapse paths.
use super::Ribbon;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, data::core::ValueType, ferro_class, ferro_class_info,
    instantiate,
};
use ferroui_controls::automation::{
    AutomationError, ExpandCollapseState,
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{IExpandCollapseProvider, ISelectionProvider},
};
use std::rc::Rc;
#[repr(C)]
pub struct RibbonAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RibbonAutomationPeer:ControlAutomationPeer);
ferro_class_info!(RibbonAutomationPeer{markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<Ribbon>)=>|owner:Ref<Ribbon>|RibbonAutomationPeer::new(&owner)],properties:[ExpandCollapseState:ExpandCollapseState{get:RibbonAutomationPeer::expand_collapse_state},CanSelectMultiple:bool{get:RibbonAutomationPeer::can_select_multiple},IsSelectionRequired:bool{get:RibbonAutomationPeer::is_selection_required}],methods:[fn Expand()=>RibbonAutomationPeer::expand,fn Collapse()=>RibbonAutomationPeer::collapse,fn GetSelection()->Vec<Ref<AutomationPeer>> =>RibbonAutomationPeer::get_selection]},interfaces:[Rc<dyn IExpandCollapseProvider> =>|v|Rc::new(RibbonExpansion(v)) as Rc<dyn IExpandCollapseProvider>,Rc<dyn ISelectionProvider> =>|v|Rc::new(RibbonSelection(v)) as Rc<dyn ISelectionProvider>]});
impl FerroObjectImpl for RibbonAutomationPeer {}
impl ControlAutomationPeerImpl for RibbonAutomationPeer {}
impl RibbonAutomationPeer {
    pub fn new(owner: &Ribbon) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn ribbon(&self) -> Ref<Ribbon> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("ribbon peer owner")
    }
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        if self.ribbon().is_minimized() {
            ExpandCollapseState::Collapsed
        } else {
            ExpandCollapseState::Expanded
        }
    }
    pub fn expand(&self) {
        if self.ribbon().is_minimized() {
            self.ribbon().toggle_minimized();
        }
    }
    pub fn collapse(&self) {
        if !self.ribbon().is_minimized() {
            self.ribbon().toggle_minimized();
        }
    }
    pub fn can_select_multiple(&self) -> bool {
        false
    }
    pub fn is_selection_required(&self) -> bool {
        true
    }
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        ControlAutomationPeer::owner(self)
            .cast::<Ribbon>()
            .expect("ribbon peer owner")
            .selected_tab()
            .and_then(|t| t.header_element())
            .and_then(|h| ControlAutomationPeer::from_element(&h))
            .into_iter()
            .collect()
    }
}
impl AutomationPeerImpl for RibbonAutomationPeer {
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IExpandCollapseProvider>>()
            && !this.ribbon().is_collapsible()
        {
            None
        } else {
            Self::parent_get_provider_core(this, provider_type)
        }
    }
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Tab
    }
    fn get_class_name_core(_: &Self) -> String {
        "Ribbon".into()
    }
}
struct RibbonSelection(Ref<RibbonAutomationPeer>);
impl ISelectionProvider for RibbonSelection {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        self.0.can_select_multiple()
    }
    fn is_selection_required(&self) -> bool {
        self.0.is_selection_required()
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        self.0.get_selection()
    }
}

struct RibbonExpansion(Ref<RibbonAutomationPeer>);
impl IExpandCollapseProvider for RibbonExpansion {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        self.0.expand_collapse_state()
    }
    fn shows_menu(&self) -> bool {
        false
    }
    fn expand(&self) -> Result<(), AutomationError> {
        self.0.expand();
        Ok(())
    }
    fn collapse(&self) -> Result<(), AutomationError> {
        self.0.collapse();
        Ok(())
    }
}
