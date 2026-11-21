//! Source-verbatim helper oracle comparisons; native control/property execution.
use super::*;
use crate::controls::{
    RibbonItemLayout, RibbonKeyTipResult, RibbonOverflowMenuItems, RibbonScreenTip,
};
use ferroribbon_core::{
    RibbonItemSize, RibbonSimplifiedVisibility, RibbonString, layout::RibbonSizeDefinition,
};
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{Application, ControlImpl, ToolTip, automation::AutomationProperties};
use serde_json::{Value, json};
#[repr(C)]
struct Item {
    base: Control,
    id: RefCell<Option<RibbonString>>,
    label: RefCell<Option<RibbonString>>,
    shortcut: RefCell<Option<RibbonString>>,
    key: RefCell<Option<RibbonString>>,
    tip: RefCell<Option<BoxedValue>>,
    owned: OwnedItemValues,
    labels: RefCell<Option<std::collections::VecDeque<Option<RibbonString>>>>,
    shortcuts: RefCell<Option<std::collections::VecDeque<Option<RibbonString>>>>,
    keys: RefCell<Option<std::collections::VecDeque<Option<RibbonString>>>>,
    reads: RefCell<Vec<&'static str>>,
    disable: Cell<bool>,
}
ferro_class!(Item:Control);
ferro_impl_classes!(Item:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(Item {interfaces:[Rc<dyn IRibbonItemInternal> =>|v|Rc::new(View(v)) as Rc<dyn IRibbonItemInternal>,Rc<dyn IRibbonItem> =>|v|Rc::new(View(v)) as Rc<dyn IRibbonItem>]});
impl Item {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            id: RefCell::new(None),
            label: RefCell::new(None),
            shortcut: RefCell::new(None),
            key: RefCell::new(None),
            tip: RefCell::new(None),
            owned: Default::default(),
            labels: Default::default(),
            shortcuts: Default::default(),
            keys: Default::default(),
            reads: Default::default(),
            disable: Cell::new(false),
        })
    }
}
fn next_value(
    values: &RefCell<Option<std::collections::VecDeque<Option<RibbonString>>>>,
    fallback: Option<RibbonString>,
) -> Option<RibbonString> {
    let mut values = values.borrow_mut();
    if let Some(values) = values.as_mut() {
        if values.len() > 1 {
            values.pop_front().unwrap()
        } else {
            values.front().cloned().flatten()
        }
    } else {
        fallback
    }
}
struct View(Ref<Item>);
impl IRibbonItem for View {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        self.0.id.borrow().clone()
    }
    fn label(&self) -> Option<RibbonString> {
        self.0.reads.borrow_mut().push("Label");
        let value = next_value(&self.0.labels, self.0.label.borrow().clone());
        if self.0.disable.get() {
            self.0.set_is_enabled(false);
        }
        value
    }
    fn key_tip(&self) -> Option<RibbonString> {
        self.0.reads.borrow_mut().push("KeyTip");
        next_value(&self.0.keys, self.0.key.borrow().clone())
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
impl IRibbonItemInternal for View {
    fn shortcut(&self) -> Option<RibbonString> {
        self.0.reads.borrow_mut().push("Shortcut");
        next_value(&self.0.shortcuts, self.0.shortcut.borrow().clone())
    }
    fn screen_tip(&self) -> Option<BoxedValue> {
        self.0.reads.borrow_mut().push("ScreenTip");
        self.0.tip.borrow().clone()
    }
    fn linked_properties(&self) -> Vec<&'static FerroProperty> {
        vec![]
    }
    fn on_unlinked(&self) {}
    fn owned_values(&self) -> &OwnedItemValues {
        &self.0.owned
    }
}
struct ThemeScope {
    app: Ref<Application>,
    style: Rc<dyn ferroui_base::styling::IStyle>,
}
impl Drop for ThemeScope {
    fn drop(&mut self) {
        self.app.styles().remove(self.style.clone());
    }
}
pub(super) fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let style = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(style.clone());
        let _scope = ThemeScope { app, style };
        test()
    })
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/item-owned.json")).unwrap()
}
fn string(v: &Value) -> Option<RibbonString> {
    if v.is_null() {
        None
    } else {
        Some(RibbonString::from_utf16(
            v.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        ))
    }
}
fn text(v: Option<String>) -> Value {
    v.map_or(Value::Null, |s| json!(s.encode_utf16().collect::<Vec<_>>()))
}
fn local(item: &Control, p: &'static ferroui_base::StyledProperty<Option<String>>) -> Value {
    match item.get_local_value(p) {
        None => json!({"kind":"unset","value":null}),
        Some(None) => json!({"kind":"null","value":null}),
        Some(Some(v)) => json!({"kind":"value","value":text(Some(v))}),
    }
}
fn snapshot(item: &Item) -> Value {
    let tip = item
        .get_value(ToolTip::tip_property())
        .and_then(|v| ValueTypes::as_object(&*v))
        .and_then(|v| v.cast::<ToolTip>());
    let content = tip.and_then(|v| v.content());
    let value = if let Some(rich) = content
        .as_ref()
        .and_then(|v| ValueTypes::as_object(&**v))
        .and_then(|v| v.cast::<RibbonScreenTip>())
    {
        json!({"kind":"rich","title":text(rich.title()),"shortcut":text(rich.shortcut()),"description":text(rich.description()),"disabled":text(rich.disabled_reason())})
    } else if let Some(s) = content.and_then(|v| v.downcast_ref::<String>().cloned()) {
        json!({"kind":"string","text":text(Some(s))})
    } else {
        json!({"kind":"none"})
    };
    json!({"tip":value,"name":local(item,AutomationProperties::name_property()),"shortcut":local(item,AutomationProperties::accelerator_key_property()),"keyTip":local(item,AutomationProperties::access_key_property()),"owned":item.owned.entries.borrow().len()})
}
#[test]
fn source_owned_local_values_and_reentrant_update_order() {
    run(|| {
        let c = corpus();
        let p = AutomationProperties::name_property();
        for row in c["values"].as_array().unwrap() {
            let item = Item::new();
            let state = &item.owned;
            match row["initial"].as_str().unwrap() {
                "null" => item.set_value(p, None),
                "application" => item.set_value(p, Some("application".into())),
                "owned" | "equal" | "overridden" => {
                    RibbonItemHelper::set_owned_value(&item, state, p, Some("seed".to_owned()))
                        .unwrap();
                    if row["initial"] == "equal" {
                        item.set_value(p, Some(["se", "ed"].concat()));
                    }
                    if row["initial"] == "overridden" {
                        item.set_value(p, Some("application".into()));
                    }
                }
                _ => {}
            }
            assert_eq!(local(&item, p), row["before"]);
            RibbonItemHelper::set_owned_value(
                &item,
                state,
                p,
                string(&row["next"]).map(|s| s.to_utf8().unwrap()),
            )
            .unwrap();
            assert_eq!(local(&item, p), row["after"]);
            assert_eq!(
                state.entries.borrow().len(),
                row["count"].as_u64().unwrap() as usize
            );
            RibbonItemHelper::set_owned_value(&item, state, p, None).unwrap();
            assert_eq!(local(&item, p), row["cleared"]);
            assert_eq!(
                state.entries.borrow().len(),
                row["remaining"].as_u64().unwrap() as usize
            );
        }
        let item = Item::new();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let weak = item.downgrade();
        item.add_property_changed_handler(Rc::new(move |e| {
            if e.property() != p.as_property() {
                return;
            }
            let item = weak.upgrade().unwrap();
            out.borrow_mut()
                .push(json!({"value":local(&item,p),"owned":item.owned.entries.borrow().len()}));
            if item.get_value(p).as_deref() == Some("outer") {
                RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("nested".to_owned()))
                    .unwrap();
            }
        }));
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("outer".to_owned())).unwrap();
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("after".to_owned())).unwrap();
        assert_eq!(json!(*trace.borrow()), c["trace"]);
        assert_eq!(local(&item, p), c["reentrant"]);
        let clear = Item::new();
        RibbonItemHelper::set_owned_value(&clear, &clear.owned, p, Some("seed".to_owned()))
            .unwrap();
        let once = Rc::new(Cell::new(false));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let weak = clear.downgrade();
        clear.add_property_changed_handler(Rc::new(move |e| {
            if e.property() != p.as_property() {
                return;
            }
            let item = weak.upgrade().unwrap();
            out.borrow_mut()
                .push(json!({"value":local(&item,p),"owned":item.owned.entries.borrow().len()}));
            if !once.replace(true) {
                RibbonItemHelper::set_owned_value(
                    &item,
                    &item.owned,
                    p,
                    Some("recreated".to_owned()),
                )
                .unwrap();
            }
        }));
        RibbonItemHelper::set_owned_value(&clear, &clear.owned, p, None).unwrap();
        assert_eq!(json!(*trace.borrow()), c["clearTrace"]);
        assert_eq!(local(&clear, p), c["clearedReentry"]);
    })
}
#[test]
fn source_tooltip_copies_automation_strings_and_fresh_identity() {
    run(|| {
        let c = corpus();
        for row in c["copies"].as_array().unwrap() {
            let item = Item::new();
            item.label.replace(string(&row["label"]));
            item.shortcut.replace(string(&row["shortcut"]));
            item.key.replace(string(&row["key"]));
            item.set_is_enabled(row["enabled"].as_bool().unwrap());
            match row["kind"].as_str().unwrap() {
                "string" => {
                    item.tip.replace(Some(Rc::new("Description".to_owned())));
                }
                "rich" => {
                    let rich = RibbonScreenTip::new();
                    rich.set_description(Some("Description".into()));
                    rich.set_disabled_reason(Some("Disabled".into()));
                    item.tip.replace(Some(Control::boxed(rich)));
                }
                _ => {}
            }
            RibbonItemHelper::update_tool_tip(&item).unwrap();
            assert_eq!(snapshot(&item), row["first"], "{row}");
            let first = item.get_value(ToolTip::tip_property());
            RibbonItemHelper::update_tool_tip(&item).unwrap();
            assert_eq!(snapshot(&item), row["second"]);
            let second = item.get_value(ToolTip::tip_property());
            assert_eq!(
                first.is_some()
                    && first.as_ref().and_then(|v| ValueTypes::as_object(&**v))
                        != second.as_ref().and_then(|v| ValueTypes::as_object(&**v)),
                row["fresh"].as_bool().unwrap()
            );
        }
        let item = Item::new();
        item.label.replace(Some("Before".into()));
        item.shortcut.replace(Some("Old".into()));
        item.key.replace(Some("A".into()));
        let weak = item.downgrade();
        item.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == ToolTip::tip_property().as_property() {
                let item = weak.upgrade().unwrap();
                item.label.replace(Some("After\nLine".into()));
                item.shortcut.replace(Some("New".into()));
                item.key.replace(Some("B".into()));
            }
        }));
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        assert_eq!(snapshot(&item), c["late"]);
    })
}
#[test]
fn source_automation_ids_preserve_explicit_null_empty_override_and_id_replacement() {
    run(|| {
        let p = AutomationProperties::automation_id_property();
        let c = corpus();
        for row in c["ids"].as_array().unwrap() {
            let item = Item::new();
            item.id.replace(string(&row["id"]));
            match row["initial"].as_str().unwrap() {
                "null" => item.set_value(p, None),
                "empty" => item.set_value(p, Some("".into())),
                "application" => item.set_value(p, Some("app".into())),
                _ => {}
            }
            let result = RibbonItemHelper::update_automation(&item);
            if row["initial"] == "unset"
                && item
                    .id
                    .borrow()
                    .as_ref()
                    .is_some_and(|v| v.to_utf8().is_err())
            {
                assert_eq!(result.unwrap_err().property, "Id");
                assert_eq!(item.get_local_value(p), None);
                continue;
            }
            result.unwrap();
            assert_eq!(local(&item, p), row["first"]);
            item.id.replace(Some("Changed".into()));
            RibbonItemHelper::update_automation(&item).unwrap();
            assert_eq!(local(&item, p), row["second"]);
        }
    })
}

#[test]
fn animated_owned_values_preserve_application_input_and_restore_style_when_cleared() {
    run(|| {
        let item = Item::new();
        let p = AutomationProperties::name_property();
        let style = item
            .set_value_with_priority(p, Some("style".into()), BindingPriority::Style)
            .unwrap();
        let animation = item
            .set_value_with_priority(p, Some("animation".into()), BindingPriority::Animation)
            .unwrap();
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("helper".to_owned()))
            .unwrap();
        assert_eq!(item.get_local_value(p), Some(Some("helper".into())));
        assert_eq!(item.get_value(p), Some("animation".into()));
        item.set_value(p, Some("application".into()));
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("next".to_owned())).unwrap();
        assert_eq!(item.get_local_value(p), Some(Some("application".into())));
        item.set_value(p, Some("helper".into()));
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, None).unwrap();
        assert_eq!(item.get_local_value(p), None);
        assert!(item.owned.entries.borrow().is_empty());
        animation.dispose();
        assert_eq!(item.get_value(p), Some("style".into()));
        style.dispose();
    })
}
#[test]
fn native_binding_override_is_protected_and_owned_clear_removes_it() {
    run(|| {
        use ferroui_base::reactive::Observable;
        let item = Item::new();
        let p = AutomationProperties::name_property();
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, Some("seed".to_owned())).unwrap();
        let binding = item.bind(
            p,
            Observable::single_value(Some("application".to_owned())),
            BindingPriority::LocalValue,
        );
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, None).unwrap();
        assert!(item.has_local_value_binding(p.as_property()));
        assert_eq!(item.get_value(p), Some("application".into()));
        assert_eq!(item.owned.entries.borrow().len(), 1);
        item.set_value(p, Some("seed".into()));
        RibbonItemHelper::set_owned_value(&item, &item.owned, p, None).unwrap();
        assert_eq!(item.get_value(p), None);
        assert!(item.owned.entries.borrow().is_empty());
        assert!(!item.has_local_value_binding(p.as_property()));
        binding.dispose();
    })
}
#[test]
fn source_owned_binding_clear_removes_subscription_in_actual_ribbon_button() {
    run(|| {
        use ferroui_base::reactive::{IObserver, LightweightSubject};
        let item = crate::controls::RibbonButton::new();
        let property = AutomationProperties::name_property();
        item.set_label(Some("helper".into()));
        let source = Rc::new(LightweightSubject::new());
        let binding = item.bind(property, source.clone(), BindingPriority::LocalValue);
        source.on_next(Some("helper".to_owned()));
        assert!(source.has_observers());
        assert_eq!(item.get_value(property).as_deref(), Some("helper"));
        item.set_label(None);
        assert_eq!(item.get_value(property), None);
        assert!(!item.has_local_value_binding(property.as_property()));
        assert!(!source.has_observers());
        source.on_next(Some("after".into()));
        assert_eq!(item.get_value(property), None);
        binding.dispose();
    })
}
#[test]
fn tooltip_base_derived_box_aliases_remain_owned_and_old_values_follow_item_lifetime() {
    run(|| {
        let item = Item::new();
        item.label.replace(Some("First".into()));
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        let first = ValueTypes::as_object(&*item.get_value(ToolTip::tip_property()).unwrap())
            .unwrap()
            .cast::<ToolTip>()
            .unwrap();
        let weak_first = first.downgrade();
        item.set_value(ToolTip::tip_property(), Some(Rc::new(first.clone())));
        item.label.replace(Some("Second".into()));
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        let second = ValueTypes::as_object(&*item.get_value(ToolTip::tip_property()).unwrap())
            .unwrap()
            .cast::<ToolTip>()
            .unwrap();
        assert_ne!(first, second);
        drop(first);
        assert!(weak_first.upgrade().is_none());
        let weak_second = second.downgrade();
        let application = ToolTip::new();
        item.set_value(
            ToolTip::tip_property(),
            Some(Control::boxed(application.clone())),
        );
        drop(second);
        assert!(weak_second.upgrade().is_some());
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        assert_eq!(
            ValueTypes::as_object(&*item.get_value(ToolTip::tip_property()).unwrap())
                .unwrap()
                .cast::<ToolTip>(),
            Some(application)
        );
        let weak = item.downgrade();
        drop(item);
        assert!(weak.upgrade().is_none());
        assert!(weak_second.upgrade().is_none());
    })
}
#[test]
fn repeated_presentations_copy_rich_content_share_images_and_refresh_disabled_reason() {
    run(|| {
        let input = RibbonScreenTip::new();
        let image: BoxedValue = Rc::new(Cell::new(1));
        input.set_image(Some(image.clone()));
        input.set_disabled_reason(Some("Reason".into()));
        input.set_description(Some("Description".into()));
        let a = Item::new();
        let b = Item::new();
        for item in [&a, &b] {
            item.label.replace(Some("Label".into()));
            item.tip.replace(Some(Control::boxed(input.clone())));
            RibbonItemHelper::update_tool_tip(item).unwrap();
        }
        let rich = |item: &Item| {
            let tip = ValueTypes::as_object(&*item.get_value(ToolTip::tip_property()).unwrap())
                .unwrap()
                .cast::<ToolTip>()
                .unwrap();
            ValueTypes::as_object(&*tip.content().unwrap())
                .unwrap()
                .cast::<RibbonScreenTip>()
                .unwrap()
        };
        let first = rich(&a);
        let second = rich(&b);
        assert_ne!(first, second);
        assert_ne!(first, input);
        assert!(Rc::ptr_eq(&first.image().unwrap(), &image));
        assert_eq!(first.disabled_reason(), None);
        a.set_is_enabled(false);
        RibbonItemHelper::update_tool_tip(&a).unwrap();
        assert_eq!(rich(&a).disabled_reason().as_deref(), Some("Reason"));
        assert_eq!(rich(&b).disabled_reason(), None);
    })
}
#[test]
fn helper_metadata_null_unknown_and_utf16_errors_are_explicit_without_early_owner_shortcuts() {
    run(|| {
        use ferroui_base::metadata::{MarkupTyped, into_markup_value};
        let plain = Control::new();
        RibbonItemHelper::update_tool_tip(None::<Ref<Control>>).unwrap();
        RibbonItemHelper::update_automation(None::<Ref<Control>>).unwrap();
        RibbonItemHelper::update_tool_tip(&plain).unwrap();
        RibbonItemHelper::update_automation(&plain).unwrap();
        assert_eq!(plain.get_local_value(ToolTip::tip_property()), None);
        for name in ["UpdateToolTip", "UpdateAutomation"] {
            let method = RibbonItemHelper::MARKUP.find_methods(name).next().unwrap();
            (method.invoke)(&[into_markup_value(None::<Ref<Control>>)]).unwrap();
            (method.invoke)(&[into_markup_value(Some(plain.clone()))]).unwrap();
        }
        let item = Item::new();
        let external = ToolTip::new();
        item.set_value(ToolTip::tip_property(), Some(Control::boxed(external)));
        let model = ferroribbon_core::model::RibbonScreenTip::new();
        model.set_title(Some(RibbonString::from_utf16(vec![0xd800])));
        item.tip.replace(Some(model));
        assert_eq!(
            RibbonItemHelper::update_tool_tip(&item)
                .unwrap_err()
                .property,
            "Title"
        );
        assert!(item.owned.entries.borrow().is_empty());
    })
}

#[test]
fn source_getter_order_enabled_snapshot_and_null_failure_after_tooltip_write() {
    run(|| {
        let c = corpus();
        for row in c["getters"].as_array().unwrap() {
            let item = Item::new();
            item.labels.replace(Some(
                row["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(string)
                    .collect(),
            ));
            item.shortcuts.replace(Some(
                row["shortcuts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(string)
                    .collect(),
            ));
            item.keys.replace(Some(
                row["keys"].as_array().unwrap().iter().map(string).collect(),
            ));
            item.disable.set(row["disable"].as_bool().unwrap());
            let rich = RibbonScreenTip::new();
            rich.set_description(Some("Description".into()));
            rich.set_disabled_reason(Some("Reason".into()));
            item.tip.replace(Some(Control::boxed(rich)));
            let result = RibbonItemHelper::update_tool_tip(&item);
            if row["error"].is_null() {
                result.unwrap();
            } else {
                assert_eq!(row["error"], "NullReferenceException");
                assert_eq!(result.unwrap_err().property, "Label");
            }
            assert_eq!(json!(*item.reads.borrow()), row["reads"]);
            assert_eq!(snapshot(&item), row["after"]);
            assert_eq!(item.is_enabled(), row["enabled"].as_bool().unwrap());
        }
    })
}

#[repr(C)]
struct PublicOnly {
    base: Control,
}
ferro_class!(PublicOnly:Control);
ferro_impl_classes!(PublicOnly:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(PublicOnly {interfaces:[Rc<dyn IRibbonItem> => |v|Rc::new(PublicView(v)) as Rc<dyn IRibbonItem>]});
struct PublicView(Ref<PublicOnly>);
impl IRibbonItem for PublicView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        Some("Public".into())
    }
    fn label(&self) -> Option<RibbonString> {
        panic!("UpdateToolTip must not treat a public-only item as internal")
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
fn public_only_item_automation_dispatch_does_not_require_the_internal_contract() {
    run(|| {
        let item = instantiate(PublicOnly {
            base: Control::construct(),
        });
        assert!(internal(&item).is_none());
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        RibbonItemHelper::update_automation(&item).unwrap();
        assert_eq!(
            item.get_value(AutomationProperties::automation_id_property())
                .as_deref(),
            Some("Public")
        );
        assert_eq!(item.get_local_value(ToolTip::tip_property()), None);
    })
}

#[test]
fn production_ribbon_base_has_real_common_item_contracts() {
    run(|| {
        let item = instantiate(crate::controls::RibbonControlBase::construct());
        assert!(internal(&item).is_some());
        item.set_label(Some("Paste".into()));
        RibbonItemHelper::update_tool_tip(&item).unwrap();
        item.set_id(Some("paste".into()));
        RibbonItemHelper::update_automation(&item).unwrap();
        assert_eq!(
            item.get_value(AutomationProperties::name_property())
                .as_deref(),
            Some("Paste")
        );
        assert_eq!(
            item.get_value(AutomationProperties::automation_id_property())
                .as_deref(),
            Some("paste")
        );
        assert!(!item.execute_command(None).unwrap());
    })
}
