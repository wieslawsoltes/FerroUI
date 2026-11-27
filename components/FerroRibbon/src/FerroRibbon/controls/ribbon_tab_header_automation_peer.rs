//! Source RibbonTabHeaderAutomationPeer: inherited Invoke plus SelectionItem.
use super::RibbonTabHeader;
use ferroui_base::{
    FerroObjectImpl, Ref, ferro_class, ferro_class_info, input::KeyboardNavigation, instantiate,
};
use ferroui_controls::automation::{
    AutomationError,
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ButtonAutomationPeer, ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{ISelectionItemProvider, ISelectionProvider},
};
use std::rc::Rc;
#[repr(C)]
pub struct RibbonTabHeaderAutomationPeer {
    base: ButtonAutomationPeer,
}
ferro_class!(RibbonTabHeaderAutomationPeer:ButtonAutomationPeer);
ferro_class_info!(RibbonTabHeaderAutomationPeer{markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<RibbonTabHeader>)=>|owner:Ref<RibbonTabHeader>|RibbonTabHeaderAutomationPeer::new(&owner)],properties:[IsSelected:bool{get:RibbonTabHeaderAutomationPeer::is_selected},SelectionContainer:Option<Ref<AutomationPeer>>{get:RibbonTabHeaderAutomationPeer::selection_container}],methods:[fn Select()=>RibbonTabHeaderAutomationPeer::select,fn AddToSelection()=>RibbonTabHeaderAutomationPeer::add_to_selection,fn RemoveFromSelection()=>RibbonTabHeaderAutomationPeer::remove_from_selection]},interfaces:[Rc<dyn ISelectionItemProvider> =>|v|Rc::new(HeaderSelection(v)) as Rc<dyn ISelectionItemProvider>]});
impl FerroObjectImpl for RibbonTabHeaderAutomationPeer {}
impl ControlAutomationPeerImpl for RibbonTabHeaderAutomationPeer {}
impl RibbonTabHeaderAutomationPeer {
    pub fn new(owner: &RibbonTabHeader) -> Ref<Self> {
        instantiate(Self {
            base: ButtonAutomationPeer::construct(owner),
        })
    }
    fn header(&self) -> Ref<RibbonTabHeader> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("header peer owner")
    }
    pub fn is_selected(&self) -> bool {
        self.header().is_selected()
    }
    pub fn selection_container(&self) -> Option<Ref<AutomationPeer>> {
        self.header()
            .tab()
            .ribbon()
            .and_then(|r| ControlAutomationPeer::from_element(&r))
    }
    pub fn select(&self) {
        self.header().select()
    }
    pub fn add_to_selection(&self) {
        self.header().select()
    }
    /// Source intentionally does nothing: single selection cannot be cleared here.
    pub fn remove_from_selection(&self) {}
}
impl AutomationPeerImpl for RibbonTabHeaderAutomationPeer {
    fn is_keyboard_focusable_core(this: &Self) -> bool {
        let header = this.header();
        // The source exposes the current roving tab stop to automation.
        KeyboardNavigation::get_is_tab_stop(&header)
            && header.is_effectively_enabled()
            && header.is_effectively_visible()
            && Self::parent_is_keyboard_focusable_core(this)
    }

    fn set_focus_core(this: &Self) {
        if Self::is_keyboard_focusable_core(this) {
            Self::parent_set_focus_core(this);
        }
    }

    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::TabItem
    }
    fn get_class_name_core(_: &Self) -> String {
        "RibbonTabHeader".into()
    }
}
struct HeaderSelection(Ref<RibbonTabHeaderAutomationPeer>);
impl ISelectionItemProvider for HeaderSelection {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0
            .selection_container()
            .and_then(|p| p.get_provider::<dyn ISelectionProvider>())
    }
    fn selection_container_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.0.selection_container()
    }
    fn select(&self) -> Result<(), AutomationError> {
        self.0.select();
        Ok(())
    }
    fn add_to_selection(&self) -> Result<(), AutomationError> {
        self.0.add_to_selection();
        Ok(())
    }
    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        self.0.remove_from_selection();
        Ok(())
    }
}
