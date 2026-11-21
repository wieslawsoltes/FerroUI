//! Native closed list/tuple metadata for Ribbon.Keyboard.cs public views.
use crate::controls::RibbonKeyTip;
use ferroribbon_core::{collections::RibbonReadOnlyList, keytips::RibbonKeyTipEntry};
use ferroui_base::Ref;
use ferroui_controls::Control;
pub type RibbonActiveKeyTips = RibbonReadOnlyList<Option<Ref<RibbonKeyTip>>>;
pub type RibbonCurrentKeyTips = RibbonReadOnlyList<RibbonKeyTipEntry<Ref<Control>>>;
pub struct RibbonActiveKeyTipsMetadata;
pub struct RibbonCurrentKeyTipsMetadata;
pub struct RibbonKeyTipTupleMetadata;
ferroui_base::ferro_markup_type!(interface RibbonActiveKeyTipsMetadata as "IReadOnlyList`1" {
 namespace:"System.Collections.Generic",this:RibbonActiveKeyTips,handles:[RibbonActiveKeyTips],generic:"IReadOnlyList`1" [Option<Ref<RibbonKeyTip>>],
 properties:[Count:i32{get:|v:&RibbonActiveKeyTips|v.count()}],indexers:[(i32)->Option<Ref<RibbonKeyTip>>{get:|v:&RibbonActiveKeyTips,i|v.get(i)}],
});
ferroui_base::ferro_markup_type!(interface RibbonCurrentKeyTipsMetadata as "IReadOnlyList`1" {
 namespace:"System.Collections.Generic",this:RibbonCurrentKeyTips,handles:[RibbonCurrentKeyTips],generic:"IReadOnlyList`1" [RibbonKeyTipEntry<Ref<Control>>],
 properties:[Count:i32{get:|v:&RibbonCurrentKeyTips|v.count()}],indexers:[(i32)->RibbonKeyTipEntry<Ref<Control>>{get:|v:&RibbonCurrentKeyTips,i|v.get(i)}],
});
ferroui_base::ferro_markup_type!(struct RibbonKeyTipTupleMetadata as "ValueTuple`2" {
 namespace:"System",this:RibbonKeyTipEntry<Ref<Control>>,handles:[RibbonKeyTipEntry<Ref<Control>>],generic:"ValueTuple`2" [Option<String>,Option<Ref<Control>>],
 properties:[Item1:Option<String>{try_get:|v:&RibbonKeyTipEntry<Ref<Control>>|v.tip.as_ref().map(|s|s.to_utf8()).transpose()},Item2:Option<Ref<Control>>{get:|v:&RibbonKeyTipEntry<Ref<Control>>|v.target.clone()}],
});
pub(crate) fn register() {
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    MarkupType::register_all(&[
        RibbonActiveKeyTipsMetadata::MARKUP,
        RibbonCurrentKeyTipsMetadata::MARKUP,
        RibbonKeyTipTupleMetadata::MARKUP,
    ]);
}
