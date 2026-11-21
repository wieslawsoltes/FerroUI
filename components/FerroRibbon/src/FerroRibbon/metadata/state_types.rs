//! Native reusable descriptors for UI-independent Core state objects.
//! UTF-16 stays intact in Rust; native String conversion rejects unpaired surrogates.
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    RibbonDensity, RibbonDisplayMode, RibbonMinimizeBehavior, RibbonQuickAccessPosition,
    RibbonString, RibbonVisibilityMode,
    state::{
        RibbonCustomGroup, RibbonCustomTab, RibbonCustomization, RibbonFloatingGroupState,
        RibbonState, RibbonStateEntries, RibbonStateFloatingGroups, RibbonStateGroupOrder,
        RibbonStateGroups, RibbonStateLabels, RibbonStateSerializer, RibbonStateStrings,
        RibbonStateTabs,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn string(value: Option<RibbonString>) -> Result<Option<String>, String> {
    value
        .map(|s| s.to_utf8().map_err(|e| e.to_string()))
        .transpose()
}
fn index(value: i32) -> usize {
    usize::try_from(value).expect("ArgumentOutOfRange: index")
}
pub struct RibbonStateMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateMetadata as "RibbonState" {namespace:"FerroRibbon.State",this:Rc<RibbonState>,handles:[RibbonState,Rc<RibbonState>,Option<Rc<RibbonState>>],constructors:[()=>RibbonState::new],properties:[SchemaVersion:i32{get:|v:&Rc<RibbonState>|v.schema_version(),set:|v:&Rc<RibbonState>,x|v.set_schema_version(x)},SelectedTabId:Option<String>{try_get:|v:&Rc<RibbonState>| string(v.selected_tab_id()),set:|v:&Rc<RibbonState>,s:Option<String>|v.set_selected_tab_id(s.map(Into::into))},DisplayMode:RibbonDisplayMode{get:|v:&Rc<RibbonState>|v.display_mode(),set:|v:&Rc<RibbonState>,x|v.set_display_mode(x)},VisibilityMode:RibbonVisibilityMode{get:|v:&Rc<RibbonState>|v.visibility_mode(),set:|v:&Rc<RibbonState>,x|v.set_visibility_mode(x)},Density:RibbonDensity{get:|v:&Rc<RibbonState>|v.density(),set:|v:&Rc<RibbonState>,x|v.set_density(x)},QuickAccessPosition:RibbonQuickAccessPosition{get:|v:&Rc<RibbonState>|v.quick_access_position(),set:|v:&Rc<RibbonState>,x|v.set_quick_access_position(x)},IsQuickAccessVisible:bool{get:|v:&Rc<RibbonState>|v.is_quick_access_visible(),set:|v:&Rc<RibbonState>,x|v.set_is_quick_access_visible(x)},ShowQuickAccessLabels:bool{get:|v:&Rc<RibbonState>|v.show_quick_access_labels(),set:|v:&Rc<RibbonState>,x|v.set_show_quick_access_labels(x)},MinimizeBehavior:RibbonMinimizeBehavior{get:|v:&Rc<RibbonState>|v.minimize_behavior(),set:|v:&Rc<RibbonState>,x|v.set_minimize_behavior(x)},ShowGroupCaptions:bool{get:|v:&Rc<RibbonState>|v.show_group_captions(),set:|v:&Rc<RibbonState>,x|v.set_show_group_captions(x)},FloatingGroups:Option<RibbonStateFloatingGroups>{get:|v:&Rc<RibbonState>|v.floating_groups(),set:|v:&Rc<RibbonState>,x|v.set_floating_groups(x)},QuickAccessItemIds:Option<RibbonStateStrings>{get:|v:&Rc<RibbonState>|v.quick_access_item_ids(),set:|v:&Rc<RibbonState>,x|v.set_quick_access_item_ids(x)},Customization:Option<Rc<RibbonCustomization>>{get:|v:&Rc<RibbonState>|v.customization(),set:|v:&Rc<RibbonState>,x|v.set_customization(x)},RecentSearchIds:Option<RibbonStateStrings>{get:|v:&Rc<RibbonState>|v.recent_search_ids(),set:|v:&Rc<RibbonState>,x|v.set_recent_search_ids(x)},RecentColors:Option<RibbonStateStrings>{get:|v:&Rc<RibbonState>|v.recent_colors(),set:|v:&Rc<RibbonState>,x|v.set_recent_colors(x)}],methods:[fn Normalize()=>|v:&Rc<RibbonState>|v.normalize()],static_properties:[CurrentSchemaVersion:i32{get:||RibbonState::CURRENT_SCHEMA_VERSION}]});
pub struct RibbonCustomizationMetadata;
ferroui_base::ferro_markup_type!(class RibbonCustomizationMetadata as "RibbonCustomization" {namespace:"FerroRibbon.State",this:Rc<RibbonCustomization>,handles:[RibbonCustomization,Rc<RibbonCustomization>,Option<Rc<RibbonCustomization>>],constructors:[()=>RibbonCustomization::new],properties:[TabOrder:Option<RibbonStateStrings>{get:|v:&Rc<RibbonCustomization>|v.tab_order(),set:|v:&Rc<RibbonCustomization>,x|v.set_tab_order(x)},GroupOrder:Option<RibbonStateGroupOrder>{get:|v:&Rc<RibbonCustomization>|v.group_order(),set:|v:&Rc<RibbonCustomization>,x|v.set_group_order(x)},HiddenTabIds:Option<RibbonStateStrings>{get:|v:&Rc<RibbonCustomization>|v.hidden_tab_ids(),set:|v:&Rc<RibbonCustomization>,x|v.set_hidden_tab_ids(x)},HiddenGroupIds:Option<RibbonStateStrings>{get:|v:&Rc<RibbonCustomization>|v.hidden_group_ids(),set:|v:&Rc<RibbonCustomization>,x|v.set_hidden_group_ids(x)},Labels:Option<RibbonStateLabels>{get:|v:&Rc<RibbonCustomization>|v.labels(),set:|v:&Rc<RibbonCustomization>,x|v.set_labels(x)},CustomTabs:Option<RibbonStateTabs>{get:|v:&Rc<RibbonCustomization>|v.custom_tabs(),set:|v:&Rc<RibbonCustomization>,x|v.set_custom_tabs(x)},CustomGroups:Option<RibbonStateGroups>{get:|v:&Rc<RibbonCustomization>|v.custom_groups(),set:|v:&Rc<RibbonCustomization>,x|v.set_custom_groups(x)},IsEmpty:bool{get:|v:&Rc<RibbonCustomization>|v.is_empty()}],methods:[fn Normalize()=>|v:&Rc<RibbonCustomization>|v.normalize()]});
pub struct RibbonCustomTabMetadata;
ferroui_base::ferro_markup_type!(class RibbonCustomTabMetadata as "RibbonCustomTab" {namespace:"FerroRibbon.State",this:Rc<RibbonCustomTab>,handles:[RibbonCustomTab,Rc<RibbonCustomTab>,Option<Rc<RibbonCustomTab>>],constructors:[()=>RibbonCustomTab::new],properties:[Id:Option<String>{try_get:|v:&Rc<RibbonCustomTab>| string(v.id()),set:|v:&Rc<RibbonCustomTab>,s:Option<String>|v.set_id(s.map(Into::into))},Label:Option<String>{try_get:|v:&Rc<RibbonCustomTab>| string(v.label()),set:|v:&Rc<RibbonCustomTab>,s:Option<String>|v.set_label(s.map(Into::into))},Groups:Option<RibbonStateGroups>{get:|v:&Rc<RibbonCustomTab>|v.groups(),set:|v:&Rc<RibbonCustomTab>,x|v.set_groups(x)}]});
pub struct RibbonCustomGroupMetadata;
ferroui_base::ferro_markup_type!(class RibbonCustomGroupMetadata as "RibbonCustomGroup" {namespace:"FerroRibbon.State",this:Rc<RibbonCustomGroup>,handles:[RibbonCustomGroup,Rc<RibbonCustomGroup>,Option<Rc<RibbonCustomGroup>>],constructors:[()=>RibbonCustomGroup::new],properties:[Id:Option<String>{try_get:|v:&Rc<RibbonCustomGroup>| string(v.id()),set:|v:&Rc<RibbonCustomGroup>,s:Option<String>|v.set_id(s.map(Into::into))},Label:Option<String>{try_get:|v:&Rc<RibbonCustomGroup>| string(v.label()),set:|v:&Rc<RibbonCustomGroup>,s:Option<String>|v.set_label(s.map(Into::into))},TabId:Option<String>{try_get:|v:&Rc<RibbonCustomGroup>| string(v.tab_id()),set:|v:&Rc<RibbonCustomGroup>,s:Option<String>|v.set_tab_id(s.map(Into::into))},ItemIds:Option<RibbonStateStrings>{get:|v:&Rc<RibbonCustomGroup>|v.item_ids(),set:|v:&Rc<RibbonCustomGroup>,x|v.set_item_ids(x)}]});
pub struct RibbonFloatingGroupStateMetadata;
ferroui_base::ferro_markup_type!(class RibbonFloatingGroupStateMetadata as "RibbonFloatingGroupState" {namespace:"FerroRibbon.State",this:Rc<RibbonFloatingGroupState>,handles:[RibbonFloatingGroupState,Rc<RibbonFloatingGroupState>,Option<Rc<RibbonFloatingGroupState>>],constructors:[()=>RibbonFloatingGroupState::new],properties:[GroupId:Option<String>{try_get:|v:&Rc<RibbonFloatingGroupState>| string(v.group_id()),set:|v:&Rc<RibbonFloatingGroupState>,s:Option<String>|v.set_group_id(s.map(Into::into))},X:f64{get:|v:&Rc<RibbonFloatingGroupState>|v.x(),set:|v:&Rc<RibbonFloatingGroupState>,x|v.set_x(x)},Y:f64{get:|v:&Rc<RibbonFloatingGroupState>|v.y(),set:|v:&Rc<RibbonFloatingGroupState>,x|v.set_y(x)}]});
pub struct RibbonStateStringsMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateStringsMetadata as "List`1" {namespace:"System.Collections.Generic",this:RibbonStateStrings,handles:[RibbonItems<Option<RibbonString>>,RibbonStateStrings,Option<RibbonStateStrings>],generic:"List`1" [Option<String>],constructors:[()=>new_strings],properties:[Count:i32{get:|v:&RibbonStateStrings|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(i32)->Option<String>{try_get:|v:&RibbonStateStrings,i|string(v.get(index(i))),set:|v:&RibbonStateStrings,i,x:Option<String>|v.set(index(i),x.map(Into::into))}],methods:[fn Add(Option<String>)=>|v:&RibbonStateStrings,x:Option<String>|v.add(x.map(Into::into)),fn Clear()=>|v:&RibbonStateStrings|v.clear(),fn RemoveAt(i32)=>|v:&RibbonStateStrings,i|v.remove_at(index(i)),fn Remove(Option<String>)->bool=>|v:&RibbonStateStrings,x:Option<String>|v.remove(&x.map(Into::into)),fn Insert(i32,Option<String>)=>|v:&RibbonStateStrings,i,x:Option<String>|v.insert(index(i),x.map(Into::into))]});
pub struct RibbonStateGroupsMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateGroupsMetadata as "List`1" {namespace:"System.Collections.Generic",this:RibbonStateGroups,handles:[RibbonItems<Option<Rc<RibbonCustomGroup>>>,RibbonStateGroups,Option<RibbonStateGroups>],generic:"List`1" [Option<Rc<RibbonCustomGroup>>],constructors:[()=>new_groups],properties:[Count:i32{get:|v:&RibbonStateGroups|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(i32)->Option<Rc<RibbonCustomGroup>>{get:|v:&RibbonStateGroups,i|v.get(index(i)),set:|v:&RibbonStateGroups,i,x:Option<Rc<RibbonCustomGroup>>|v.set(index(i),x)}],methods:[fn Add(Option<Rc<RibbonCustomGroup>>)=>|v:&RibbonStateGroups,x:Option<Rc<RibbonCustomGroup>>|v.add(x),fn Clear()=>|v:&RibbonStateGroups|v.clear(),fn RemoveAt(i32)=>|v:&RibbonStateGroups,i|v.remove_at(index(i)),fn Remove(Option<Rc<RibbonCustomGroup>>)->bool=>|v:&RibbonStateGroups,x:Option<Rc<RibbonCustomGroup>>|v.remove(&x),fn Insert(i32,Option<Rc<RibbonCustomGroup>>)=>|v:&RibbonStateGroups,i,x:Option<Rc<RibbonCustomGroup>>|v.insert(index(i),x)]});
pub struct RibbonStateTabsMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateTabsMetadata as "List`1" {namespace:"System.Collections.Generic",this:RibbonStateTabs,handles:[RibbonItems<Option<Rc<RibbonCustomTab>>>,RibbonStateTabs,Option<RibbonStateTabs>],generic:"List`1" [Option<Rc<RibbonCustomTab>>],constructors:[()=>new_tabs],properties:[Count:i32{get:|v:&RibbonStateTabs|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(i32)->Option<Rc<RibbonCustomTab>>{get:|v:&RibbonStateTabs,i|v.get(index(i)),set:|v:&RibbonStateTabs,i,x:Option<Rc<RibbonCustomTab>>|v.set(index(i),x)}],methods:[fn Add(Option<Rc<RibbonCustomTab>>)=>|v:&RibbonStateTabs,x:Option<Rc<RibbonCustomTab>>|v.add(x),fn Clear()=>|v:&RibbonStateTabs|v.clear(),fn RemoveAt(i32)=>|v:&RibbonStateTabs,i|v.remove_at(index(i)),fn Remove(Option<Rc<RibbonCustomTab>>)->bool=>|v:&RibbonStateTabs,x:Option<Rc<RibbonCustomTab>>|v.remove(&x),fn Insert(i32,Option<Rc<RibbonCustomTab>>)=>|v:&RibbonStateTabs,i,x:Option<Rc<RibbonCustomTab>>|v.insert(index(i),x)]});
pub struct RibbonStateFloatingGroupsMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateFloatingGroupsMetadata as "List`1" {namespace:"System.Collections.Generic",this:RibbonStateFloatingGroups,handles:[RibbonItems<Option<Rc<RibbonFloatingGroupState>>>,RibbonStateFloatingGroups,Option<RibbonStateFloatingGroups>],generic:"List`1" [Option<Rc<RibbonFloatingGroupState>>],constructors:[()=>new_floating_groups],properties:[Count:i32{get:|v:&RibbonStateFloatingGroups|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(i32)->Option<Rc<RibbonFloatingGroupState>>{get:|v:&RibbonStateFloatingGroups,i|v.get(index(i)),set:|v:&RibbonStateFloatingGroups,i,x:Option<Rc<RibbonFloatingGroupState>>|v.set(index(i),x)}],methods:[fn Add(Option<Rc<RibbonFloatingGroupState>>)=>|v:&RibbonStateFloatingGroups,x:Option<Rc<RibbonFloatingGroupState>>|v.add(x),fn Clear()=>|v:&RibbonStateFloatingGroups|v.clear(),fn RemoveAt(i32)=>|v:&RibbonStateFloatingGroups,i|v.remove_at(index(i)),fn Remove(Option<Rc<RibbonFloatingGroupState>>)->bool=>|v:&RibbonStateFloatingGroups,x:Option<Rc<RibbonFloatingGroupState>>|v.remove(&x),fn Insert(i32,Option<Rc<RibbonFloatingGroupState>>)=>|v:&RibbonStateFloatingGroups,i,x:Option<Rc<RibbonFloatingGroupState>>|v.insert(index(i),x)]});
pub struct RibbonStateLabelsMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateLabelsMetadata as "Dictionary`2" {namespace:"System.Collections.Generic",this:RibbonStateLabels,handles:[RibbonStateEntries<Option<RibbonString>>,RibbonStateLabels,Option<RibbonStateLabels>],generic:"Dictionary`2" [String,Option<String>],constructors:[()=>new_labels],properties:[Count:i32{get:|v:&RibbonStateLabels|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(String)->Option<String>{try_get:|v:&RibbonStateLabels,k:String|string(v.get(&k.into()).expect("KeyNotFoundException")),set:|v:&RibbonStateLabels,k:String,x:Option<String>|{v.insert(k.into(),x.map(Into::into));}}],methods:[fn Clear()=>|v:&RibbonStateLabels|v.clear(),fn ContainsKey(String)->bool=>|v:&RibbonStateLabels,k:String|v.get(&k.into()).is_some(),fn Remove(String)->bool=>|v:&RibbonStateLabels,k:String|v.remove(&k.into()).is_some()]});
pub struct RibbonStateGroupOrderMetadata;
ferroui_base::ferro_markup_type!(class RibbonStateGroupOrderMetadata as "Dictionary`2" {namespace:"System.Collections.Generic",this:RibbonStateGroupOrder,handles:[RibbonStateEntries<Option<RibbonStateStrings>>,RibbonStateGroupOrder,Option<RibbonStateGroupOrder>],generic:"Dictionary`2" [String,Option<RibbonStateStrings>],constructors:[()=>new_group_order],properties:[Count:i32{get:|v:&RibbonStateGroupOrder|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(String)->Option<RibbonStateStrings>{get:|v:&RibbonStateGroupOrder,k:String|v.get(&k.into()).expect("KeyNotFoundException"),set:|v:&RibbonStateGroupOrder,k:String,x:Option<RibbonStateStrings>|{v.insert(k.into(),x);}}],methods:[fn Clear()=>|v:&RibbonStateGroupOrder|v.clear(),fn ContainsKey(String)->bool=>|v:&RibbonStateGroupOrder,k:String|v.get(&k.into()).is_some(),fn Remove(String)->bool=>|v:&RibbonStateGroupOrder,k:String|v.remove(&k.into()).is_some()]});
pub struct RibbonStateSerializerMetadata;
ferroui_base::ferro_markup_type!(static RibbonStateSerializerMetadata as "RibbonStateSerializer" {namespace:"FerroRibbon.State",methods:[
 static try fn Serialize(Option<Rc<RibbonState>>)->String =>|v:Option<Rc<RibbonState>>|serialize(v,true),
 static try fn Serialize(Option<Rc<RibbonState>>,bool)->String =>serialize,
 static try fn Deserialize(Option<String>)->Option<Rc<RibbonState>> =>|v:Option<String>|RibbonStateSerializer::deserialize(v.map(RibbonString::from).as_ref()).map_err(|e|e.to_string())
]});
fn serialize(value: Option<Rc<RibbonState>>, indented: bool) -> Result<String, String> {
    RibbonStateSerializer::serialize(value.as_deref(), indented)
        .map_err(|e| e.to_string())?
        .to_utf8()
        .map_err(|e| e.to_string())
}
pub(crate) fn register() {
    ValueTypes::register_reference::<RibbonState>();
    // Explicit concrete aliases are facts required by the selected XAML scanner.
    ValueTypes::register_nullable::<RibbonStateStrings>();
    ValueTypes::register_nullable::<RibbonStateGroups>();
    ValueTypes::register_nullable::<RibbonStateTabs>();
    ValueTypes::register_nullable::<RibbonStateFloatingGroups>();
    ValueTypes::register_reference::<RibbonCustomization>();
    ValueTypes::register_reference::<RibbonCustomTab>();
    ValueTypes::register_reference::<RibbonCustomGroup>();
    ValueTypes::register_reference::<RibbonFloatingGroupState>();
    ValueTypes::register_reference::<RibbonItems<Option<RibbonString>>>();
    ValueTypes::register_reference::<RibbonItems<Option<Rc<RibbonCustomGroup>>>>();
    ValueTypes::register_reference::<RibbonItems<Option<Rc<RibbonCustomTab>>>>();
    ValueTypes::register_reference::<RibbonItems<Option<Rc<RibbonFloatingGroupState>>>>();
    ValueTypes::register_reference::<RibbonStateEntries<Option<RibbonString>>>();
    ValueTypes::register_reference::<RibbonStateEntries<Option<RibbonStateStrings>>>();
    MarkupType::register_all(&[
        RibbonStateMetadata::MARKUP,
        RibbonCustomizationMetadata::MARKUP,
        RibbonCustomTabMetadata::MARKUP,
        RibbonCustomGroupMetadata::MARKUP,
        RibbonFloatingGroupStateMetadata::MARKUP,
        RibbonStateStringsMetadata::MARKUP,
        RibbonStateGroupsMetadata::MARKUP,
        RibbonStateTabsMetadata::MARKUP,
        RibbonStateFloatingGroupsMetadata::MARKUP,
        RibbonStateLabelsMetadata::MARKUP,
        RibbonStateGroupOrderMetadata::MARKUP,
        RibbonStateSerializerMetadata::MARKUP,
    ]);
}

fn new_labels() -> RibbonStateLabels {
    RibbonStateEntries::new()
}
fn new_group_order() -> RibbonStateGroupOrder {
    RibbonStateEntries::new()
}

fn new_strings() -> RibbonStateStrings {
    RibbonItems::new()
}

fn new_groups() -> RibbonStateGroups {
    RibbonItems::new()
}

fn new_tabs() -> RibbonStateTabs {
    RibbonItems::new()
}

fn new_floating_groups() -> RibbonStateFloatingGroups {
    RibbonItems::new()
}
