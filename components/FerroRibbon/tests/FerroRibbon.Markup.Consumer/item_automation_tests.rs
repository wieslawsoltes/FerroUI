//! Independent Rust/interface and public static-metadata consumer. Eligible item
//! XAML declaration awaits the complete public item-interface metadata surface.
use super::tests::run;
use ferroribbon::controls::{
    IRibbonItem, RibbonItemHelper, RibbonItemLayout, RibbonKeyTipResult, RibbonOverflowMenuItems,
};
use ferroribbon_core::{
    RibbonItemSize, RibbonSimplifiedVisibility, RibbonString, layout::RibbonSizeDefinition,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{MarkupTyped, into_markup_value},
};
use ferroui_controls::{Control, ControlImpl, automation::AutomationProperties};
use std::rc::Rc;
#[repr(C)]
struct ConsumerItem {
    base: Control,
}
ferro_class!(ConsumerItem:Control);
ferro_impl_classes!(ConsumerItem:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(ConsumerItem {interfaces:[Rc<dyn IRibbonItem> => |v|Rc::new(ItemView(v)) as Rc<dyn IRibbonItem>]});
struct ItemView(Ref<ConsumerItem>);
impl IRibbonItem for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        Some("Independent\nID".into())
    }
    fn label(&self) -> Option<RibbonString> {
        panic!("public-only item must not be used as an internal tooltip item")
    }
    fn key_tip(&self) -> Option<RibbonString> {
        None
    }
    fn icon(&self) -> Option<BoxedValue> {
        None
    }
    fn command_id(&self) -> Option<RibbonString> {
        None
    }
    fn size(&self) -> RibbonItemSize {
        RibbonItemSize::Medium
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        RibbonSizeDefinition::medium_small()
    }
    fn current_size(&self) -> RibbonItemSize {
        self.size()
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        RibbonSimplifiedVisibility::Auto
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        false
    }
    fn can_add_to_quick_access(&self) -> bool {
        true
    }
    fn apply_layout(&self, _: RibbonItemLayout) {
        panic!("fixture has no presentation")
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        panic!("fixture has no copy factory")
    }
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems {
        panic!("fixture has no menu")
    }
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult> {
        panic!("fixture has no KeyTip action")
    }
    fn invoke(&self) -> bool {
        panic!("fixture has no command")
    }
}
#[test]
fn public_item_and_nullable_helper_static_methods_are_consumable_across_crates() {
    run(|| {
        let item = instantiate(ConsumerItem {
            base: Control::construct(),
        });
        RibbonItemHelper::update_automation(&item).unwrap();
        let p = AutomationProperties::automation_id_property();
        assert_eq!(item.get_value(p).as_deref(), Some("Independent\nID"));
        item.set_value(p, Some("Application".into()));
        let method = RibbonItemHelper::MARKUP
            .find_methods("UpdateAutomation")
            .next()
            .unwrap();
        (method.invoke)(&[into_markup_value(Some(item.clone().upcast::<Control>()))]).unwrap();
        assert_eq!(item.get_value(p).as_deref(), Some("Application"));
        item.clear_local_value(p);
        (method.invoke)(&[into_markup_value(Some(item.clone().upcast::<Control>()))]).unwrap();
        assert_eq!(item.get_value(p).as_deref(), Some("Independent\nID"));
        let tip = RibbonItemHelper::MARKUP
            .find_methods("UpdateToolTip")
            .next()
            .unwrap();
        (tip.invoke)(&[into_markup_value(Some(item.clone().upcast::<Control>()))]).unwrap();
        (tip.invoke)(&[into_markup_value(None::<Ref<Control>>)]).unwrap();
        (method.invoke)(&[into_markup_value(None::<Ref<Control>>)]).unwrap();
    })
}
