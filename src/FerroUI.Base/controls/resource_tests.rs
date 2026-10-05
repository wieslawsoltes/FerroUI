//! Tests for resource dictionaries, resource lookup on styled elements and
//! name scopes.
//!
//! Ported from the resource tests of the reference implementation.

use super::*;
use crate::reactive::{Disposable, IDisposable, ObservableExt};
use crate::styling::test_support::*;
use crate::styling::{Style, Styles, ThemeVariant};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn key(name: &str) -> ResourceKey {
    name.into()
}

fn string(value: Option<ResourceValue>) -> Option<String> {
    let value = value??;
    (*value).downcast_ref::<String>().cloned()
}

#[derive(Default)]
struct TestHost {
    notifications: Cell<u32>,
}

impl IResourceNode for TestHost {
    fn has_resources(&self) -> bool {
        false
    }

    fn try_get_resource(&self, _key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        None
    }
}

impl IResourceHost for TestHost {
    fn resources_changed(&self, _handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn notify_hosted_resources_changed(&self, _e: ResourcesChangedEventArgs) {
        self.notifications.set(self.notifications.get() + 1);
    }
}

fn test_host() -> (Rc<TestHost>, ResourceHostRef) {
    let host = Rc::new(TestHost::default());
    (host.clone(), ResourceHostRef::from(host))
}

// --- ResourceDictionary ------------------------------------------------------

#[test]
fn can_add_null_value() {
    let target = ResourceDictionary::new();
    target.add("null", None);
    assert_eq!(target.count(), 1);
    assert!(matches!(target.try_get_resource(&key("null"), None), Some(None)));
}

#[test]
#[should_panic(expected = "An item with the same key has already been added. Key: foo")]
fn cannot_add_duplicate_key() {
    let target = ResourceDictionary::new();
    target.add_value("foo", 1);
    target.add_value("foo", 2);
}

#[test]
fn try_get_resource_should_find_resource() {
    let target = ResourceDictionary::new();
    target.add_value("foo", "bar".to_string());

    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("bar"));
    assert!(target.try_get_resource(&key("missing"), None).is_none());
    assert!(target.has_resources());
    assert!(target.contains_key(&key("foo")));
}

#[test]
fn resources_can_be_keyed_by_type_and_object() {
    let target = ResourceDictionary::new();
    target.add_value(Class1::TYPE, 1);
    target.add_value(ResourceKey::object(42u8), 2);

    assert!(target.try_get_resource(&ResourceKey::Type(Class1::TYPE), None).is_some());
    assert!(target.try_get_resource(&ResourceKey::Type(Class2::TYPE), None).is_none());
    assert!(target.try_get_resource(&ResourceKey::object(42u8), None).is_some());
    assert!(target.try_get_resource(&ResourceKey::object(42u16), None).is_none());
}

#[test]
fn try_get_resource_should_find_resource_from_merged_dictionary() {
    let target = ResourceDictionary::new();
    let merged = ResourceDictionary::new();
    merged.add_value("foo", "bar".to_string());
    target.add_merged_dictionary(&merged);

    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("bar"));
    assert!(target.has_resources());
}

#[test]
fn try_get_resource_should_find_resource_from_itself_before_merged_dictionary() {
    let target = ResourceDictionary::new();
    target.add_value("foo", "bar".to_string());
    let merged = ResourceDictionary::new();
    merged.add_value("foo", "baz".to_string());
    target.add_merged_dictionary(&merged);

    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("bar"));
}

#[test]
fn try_get_resource_should_find_resource_from_later_merged_dictionary() {
    let target = ResourceDictionary::new();
    let first = ResourceDictionary::new();
    first.add_value("foo", "bar".to_string());
    let second = ResourceDictionary::new();
    second.add_value("foo", "baz".to_string());
    target.add_merged_dictionary(&first);
    target.add_merged_dictionary(&second);

    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("baz"));
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_add_owner() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::new();
    target.add_value("foo", "bar".to_string());

    target.add_owner(&host_ref);

    assert_eq!(host.notifications.get(), 1);
    assert!(target.owner() == Some(host_ref));
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_remove_owner() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::new();
    target.add_value("foo", "bar".to_string());
    target.add_owner(&host_ref);
    host.notifications.set(0);

    target.remove_owner(&host_ref);

    assert_eq!(host.notifications.get(), 1);
    assert!(target.owner().is_none());
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_resource_add() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);

    target.add_value("foo", "bar".to_string());

    assert_eq!(host.notifications.get(), 1);
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_merged_dictionary_add() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);
    let merged = ResourceDictionary::new();
    merged.add_value("foo", "bar".to_string());

    target.add_merged_dictionary(&merged);

    assert_eq!(host.notifications.get(), 1);
}

#[test]
fn notify_hosted_resources_changed_should_not_be_called_on_empty_merged_dictionary_add() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);

    target.add_merged_dictionary(ResourceDictionary::new());

    assert_eq!(host.notifications.get(), 0);
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_merged_dictionary_remove() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);
    let merged = ResourceDictionary::new();
    merged.add_value("foo", "bar".to_string());
    target.add_merged_dictionary(&merged);
    host.notifications.set(0);

    assert!(target.remove_merged_dictionary(&(&merged).into()));

    assert_eq!(host.notifications.get(), 1);
    assert!(merged.owner().is_none());
}

#[test]
fn notify_hosted_resources_changed_should_be_called_on_merged_dictionary_resource_add() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);
    let merged = ResourceDictionary::new();
    target.add_merged_dictionary(&merged);
    host.notifications.set(0);

    merged.add_value("foo", "bar".to_string());

    assert_eq!(host.notifications.get(), 1);
}

#[test]
fn sets_added_merged_dictionary_owner() {
    let (_host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref.clone());
    let merged = ResourceDictionary::new();

    target.add_merged_dictionary(&merged);

    assert!(merged.owner() == Some(host_ref));
}

#[test]
fn add_owner_sets_merged_dictionary_owner() {
    let (_host, host_ref) = test_host();
    let target = ResourceDictionary::new();
    let merged = ResourceDictionary::new();
    target.add_merged_dictionary(&merged);

    target.add_owner(&host_ref);

    assert!(merged.owner() == Some(host_ref));
}

#[test]
fn remove_owner_clears_merged_dictionary_owner() {
    let (_host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref.clone());
    let merged = ResourceDictionary::new();
    target.add_merged_dictionary(&merged);

    target.remove_owner(&host_ref);

    assert!(merged.owner().is_none());
}

#[test]
fn deferred_resource_is_built_once_on_demand() {
    let target = ResourceDictionary::new();
    let built = Rc::new(Cell::new(0));
    let b = built.clone();
    target.add_deferred_fn("foo", move |service_provider| {
        assert!(service_provider.is_none());
        b.set(b.get() + 1);
        Some(Rc::new("bar".to_string()))
    });
    assert!(target.contains_deferred_key(&key("foo")));
    assert_eq!(built.get(), 0);

    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("bar"));
    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("bar"));

    assert_eq!(built.get(), 1);
    assert!(!target.contains_deferred_key(&key("foo")));
}

#[test]
fn not_shared_deferred_resource_is_built_on_every_request() {
    struct Content(Rc<Cell<u32>>);
    impl IDeferredContent for Content {
        fn build(&self, _service_provider: Option<&Rc<dyn crate::metadata::IServiceProvider>>) -> Option<BoxedValue> {
            self.0.set(self.0.get() + 1);
            Some(Rc::new(self.0.get()))
        }
    }

    let target = ResourceDictionary::new();
    let built = Rc::new(Cell::new(0));
    target.add_not_shared_deferred("foo", Rc::new(Content(built.clone())));

    target.try_get_resource(&key("foo"), None);
    target.try_get_resource(&key("foo"), None);

    assert_eq!(built.get(), 2);
}

#[test]
fn theme_dictionaries_are_searched_for_the_theme_variant() {
    let target = ResourceDictionary::new();
    let light = ResourceDictionary::new();
    light.add_value("foo", "light".to_string());
    let dark = ResourceDictionary::new();
    dark.add_value("foo", "dark".to_string());
    let default = ResourceDictionary::new();
    default.add_value("foo", "default".to_string());
    default.add_value("only-default", "x".to_string());
    target.add_theme_dictionary(ThemeVariant::light(), &light);
    target.add_theme_dictionary(ThemeVariant::dark(), &dark);

    assert_eq!(string(target.try_get_resource(&key("foo"), Some(&ThemeVariant::dark()))).as_deref(), Some("dark"));
    assert_eq!(string(target.try_get_resource(&key("foo"), Some(&ThemeVariant::light()))).as_deref(), Some("light"));
    assert!(target.try_get_resource(&key("foo"), None).is_none());

    target.add_theme_dictionary(ThemeVariant::default(), &default);
    assert_eq!(string(target.try_get_resource(&key("foo"), None)).as_deref(), Some("default"));
    assert_eq!(
        string(target.try_get_resource(&key("only-default"), Some(&ThemeVariant::dark()))).as_deref(),
        Some("x")
    );

    // A custom variant falls back to the variant it inherits.
    let custom = ThemeVariant::new("Custom", Some(ThemeVariant::dark()));
    assert_eq!(string(target.try_get_resource(&key("foo"), Some(&custom))).as_deref(), Some("dark"));

    assert!(target.remove_theme_dictionary(&ThemeVariant::dark()));
    assert_eq!(string(target.try_get_resource(&key("foo"), Some(&custom))).as_deref(), Some("default"));
}

#[test]
fn theme_dictionaries_get_the_owner_of_the_dictionary() {
    let (_host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref.clone());
    let dark = ResourceDictionary::new();
    target.add_theme_dictionary(ThemeVariant::dark(), &dark);
    assert!(dark.owner() == Some(host_ref.clone()));

    target.remove_theme_dictionary(&ThemeVariant::dark());
    assert!(dark.owner().is_none());
}

#[test]
fn remove_clear_and_set_raise_notifications() {
    let (host, host_ref) = test_host();
    let target = ResourceDictionary::with_owner(host_ref);
    target.set("foo", Some(Rc::new(1)));
    target.set("foo", Some(Rc::new(2)));
    assert_eq!(target.count(), 1);
    assert!(target.remove(&key("foo")));
    assert!(!target.remove(&key("foo")));
    target.set_items([(key("a"), None), (key("b"), None)]);
    target.clear();
    target.clear();
    assert_eq!(host.notifications.get(), 5);
    assert!(!target.has_resources());
}

// --- StyledElement resources ---------------------------------------------------

#[test]
fn find_resource_should_find_control_resource() {
    let target = Class1::new();
    target.resources().add_value("foo", "foo-value".to_string());

    assert_eq!(string(Some(target.find_resource(&key("foo")))).as_deref(), Some("foo-value"));
}

#[test]
fn find_resource_returns_unset_value_when_not_found() {
    let target = Class1::new();
    let value = target.find_resource(&key("foo")).unwrap();
    assert!((*value).is::<UnsetValueType>());
}

#[test]
fn find_resource_should_find_control_resource_in_parent() {
    let target = Class1::new();
    let parent = Class3::new();
    parent.resources().add_value("foo", "foo-value".to_string());
    set_child(&parent, &target);

    assert_eq!(string(Some(target.find_resource(&key("foo")))).as_deref(), Some("foo-value"));
}

#[test]
fn find_resource_should_find_style_resource() {
    let target = Class1::new();
    let parent = Class3::new();
    let style = Style::new();
    style.resources().add_value("foo", "foo-value".to_string());
    parent.styles().add(&style);
    set_child(&parent, &target);

    assert_eq!(string(Some(target.find_resource(&key("foo")))).as_deref(), Some("foo-value"));
}

#[test]
fn find_resource_should_find_styles_resource() {
    let target = Class1::new();
    let parent = Class3::new();
    let styles = Styles::new();
    styles.resources().add_value("foo", "foo-value".to_string());
    parent.styles().add(&styles);
    set_child(&parent, &target);

    assert_eq!(string(Some(target.find_resource(&key("foo")))).as_deref(), Some("foo-value"));
    assert!(parent.has_resources());
}

#[test]
fn adding_resource_should_raise_resources_changed_on_logical_children() {
    let child = Class1::new();
    let target = Class3::new();
    set_child(&target, &child);

    let raised_on_target = Rc::new(Cell::new(false));
    let raised_on_child = Rc::new(Cell::new(false));
    let (t, c) = (raised_on_target.clone(), raised_on_child.clone());
    target.resources_changed(move |_| t.set(true));
    child.resources_changed(move |_| c.set(true));

    target.resources().add_value("foo", "bar".to_string());

    assert!(raised_on_target.get());
    assert!(raised_on_child.get());
}

#[test]
fn adding_resource_to_styles_should_raise_resources_changed() {
    let target = Class1::new();
    let raised = Rc::new(Cell::new(false));
    let r = raised.clone();
    target.resources_changed(move |_| r.set(true));

    target.styles().resources().add_value("foo", "bar".to_string());

    assert!(raised.get());
}

#[test]
fn adding_resource_to_nested_style_should_raise_resources_changed() {
    let target = Class1::new();
    let style = Style::new();
    target.styles().add(&style);
    let raised = Rc::new(Cell::new(false));
    let r = raised.clone();
    target.resources_changed(move |_| r.set(true));

    style.resources().add_value("foo", "bar".to_string());

    assert!(raised.get());
}

#[test]
fn setting_resources_moves_the_owner() {
    let target = Class1::new();
    let old = target.resources();
    let new = ResourceDictionary::new();
    new.add_value("foo", "bar".to_string());
    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    target.resources_changed(move |_| r.set(r.get() + 1));

    target.set_resources(new.clone());

    assert!(old.owner().is_none());
    assert!(new.owner().and_then(|o| o.as_element().cloned()).is_some_and(|e| e == target));
    assert_eq!(raised.get(), 1);
}

#[test]
fn resources_changed_is_raised_when_attached_to_and_detached_from_logical_tree() {
    let root = TestRoot::new();
    let target = Class1::new();
    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    target.resources_changed(move |_| r.set(r.get() + 1));

    set_child(&root, &target);
    assert_eq!(raised.get(), 1);

    remove_child(&root, &target);
    assert_eq!(raised.get(), 2);
}

#[test]
fn resource_observable_tracks_resource_changes() {
    let root = TestRoot::new();
    let target = Class1::new();
    set_child(&root, &target);

    let values = Rc::new(RefCell::new(Vec::<String>::new()));
    let v = values.clone();
    let observable = ResourceHostRef::from(&target).resource_observable("foo", None);
    let subscription = observable.subscribe_fn(move |value| {
        let value = value.expect("a value or the unset marker");
        match (*value).downcast_ref::<String>() {
            Some(s) => v.borrow_mut().push(s.clone()),
            None => v.borrow_mut().push("(unset)".to_string()),
        }
    });

    root.resources().add_value("foo", "root".to_string());
    target.resources().add_value("foo", "target".to_string());
    assert_eq!(*values.borrow(), vec!["(unset)", "root", "target"]);

    subscription.dispose();
    target.resources().remove(&key("foo"));
    assert_eq!(values.borrow().len(), 3);
}

#[test]
fn resource_observable_uses_the_theme_variant_of_the_host() {
    let target = TestRoot::new();
    let light = ResourceDictionary::new();
    light.add_value("foo", "light".to_string());
    let dark = ResourceDictionary::new();
    dark.add_value("foo", "dark".to_string());
    target.resources().add_theme_dictionary(ThemeVariant::light(), &light);
    target.resources().add_theme_dictionary(ThemeVariant::dark(), &dark);
    target.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));

    let values = Rc::new(RefCell::new(Vec::<String>::new()));
    let v = values.clone();
    ResourceHostRef::from(&target).resource_observable("foo", None).subscribe_fn(move |value| {
        v.borrow_mut().push(string(Some(value)).unwrap_or_default());
    });

    target.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));

    assert_eq!(*values.borrow(), vec!["light", "dark"]);
}

#[test]
fn floating_resource_observable_tracks_the_owner_of_the_provider() {
    let dictionary = ResourceDictionary::new();
    let values = Rc::new(RefCell::new(Vec::<String>::new()));
    let v = values.clone();
    get_floating_resource_observable((&dictionary).into(), "foo", None, None).subscribe_fn(move |value| {
        v.borrow_mut().push(string(Some(value)).unwrap_or_else(|| "(unset)".to_string()));
    });
    assert!(values.borrow().is_empty());

    let target = Class1::new();
    target.resources().add_value("foo", "bar".to_string());
    target.resources().add_merged_dictionary(&dictionary);

    assert_eq!(*values.borrow(), vec!["bar"]);
}

// --- name scopes ---------------------------------------------------------------

#[test]
fn name_scope_registers_and_finds_elements() {
    let scope = NameScope::new();
    let element = Class1::new();
    scope.register("foo", element.clone().upcast());
    // Registering the same element again is allowed.
    scope.register("foo", element.clone().upcast());

    assert_eq!(scope.find("foo").unwrap(), element);
    assert!(scope.find("bar").is_none());

    let scope: &dyn INameScope = &scope;
    assert_eq!(scope.find_as::<Class1>("foo").unwrap(), element);
    assert_eq!(scope.get_as::<Class1>("foo"), element);
}

#[test]
#[should_panic(expected = "Control with the name 'foo' already registered.")]
fn name_scope_rejects_duplicate_names() {
    let scope = NameScope::new();
    scope.register("foo", Class1::new().upcast());
    scope.register("foo", Class1::new().upcast());
}

#[test]
#[should_panic(expected = "NameScope is completed, no further registrations are allowed")]
fn completed_name_scope_rejects_registrations() {
    let scope = NameScope::new();
    scope.complete();
    scope.register("foo", Class1::new().upcast());
}

#[test]
#[should_panic(expected = "Expected control 'foo' to be 'Class3' but it was 'Class1'.")]
fn name_scope_find_as_checks_the_class() {
    let scope = NameScope::new();
    scope.register("foo", Class1::new().upcast());
    let scope: &dyn INameScope = &scope;
    scope.find_as::<Class3>("foo");
}

#[test]
fn find_async_completes_when_the_element_is_registered_or_the_scope_completes() {
    let scope = NameScope::new();
    let found = scope.find_async("foo");
    let missing = scope.find_async("bar");
    assert!(!found.is_completed());

    let completed = Rc::new(Cell::new(0));
    let c = completed.clone();
    found.on_completed(move || c.set(c.get() + 1));

    let element = Class1::new();
    scope.register("foo", element.clone().upcast());
    assert!(found.is_completed());
    assert_eq!(found.get_result().unwrap(), element);
    assert_eq!(completed.get(), 1);

    assert!(!missing.is_completed());
    scope.complete();
    assert!(missing.is_completed());
    assert!(missing.get_result().is_none());
    assert!(scope.find_async("baz").get_result().is_none());
}

#[test]
fn child_name_scope_falls_back_to_parent_scope_when_complete() {
    let parent = NameScopeRef::new(NameScope::new());
    let in_parent = Class1::new();
    parent.register("parent", in_parent.clone().upcast());
    let child = ChildNameScope::new(parent.clone());
    let in_child = Class1::new();
    child.register("child", in_child.clone().upcast());

    assert_eq!(child.find("child").unwrap(), in_child);
    // The parent scope is only consulted once the child scope is complete.
    assert!(child.find("parent").is_none());
    let pending = child.find_async("parent");
    assert!(!pending.is_completed());

    child.complete();
    assert_eq!(child.find("parent").unwrap(), in_parent);
    assert_eq!(pending.get_result().unwrap(), in_parent);
    assert!(!child.is_completed());
    parent.complete();
    assert!(child.is_completed());
    assert!(child.find_async("missing").get_result().is_none());
}

#[test]
fn name_scope_attached_property_and_lookup_through_ancestors() {
    let root = TestRoot::new();
    let child = Class1::new();
    set_child(&root, &child);
    assert!(NameScopeExtensions::find_name_scope(&child).is_none());

    let scope = NameScopeRef::new(NameScope::new());
    scope.register("child", child.clone().upcast());
    NameScope::set_name_scope(&root, Some(scope.clone()));

    assert!(NameScope::get_name_scope(&root) == Some(scope.clone()));
    assert!(NameScopeExtensions::find_name_scope(&child) == Some(scope));
    assert_eq!(NameScopeExtensions::find::<Class1>(&root, "child").unwrap(), child);
    assert_eq!(NameScopeExtensions::get::<Class1>(&root, "child"), child);
    assert!(NameScopeExtensions::find::<Class1>(&child, "child").is_none());
}

#[test]
fn name_scope_locator_tracks_late_registration() {
    let scope = NameScopeRef::new(NameScope::new());
    let results = Rc::new(RefCell::new(Vec::new()));
    let r = results.clone();
    NameScopeLocator::track(&scope, "foo").subscribe_fn(move |element| r.borrow_mut().push(element.is_some()));
    assert!(results.borrow().is_empty());

    scope.register("foo", Class1::new().upcast());
    assert_eq!(*results.borrow(), vec![true]);

    let r = results.clone();
    NameScopeLocator::track(&scope, "foo").subscribe_fn(move |element| r.borrow_mut().push(element.is_some()));
    assert_eq!(*results.borrow(), vec![true, true]);
}
