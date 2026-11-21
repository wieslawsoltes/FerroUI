//! Source IReadOnlyList view retains the actual mutable List allocation.
use crate::primitives::RibbonToolBarOverflowItems;
use ferroui_base::{
    Ref,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::Control;
pub struct RibbonToolBarOverflowMetadata;
ferroui_base::ferro_markup_type!(interface RibbonToolBarOverflowMetadata as "IReadOnlyList`1" {
namespace:"System.Collections.Generic",this:RibbonToolBarOverflowItems,handles:[RibbonToolBarOverflowItems],generic:"IReadOnlyList`1" [Option<Ref<Control>>],
properties:[Count:i32{get:|v:&RibbonToolBarOverflowItems|v.count()}],
indexers:[(i32)->Option<Ref<Control>>{get:|v:&RibbonToolBarOverflowItems,i|v.get(i)}]
});
pub(crate) fn register() {
    MarkupType::register_all(&[RibbonToolBarOverflowMetadata::MARKUP]);
}
