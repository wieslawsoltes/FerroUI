//! Tests of the reference semantics of the object model: collections and
//! reference objects are shared handles with identity equality, so that
//! untyped code (markup) can hold them in values, and the contracts untyped
//! code asks for are reachable from class handles.

use super::binding_model_tests::PersonVm;
use crate::animation::{Animation, IAnimation, KeyFrame};
use crate::collections::{FerroDictionary, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::controls::{
    Classes, INameScope, IResourceDictionary, IResourceNode, IResourceProvider, IThemeVariantProvider, NameScope,
    ResourceDictionary, ResourceHostRef,
};
use crate::data::core::plugins::{InpcPropertyAccessor, TaskStreamPlugin, TaskValue, TaskValueSource};
use crate::data::core::{IPropertyInfo, ValueType, ValueTypes, WeakValue};
use crate::data::model::ModelTypes;
use crate::data::{
    BindingBase, BindingMode, BindingPriority, BindingValueType, CompiledBinding, CompiledBindingPath,
    CompiledBindingPathBuilder, MultiBinding, ReflectionBinding, RelativeSource, RelativeSourceMode,
    TemplateBinding, TreeType, UpdateSourceTrigger,
};
use crate::input::{InputElement, KeyBinding};
use crate::logical_tree::LogicalTreeAttachmentEventArgs;
use crate::media::transformation::TransformOperations;
use crate::metadata::{from_markup_value, into_markup_value, IAddChild};
use crate::reactive::IObserver;
use crate::styling::{
    IStyle, Selectors, Setter, SetterBase, Style, StyleHostRef, StyleQueries, StyleQueryComparisonOperator, Styles,
    ThemeVariant,
};
use crate::utilities::CancelEventArgs;
use crate::{
    BoxedValue, FerroObject, FerroProperty, ISupportInitialize, Ref, StyledElement,
};
use std::cell::RefCell;
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

// --- collections are shared handles -----------------------------------------

#[test]
fn ferro_list_is_a_shared_handle_with_identity_equality() {
    let list = FerroList::<i32>::new();
    let other = list.clone();
    other.add(1);
    assert_eq!(list.to_vec(), vec![1]);
    assert!(list == other);
    assert!(list != FerroList::<i32>::from_items([1]));

    let value = boxed(list.clone());
    let unboxed = value.downcast_ref::<FerroList<i32>>().expect("the list");
    unboxed.add(2);
    assert_eq!(list.to_vec(), vec![1, 2]);
}

#[test]
fn ferro_dictionary_is_a_shared_handle_with_identity_equality() {
    let dictionary = FerroDictionary::<String, i32>::new();
    let other = dictionary.clone();
    other.add("a".to_string(), 1);
    assert_eq!(dictionary.try_get_value(&"a".to_string()), Some(1));
    assert!(dictionary == other);
    assert!(dictionary != FerroDictionary::<String, i32>::new());
}

#[test]
fn classes_of_an_element_are_returned_by_handle() {
    let element = StyledElement::new();
    let classes = element.classes();
    classes.add("foo");
    assert!(element.classes().contains("foo"));
    assert!(classes == element.classes());
    assert!(classes != Classes::parse("foo"));

    let value = boxed(element.classes());
    value.downcast_ref::<Classes>().expect("the classes").add("bar");
    assert_eq!(*element.classes().snapshot(), vec!["foo".to_string(), "bar".to_string()]);
}

#[test]
fn animation_children_and_key_frame_setters_are_returned_by_handle() {
    let animation = Animation::new();
    let children = animation.children();
    let key_frame = KeyFrame::new();
    children.add(key_frame.clone());
    assert_eq!(animation.children().count(), 1);
    assert!(children == animation.children());

    let setters = key_frame.setters();
    setters.add(Setter::empty());
    assert_eq!(key_frame.setters().count(), 1);
    assert!(setters == key_frame.setters());
}

#[test]
fn style_collections_are_returned_by_handle() {
    let style = Style::new();

    let setters = style.setters();
    let setter: Rc<dyn SetterBase> = Setter::empty();
    setters.add(setter.clone());
    assert_eq!(style.setters().count(), 1);
    assert!(setters == style.setters());
    assert!(style.setters().contains(&setter));

    let children = style.children();
    children.add(Style::new());
    assert_eq!(style.children().count(), 1);
    assert!(children == style.children());

    let animations = style.animations();
    animations.add(Animation::new().into());
    assert_eq!(style.animations().count(), 1);
    assert!(animations == style.animations());
}

#[test]
fn merged_dictionaries_are_returned_by_handle() {
    let dictionary = ResourceDictionary::new();
    let merged = dictionary.merged_dictionaries();
    let child = ResourceDictionary::new();
    merged.add(child.clone().into());
    assert_eq!(dictionary.merged_dictionaries().count(), 1);
    assert!(merged == dictionary.merged_dictionaries());
}

#[test]
fn theme_dictionaries_are_a_dictionary_whose_entries_get_the_owner() {
    let owner = StyledElement::new();
    let dictionary = ResourceDictionary::with_owner(&owner);
    let themes = dictionary.theme_dictionaries();
    assert!(themes == dictionary.theme_dictionaries());

    let dark = ResourceDictionary::new();
    themes.add(ThemeVariant::dark(), dark.clone().into());
    assert!(dark.owner() == Some(ResourceHostRef::from(&owner)));
    assert!(dictionary.theme_dictionary(&ThemeVariant::dark()).is_some());

    // Replacing an entry moves the owner from the old provider to the new.
    let other = ResourceDictionary::new();
    themes.set(ThemeVariant::dark(), other.clone().into());
    assert!(dark.owner().is_none());
    assert!(other.owner().is_some());

    assert!(themes.remove(&ThemeVariant::dark()));
    assert!(other.owner().is_none());
    assert!(dictionary.theme_dictionary(&ThemeVariant::dark()).is_none());
}

#[test]
fn key_bindings_and_gesture_recognizers_are_returned_by_handle() {
    let element = InputElement::new();
    let key_bindings = element.key_bindings();
    key_bindings.add(KeyBinding::new());
    assert_eq!(element.key_bindings().count(), 1);
    assert!(key_bindings == element.key_bindings());

    let recognizers = element.gesture_recognizers();
    assert!(recognizers == element.gesture_recognizers());
    assert_eq!(recognizers.count(), 0);
}

// --- reference objects compare by identity ------------------------------------

#[test]
fn reference_objects_compare_by_identity() {
    let setter = Setter::empty();
    assert!(setter == setter.clone());
    assert!(setter != Setter::empty());

    let selector = Selectors::of_type::<StyledElement>();
    assert!(selector == selector.clone());
    assert!(selector != Selectors::of_type::<StyledElement>());

    let query = StyleQueries::width(None, StyleQueryComparisonOperator::Equals, 1.0);
    assert!(query == query.clone());
    assert!(query != StyleQueries::width(None, StyleQueryComparisonOperator::Equals, 1.0));

    let operations = TransformOperations::parse("scale(2)").unwrap();
    assert!(operations == operations.clone());
    assert!(operations != TransformOperations::parse("scale(2)").unwrap());

    let path = CompiledBindingPathBuilder::new().build();
    assert!(path == path.clone());
    assert!(path != CompiledBindingPathBuilder::new().build());
    assert!(CompiledBindingPath::new() != CompiledBindingPath::new());
}

#[test]
fn interface_handles_of_the_same_object_are_equal_whichever_adapter_made_them() {
    let style = Style::new();
    let a: Rc<dyn IStyle> = style.clone().into();
    let b: Rc<dyn IStyle> = (&style).into();
    assert!(*a == *b);
    let other: Rc<dyn IStyle> = Style::new().into();
    assert!(*a != *other);

    let dictionary = ResourceDictionary::new();
    let a: Rc<dyn IResourceProvider> = dictionary.clone().into();
    let b: Rc<dyn IResourceProvider> = dictionary.clone().into();
    assert!(*a == *b);
    let d: Rc<dyn IResourceDictionary> = dictionary.clone().into();
    let t: Rc<dyn IThemeVariantProvider> = dictionary.clone().into();
    let d2: Rc<dyn IResourceDictionary> = dictionary.clone().into();
    let t2: Rc<dyn IThemeVariantProvider> = dictionary.clone().into();
    assert!(*d == *d2);
    assert!(*t == *t2);
    let node: Rc<dyn IResourceNode> = a.clone();
    let node2: Rc<dyn IResourceNode> = d;
    assert!(*node == *node2);

    let animation = Animation::new();
    let a: Rc<dyn IAnimation> = animation.clone().into();
    let b: Rc<dyn IAnimation> = (&animation).into();
    assert!(*a == *b);
    let other: Rc<dyn IAnimation> = Animation::new().into();
    assert!(*a != *other);

    let scope: Rc<dyn INameScope> = Rc::new(NameScope::new());
    assert!(*scope == *scope.clone());
    let other: Rc<dyn INameScope> = Rc::new(NameScope::new());
    assert!(*scope != *other);
}

#[test]
fn class_handles_convert_to_their_declared_interfaces_in_untyped_values() {
    let style = Style::new();
    let untyped = into_markup_value(style.clone());
    let as_style = from_markup_value::<Rc<dyn IStyle>>(&untyped).expect("a style");
    let direct: Rc<dyn IStyle> = style.clone().into();
    assert!(*as_style == *direct);
    assert!(from_markup_value::<Rc<dyn IResourceProvider>>(&untyped).is_some());
    assert!(from_markup_value::<Rc<dyn IAddChild<BoxedValue>>>(&untyped).is_some());

    let styles = into_markup_value(Styles::new());
    assert!(from_markup_value::<Rc<dyn IStyle>>(&styles).is_some());
    assert!(from_markup_value::<Rc<dyn IResourceProvider>>(&styles).is_some());

    let dictionary = into_markup_value(ResourceDictionary::new());
    assert!(from_markup_value::<Rc<dyn IResourceProvider>>(&dictionary).is_some());
    assert!(from_markup_value::<Rc<dyn IResourceDictionary>>(&dictionary).is_some());
    assert!(from_markup_value::<Rc<dyn IThemeVariantProvider>>(&dictionary).is_some());

    let animation = into_markup_value(Animation::new());
    assert!(from_markup_value::<Rc<dyn IAnimation>>(&animation).is_some());

    let element = StyledElement::new();
    let untyped = into_markup_value(element.clone());
    assert!(from_markup_value::<ResourceHostRef>(&untyped) == Some(ResourceHostRef::from(&element)));
    assert!(from_markup_value::<StyleHostRef>(&untyped) == Some(StyleHostRef::from(&element)));
    assert!(from_markup_value::<Rc<dyn ISupportInitialize>>(&untyped).is_some());
}

// --- bindings are reference objects -------------------------------------------

fn object_source(object: &Ref<StyledElement>) -> Option<BoxedValue> {
    Some(Rc::new(object.clone().upcast::<FerroObject>()))
}

fn string_context(value: &str) -> Ref<StyledElement> {
    let source = StyledElement::new();
    source.set_data_context(Some(boxed(value.to_string())));
    source
}

fn data_context_string(target: &StyledElement) -> Option<String> {
    target.data_context().and_then(|v| v.downcast_ref::<String>().cloned())
}

#[test]
fn reflection_binding_configured_by_property_equals_the_constructor_form() {
    let source = string_context("foo");

    let by_property = ReflectionBinding::empty();
    by_property.set_path("DataContext".to_string());
    by_property.set_mode(BindingMode::OneWay);
    by_property.set_source(object_source(&source));
    by_property.set_priority(BindingPriority::Style);
    by_property.set_delay(5);
    by_property.set_string_format(Some("{0}".to_string()));
    by_property.set_element_name(None);
    by_property.set_update_source_trigger(UpdateSourceTrigger::LostFocus);

    let by_constructor = ReflectionBinding::new("DataContext")
        .with_mode(BindingMode::OneWay)
        .with_source(object_source(&source))
        .with_priority(BindingPriority::Style)
        .with_delay(5)
        .with_string_format(Some("{0}".to_string()))
        .with_update_source_trigger(UpdateSourceTrigger::LostFocus);

    assert_eq!(by_property.path(), by_constructor.path());
    assert_eq!(by_property.mode(), by_constructor.mode());
    assert!(by_property.source() == by_constructor.source());
    assert_eq!(by_property.priority(), by_constructor.priority());
    assert_eq!(by_property.delay(), by_constructor.delay());
    assert_eq!(by_property.string_format(), by_constructor.string_format());
    assert_eq!(by_property.update_source_trigger(), by_constructor.update_source_trigger());
    assert!(by_property.fallback_value() == by_constructor.fallback_value());
    assert!(by_property.target_null_value() == by_constructor.target_null_value());

    let a = StyledElement::new();
    let b = StyledElement::new();
    a.bind_binding(StyledElement::data_context_property(), &by_property);
    b.bind_binding(StyledElement::data_context_property(), &by_constructor);
    assert_eq!(data_context_string(&a).as_deref(), Some("foo"));
    assert_eq!(data_context_string(&a), data_context_string(&b));
}

#[test]
fn the_defaults_of_a_binding_are_the_ones_of_the_parameterless_constructor() {
    let binding = ReflectionBinding::empty();
    assert_eq!(binding.path(), "");
    assert_eq!(binding.mode(), BindingMode::Default);
    assert_eq!(binding.priority(), BindingPriority::LocalValue);
    assert_eq!(binding.update_source_trigger(), UpdateSourceTrigger::Default);
    assert_eq!(binding.delay(), 0);
    let is_unset = |v: Option<BoxedValue>| v.is_some_and(|v| v.is::<crate::UnsetValueType>());
    assert!(is_unset(binding.source()));
    assert!(is_unset(binding.fallback_value()));
    assert!(is_unset(binding.target_null_value()));
    assert!(binding.relative_source().is_none());
    assert!(binding.converter().is_none());

    let compiled = CompiledBinding::empty();
    assert!(compiled.path().is_none());
    assert!(is_unset(compiled.source()));
    assert_eq!(compiled.mode(), BindingMode::Default);

    let template = TemplateBinding::empty();
    assert!(template.property().is_none());
    assert_eq!(template.mode(), BindingMode::Default);
    let template = TemplateBinding::new(StyledElement::data_context_property().as_property());
    assert!(template.property().is_some());

    let multi = MultiBinding::new();
    assert_eq!(multi.mode(), BindingMode::OneWay);
    assert!(multi.bindings().is_empty());
    assert!(is_unset(multi.fallback_value()));

    let relative = RelativeSource::empty();
    assert_eq!(relative.mode(), RelativeSourceMode::FindAncestor);
    assert_eq!(relative.ancestor_level(), 1);
    assert_eq!(relative.tree(), TreeType::Visual);
    assert!(relative.ancestor_type().is_none());
    relative.set_mode(RelativeSourceMode::SelfMode);
    relative.set_tree(TreeType::Logical);
    assert_eq!(relative.mode(), RelativeSourceMode::SelfMode);
    assert_eq!(relative.tree(), TreeType::Logical);
    assert!(relative == relative.clone());
    assert!(relative != RelativeSource::new(RelativeSourceMode::SelfMode));
}

#[test]
fn bindings_have_identity_equality_and_round_trip_through_untyped_values() {
    let binding = ReflectionBinding::new("Foo");
    assert!(binding == binding.clone());
    assert!(binding != ReflectionBinding::new("Foo"));

    let handle: Rc<dyn BindingBase> = binding.clone();
    let again: Rc<dyn BindingBase> = binding.clone();
    assert!(*handle == *again);
    // A handle of a handle still has the identity of the binding.
    let nested: Rc<dyn BindingBase> = Rc::new(binding.clone());
    assert!(*handle == *nested);
    let other: Rc<dyn BindingBase> = ReflectionBinding::new("Foo");
    assert!(*handle != *other);

    let value = boxed(handle.clone());
    let unboxed = value.downcast_ref::<Rc<dyn BindingBase>>().expect("the binding");
    assert!(**unboxed == *handle);

    let value = boxed(binding.clone());
    let unboxed = value.downcast_ref::<Rc<ReflectionBinding>>().expect("the binding");
    unboxed.set_path("Bar".to_string());
    assert_eq!(binding.path(), "Bar");
}

#[test]
fn a_binding_changed_after_it_was_applied_affects_later_instances_only() {
    let first_source = string_context("first");
    let second_source = string_context("second");

    let binding = ReflectionBinding::new("DataContext").with_source(object_source(&first_source));
    let first = StyledElement::new();
    first.bind_binding(StyledElement::data_context_property(), &binding);
    assert_eq!(data_context_string(&first).as_deref(), Some("first"));

    binding.set_source(object_source(&second_source));
    assert_eq!(data_context_string(&first).as_deref(), Some("first"));

    let second = StyledElement::new();
    second.bind_binding(StyledElement::data_context_property(), &binding);
    assert_eq!(data_context_string(&second).as_deref(), Some("second"));
    assert_eq!(data_context_string(&first).as_deref(), Some("first"));
}

#[test]
fn child_bindings_are_added_to_the_collection_of_a_multi_binding() {
    let multi = MultiBinding::new();
    let bindings = multi.bindings();
    let child: Rc<dyn BindingBase> = ReflectionBinding::new("Name");
    bindings.add(child.clone());
    assert_eq!(multi.bindings().count(), 1);
    assert!(multi.bindings().contains(&child));
    assert!(bindings == multi.bindings());

    let by_constructor = MultiBinding::new().with_bindings(vec![child.clone()]);
    assert_eq!(by_constructor.bindings().to_vec().len(), multi.bindings().to_vec().len());
    assert!(*by_constructor.bindings().get(0) == *multi.bindings().get(0));

    let replacement = FerroList::new();
    multi.set_bindings(replacement.clone());
    assert!(multi.bindings() == replacement);
}

// --- contracts ---------------------------------------------------------------

#[test]
fn support_initialize_defers_initialization_of_a_styled_element() {
    let element = StyledElement::new();
    let init: Rc<dyn ISupportInitialize> = element.clone().into();
    let again: Rc<dyn ISupportInitialize> = (&element).into();
    assert!(*init == *again);

    init.begin_init();
    init.begin_init();
    init.end_init();
    init.end_init();
}

#[test]
#[should_panic(expected = "BeginInit was not called.")]
fn support_initialize_end_without_begin_panics() {
    let init: Rc<dyn ISupportInitialize> = StyledElement::new().into();
    init.end_init();
}

#[test]
fn a_style_adds_setters_and_styles_as_untyped_children() {
    let style = Style::new();
    let add: Rc<dyn IAddChild<BoxedValue>> = style.clone().upcast::<crate::styling::StyleBase>().into();

    let setter = Setter::empty();
    add.add_child(boxed(setter.clone()));
    assert_eq!(style.setters().count(), 1);
    let expected: Rc<dyn SetterBase> = setter;
    assert!(*style.setters().get(0) == *expected);

    let child = Style::new();
    add.add_child(boxed(child.clone()));
    assert_eq!(style.children().count(), 1);
    let expected: Rc<dyn IStyle> = child.into();
    assert!(*style.children().get(0) == *expected);
}

#[test]
#[should_panic(expected = "to a style.")]
fn a_style_rejects_other_untyped_children() {
    Style::new().add_child(&boxed(1_i32));
}

// --- event args --------------------------------------------------------------

#[test]
fn a_copy_of_cancel_event_args_shares_the_cancel_flag() {
    let args = CancelEventArgs::new();
    let value = boxed(args.clone());
    value.downcast_ref::<CancelEventArgs>().expect("the args").set_cancel(true);
    assert!(args.cancel());
    assert!(args == args.clone());
    assert!(args != CancelEventArgs::new());
}

#[test]
fn attachment_event_args_can_be_held_in_untyped_values() {
    let root = StyledElement::new();
    let args = LogicalTreeAttachmentEventArgs::new(root.clone(), root.clone(), None);
    let value = boxed(args.clone());
    assert!(value.downcast_ref::<LogicalTreeAttachmentEventArgs>() == Some(&args));
    let other = LogicalTreeAttachmentEventArgs::new(StyledElement::new(), root, None);
    assert!(args != other);
}

#[test]
fn collection_changes_can_be_shared_with_untyped_handlers() {
    let list = FerroList::<i32>::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, i32>| {
        sink.borrow_mut().push(boxed(e.share()));
    }));
    list.add(7);

    let received = received.borrow();
    let shared = received[0]
        .downcast_ref::<crate::collections::SharedNotifyCollectionChangedEventArgs<i32>>()
        .expect("the shared args");
    assert_eq!(shared.action(), NotifyCollectionChangedAction::Add);
    assert_eq!(shared.new_items(), &[7]);
    assert!(shared.old_items().is_empty());
    assert_eq!(shared.new_starting_index(), 0);
    assert_eq!(shared.old_starting_index(), -1);
    assert_eq!(shared.as_args().new_items, &[7]);
    assert!(*shared == shared.clone());
}

#[test]
fn styles_collection_changes_can_be_shared_with_untyped_handlers() {
    let styles = Styles::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let _subscription = styles.collection_changed(move |e| sink.borrow_mut().push(boxed(e.share())));
    styles.add(Style::new());
    assert_eq!(received.borrow().len(), 1);
}

// --- binding value type ------------------------------------------------------

#[test]
fn binding_value_type_has_the_flag_members_and_values_of_the_reference() {
    assert_eq!(BindingValueType::UNSET_VALUE.bits(), 0);
    assert_eq!(BindingValueType::DO_NOTHING.bits(), 1);
    assert_eq!(BindingValueType::VALUE.bits(), 2 | 0x0100);
    assert_eq!(BindingValueType::BINDING_ERROR.bits(), 3 | 0x0200);
    assert_eq!(BindingValueType::DATA_VALIDATION_ERROR.bits(), 4 | 0x0200);
    assert_eq!(BindingValueType::BINDING_ERROR_WITH_FALLBACK.bits(), 3 | 0x0200 | 0x0100);
    assert_eq!(BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK.bits(), 4 | 0x0200 | 0x0100);
    assert_eq!(BindingValueType::TYPE_MASK.bits(), 0x00ff);
    assert_eq!(BindingValueType::HAS_VALUE.bits(), 0x0100);
    assert_eq!(BindingValueType::HAS_ERROR.bits(), 0x0200);

    assert!(BindingValueType::VALUE.contains(BindingValueType::HAS_VALUE));
    assert!(!BindingValueType::VALUE.contains(BindingValueType::HAS_ERROR));
    assert!(BindingValueType::BINDING_ERROR.contains(BindingValueType::HAS_ERROR));
    assert!(BindingValueType::BINDING_ERROR_WITH_FALLBACK
        .contains(BindingValueType::HAS_ERROR | BindingValueType::HAS_VALUE));
    assert_eq!(
        BindingValueType::BINDING_ERROR | BindingValueType::HAS_VALUE,
        BindingValueType::BINDING_ERROR_WITH_FALLBACK
    );
    assert_eq!((BindingValueType::DATA_VALIDATION_ERROR & BindingValueType::TYPE_MASK).bits(), 4);
}

// --- property infos, accessors and sources -----------------------------------

#[test]
fn a_registered_property_is_a_property_info() {
    let property: &'static FerroProperty = StyledElement::data_context_property().as_property();
    let info: &dyn IPropertyInfo = property;
    assert_eq!(info.name(), "DataContext");
    assert!(info.can_get());
    assert!(info.can_set());
    assert!(info.property_type() == ValueType::new(property.property_type(), property.property_type_name()));
    assert!(*info == *(property as &dyn IPropertyInfo));

    let element = StyledElement::new();
    let target = boxed(element.clone());
    assert!(info.get(&*target).is_none());
    info.set(&*target, Some(&boxed("foo".to_string()))).expect("the value is set");
    assert_eq!(data_context_string(&element).as_deref(), Some("foo"));
    let read = info.get(&*target).expect("a value");
    assert_eq!(read.downcast_ref::<String>().map(String::as_str), Some("foo"));

    // A target that is not an object is an error, not a panic.
    assert!(info.try_get(&*boxed(1_i32)).is_err());
    assert!(info.set(&*boxed(1_i32), None).is_err());

    let read_only: &dyn IPropertyInfo = StyledElement::parent_property().as_property();
    assert!(!read_only.can_set());
}

#[test]
fn inpc_accessor_looks_up_the_notifier_at_run_time() {
    let vm = PersonVm::new("Ann", 30);
    let source: BoxedValue = vm.clone();
    let model = ModelTypes::find(&*source).expect("the model metadata");
    let property = model.find_property("Name").expect("the property");

    let accessor = InpcPropertyAccessor::with_runtime_notifier(WeakValue::new(&source), property);
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    accessor.subscribe(Rc::new(move |value: Option<BoxedValue>| {
        sink.borrow_mut().push(value.and_then(|v| v.downcast_ref::<String>().cloned()));
    }));
    vm.set_name("Bob".to_string());

    assert_eq!(*received.borrow(), vec![Some("Ann".to_string()), Some("Bob".to_string())]);
    accessor.unsubscribe();
}

#[test]
fn task_values_can_be_continued_and_observed() {
    let source = TaskValueSource::new();
    let task = source.task();
    let continued = Rc::new(RefCell::new(false));
    let flag = continued.clone();
    task.continue_with(move || *flag.borrow_mut() = true);
    assert!(!*continued.borrow());
    source.set_result(5_i32);
    assert!(*continued.borrow());

    struct Collect(Rc<RefCell<Vec<Option<i32>>>>);
    impl IObserver<Option<BoxedValue>> for Collect {
        fn on_next(&self, value: Option<BoxedValue>) {
            self.0.borrow_mut().push(value.and_then(|v| v.downcast_ref::<i32>().copied()));
        }
    }
    let received = Rc::new(RefCell::new(Vec::new()));
    TaskStreamPlugin::observe(&task).subscribe(Rc::new(Collect(received.clone())));
    assert_eq!(*received.borrow(), vec![Some(5)]);
    assert!(TaskValue::from_result(1_i32) != task);
}

#[test]
fn weak_values_compare_by_the_source_they_refer_to() {
    let vm: BoxedValue = PersonVm::new("Ann", 30);
    assert!(WeakValue::new(&vm) == WeakValue::new(&vm));
    let other: BoxedValue = PersonVm::new("Ann", 30);
    assert!(WeakValue::new(&vm) != WeakValue::new(&other));

    let element = boxed(StyledElement::new());
    assert!(WeakValue::new(&element) == WeakValue::new(&element));
    assert!(WeakValue::new(&element) != WeakValue::new(&vm));

    assert!(WeakValue::new(&boxed(1_i32)) == WeakValue::new(&boxed(1_i32)));
    assert!(WeakValue::new(&boxed(1_i32)) != WeakValue::new(&boxed(2_i32)));
    let _ = ValueTypes::normalize(boxed(1_i32));
}

// --- disposables and binding expressions -------------------------------------

#[test]
fn disposables_and_binding_expressions_can_be_held_in_untyped_values() {
    use crate::data::BindingExpressionBase;
    use crate::reactive::{Disposable, IDisposable};

    let disposable = Disposable::create(|| {});
    let value = boxed(disposable.clone());
    let unboxed = value.downcast_ref::<Rc<dyn IDisposable>>().expect("the disposable");
    assert!(**unboxed == *disposable);
    assert!(*disposable != *Disposable::create(|| {}));

    let source = string_context("foo");
    let binding = ReflectionBinding::new("DataContext").with_source(object_source(&source));
    let target = StyledElement::new();
    let expression = target.bind_binding(StyledElement::data_context_property(), &binding);
    let value = boxed(expression.clone());
    let unboxed = value.downcast_ref::<Rc<dyn BindingExpressionBase>>().expect("the expression");
    assert!(**unboxed == *expression);

    let other = StyledElement::new().bind_binding(StyledElement::data_context_property(), &binding);
    assert!(*expression != *other);
}

// --- compiled binding path builder -------------------------------------------

fn person_notifier(value: &dyn crate::AnyValue) -> Option<&dyn crate::data::model::INotifyPropertyChanged> {
    value.downcast_ref::<PersonVm>().map(|vm| vm as &dyn crate::data::model::INotifyPropertyChanged)
}

fn person(owner: &BoxedValue) -> &PersonVm {
    owner.downcast_ref::<PersonVm>().expect("a person")
}

#[test]
fn path_builder_is_a_reference_object_usable_one_call_at_a_time() {
    let builder = CompiledBindingPathBuilder::new();
    let same = builder.clone();
    assert!(builder == same);
    assert!(builder != CompiledBindingPathBuilder::new());

    // Elements added through any handle, without chaining.
    builder.ferro_property(StyledElement::data_context_property().as_property());
    same.not();
    assert_eq!(builder.build().len(), 2);
    assert_eq!(builder.build().to_string(), "DataContext!");

    // The chained form is the same thing.
    let chained = CompiledBindingPathBuilder::new()
        .ferro_property(StyledElement::data_context_property().as_property())
        .not()
        .build();
    assert_eq!(chained.to_string(), builder.build().to_string());

    let value = boxed(builder.clone());
    value.downcast_ref::<CompiledBindingPathBuilder>().expect("the builder").self_();
    assert_eq!(builder.build().len(), 3);
}

#[test]
fn untyped_property_element_reads_writes_and_tracks_the_owner() {
    use super::binding_model_tests::Target;

    let path = CompiledBindingPathBuilder::new()
        .property_untyped(
            "Name",
            ValueType::of::<String>(),
            Rc::new(|owner| Ok(Some(boxed(person(owner).name())))),
            Some(Rc::new(|owner, value| {
                let name = value.and_then(|v| v.downcast_ref::<String>().cloned()).unwrap_or_default();
                person(owner).set_name(name);
                Ok(())
            })),
            Some(person_notifier),
        )
        .build();
    assert_eq!(path.to_string(), "Name");

    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    target.bind_binding(Target::text_property(), &CompiledBinding::new(path).with_mode(BindingMode::TwoWay));
    assert_eq!(target.get_value(Target::text_property()), "Ann");

    vm.set_name("Bob".to_string());
    assert_eq!(target.get_value(Target::text_property()), "Bob");

    target.set_value(Target::text_property(), "Zed".to_string());
    assert_eq!(vm.name(), "Zed");
}

#[test]
fn untyped_property_element_without_notifier_reads_the_owner_and_reports_getter_errors() {
    use super::binding_model_tests::Target;

    let path = CompiledBindingPathBuilder::new()
        .property_untyped(
            "Name",
            ValueType::of::<String>(),
            Rc::new(|owner| Ok(Some(boxed(person(owner).name())))),
            None,
            None,
        )
        .build();
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    target.bind_binding(Target::text_property(), &CompiledBinding::new(path));
    assert_eq!(target.get_value(Target::text_property()), "Ann");

    let failing = CompiledBindingPathBuilder::new()
        .property_untyped(
            "Name",
            ValueType::of::<String>(),
            Rc::new(|_| Err(crate::data::BindingError::message("boom"))),
            None,
            None,
        )
        .build();
    let target = Target::new();
    target.set_data_context(Some(vm));
    target.bind_binding(Target::text_property(), &CompiledBinding::new(failing));
    assert_eq!(target.get_value(Target::text_property()), "default");
}

#[test]
fn untyped_command_element_produces_a_command_over_the_owner() {
    use super::binding_model_tests::Target;

    let path = CompiledBindingPathBuilder::new()
        .command_untyped(
            "Save",
            Rc::new(|owner, _| person(owner).save()),
            Some(Rc::new(|owner, _| person(owner).can_save())),
            &["Name"],
            Some(person_notifier),
        )
        .build();
    assert_eq!(path.to_string(), "Save()");

    let vm = PersonVm::new("Ann", 30);
    let can_save = vm.can_save();
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    target.bind_binding(Target::command_property(), &CompiledBinding::new(path));

    let command = target.get_value(Target::command_property()).expect("a command");
    assert_eq!(command.can_execute(None), can_save);
    vm.set_name("Bob".to_string());
    command.execute(None);
    command.execute(None);
    assert_eq!(command.can_execute(None), vm.can_save());
    drop(vm);
}

#[test]
fn untyped_method_element_produces_the_delegate_made_for_the_owner() {
    use super::binding_model_tests::Target;
    use crate::metadata::MarkupDelegate;

    let path = CompiledBindingPathBuilder::new()
        .method_untyped(
            "Greet",
            Rc::new(|owner| {
                let owner = owner.clone();
                boxed(MarkupDelegate::new(move |arguments| {
                    let greeting = arguments[0].as_ref().and_then(|v| v.downcast_ref::<String>().cloned());
                    Some(boxed(format!("{} {}", greeting.unwrap_or_default(), person(&owner).name())))
                }))
            }),
            false,
        )
        .build();
    assert_eq!(path.to_string(), "Greet()");

    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm));
    target.bind_binding(Target::tag_property(), &CompiledBinding::new(path));

    let delegate = target.get_value(Target::tag_property()).expect("a delegate");
    let delegate = delegate.downcast_ref::<MarkupDelegate>().expect("a delegate");
    let result = delegate.invoke(&[Some(boxed("Hello".to_string()))]).expect("a result");
    assert_eq!(result.downcast_ref::<String>().map(String::as_str), Some("Hello Ann"));
}

#[test]
fn property_infos_are_read_through_the_boxed_forms_by_default() {
    let vm = PersonVm::new("Ann", 30);
    let source: BoxedValue = vm.clone();
    let model = ModelTypes::find(&*source).expect("the model metadata");
    let property = model.find_property("Name").expect("the property");

    let read = property.try_get_boxed(&source).expect("readable").expect("a value");
    assert_eq!(read.downcast_ref::<String>().map(String::as_str), Some("Ann"));
    property.set_boxed(&source, Some(&boxed("Bob".to_string()))).expect("writable");
    assert_eq!(vm.name(), "Bob");
    let read = property.get_boxed(&source).expect("a value");
    assert_eq!(read.downcast_ref::<String>().map(String::as_str), Some("Bob"));
}

// --- name scopes -------------------------------------------------------------

#[test]
fn try_register_reports_duplicate_names_and_completed_scopes() {
    use crate::controls::NameScopeError;

    let scope = NameScope::new();
    let first = StyledElement::new().upcast::<FerroObject>();
    let second = StyledElement::new().upcast::<FerroObject>();

    assert_eq!(scope.try_register("foo", first.clone()), Ok(()));
    // Registering the same element again is not an error.
    assert_eq!(scope.try_register("foo", first.clone()), Ok(()));
    let duplicate = scope.try_register("foo", second.clone());
    assert_eq!(duplicate, Err(NameScopeError::DuplicateName("foo".to_string())));
    assert_eq!(duplicate.unwrap_err().to_string(), "Control with the name 'foo' already registered.");
    assert!(scope.find("foo") == Some(first));

    scope.complete();
    let completed = scope.try_register("bar", second);
    assert_eq!(completed, Err(NameScopeError::Completed));
    assert_eq!(
        completed.unwrap_err().to_string(),
        "NameScope is completed, no further registrations are allowed"
    );
}

#[test]
#[should_panic(expected = "Control with the name 'foo' already registered.")]
fn register_panics_for_a_duplicate_name() {
    let scope = NameScope::new();
    scope.register("foo", StyledElement::new().upcast::<FerroObject>());
    scope.register("foo", StyledElement::new().upcast::<FerroObject>());
}
