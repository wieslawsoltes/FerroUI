//! Reusable native metadata for source item contracts.
use crate::controls::{
    IRibbonItem, IRibbonItemEditable, RibbonItemLayout, RibbonKeyTipResult,
    RibbonOverflowMenuItems, RibbonSimplifiedLabel,
};
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{
    RibbonGroupState, RibbonItemSize, RibbonSimplifiedVisibility, RibbonString,
    layout::RibbonSizeDefinition,
};
use ferroui_base::{
    BoxedValue, Ref,
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::Control;
use std::{rc::Rc, sync::Arc};

fn native_text(value: Option<RibbonString>) -> Result<Option<String>, std::string::FromUtf16Error> {
    value.map(|v| v.to_utf8()).transpose()
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonItem as "IRibbonItem" {
    namespace:"FerroRibbon.Controls",
    handles:[Rc<dyn IRibbonItem>,Option<Rc<dyn IRibbonItem>>],this:Rc<dyn IRibbonItem>,
    properties:[
        Id:Option<String>{try_get:|v:&Rc<dyn IRibbonItem>|native_text(v.id())},
        Label:Option<String>{try_get:|v:&Rc<dyn IRibbonItem>|native_text(v.label())},
        Icon:Option<BoxedValue>{get:|v:&Rc<dyn IRibbonItem>|v.icon()},
        KeyTip:Option<String>{try_get:|v:&Rc<dyn IRibbonItem>|native_text(v.key_tip())},
        CommandId:Option<String>{try_get:|v:&Rc<dyn IRibbonItem>|native_text(v.command_id())},
        Size:RibbonItemSize{get:|v:&Rc<dyn IRibbonItem>|v.size()},
        EffectiveSizeDefinition:RibbonSizeDefinition{get:|v:&Rc<dyn IRibbonItem>|v.effective_size_definition()},
        CurrentSize:RibbonItemSize{get:|v:&Rc<dyn IRibbonItem>|v.current_size()},
        SimplifiedVisibility:RibbonSimplifiedVisibility{get:|v:&Rc<dyn IRibbonItem>|v.simplified_visibility()},
        ShowLabelInSimplifiedMode:bool{get:|v:&Rc<dyn IRibbonItem>|v.show_label_in_simplified_mode()},
        CanAddToQuickAccess:bool{get:|v:&Rc<dyn IRibbonItem>|v.can_add_to_quick_access()},
    ],
    methods:[
        fn ApplyLayout(RibbonItemLayout)=>|v:&Rc<dyn IRibbonItem>,layout|v.apply_layout(layout),
        fn CreateLinkedCopy()->Option<Ref<Control>> =>|v:&Rc<dyn IRibbonItem>|v.create_linked_copy(),
        fn CreateOverflowMenuItems()->RibbonOverflowMenuItems=>|v:&Rc<dyn IRibbonItem>|v.create_overflow_menu_items(),
        fn OnKeyTip()->Rc<RibbonKeyTipResult> =>|v:&Rc<dyn IRibbonItem>|v.on_key_tip(),
        fn Invoke()->bool=>|v:&Rc<dyn IRibbonItem>|v.invoke(),
    ],
});
ferroui_base::ferro_markup_type!(interface dyn IRibbonItemEditable as "IRibbonItemEditable" {
    namespace:"FerroRibbon.Controls",interfaces:[Rc<dyn IRibbonItem>],
    handles:[Rc<dyn IRibbonItemEditable>,Option<Rc<dyn IRibbonItemEditable>>],this:Rc<dyn IRibbonItemEditable>,
    properties:[
        Id:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(IRibbonItemEditable::id(&**v)),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_id(s.map(Into::into))},
        Label:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(IRibbonItemEditable::label(&**v)),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_label(s.map(Into::into))},
        Icon:Option<BoxedValue>{get:|v:&Rc<dyn IRibbonItemEditable>|IRibbonItemEditable::icon(&**v),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_icon(s)},
        LargeIcon:Option<BoxedValue>{get:|v:&Rc<dyn IRibbonItemEditable>|v.large_icon(),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_large_icon(s)},
        Size:RibbonItemSize{get:|v:&Rc<dyn IRibbonItemEditable>|IRibbonItemEditable::size(&**v),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_size(s)},
        SizeDefinition:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(v.size_definition()),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_size_definition(s.map(Into::into))},
        KeyTip:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(IRibbonItemEditable::key_tip(&**v)),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_key_tip(s.map(Into::into))},
        ScreenTip:Option<BoxedValue>{get:|v:&Rc<dyn IRibbonItemEditable>|v.screen_tip(),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_screen_tip(s)},
        CommandId:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(IRibbonItemEditable::command_id(&**v)),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_command_id(s.map(Into::into))},
        Shortcut:Option<String>{try_get:|v:&Rc<dyn IRibbonItemEditable>|native_text(v.shortcut()),set:|v:&Rc<dyn IRibbonItemEditable>,s:Option<String>|v.set_shortcut(s.map(Into::into))},
        SimplifiedVisibility:RibbonSimplifiedVisibility{get:|v:&Rc<dyn IRibbonItemEditable>|IRibbonItemEditable::simplified_visibility(&**v),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_simplified_visibility(s)},
        SimplifiedLabel:RibbonSimplifiedLabel{get:|v:&Rc<dyn IRibbonItemEditable>|v.simplified_label(),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_simplified_label(s)},
        ShowLabel:bool{get:|v:&Rc<dyn IRibbonItemEditable>|v.show_label(),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_show_label(s)},
        CanAddToQuickAccess:bool{get:|v:&Rc<dyn IRibbonItemEditable>|IRibbonItemEditable::can_add_to_quick_access(&**v),set:|v:&Rc<dyn IRibbonItemEditable>,s|v.set_can_add_to_quick_access(s)},
    ],
});
ferroui_base::ferro_markup_type!(struct RibbonItemLayout {
    namespace:"FerroRibbon.Controls",handles:[RibbonItemLayout],
    constructors:[
        ()=>RibbonItemLayout::default,
        (RibbonItemSize,Option<Arc<RibbonMetrics>>)=>RibbonItemLayout::new,
        (RibbonItemSize,Option<Arc<RibbonMetrics>>,bool)=>|size,metrics,is_simplified|RibbonItemLayout{size,metrics,is_simplified,..RibbonItemLayout::default()},
        (RibbonItemSize,Option<Arc<RibbonMetrics>>,bool,Option<bool>)=>|size,metrics,is_simplified,show_label|RibbonItemLayout{size,metrics,is_simplified,show_label,..RibbonItemLayout::default()},
        (RibbonItemSize,Option<Arc<RibbonMetrics>>,bool,Option<bool>,RibbonGroupState)=>|size,metrics,is_simplified,show_label,group_state|RibbonItemLayout{size,metrics,is_simplified,show_label,group_state},
    ],
    properties:[
        Size:RibbonItemSize{get:|v:&RibbonItemLayout|v.size},
        Metrics:Option<Arc<RibbonMetrics>>{get:|v:&RibbonItemLayout|v.metrics.clone()},
        IsSimplified:bool{get:|v:&RibbonItemLayout|v.is_simplified},
        ShowLabel:Option<bool>{get:|v:&RibbonItemLayout|v.show_label},
        GroupState:RibbonGroupState{get:|v:&RibbonItemLayout|v.group_state},
    ],
});
// The source callback/NextScope init accessor and synthesized record operations
// remain tracked gaps; no unrelated callback or collection API substitutes them.
ferroui_base::ferro_markup_type!(class RibbonKeyTipResult {
    namespace:"FerroRibbon.Controls",this:Rc<RibbonKeyTipResult>,
    handles:[Rc<RibbonKeyTipResult>,Option<Rc<RibbonKeyTipResult>>],
    constructors:[()=>RibbonKeyTipResult::new],
    static_properties:[Close:Rc<RibbonKeyTipResult>{get:RibbonKeyTipResult::close}],
});
#[doc(hidden)]
pub struct RibbonSizeDefinitionMembers;
ferroui_base::ferro_markup_type!(struct RibbonSizeDefinitionMembers as "RibbonSizeDefinition" {
    namespace:"FerroRibbon.Layout",this:RibbonSizeDefinition,handles:[RibbonSizeDefinition],
    parse:RibbonSizeDefinition::parse,
    constructors:[()=>RibbonSizeDefinition::default,(RibbonItemSize,RibbonItemSize,RibbonItemSize)=>RibbonSizeDefinition::new],
    properties:[
        Large:RibbonItemSize{get:|v:&RibbonSizeDefinition|v.large},
        Medium:RibbonItemSize{get:|v:&RibbonSizeDefinition|v.medium},
        Small:RibbonItemSize{get:|v:&RibbonSizeDefinition|v.small},
    ],
    static_properties:[
        LargeMediumSmall:RibbonSizeDefinition{get:RibbonSizeDefinition::large_medium_small},
        AlwaysLarge:RibbonSizeDefinition{get:RibbonSizeDefinition::always_large},
        MediumSmall:RibbonSizeDefinition{get:RibbonSizeDefinition::medium_small},
        AlwaysMedium:RibbonSizeDefinition{get:RibbonSizeDefinition::always_medium},
        AlwaysSmall:RibbonSizeDefinition{get:RibbonSizeDefinition::always_small},
    ],
    methods:[
        static fn ForPreferredSize(RibbonItemSize)->RibbonSizeDefinition=>RibbonSizeDefinition::for_preferred_size,
        fn GetSize(RibbonGroupState)->RibbonItemSize=>|v:&RibbonSizeDefinition,s|v.get_size(s),
        static try fn Parse(String)->RibbonSizeDefinition=>|s:String|RibbonSizeDefinition::parse(&s),
        fn ToString()->String=>|v:&RibbonSizeDefinition|v.to_string(),
    ],
});

/// The nullable descriptor belongs to the Core value, not the declaration marker.
#[doc(hidden)]
pub struct RibbonSizeDefinitionMetadata;
impl MarkupTyped for RibbonSizeDefinitionMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut meta = *RibbonSizeDefinitionMembers::MARKUP;
            meta.nullable =
                Some(ferroui_base::data::core::ValueType::of::<Option<RibbonSizeDefinition>>);
            meta
        };
        &META
    };
}
pub(crate) fn register() {
    MarkupType::register_all(&[
        <dyn IRibbonItem as MarkupTyped>::MARKUP,
        <dyn IRibbonItemEditable as MarkupTyped>::MARKUP,
        RibbonItemLayout::MARKUP,
        RibbonKeyTipResult::MARKUP,
        RibbonSizeDefinitionMetadata::MARKUP,
        super::simplified_label::RibbonSimplifiedLabelMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<Rc<dyn IRibbonItemEditable>>();
    ValueTypes::register_interface_object::<Rc<dyn IRibbonItemEditable>>(|v| Some(v.as_object()));
    ValueTypes::register_cast::<Rc<dyn IRibbonItemEditable>, Rc<dyn IRibbonItem>>(|v| v.clone());
    ValueTypes::register_nullable::<RibbonItemLayout>();
    ValueTypes::register_nullable::<RibbonSizeDefinition>();
    ValueTypes::register_display::<RibbonSizeDefinition>();
    ValueTypes::register_reference::<RibbonKeyTipResult>();
}
