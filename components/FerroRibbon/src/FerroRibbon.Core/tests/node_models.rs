use ferroribbon_core::{
    ObservableObjectImpl, RibbonItemSize, RibbonMergeAction, RibbonString,
    commands::{ICommand, RibbonRelayCommand},
    layout::RibbonSizeDefinition,
    model::*,
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn source() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/nodes.json")).unwrap()
}
fn units(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |s| json!(s.as_utf16()))
}
fn snapshot(
    n: &dyn RibbonNodeModelImpl,
    item: Option<&dyn RibbonItemModelImpl>,
    menu: Option<&dyn RibbonMenuItemModelImpl>,
) -> Value {
    json!({"type":n.model_type_name(),"id":units(n.id()),"label":units(n.label()),"IsVisible":n.is_visible(),"IsEnabled":n.is_enabled(),"Order":n.order(),"merge":n.merge_action().raw(),"icon":n.icon().is_some(),"largeIcon":n.large_icon().is_some(),"keyTip":units(n.key_tip()),"screenTip":n.screen_tip().is_some(),"description":units(n.description()),"tag":n.tag().is_some(),"automationId":units(n.automation_id()),"keywords":n.keywords().count(),"size":item.map(|i|i.size().raw()),"definition":item.map(|i|i.effective_size_definition().to_string()),"command":item.map_or_else(||menu.is_some_and(|m|m.command().is_some()),|i|i.command().is_some()),"text":n.to_ribbon_string().as_utf16()})
}
type Trace = Rc<RefCell<Vec<String>>>;
fn observe(n: &dyn INotifyPropertyChanged) -> Trace {
    let trace = Rc::new(RefCell::new(vec![]));
    let output = trace.clone();
    n.property_changed()
        .add(Rc::new(move |name| output.borrow_mut().push(name.into())));
    trace
}
#[test]
fn original_model_property_changes_are_notified() {
    let node = RibbonToggleButtonModel::new(Some("x".into()), Some("X".into()));
    let trace = observe(node.as_ref());
    node.set_is_checked(true);
    node.set_is_checked(true);
    node.set_label(Some("Y".into()));
    assert_eq!(json!(*trace.borrow()), source()["originalChanges"]);
}
#[test]
fn source_node_and_item_defaults_are_preserved() {
    let button = RibbonButtonModel::new(Some("x".into()), Some("X".into()));
    let toggle = RibbonToggleButtonModel::new(Some("x".into()), Some("X".into()));
    let menu = RibbonMenuItemModel::new(Some("x".into()), Some("X".into()));
    let dropdown = RibbonDropDownButtonModel::new(Some("x".into()), Some("X".into()));
    let split = RibbonSplitButtonModel::new(Some("x".into()), Some("X".into()));
    assert_eq!(
        json!([
            snapshot(button.as_ref(), Some(button.as_ref()), None),
            snapshot(toggle.as_ref(), Some(toggle.as_ref()), None),
            snapshot(menu.as_ref(), None, Some(menu.as_ref())),
            snapshot(dropdown.as_ref(), Some(dropdown.as_ref()), None),
            snapshot(split.as_ref(), Some(split.as_ref()), None)
        ]),
        source()["defaults"]
    );
    assert!(button.show_label());
    assert!(button.can_add_to_quick_access());
    assert!(button.show_label_in_simplified().is_none());
    assert!(button.size_definition().is_none());
    assert!(button.command_parameter().is_none());
    assert!(button.command_id().is_none());
    assert!(toggle.group_name().is_none());
    assert!(!toggle.is_checked());
    assert!(!menu.is_checked());
    assert!(!menu.is_checkable());
    assert_eq!(menu.items().count(), 0);
}
#[test]
fn source_node_setters_preserve_null_utf16_reference_and_structural_equality() {
    let node = RibbonToggleButtonModel::new(Some("x".into()), Some("X".into()));
    node.set_is_checked(true);
    node.set_label(Some("Y".into()));
    let trace = observe(node.as_ref());
    node.set_id(None);
    node.set_label(Some(RibbonString::from_utf16(vec![0xd800, 0xd83d, 0xde00])));
    node.set_icon(Some(RibbonIcon::glyph(Some("a".into()))));
    let icon = node.icon().unwrap();
    node.set_icon(Some(RibbonIcon::glyph(Some("a".into()))));
    node.set_large_icon(node.icon());
    node.set_is_visible(false);
    node.set_is_enabled(false);
    node.set_key_tip(Some(RibbonString::from_utf16(vec![0xdc00])));
    node.set_screen_tip(Some(RibbonScreenTip::new()));
    node.set_description(Some("text".into()));
    node.set_order(-42);
    node.set_tag(Some(RibbonObject::value(7_i32)));
    node.set_tag(Some(RibbonObject::value(7_i32)));
    node.set_automation_id(Some("id".into()));
    node.set_merge_action(RibbonMergeAction::from_raw(99));
    assert_eq!(
        json!({"trace":*trace.borrow(),"snapshot":snapshot(node.as_ref(),Some(node.as_ref()),None),"equalIconRetained":Rc::ptr_eq(&icon,&node.icon().unwrap())}),
        source()["changed"]
    );
}
#[test]
fn dependent_size_notification_reads_state_after_reentrant_callbacks() {
    let node = RibbonButtonModel::new(Some("i".into()), None);
    let trace = observe(node.as_ref());
    let weak = Rc::downgrade(&node);
    node.property_changed().add(Rc::new(move |name| {
        if name == "Size" {
            weak.upgrade()
                .unwrap()
                .set_size_definition(Some(RibbonSizeDefinition::always_large()));
        }
    }));
    node.set_size(RibbonItemSize::Large);
    assert_eq!(json!(*trace.borrow()), source()["reentry"]);
    let node = RibbonButtonModel::new(Some("j".into()), None);
    let trace = observe(node.as_ref());
    let weak = Rc::downgrade(&node);
    node.property_changed().add(Rc::new(move |name| {
        let node = weak.upgrade().unwrap();
        if name == "SizeDefinition" && node.size_definition().is_some() {
            node.set_size_definition(None);
        }
    }));
    node.set_size_definition(Some(RibbonSizeDefinition::always_small()));
    assert_eq!(json!(*trace.borrow()), source()["reset"]);
}
#[test]
fn virtual_object_equality_may_reenter_before_outer_assignment() {
    let node = RibbonButtonModel::new(Some("t".into()), None);
    let trace: Trace = Rc::new(RefCell::new(vec![]));
    let log = trace.clone();
    let weak = Rc::downgrade(&node);
    node.set_tag(Some(RibbonObject::with_equality(
        Rc::new(1_i32),
        move |old, new| {
            log.borrow_mut().push(format!(
                "equals:{old}:{}",
                new.downcast_ref::<i32>().unwrap()
            ));
            weak.upgrade()
                .unwrap()
                .set_label(Some("from-equals".into()));
            false
        },
    )));
    let log = trace.clone();
    node.property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.into())));
    node.set_tag(Some(RibbonObject::value(2_i32)));
    assert_eq!(json!(*trace.borrow()), source()["equalTrace"]);
    assert_eq!(node.tag().unwrap().downcast_ref::<i32>(), Some(&2));
}
#[test]
fn keywords_keep_shared_collection_identity_and_source_event_order() {
    let node = RibbonButtonModel::new(Some("x".into()), None);
    let keywords = node.keywords();
    let trace = observe(&keywords);
    let log = trace.clone();
    keywords.add_collection_changed(Rc::new(move |e| {
        log.borrow_mut().push(format!("{:?}", e.action))
    }));
    let model_trace = observe(node.as_ref());
    keywords.add(None);
    keywords.add(Some("a".into()));
    keywords.set(1, Some("a".into()));
    keywords.move_item(1, 0);
    keywords.remove_at(0);
    keywords.clear();
    assert_eq!(json!(*trace.borrow()), source()["keywordTrace"]);
    assert_eq!(
        keywords.ptr_eq(&node.keywords()),
        source()["keywordsSame"].as_bool().unwrap()
    );
    assert!(model_trace.borrow().is_empty());
}
struct DerivedButton {
    base: RibbonButtonModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedButton {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for DerivedButton {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(format!(
            "{name}:{}:{}",
            self.size(),
            if self.command().is_none() {
                "null"
            } else {
                "command"
            }
        ));
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedButton {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedButton {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for DerivedButton {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
struct DerivedMenu {
    base: RibbonMenuItemModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedMenu {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for DerivedMenu {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(format!(
            "{name}:{}",
            if self.command().is_none() {
                "null"
            } else {
                "command"
            }
        ));
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedMenu {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonMenuItemModelImpl for DerivedMenu {
    fn menu_item_model(&self) -> &RibbonMenuItemModel {
        &self.base
    }
}
#[test]
fn constructor_and_base_handle_calls_reach_derived_virtual_overrides() {
    let button = Rc::new(DerivedButton {
        base: RibbonButtonModel::construct(Some("derived".into()), Some("Derived".into())),
        trace: RefCell::default(),
    });
    RibbonButtonModel::initialize(
        button.as_ref(),
        Some(RibbonIcon::glyph(Some("a".into()))),
        Some(RibbonRelayCommand::new(|| {})),
        RibbonItemSize::Large,
    );
    assert_eq!(json!(*button.trace.borrow()), source()["constructor"]);
    let item: Rc<dyn RibbonItemModelImpl> = button.clone();
    let node: Rc<dyn RibbonNodeModelImpl> = item.clone();
    node.set_label(Some("New".into()));
    assert_eq!(button.trace.borrow().last().unwrap(), "Label:Large:command");
    assert_eq!(
        button.to_ribbon_string().to_utf8().unwrap(),
        "DerivedButton(derived, New)"
    );
    let down = downcast_node::<DerivedButton>(node).ok().unwrap();
    assert!(Rc::ptr_eq(&down, &button));
    let menu = DerivedMenu {
        base: RibbonMenuItemModel::construct(Some("derived".into()), Some("Derived".into())),
        trace: RefCell::default(),
    };
    RibbonMenuItemModel::initialize(
        &menu,
        Some(RibbonIcon::glyph(Some("a".into()))),
        Some(RibbonRelayCommand::new(|| {})),
    );
    assert_eq!(json!(*menu.trace.borrow()), source()["menuConstructor"]);
    assert!(menu.command().is_some());
}
#[test]
fn menu_collections_retain_nodes_null_entries_and_reference_command_state() {
    let menu = RibbonMenuItemModel::new(None, None);
    let child = RibbonToggleButtonModel::new(Some("c".into()), None);
    let node: Rc<dyn RibbonNodeModelImpl> = child.clone();
    menu.items().add(Some(node));
    menu.items().add(None);
    let stored = menu.items().get(0).unwrap();
    assert!(Rc::ptr_eq(
        &downcast_node::<RibbonToggleButtonModel>(stored.clone())
            .ok()
            .unwrap(),
        &child
    ));
    stored.set_is_enabled(false);
    assert!(!child.is_enabled());
    assert!(menu.items().get(1).is_none());
    let command = RibbonRelayCommand::new(|| {});
    let erased: Rc<dyn ICommand> = command.clone();
    let trace = observe(menu.as_ref());
    menu.set_command(Some(erased.clone()));
    menu.set_command(Some(erased.clone()));
    assert!(Rc::ptr_eq(&menu.command().unwrap(), &erased));
    assert_eq!(*trace.borrow(), ["Command"]);
    let inert: Rc<dyn ICommand> = Rc::new(ModelCommand);
    menu.set_command(Some(inert.clone()));
    child.set_command(Some(inert));
    let value = RibbonObject::reference(Rc::new(42_i32));
    menu.set_command_parameter(Some(value.clone()));
    assert!(menu.command_parameter().unwrap().ptr_eq(&value));
    assert!(Rc::ptr_eq(
        &menu.command_parameter().unwrap().payload(),
        &value.payload()
    ));
}
#[test]
fn absent_ids_generate_guid_v4_but_empty_and_runtime_null_ids_are_retained() {
    let ids = (0..128)
        .map(|_| {
            RibbonMenuSeparatorModel::new()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let expected = &source()["guids"];
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len() == 128,
        expected["unique"].as_bool().unwrap()
    );
    assert_eq!(
        ids.iter().all(|s| s.len() == 32
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))),
        expected["format"].as_bool().unwrap()
    );
    assert_eq!(
        ids.iter()
            .all(|s| s.as_bytes()[12] == b'4' && b"89ab".contains(&s.as_bytes()[16])),
        expected["version"].as_bool().unwrap()
    );
    let node = RibbonMenuHeaderModel::new(None);
    node.set_id(None);
    assert_eq!(
        json!(node.to_ribbon_string().as_utf16()),
        source()["nullText"]
    );
    assert_eq!(
        RibbonButtonModel::new(Some("".into()), None).id(),
        Some("".into())
    );
}
#[test]
fn payload_equality_supports_nan_reference_values_and_non_reflexive_overrides() {
    let node = RibbonButtonModel::new(None, None);
    let trace = observe(node.as_ref());
    node.set_tag(Some(RibbonObject::value(f64::NAN)));
    node.set_tag(Some(RibbonObject::value(f64::NAN)));
    assert_eq!(trace.borrow().len(), 1);
    let payload = Rc::new(7_i32);
    node.set_tag(Some(RibbonObject::reference(payload.clone())));
    node.set_tag(Some(RibbonObject::reference(payload)));
    assert_eq!(trace.borrow().len(), 2);
    node.set_tag(Some(RibbonObject::reference(Rc::new(7_i32))));
    assert_eq!(trace.borrow().len(), 3);
    let value = RibbonObject::with_equality(Rc::new(()), |_, _| false);
    node.set_tag(Some(value.clone()));
    node.set_tag(Some(value));
    assert_eq!(trace.borrow().len(), 5);
}

struct ModelCommand;
impl ICommand for ModelCommand {
    fn can_execute(&self, _: Option<&dyn std::any::Any>) -> bool {
        panic!("Core models must not evaluate CanExecute")
    }
    fn execute(&self, _: Option<&dyn std::any::Any>) {
        panic!("Core models must not invoke commands")
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn ferroui_system::IDisposable> {
        panic!("Core models must not subscribe commands")
    }
}

#[path = "../../../tests/fixtures/framework/collection_equality_reentry.rs"]
mod collection_equality_reentry;

struct DerivedSplit {
    base: RibbonSplitButtonModel,
    trace: RefCell<Vec<String>>,
}
impl INotifyPropertyChanged for DerivedSplit {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for DerivedSplit {
    fn on_property_changed(&self, name: &str) {
        self.trace.borrow_mut().push(format!(
            "{name}:{}:{}",
            self.size(),
            if self.command().is_none() {
                "null"
            } else {
                "command"
            }
        ));
        self.property_changed().raise(name);
    }
}
impl RibbonNodeModelImpl for DerivedSplit {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.drop_down_button_model().item_model().node_model()
    }
}
impl RibbonItemModelImpl for DerivedSplit {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.drop_down_button_model().item_model()
    }
}
impl RibbonDropDownButtonModelImpl for DerivedSplit {
    fn drop_down_button_model(&self) -> &RibbonDropDownButtonModel {
        self.base.drop_down_button_model()
    }
}
impl RibbonSplitButtonModelImpl for DerivedSplit {
    fn split_button_model(&self) -> &RibbonSplitButtonModel {
        &self.base
    }
}
#[test]
fn split_constructor_order_and_dropdown_shared_state_match_source() {
    let split = Rc::new(DerivedSplit {
        base: RibbonSplitButtonModel::construct(Some("derived".into()), Some("Derived".into())),
        trace: RefCell::default(),
    });
    RibbonSplitButtonModel::initialize(
        split.as_ref(),
        Some(RibbonIcon::glyph(Some("a".into()))),
        Some(RibbonRelayCommand::new(|| {})),
        RibbonItemSize::Large,
    );
    assert_eq!(json!(*split.trace.borrow()), source()["splitConstructor"]);
    let changes = observe(split.as_ref());
    split.set_is_checkable(true);
    split.set_is_checked(true);
    split.set_is_checked(true);
    split.set_follow_last_choice(true);
    let payload = RibbonObject::value(RibbonString::from("content"));
    split.set_drop_down_content(Some(payload.clone()));
    split.set_drop_down_content(Some(RibbonObject::value(RibbonString::from("content"))));
    assert_eq!(json!(*changes.borrow()), source()["splitChanges"]);
    assert!(split.drop_down_content().unwrap().ptr_eq(&payload));
    let dropdown: Rc<dyn RibbonDropDownButtonModelImpl> = split.clone();
    let menus = dropdown.menu_items();
    assert!(menus.ptr_eq(&split.menu_items()));
    let child: Rc<dyn RibbonNodeModelImpl> = RibbonMenuItemModel::new(Some("action".into()), None);
    menus.add(Some(child.clone()));
    menus.add(Some(child.clone()));
    menus.add(None);
    assert!(Rc::ptr_eq(&menus.get(0).unwrap(), &menus.get(1).unwrap()));
    assert!(menus.get(2).is_none());
}
