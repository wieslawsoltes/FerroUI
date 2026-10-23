//! Public lifecycle tests: a WinUI source is neither implicit DataContext nor parse-time data.

use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::data::core::{Value, ValueType};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, UpdateSourceTrigger};
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ferro_model, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::{Control, Slider, StackPanel, TextBlock, TextBox};
use ferroui_markup_xaml::winui::{WinUIBindExtension, WinUIBindingScope, WinUIBindings};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Item {
    value: RefCell<String>,
    number: Cell<f64>,
    changed: Event<str>,
}

impl Item {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self {
            value: RefCell::new(value.into()),
            number: Cell::new(1.0),
            changed: Event::new(),
        })
    }
    fn value(&self) -> String {
        let value = self.value.borrow().clone();
        assert_ne!(value, "panic", "user getter failure");
        value
    }
    fn set_value(&self, value: String) {
        *self.value.borrow_mut() = value;
        self.changed.raise("Value");
    }
    fn set_number(&self, value: f64) {
        self.number.set(value);
        self.changed.raise("Number");
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

ferro_model!(Item, |builder| builder
    .notify_property_changed()
    .property::<Value<String>>("Value", Item::value, Item::set_value)
    .property::<Value<f64>>("Number", |item| item.number.get(), Item::set_number)
    .read_only::<Value<String>>("ReadOnly", Item::value)
    .method("Run", |_, _| {}));

fn setup() -> (Ref<StackPanel>, Ref<TextBlock>) {
    ferroui_markup_xaml::register_types();
    Item::register();
    let root = StackPanel::new();
    let target = TextBlock::new();
    root.children().add(target.clone());
    (root, target)
}

fn bind(root: &StackPanel, target: &TextBlock, path: &str, mode: BindingMode) -> Rc<WinUIBindings> {
    let extension = WinUIBindExtension::with_path(path);
    extension.set_mode(mode);
    let binding = extension.create_binding(root, None).unwrap();
    target.bind_binding(TextBlock::text_property(), &binding);
    WinUIBindings::for_root(root)
}

#[test]
fn one_time_waits_for_initialize_and_explicit_update_without_subscribing() {
    let (root, target) = setup();
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneTime);
    assert!(!group.is_initialized());
    assert_eq!(target.text(), None);
    let item = Item::new("after construction");
    root.set_tag(Some(item.clone()));
    group.initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("after construction"));
    assert_eq!(item.changed.handler_count(), 0);
    item.set_value("later".into());
    root.set_data_context(Some(Item::new("unrelated data context")));
    group.initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("after construction"));
    group.update().unwrap();
    assert_eq!(target.text().as_deref(), Some("later"));
    assert_eq!(item.changed.handler_count(), 0);
}

#[test]
fn one_way_tracks_source_replacement_and_stop_preserves_the_target_value() {
    let (root, target) = setup();
    let first = Item::new("first");
    root.set_tag(Some(first.clone()));
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneWay);
    group.initialize().unwrap();
    assert_eq!(first.changed.handler_count(), 1);
    first.set_value("changed".into());
    assert_eq!(target.text().as_deref(), Some("changed"));
    group.stop_tracking();
    assert_eq!(first.changed.handler_count(), 0);
    first.set_value("while stopped".into());
    assert_eq!(target.text().as_deref(), Some("changed"));
    group.initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("while stopped"));
    let second = Item::new("second");
    root.set_tag(Some(second.clone()));
    assert_eq!(target.text().as_deref(), Some("second"));
    assert_eq!(first.changed.handler_count(), 0);
    assert_eq!(second.changed.handler_count(), 1);
}

#[test]
fn two_way_text_defaults_to_lost_focus_and_stop_blocks_writes() {
    let _app = ferroui_controls::testing::UnitTestApplication::start(
        ferroui_controls::testing::TestServices::mock_threading_interface(),
    );
    let (root, _) = setup();
    let target = TextBox::new();
    let item = Item::new("initial");
    root.set_tag(Some(item.clone()));
    let extension = WinUIBindExtension::with_path("Tag.Value");
    extension.set_mode(BindingMode::TwoWay);
    let binding = extension.create_binding(&root, None).unwrap();
    target.bind_binding(TextBox::text_property(), &binding);
    let group = WinUIBindings::for_root(&root);
    group.initialize().unwrap();
    target.set_current_value(TextBox::text_property(), Some("edited".into()));
    assert_eq!(item.value(), "initial");
    target.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(item.value(), "edited");
    group.stop_tracking();
    target.set_current_value(TextBox::text_property(), Some("stopped".into()));
    target.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(item.value(), "edited");
}

#[test]
fn explicit_property_changed_writes_text_without_waiting_for_focus() {
    let (root, _) = setup();
    let item = Item::new("initial");
    root.set_tag(Some(item.clone()));
    let target = TextBox::new();
    let extension = WinUIBindExtension::with_path("Tag.Value");
    extension.set_mode(BindingMode::TwoWay);
    extension.set_update_source_trigger(UpdateSourceTrigger::PropertyChanged);
    let binding = extension.create_binding(&root, None).unwrap();
    target.bind_binding(TextBox::text_property(), &binding);
    target.set_current_value(TextBox::text_property(), Some("before initialize".into()));
    assert_eq!(item.value(), "initial");
    WinUIBindings::for_root(&root).initialize().unwrap();
    target.set_current_value(TextBox::text_property(), Some("edited".into()));
    assert_eq!(item.value(), "edited");
}

#[test]
fn other_two_way_dependency_properties_default_to_property_changed() {
    let (root, _) = setup();
    let item = Item::new("item");
    root.set_tag(Some(item.clone()));
    let target = Slider::new();
    let extension = WinUIBindExtension::with_path("Tag.Number");
    extension.set_mode(BindingMode::TwoWay);
    let binding = extension.create_binding(&root, None).unwrap();
    target.bind_binding(RangeBase::value_property(), &binding);
    WinUIBindings::for_root(&root).initialize().unwrap();
    target.set_current_value(RangeBase::value_property(), 5.0);
    assert_eq!(item.number.get(), 5.0);
}

#[test]
fn templates_keep_instances_separate_and_one_time_refreshes_for_a_recycled_item() {
    let (first, target) = setup();
    let (second, second_target) = setup();
    let item = Item::new("first item");
    first.set_data_context(Some(item.clone()));
    second.set_data_context(Some(Item::new("second item")));
    let extension = WinUIBindExtension::with_path("Value");
    extension.set_scope(WinUIBindingScope::Template);
    extension.set_data_type(Some(ValueType::of::<Item>()));
    for (root, target) in [(&first, &target), (&second, &second_target)] {
        let binding = extension.create_binding(root, None).unwrap();
        target.bind_binding(TextBlock::text_property(), &binding);
    }
    let group = WinUIBindings::for_root(&first);
    group.initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("first item"));
    assert_eq!(second_target.text(), None);
    WinUIBindings::for_root(&second).initialize().unwrap();
    assert_eq!(second_target.text().as_deref(), Some("second item"));
    item.set_value("same item changed".into());
    assert_eq!(target.text().as_deref(), Some("first item"));
    first.set_data_context(Some(Item::new("recycled")));
    assert_eq!(target.text().as_deref(), Some("recycled"));
    assert_eq!(second_target.text().as_deref(), Some("second item"));
    first.set_data_context(Some(Rc::new(42_i32)));
    assert!(group.diagnostics()[0].message.contains("declared type"));
    first.set_data_context(Some(Item::new("valid again")));
    assert_eq!(target.text().as_deref(), Some("valid again"));
    assert!(group.diagnostics().is_empty());
}

#[test]
fn root_names_resolve_after_construction_and_template_item_members_take_precedence() {
    let (root, target) = setup();
    let scope = NameScopeRef::new(NameScope::new());
    NameScope::set_name_scope(&root, Some(scope.clone()));
    let extension = WinUIBindExtension::with_path("Later.Text");
    let binding = extension.create_binding(&root, Some(&scope)).unwrap();
    target.bind_binding(TextBlock::text_property(), &binding);
    let later = TextBlock::new();
    later.set_text(Some("named element"));
    root.children().add(later.clone());
    scope.register("Later", later.upcast());
    scope.complete();
    WinUIBindings::for_root(&root).initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("named element"));

    let (root, target) = setup();
    let scope = NameScopeRef::new(NameScope::new());
    NameScope::set_name_scope(&root, Some(scope.clone()));
    scope.register("Value", TextBlock::new().upcast());
    scope.complete();
    root.set_data_context(Some(Item::new("item member")));
    let extension = WinUIBindExtension::with_path("Value");
    extension.set_scope(WinUIBindingScope::Template);
    extension.set_data_type(Some(ValueType::of::<Item>()));
    let binding = extension.create_binding(&root, Some(&scope)).unwrap();
    target.bind_binding(TextBlock::text_property(), &binding);
    WinUIBindings::for_root(&root).initialize().unwrap();
    assert_eq!(target.text().as_deref(), Some("item member"));
}

#[test]
fn retained_controller_does_not_retain_root_target_or_model_subscriptions() {
    let (root, target) = setup();
    let weak_root = root.downgrade();
    let weak_target = target.downgrade();
    let item = Item::new("value");
    root.set_tag(Some(item.clone()));
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneWay);
    group.initialize().unwrap();
    drop(target);
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_target.upgrade().is_none());
    assert_eq!(item.changed.handler_count(), 0);
    assert_eq!(group.binding_count(), 0);
}

#[test]
fn root_release_stops_a_separately_retained_target_and_controller() {
    let (root, target) = setup();
    let item = Item::new("value");
    root.set_tag(Some(item.clone()));
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneWay);
    group.initialize().unwrap();
    assert_eq!(item.changed.handler_count(), 1);
    root.children().clear();
    drop(root);
    assert!(!group.is_initialized());
    assert_eq!(item.changed.handler_count(), 0);
    item.set_value("orphaned".into());
    assert_eq!(target.text().as_deref(), Some("value"));
}

#[test]
fn unload_stopping_is_opt_in_and_does_not_initialize_at_loaded() {
    let (root, target) = setup();
    let item = Item::new("value");
    root.set_tag(Some(item.clone()));
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneWay);
    assert!(group.set_stop_on_unload(true));
    root.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
    assert_eq!(target.text(), None);
    group.initialize().unwrap();
    root.raise_event(&RoutedEventArgs::with_event(Control::unloaded_event()));
    assert!(!group.is_initialized());
    assert_eq!(item.changed.handler_count(), 0);
    assert_eq!(target.text().as_deref(), Some("value"));
}

#[test]
fn source_errors_keep_original_locations_and_read_only_two_way_fails_before_edit() {
    let (root, target) = setup();
    root.set_tag(Some(Item::new("value")));
    let extension = WinUIBindExtension::with_path("Tag.ReadOnly");
    extension.set_mode(BindingMode::TwoWay);
    extension.set_source_line(15);
    extension.set_source_column(8);
    let binding = extension.create_binding(&root, None).unwrap();
    target.bind_binding(TextBlock::text_property(), &binding);
    let error = WinUIBindings::for_root(&root).initialize().unwrap_err();
    assert!(error.to_string().contains("15:8: x:Bind 'Tag.ReadOnly'"));
    assert!(error.to_string().contains("write"));
}

#[test]
fn unsupported_paths_modes_and_source_methods_are_errors() {
    let (root, target) = setup();
    for path in [
        "DoWork()",
        "Items[0]",
        "((Model)Tag).Value",
        "Tag?.Value",
        "A..B",
    ] {
        assert!(
            WinUIBindExtension::with_path(path)
                .create_binding(&root, None)
                .is_err(),
            "{path}"
        );
    }
    let extension = WinUIBindExtension::with_path("Tag.Value");
    extension.set_mode(BindingMode::Default);
    assert!(extension.create_binding(&root, None).is_err());
    root.set_tag(Some(Item::new("value")));
    let group = bind(&root, &target, "Tag.Run", BindingMode::OneTime);
    assert!(group
        .initialize()
        .unwrap_err()
        .to_string()
        .contains("methods are unsupported"));
}

#[test]
fn arbitrary_getter_panics_resume_and_the_update_guard_recovers() {
    let (root, target) = setup();
    let item = Item::new("panic");
    root.set_tag(Some(item.clone() as BoxedValue));
    let group = bind(&root, &target, "Tag.Value", BindingMode::OneTime);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| group.initialize()));
    assert!(panic.is_err());
    item.set_value("recovered".into());
    group.update().unwrap();
    assert_eq!(target.text().as_deref(), Some("recovered"));
}
