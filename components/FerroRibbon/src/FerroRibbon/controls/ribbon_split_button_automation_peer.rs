//! Source RibbonSplitButtonAutomationPeer through native provider dispatch.
use super::RibbonSplitButton;
use ferroui_base::{BoxedValue, FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::automation::{
    AutomationError, ElementNotEnabledError, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{IExpandCollapseProvider, IInvokeProvider},
};
use std::rc::Rc;
#[repr(C)]
pub struct RibbonSplitButtonAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RibbonSplitButtonAutomationPeer:ControlAutomationPeer);
ferro_class_info!(RibbonSplitButtonAutomationPeer{markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<RibbonSplitButton>)=>|owner:Ref<RibbonSplitButton>|RibbonSplitButtonAutomationPeer::new(&owner)],properties:[ExpandCollapseState:ExpandCollapseState{get:RibbonSplitButtonAutomationPeer::expand_collapse_state}],methods:[fn Invoke()=>|this:&RibbonSplitButtonAutomationPeer|this.invoke(),fn Expand()=>|this:&RibbonSplitButtonAutomationPeer|this.expand(),fn Collapse()=>|this:&RibbonSplitButtonAutomationPeer|this.collapse()]},interfaces:[Rc<dyn IInvokeProvider> => |v|Rc::new(SplitProvider(v)) as Rc<dyn IInvokeProvider>,Rc<dyn IExpandCollapseProvider> => |v|Rc::new(SplitProvider(v)) as Rc<dyn IExpandCollapseProvider>]});
impl FerroObjectImpl for RibbonSplitButtonAutomationPeer {}
impl ControlAutomationPeerImpl for RibbonSplitButtonAutomationPeer {}
impl RibbonSplitButtonAutomationPeer {
    pub fn new(owner: &RibbonSplitButton) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn button(&self) -> Ref<RibbonSplitButton> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("split peer owner")
    }
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        if self.button().is_drop_down_open() {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
    pub fn invoke(&self) {
        let button = self.button();
        if button.is_enabled() {
            button.invoke_core();
        }
    }
    pub fn expand(&self) {
        self.button().open_drop_down();
    }
    pub fn collapse(&self) {
        self.button().close_drop_down();
    }
    pub(crate) fn source_flyout_changed(&self) {
        let new = self.expand_collapse_state();
        let old = if new == ExpandCollapseState::Expanded {
            ExpandCollapseState::Collapsed
        } else {
            ExpandCollapseState::Expanded
        };
        self.raise_property_changed_event(
            ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
            Some(Rc::new(old) as BoxedValue),
            Some(Rc::new(new) as BoxedValue),
        );
    }
}
impl AutomationPeerImpl for RibbonSplitButtonAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::SplitButton
    }
    fn get_class_name_core(_: &Self) -> String {
        "RibbonSplitButton".into()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        if name.as_ref().is_none_or(String::is_empty) {
            Some(this.button().label().unwrap_or_default().replace('\n', " "))
        } else {
            name
        }
    }
}
impl IInvokeProvider for SplitProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.0.invoke();
        Ok(())
    }
}
impl IExpandCollapseProvider for SplitProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        self.0.expand_collapse_state()
    }
    fn shows_menu(&self) -> bool {
        true
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

struct SplitProvider(Ref<RibbonSplitButtonAutomationPeer>);
