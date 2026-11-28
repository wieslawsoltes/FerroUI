//! Focused collection metadata for actual application-menu XAML consumers.
//! Rust callers use the native ObservableCollection API directly.
use crate::controls::{
    RibbonApplicationMenuItem, RibbonApplicationMenuItems, RibbonApplicationMenuRecentItem,
    RibbonApplicationMenuRecentItems,
};
use ferroui_base::{
    Ref,
    data::model::ModelTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::any::TypeId;
pub struct RibbonApplicationMenuItemsMetadata;
pub struct RibbonApplicationMenuRecentItemsMetadata;
ferroui_base::ferro_markup_type!(class RibbonApplicationMenuItemsMetadata as "RibbonApplicationMenuItems" {
 namespace:"FerroRibbon.Controls", this:RibbonApplicationMenuItems, handles:[RibbonApplicationMenuItems], notify_property_changed:RibbonApplicationMenuItems,
 properties:[Count:i32{get:|v:&RibbonApplicationMenuItems|i32::try_from(v.count()).expect("application-menu collection exceeds Int32 count")}],
 indexers:[(i32)->Option<Ref<RibbonApplicationMenuItem>>{try_get:|v:&RibbonApplicationMenuItems,i|checked_index(i,v.count()).map(|index|v.get(index))}],
 methods:[fn Add(Option<Ref<RibbonApplicationMenuItem>>)=>|v:&RibbonApplicationMenuItems,item|v.add(item)]
});
ferroui_base::ferro_markup_type!(class RibbonApplicationMenuRecentItemsMetadata as "RibbonApplicationMenuRecentItems" {
 namespace:"FerroRibbon.Controls", this:RibbonApplicationMenuRecentItems, handles:[RibbonApplicationMenuRecentItems], notify_property_changed:RibbonApplicationMenuRecentItems,
 properties:[Count:i32{get:|v:&RibbonApplicationMenuRecentItems|i32::try_from(v.count()).expect("application-menu collection exceeds Int32 count")}],
 indexers:[(i32)->Option<Ref<RibbonApplicationMenuRecentItem>>{try_get:|v:&RibbonApplicationMenuRecentItems,i|checked_index(i,v.count()).map(|index|v.get(index))}],
 methods:[fn Add(Option<Ref<RibbonApplicationMenuRecentItem>>)=>|v:&RibbonApplicationMenuRecentItems,item|v.add(item)]
});
pub(crate) fn register() {
    MarkupType::register_all(&[
        RibbonApplicationMenuItemsMetadata::MARKUP,
        RibbonApplicationMenuRecentItemsMetadata::MARKUP,
    ]);
    if !ModelTypes::is_registered(TypeId::of::<RibbonApplicationMenuItems>()) {
        ModelTypes::register::<RibbonApplicationMenuItems>(|b| {
            b.notify_property_changed().notify_collection_changed()
        });
    }
    if !ModelTypes::is_registered(TypeId::of::<RibbonApplicationMenuRecentItems>()) {
        ModelTypes::register::<RibbonApplicationMenuRecentItems>(|b| {
            b.notify_property_changed().notify_collection_changed()
        });
    }
}

// FooterItems shares the existing RibbonControlItems concrete native collection.
// Its metadata is registered once by metadata::control_items; registering another
// descriptor for the same Rust TypeId would shadow its indexer for all containers.

fn checked_index(index: i32, count: usize) -> Result<usize, String> {
    usize::try_from(index)
        .ok()
        .filter(|&index| index < count)
        .ok_or_else(|| {
            format!("application-menu index {index} is outside a collection of {count} items")
        })
}
