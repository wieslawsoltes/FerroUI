// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source typed abstract item peer, specialized with native class macros for each owner type.
macro_rules! define_dock_tab_strip_item_peer {
    ($name:ident,$virtuals:ident,$owner:ident,$provider:ident) => {
        #[repr(C)]
        pub(crate) struct $name {base:ListItemAutomationPeer,owner:Ref<$owner>}
        ferro_class!($name:ListItemAutomationPeer,virtuals $virtuals:ControlAutomationPeerImpl {
            fn class_name(this)->String;
            fn fallback_name(this)->String;
            fn build_state_text(this,dockable:Option<Rc<dyn IDockable>>)->String;
            fn invoke(this);
        });
        ferro_class_info!($name {interfaces:[Rc<dyn IInvokeProvider> => $name::as_invoke_provider,Rc<dyn ISelectionItemProvider> => $name::as_selection_item_provider]});
        ferro_impl_classes!($name:FerroObjectImpl,ControlAutomationPeerImpl);
        impl $virtuals for $name {
            fn class_name(_this:&Self)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
            fn fallback_name(_this:&Self)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
            fn build_state_text(_this:&Self,_dockable:Option<Rc<dyn IDockable>>)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
            fn invoke(this:&Self) {Helper::try_activate_dockable(model_projection::dockable(this.owner.data_context().as_ref()));this.owner.set_is_selected(true);this.owner.focus();}
        }
        impl AutomationPeerImpl for $name {
            fn get_class_name_core(this:&Self)->String {this.class_name()}
            fn get_automation_control_type_core(_this:&Self)->AutomationControlType {AutomationControlType::TabItem}
            fn get_name_core(this:&Self)->Option<String> {
                let base=Self::parent_get_name_core(this);if base.as_ref().is_some_and(|n|!n.trim().is_empty()) {return base;}
                Some(Helper::resolve_name(&this.owner,&this.fallback_name(),model_projection::dockable(this.owner.data_context().as_ref())))
            }
            fn get_automation_id_core(this:&Self)->Option<String> {
                let id=Helper::resolve_automation_id(&this.owner,model_projection::dockable(this.owner.data_context().as_ref()));Some(if !id.trim().is_empty(){id}else{Self::parent_get_automation_id_core(this).unwrap_or_default()})
            }
            fn get_help_text_core(this:&Self)->Option<String> {Some(this.build_state_text(model_projection::dockable(this.owner.data_context().as_ref())))}
        }
        impl $name {
            pub(crate) fn construct(owner:&$owner)->Self {Self{base:ListItemAutomationPeer::construct(owner),owner:owner.to_ref()}}
            pub(crate) fn owner_control(&self)->Ref<$owner> {self.owner.clone()}
            fn as_invoke_provider(this:Ref<Self>)->Rc<dyn IInvokeProvider> {Rc::new($provider(this))}
            fn as_selection_item_provider(this:Ref<Self>)->Rc<dyn ISelectionItemProvider> {Rc::new($provider(this))}
            pub(crate) fn notify_is_selected_changed(&self,old:bool,new:bool) {if old!=new {self.raise_property_changed_event(SelectionItemPatternIdentifiers::is_selected_property(),Some(Rc::new(old)),Some(Rc::new(new)));}}
        }
        struct $provider(Ref<$name>);
        impl IInvokeProvider for $provider {fn peer(&self)->Ref<AutomationPeer> {self.0.clone().upcast()}fn invoke(&self)->Result<(),ElementNotEnabledException> {self.0.invoke();Ok(())}}
        impl ISelectionItemProvider for $provider {
            fn peer(&self)->Ref<AutomationPeer> {self.0.clone().upcast()}
            fn is_selected(&self)->bool {self.0.owner.is_selected()}
            fn selection_container(&self)->Option<Rc<dyn ISelectionProvider>> {let strip=self.0.owner.find_ancestor_of_type::<TabStrip>(false)?;ControlAutomationPeer::create_peer_for_element(&strip).get_provider::<dyn ISelectionProvider>()}
            fn add_to_selection(&self)->Result<(),ElementNotEnabledException> {self.0.invoke();Ok(())}
            fn select(&self)->Result<(),ElementNotEnabledException> {self.0.invoke();Ok(())}
            fn remove_from_selection(&self)->Result<(),ElementNotEnabledException> {if self.0.owner.is_selected() {self.0.owner.set_is_selected(false);}Ok(())}
        }
    };
}
pub(crate) use define_dock_tab_strip_item_peer;
