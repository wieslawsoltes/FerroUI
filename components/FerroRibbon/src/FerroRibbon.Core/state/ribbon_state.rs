//! Plain mutable source state classes, with shared collection/reference identity.
use crate::collections::RibbonItems;
use crate::{collections::RibbonList, text::empty, *};
use std::{cell::RefCell, rc::Rc};
pub type RibbonStateStrings = RibbonList<Option<RibbonString>>;
pub type RibbonStateGroupOrder = Rc<super::RibbonStateEntries<Option<RibbonStateStrings>>>;
pub type RibbonStateLabels = Rc<super::RibbonStateEntries<Option<RibbonString>>>;
pub type RibbonStateGroups = RibbonList<Option<Rc<RibbonCustomGroup>>>;
pub type RibbonStateTabs = RibbonList<Option<Rc<RibbonCustomTab>>>;
pub type RibbonStateFloatingGroups = RibbonList<Option<Rc<RibbonFloatingGroupState>>>;
macro_rules! state_class {
    ($name:ident {$($get:ident/$set:ident:$ty:ty=$default:expr),*$(,)?})=>{
        pub struct $name{$($get:RefCell<$ty>),*}
        impl Default for $name{fn default()->Self{Self{$($get:RefCell::new($default)),*}}}
        impl PartialEq for $name{fn eq(&self,other:&Self)->bool{std::ptr::eq(self,other)}}
        impl $name{
            pub fn new()->Rc<Self>{Rc::new(Self::default())}
            $(pub fn $get(&self)->$ty{self.$get.borrow().to_owned()}
              pub fn $set(&self,value:$ty){let _old=self.$get.replace(value);})*
        }
    };
}
state_class!(RibbonState{
    schema_version/set_schema_version:i32=1,
    selected_tab_id/set_selected_tab_id:Option<RibbonString>=None,
    display_mode/set_display_mode:RibbonDisplayMode=RibbonDisplayMode::default(),
    visibility_mode/set_visibility_mode:RibbonVisibilityMode=RibbonVisibilityMode::default(),
    density/set_density:RibbonDensity=RibbonDensity::default(),
    quick_access_position/set_quick_access_position:RibbonQuickAccessPosition=RibbonQuickAccessPosition::default(),
    is_quick_access_visible/set_is_quick_access_visible:bool=true,
    show_quick_access_labels/set_show_quick_access_labels:bool=false,
    minimize_behavior/set_minimize_behavior:RibbonMinimizeBehavior=RibbonMinimizeBehavior::default(),
    show_group_captions/set_show_group_captions:bool=true,
    floating_groups/set_floating_groups:Option<RibbonStateFloatingGroups>=None,
    quick_access_item_ids/set_quick_access_item_ids:Option<RibbonStateStrings>=None,
    customization/set_customization:Option<Rc<RibbonCustomization>>=Some(RibbonCustomization::new()),
    recent_search_ids/set_recent_search_ids:Option<RibbonStateStrings>=None,
    recent_colors/set_recent_colors:Option<RibbonStateStrings>=None,
});
impl RibbonState {
    pub const CURRENT_SCHEMA_VERSION: i32 = 1;
    pub fn normalize(&self) {
        let customization = if let Some(value) = self.customization() {
            value
        } else {
            let value = RibbonCustomization::new();
            self.set_customization(Some(value.clone()));
            value
        };
        customization.normalize();
        for list in [
            self.quick_access_item_ids(),
            self.recent_search_ids(),
            self.recent_colors(),
        ]
        .into_iter()
        .flatten()
        {
            clean(&list);
        }
        if let Some(groups) = self.floating_groups() {
            groups.remove_all_state(|g| g.as_ref().is_none_or(|g| absent(&g.group_id())));
        }
    }
}
state_class!(RibbonCustomization{
    tab_order/set_tab_order:Option<RibbonStateStrings>=Some(RibbonItems::new()),
    group_order/set_group_order:Option<RibbonStateGroupOrder>=Some(super::RibbonStateEntries::new()),
    hidden_tab_ids/set_hidden_tab_ids:Option<RibbonStateStrings>=Some(RibbonItems::new()),
    hidden_group_ids/set_hidden_group_ids:Option<RibbonStateStrings>=Some(RibbonItems::new()),
    labels/set_labels:Option<RibbonStateLabels>=Some(super::RibbonStateEntries::new()),
    custom_tabs/set_custom_tabs:Option<RibbonStateTabs>=Some(RibbonItems::new()),
    custom_groups/set_custom_groups:Option<RibbonStateGroups>=Some(RibbonItems::new()),
});
fn absent(value: &Option<RibbonString>) -> bool {
    value.as_ref().is_none_or(RibbonString::is_empty)
}
fn clean(list: &RibbonStateStrings) {
    list.remove_all_state(absent);
}
impl RibbonCustomization {
    pub fn normalize(&self) {
        if self.tab_order().is_none() {
            self.set_tab_order(Some(RibbonItems::new()));
        }
        if self.group_order().is_none() {
            self.set_group_order(Some(super::RibbonStateEntries::new()));
        }
        let groups = self.group_order().unwrap();
        for (key, _) in groups
            .entries_snapshot()
            .into_iter()
            .filter(|(_, v)| v.is_none())
        {
            groups.remove(&key);
        }
        for (_, order) in groups.entries_snapshot() {
            clean(&order.unwrap());
        }
        if self.hidden_tab_ids().is_none() {
            self.set_hidden_tab_ids(Some(RibbonItems::new()));
        }
        if self.hidden_group_ids().is_none() {
            self.set_hidden_group_ids(Some(RibbonItems::new()));
        }
        if self.labels().is_none() {
            self.set_labels(Some(super::RibbonStateEntries::new()));
        }
        if self.custom_tabs().is_none() {
            self.set_custom_tabs(Some(RibbonItems::new()));
        }
        if self.custom_groups().is_none() {
            self.set_custom_groups(Some(RibbonItems::new()));
        }
        clean(&self.tab_order().unwrap());
        clean(&self.hidden_tab_ids().unwrap());
        clean(&self.hidden_group_ids().unwrap());
        let tabs = self.custom_tabs().unwrap();
        let custom_groups = self.custom_groups().unwrap();
        tabs.remove_all_state(|t| t.as_ref().is_none_or(|t| absent(&t.id())));
        custom_groups.remove_all_state(|g| g.as_ref().is_none_or(|g| absent(&g.id())));
        for tab in tabs.iter().map(Option::unwrap) {
            if tab.label().is_none() {
                tab.set_label(Some(empty()));
            }
            if tab.groups().is_none() {
                tab.set_groups(Some(RibbonItems::new()));
            }
            let groups = tab.groups().unwrap();
            groups.remove_all_state(|g| g.as_ref().is_none_or(|g| absent(&g.id())));
            for group in groups.iter().map(Option::unwrap) {
                group.normalize();
            }
        }
        for group in custom_groups.iter().map(Option::unwrap) {
            group.normalize();
        }
    }
    pub fn is_empty(&self) -> bool {
        self.tab_order()
            .expect("NullReference: TabOrder")
            .is_empty()
            && self
                .group_order()
                .expect("NullReference: GroupOrder")
                .is_empty()
            && self
                .hidden_tab_ids()
                .expect("NullReference: HiddenTabIds")
                .is_empty()
            && self
                .hidden_group_ids()
                .expect("NullReference: HiddenGroupIds")
                .is_empty()
            && self.labels().expect("NullReference: Labels").is_empty()
            && self
                .custom_tabs()
                .expect("NullReference: CustomTabs")
                .is_empty()
            && self
                .custom_groups()
                .expect("NullReference: CustomGroups")
                .is_empty()
    }
}
state_class!(RibbonCustomTab{
    id/set_id:Option<RibbonString>=Some(empty()),label/set_label:Option<RibbonString>=Some(empty()),groups/set_groups:Option<RibbonStateGroups>=Some(RibbonItems::new()),
});
state_class!(RibbonCustomGroup{
    id/set_id:Option<RibbonString>=Some(empty()),label/set_label:Option<RibbonString>=Some(empty()),tab_id/set_tab_id:Option<RibbonString>=None,item_ids/set_item_ids:Option<RibbonStateStrings>=Some(RibbonItems::new()),
});
impl RibbonCustomGroup {
    pub(crate) fn normalize(&self) {
        if self.label().is_none() {
            self.set_label(Some(empty()));
        }
        if self.item_ids().is_none() {
            self.set_item_ids(Some(RibbonItems::new()));
        }
        clean(&self.item_ids().unwrap());
    }
}
state_class!(RibbonFloatingGroupState{group_id/set_group_id:Option<RibbonString>=Some(empty()),x/set_x:f64=0.,y/set_y:f64=0.,});
