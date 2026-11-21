//! Closed System collection metadata for native tabs, groups and contextual groups.
use crate::controls::{RibbonContextualTabGroup, RibbonGroup, RibbonTab};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::Ref;
use ferroui_system::collections::object_model::ObservableCollection;
pub type RibbonGroups = ObservableCollection<Option<Ref<RibbonGroup>>>;
pub type RibbonTabs = ObservableCollection<Option<Ref<RibbonTab>>>;
pub type RibbonContextualGroups = ObservableCollection<Option<Ref<RibbonContextualTabGroup>>>;
macro_rules! collection {
 ($marker:ident,$items:ident,$element:ty)  =>  {
 pub struct $marker;
 ferroui_base::ferro_markup_type!(class $marker as "ObservableCollection`1" {
  namespace:"System.Collections.ObjectModel",handles:[$items],this:$items,
  generic:"ObservableCollection`1" [Option<Ref<$element>>],
  interfaces:[std::rc::Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],
  notify_property_changed:$items,
  properties:[Count:i32{get:|items:&$items|i32::try_from(items.count()).expect("Int32 Count")}],
  indexers:[(i32)->Option<Ref<$element>>{get:|items:&$items,i|items.get(usize::try_from(i).expect("index")),set:|items:&$items,i,value|items.set(usize::try_from(i).expect("index"),value)}],
  methods:[fn Add(Option<Ref<$element>>) => |items:&$items,item|items.add(item),
  fn Clear() => |items:&$items|items.clear(),
  fn Remove(Option<Ref<$element>>)->bool => |items:&$items,item|items.remove(&item),
  fn Insert(i32,Option<Ref<$element>>) => |items:&$items,i,item|items.insert(usize::try_from(i).expect("index"),item),
  fn Move(i32,i32) => |items:&$items,from,to|items.move_item(usize::try_from(from).expect("index"),usize::try_from(to).expect("index")),
  fn RemoveAt(i32) => |items:&$items,i|items.remove_at(usize::try_from(i).expect("index"))],
 });
 };
}
collection!(RibbonGroupsMetadata, RibbonGroups, RibbonGroup);
collection!(RibbonTabsMetadata, RibbonTabs, RibbonTab);
collection!(
    RibbonContextualGroupsMetadata,
    RibbonContextualGroups,
    RibbonContextualTabGroup
);
pub(crate) fn register() {
    use ferroui_base::data::model::ModelTypes;
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    ModelTypes::register_notifying_collection::<RibbonGroups>();
    ModelTypes::register_notifying_collection::<RibbonTabs>();
    ModelTypes::register_notifying_collection::<RibbonContextualGroups>();
    MarkupType::register_all(&[
        RibbonFloatingGroupsMetadata::MARKUP,
        RibbonGroupsMetadata::MARKUP,
        RibbonTabsMetadata::MARKUP,
        RibbonContextualGroupsMetadata::MARKUP,
    ]);
}

/// Source FloatingGroups is a fresh array snapshot, with native reference identity.
pub type RibbonFloatingGroups =
    ferroribbon_core::collections::RibbonReadOnlyList<Option<Ref<RibbonGroup>>>;
pub struct RibbonFloatingGroupsMetadata;
ferroui_base::ferro_markup_type!(interface RibbonFloatingGroupsMetadata as "IReadOnlyList`1"{
    namespace:"System.Collections.Generic",this:RibbonFloatingGroups,handles:[RibbonFloatingGroups],generic:"IReadOnlyList`1" [Option<Ref<RibbonGroup>>],
    properties:[Count:i32{get:|groups:&RibbonFloatingGroups|groups.count()}],
    indexers:[(i32)->Option<Ref<RibbonGroup>>{get:|groups:&RibbonFloatingGroups,i|groups.get(i)}]
});
ferroui_base::ferro_thread_local! {
    static EMPTY_FLOATING_GROUPS: RibbonFloatingGroups =
        RibbonFixedItems::new(Vec::new()) as ferroribbon_core::collections::RibbonReadOnlyList<_>;
}
pub(crate) fn floating_groups_snapshot(
    groups: Vec<Option<Ref<RibbonGroup>>>,
) -> RibbonFloatingGroups {
    if groups.is_empty() {
        EMPTY_FLOATING_GROUPS.with(Clone::clone)
    } else {
        RibbonFixedItems::new(groups) as ferroribbon_core::collections::RibbonReadOnlyList<_>
    }
}
