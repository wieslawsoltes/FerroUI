//! Native selector construction and init accessors over original Core allocations.
use ferroboard_core::layouts::{
    CaseSelector, CharWidthSelector, KanaSelector, Key, LayoutDirectionSelector,
    ShiftStateSelector, VariationSelector,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::{any::Any, rc::Rc};
#[doc(hidden)]
pub struct CaseSelectorMetadata;
ferroui_base::ferro_markup_type!(class CaseSelectorMetadata as "CaseSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<CaseSelector>,handles:[CaseSelector,Rc<CaseSelector>,Option<Rc<CaseSelector>>],base:Key,
 attributes:[Sealed,RequiredMember],constructors:[()=>(||Rc::new(CaseSelector::default()))],
 properties:[
 Lower:Option<Key>{get:|k:&Rc<CaseSelector>|k.lower(),set:|k:&Rc<CaseSelector>,v|k.set_lower(v)}[RequiredMember],
 Upper:Option<Key>{get:|k:&Rc<CaseSelector>|k.upper(),set:|k:&Rc<CaseSelector>,v|k.set_upper(v)}[RequiredMember]
 ]
});
#[doc(hidden)]
pub struct ShiftStateSelectorMetadata;
ferroui_base::ferro_markup_type!(class ShiftStateSelectorMetadata as "ShiftStateSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<ShiftStateSelector>,handles:[ShiftStateSelector,Rc<ShiftStateSelector>,Option<Rc<ShiftStateSelector>>],base:Key,
 attributes:[Sealed],constructors:[()=>(||Rc::new(ShiftStateSelector::default()))],
 properties:[
 Unshifted:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.unshifted(),set:|k:&Rc<ShiftStateSelector>,v|k.set_unshifted(v)},
 Shifted:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.shifted(),set:|k:&Rc<ShiftStateSelector>,v|k.set_shifted(v)},
 ShiftedManual:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.shifted_manual(),set:|k:&Rc<ShiftStateSelector>,v|k.set_shifted_manual(v)},
 ShiftedAutomatic:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.shifted_automatic(),set:|k:&Rc<ShiftStateSelector>,v|k.set_shifted_automatic(v)},
 CapsLock:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.caps_lock(),set:|k:&Rc<ShiftStateSelector>,v|k.set_caps_lock(v)},
 Default:Option<Key>{get:|k:&Rc<ShiftStateSelector>|k.default_key(),set:|k:&Rc<ShiftStateSelector>,v|k.set_default(v)}
 ]
});
#[doc(hidden)]
pub struct VariationSelectorMetadata;
ferroui_base::ferro_markup_type!(class VariationSelectorMetadata as "VariationSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<VariationSelector>,handles:[VariationSelector,Rc<VariationSelector>,Option<Rc<VariationSelector>>],base:Key,
 attributes:[Sealed],constructors:[()=>(||Rc::new(VariationSelector::default()))],
 properties:[
 Default:Option<Key>{get:|k:&Rc<VariationSelector>|k.default_key(),set:|k:&Rc<VariationSelector>,v|k.set_default(v)},
 Email:Option<Key>{get:|k:&Rc<VariationSelector>|k.email(),set:|k:&Rc<VariationSelector>,v|k.set_email(v)},
 Uri:Option<Key>{get:|k:&Rc<VariationSelector>|k.uri(),set:|k:&Rc<VariationSelector>,v|k.set_uri(v)},
 Normal:Option<Key>{get:|k:&Rc<VariationSelector>|k.normal(),set:|k:&Rc<VariationSelector>,v|k.set_normal(v)},
 Password:Option<Key>{get:|k:&Rc<VariationSelector>|k.password(),set:|k:&Rc<VariationSelector>,v|k.set_password(v)}
 ]
});
#[doc(hidden)]
pub struct LayoutDirectionSelectorMetadata;
ferroui_base::ferro_markup_type!(class LayoutDirectionSelectorMetadata as "LayoutDirectionSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<LayoutDirectionSelector>,handles:[LayoutDirectionSelector,Rc<LayoutDirectionSelector>,Option<Rc<LayoutDirectionSelector>>],base:Key,
 attributes:[Sealed,RequiredMember],constructors:[()=>(||Rc::new(LayoutDirectionSelector::default()))],
 properties:[
 Ltr:Option<Key>{get:|k:&Rc<LayoutDirectionSelector>|k.ltr(),set:|k:&Rc<LayoutDirectionSelector>,v|k.set_ltr(v)}[RequiredMember],
 Rtl:Option<Key>{get:|k:&Rc<LayoutDirectionSelector>|k.rtl(),set:|k:&Rc<LayoutDirectionSelector>,v|k.set_rtl(v)}[RequiredMember]
 ]
});
#[doc(hidden)]
pub struct CharWidthSelectorMetadata;
ferroui_base::ferro_markup_type!(class CharWidthSelectorMetadata as "CharWidthSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<CharWidthSelector>,handles:[CharWidthSelector,Rc<CharWidthSelector>,Option<Rc<CharWidthSelector>>],base:Key,
 attributes:[Sealed],constructors:[()=>(||Rc::new(CharWidthSelector::default()))],
 properties:[
 Full:Option<Key>{get:|k:&Rc<CharWidthSelector>|k.full(),set:|k:&Rc<CharWidthSelector>,v|k.set_full(v)},
 Half:Option<Key>{get:|k:&Rc<CharWidthSelector>|k.half(),set:|k:&Rc<CharWidthSelector>,v|k.set_half(v)}
 ]
});
#[doc(hidden)]
pub struct KanaSelectorMetadata;
ferroui_base::ferro_markup_type!(class KanaSelectorMetadata as "KanaSelector" {
 namespace:"FerroBoard.Layouts",this:Rc<KanaSelector>,handles:[KanaSelector,Rc<KanaSelector>,Option<Rc<KanaSelector>>],base:Key,
 attributes:[Sealed,RequiredMember],constructors:[()=>(||Rc::new(KanaSelector::default()))],
 properties:[
 Hira:Option<Key>{get:|k:&Rc<KanaSelector>|k.hira(),set:|k:&Rc<KanaSelector>,v|k.set_hira(v)}[RequiredMember],
 Kata:Option<Key>{get:|k:&Rc<KanaSelector>|k.kata(),set:|k:&Rc<KanaSelector>,v|k.set_kata(v)}[RequiredMember]
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::CaseSelectorMetadata,crate::ShiftStateSelectorMetadata,crate::VariationSelectorMetadata,crate::LayoutDirectionSelectorMetadata,crate::CharWidthSelectorMetadata,crate::KanaSelectorMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        CaseSelectorMetadata::MARKUP,
        ShiftStateSelectorMetadata::MARKUP,
        VariationSelectorMetadata::MARKUP,
        LayoutDirectionSelectorMetadata::MARKUP,
        CharWidthSelectorMetadata::MARKUP,
        KanaSelectorMetadata::MARKUP,
    ]);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<CaseSelector>();
        ValueTypes::register_cast::<Rc<CaseSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<CaseSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<CaseSelector>().ok()
        });
        ValueTypes::register_reference::<ShiftStateSelector>();
        ValueTypes::register_cast::<Rc<ShiftStateSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<ShiftStateSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<ShiftStateSelector>().ok()
        });
        ValueTypes::register_reference::<VariationSelector>();
        ValueTypes::register_cast::<Rc<VariationSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<VariationSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<VariationSelector>().ok()
        });
        ValueTypes::register_reference::<LayoutDirectionSelector>();
        ValueTypes::register_cast::<Rc<LayoutDirectionSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<LayoutDirectionSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<LayoutDirectionSelector>().ok()
        });
        ValueTypes::register_reference::<CharWidthSelector>();
        ValueTypes::register_cast::<Rc<CharWidthSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<CharWidthSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<CharWidthSelector>().ok()
        });
        ValueTypes::register_reference::<KanaSelector>();
        ValueTypes::register_cast::<Rc<KanaSelector>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<Key, Rc<KanaSelector>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<KanaSelector>().ok()
        });
    });
}
