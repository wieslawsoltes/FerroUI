// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native closed specializations of the source typed abstract peer, retaining typed owners.
macro_rules! define_dock_tab_strip_peer {
    ($name:ident,$virtuals:ident,$owner:ident) => {
        #[repr(C)]
        pub(crate) struct $name {base:SelectingItemsControlAutomationPeer,owner:Ref<$owner>}
        ferro_class!($name:SelectingItemsControlAutomationPeer,virtuals $virtuals:SelectingItemsControlAutomationPeerImpl {
            fn class_name(this)->String;
            fn fallback_name(this)->String;
            fn build_state_text(this,dock:Option<Rc<dyn IDock>>)->String;
        });
        ferro_class_info!($name {});
        ferro_impl_classes!($name:FerroObjectImpl,ControlAutomationPeerImpl,ItemsControlAutomationPeerImpl,SelectingItemsControlAutomationPeerImpl);
        impl $virtuals for $name {
            fn class_name(_this:&Self)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
            fn fallback_name(_this:&Self)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
            fn build_state_text(_this:&Self,_dock:Option<Rc<dyn IDock>>)->String {panic!("abstract typed peer dispatch requires a concrete peer")}
        }
        impl AutomationPeerImpl for $name {
            fn get_class_name_core(this:&Self)->String {this.class_name()}
            fn get_automation_control_type_core(_this:&Self)->AutomationControlType {AutomationControlType::Tab}
            fn get_name_core(this:&Self)->Option<String> {
                let base=Self::parent_get_name_core(this);if base.as_ref().is_some_and(|n|!n.trim().is_empty()) {return base;}
                Some(Helper::resolve_name(&this.owner,&this.fallback_name(),model_projection::dockable(this.owner.data_context().as_ref())))
            }
            fn get_automation_id_core(this:&Self)->Option<String> {
                let id=Helper::resolve_automation_id(&this.owner,model_projection::dockable(this.owner.data_context().as_ref()));
                Some(if !id.trim().is_empty(){id}else{Self::parent_get_automation_id_core(this).unwrap_or_default()})
            }
            fn get_help_text_core(this:&Self)->Option<String> {Some(this.build_state_text(model_projection::dockable(this.owner.data_context().as_ref()).and_then(|d|d.as_dock())))}
        }
        impl $name {
            pub(crate) fn construct(owner:&$owner)->Self {Self{base:SelectingItemsControlAutomationPeer::construct(owner),owner:owner.to_ref()}}
            pub(crate) fn owner_control(&self)->Ref<$owner> {self.owner.clone()}
        }
    };
}
pub(crate) use define_dock_tab_strip_peer;
