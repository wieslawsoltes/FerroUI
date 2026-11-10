use super::test_support::*;
use super::*;
use crate::*;
use std::rc::Rc;

fn indexed(selector: Selector, value: &str) -> Ref<Style> {
    let style = Style::with_setters(
        selector,
        [Setter::new(Class1::foo_property(), value.to_string())],
    );
    style.enable_selector_index();
    style
}

fn styles() -> Ref<Styles> {
    let styles = Styles::new();
    for _ in 0..8 {
        styles.add(indexed(Selectors::of_type::<Class3>(), "irrelevant"));
    }
    styles
}

fn selected(styles: &Styles, type_: &'static TypeInfo) -> Vec<Rc<dyn IStyle>> {
    let target: Ref<StyledElement> = if type_ == Class1::TYPE {
        Class1::new().upcast()
    } else if type_ == Class2::TYPE {
        Class2::new().upcast()
    } else {
        panic!("unknown test style key")
    };
    styles.candidates(&target).collect()
}

#[test]
fn indexes_respect_exact_derived_and_overridden_style_keys() {
    let styles = styles();
    let exact = indexed(Selectors::of_type::<Class1>(), "exact");
    let derived = indexed(Selectors::is::<Class1>(), "derived");
    styles.add(exact.clone());
    styles.add(derived.clone());
    assert!(selected(&styles, Class1::TYPE) == vec![exact.clone().into(), derived.clone().into()]);
    assert!(selected(&styles, Class2::TYPE) == vec![derived.into()]);
    let target = StyledAsClass1::new();
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "derived");
    drop(target);
    styles.remove_at(9);
    let target = StyledAsClass1::new();
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "exact");
}

#[test]
fn replacing_selector_invalidates_excluded_entries_and_can_deoptimize() {
    let styles = styles();
    assert!(selected(&styles, Class1::TYPE).is_empty());
    let style = styles.get(3);
    let style = style.as_object().unwrap().downcast_ref::<Style>().unwrap();
    style.set_selector(Some(Selectors::of_type::<Class1>()));
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    style.set_selector(Some(Selectors::of_type::<Class3>()));
    // Replacement deoptimizes, preserving arbitrary mutable selector behavior.
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    style.enable_selector_index();
    assert!(selected(&styles, Class1::TYPE).is_empty());
    style.set_selector(None);
    assert!(selected(&styles, Class1::TYPE).len() == 1);
}

#[test]
fn collection_edits_preserve_source_order_and_do_not_retain_removed_styles() {
    let styles = styles();
    let first = indexed(Selectors::of_type::<Class1>(), "first");
    let last = indexed(Selectors::of_type::<Class1>(), "last");
    styles.insert(2, first.clone());
    styles.add(last.clone());
    assert!(selected(&styles, Class1::TYPE) == vec![first.clone().into(), last.clone().into()]);
    styles.move_item(9, 0);
    assert!(selected(&styles, Class1::TYPE) == vec![last.clone().into(), first.clone().into()]);
    styles.set(0, indexed(Selectors::of_type::<Class3>(), "replacement"));
    assert!(selected(&styles, Class1::TYPE) == vec![first.clone().into()]);
    let weak = first.downgrade();
    styles.remove(first.clone());
    drop(first);
    assert!(weak.upgrade().is_none());
    assert!(selected(&styles, Class1::TYPE).is_empty());
    let style = indexed(Selectors::of_type::<Class1>(), "clear");
    let weak = style.downgrade();
    styles.add(style.clone());
    drop(style);
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    styles.clear();
    assert!(weak.upgrade().is_none());
}

#[test]
fn new_children_of_excluded_leaf_are_still_visited() {
    let styles = styles();
    let wrapper = indexed(Selectors::of_type::<Class3>(), "parent");
    styles.add(wrapper.clone());
    assert!(selected(&styles, Class1::TYPE).is_empty());
    let child = indexed(Selectors::of_type::<Class1>(), "child");
    // A nested collection permits an independent selector, as apply_style does.
    let nested = Styles::new();
    nested.add(child);
    wrapper.children().add(nested);
    assert!(selected(&styles, Class1::TYPE) == vec![wrapper.clone().into()]);
    let target = Class1::new();
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "child");
    wrapper.children().clear();
    assert!(selected(&styles, Class1::TYPE).is_empty());
}

#[test]
fn shared_name_class_and_property_conditions_keep_dynamic_activators() {
    let styles = styles();
    let selector = Selectors::of_type::<Class1>().name("old");
    let style = indexed(selector.clone(), "active");
    styles.add(style.clone());
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    // These mutate the same shared node after it was indexed.
    let _ = selector.name("new").class(":active");
    let target = Class1::new();
    target.set_name(Some("new".to_string()));
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "foodefault");
    target.pseudo_classes().add_pseudo(":active");
    assert_eq!(target.foo(), "active");
    target.pseudo_classes().remove_pseudo(":active");
    assert_eq!(target.foo(), "foodefault");

    drop(target);
    let property = indexed(
        Selectors::property_equals(
            Some(Selectors::of_type::<Class1>()),
            Class1::double_property(),
            1.0,
        ),
        "property",
    );
    styles.add(property);
    let target = Class1::new();
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "foodefault");
    target.set_value(Class1::double_property(), 1.0);
    assert_eq!(target.foo(), "property");
    target.set_value(Class1::double_property(), 0.0);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn unknown_and_composite_selectors_remain_candidates() {
    let styles = styles();
    let or = indexed(
        Selectors::or([
            Selectors::of_type::<Class1>(),
            Selectors::of_type::<Class3>(),
        ]),
        "or",
    );
    let not = indexed(Selectors::not(None, Selectors::of_type::<Class3>()), "not");
    let class = indexed(Selectors::class(None, "active"), "class");
    styles.add(or.clone());
    styles.add(not.clone());
    styles.add(class.clone());
    assert!(selected(&styles, Class1::TYPE) == vec![or.into(), not.into(), class.into()]);
}

#[test]
fn mid_pass_selector_mutation_falls_back_from_next_source_position() {
    let styles = styles();
    let later = indexed(Selectors::of_type::<Class3>(), "changed");
    let first = Style::with_setters(
        Selectors::of_type::<Class1>(),
        [Setter::new_untyped(
            Class1::direct_property(),
            boxed("trigger".to_string()),
        )],
    );
    first.enable_selector_index();
    styles.insert(0, first);
    styles.add(later.clone());
    // Warm the exact list that excludes `later`.
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    let target = Class1::new();
    let later_clone = later.clone();
    target.property_changed(move |e| {
        if e.property() == Class1::direct_property().as_property() {
            later_clone.set_selector(Some(Selectors::of_type::<Class1>()));
        }
    });
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "changed");
}

#[test]
fn compatible_empty_styles_can_gain_setters_after_indexing() {
    let styles = styles();
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.enable_selector_index();
    styles.add(style.clone());
    assert!(selected(&styles, Class1::TYPE).len() == 1);
    style.add_setter(Setter::new(Class1::foo_property(), "added".to_string()));
    let target = Class1::new();
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "added");
}

#[repr(C)]
struct MutableKey {
    base: Class1,
    alternate: std::cell::Cell<bool>,
    on_key: std::cell::RefCell<Option<Box<dyn Fn()>>>,
}
ferro_class!(MutableKey: Class1);
ferro_impl_classes!(MutableKey: FerroObjectImpl, VisualImpl, crate::layout::LayoutableImpl);
impl StyledElementImpl for MutableKey {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        let callback = this.on_key.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        if this.alternate.get() {
            Class2::TYPE
        } else {
            Class1::TYPE
        }
    }
}

#[test]
fn changing_style_key_mid_pass_uses_remaining_source_styles() {
    let styles = styles();
    let first = Style::with_setters(
        Selectors::of_type::<Class1>(),
        [Setter::new_untyped(
            Class1::direct_property(),
            boxed("trigger".to_string()),
        )],
    );
    first.enable_selector_index();
    styles.insert(0, first);
    styles.add(indexed(Selectors::of_type::<Class2>(), "new key"));
    let target = instantiate(MutableKey {
        base: Class1::construct(),
        alternate: std::cell::Cell::new(false),
        on_key: Default::default(),
    });
    let weak = target.downgrade();
    target.property_changed(move |e| {
        if e.property() == Class1::direct_property().as_property() {
            weak.upgrade().unwrap().alternate.set(true);
        }
    });
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "new key");
}

struct CustomStyle {
    calls: Rc<std::cell::Cell<usize>>,
}
impl crate::controls::IResourceNode for CustomStyle {
    fn has_resources(&self) -> bool {
        false
    }
    fn try_get_resource(
        &self,
        _: &crate::controls::ResourceKey,
        _: Option<&ThemeVariant>,
    ) -> Option<crate::controls::ResourceValue> {
        None
    }
}
impl IStyle for CustomStyle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        empty_styles()
    }
    fn as_object(&self) -> Option<&FerroObject> {
        self.calls.set(self.calls.get() + 1);
        None
    }
}

#[test]
fn indexing_does_not_call_custom_adapter_methods_ahead_of_attachment() {
    let styles = styles();
    let calls = Rc::new(std::cell::Cell::new(0));
    styles.add(Rc::new(CustomStyle {
        calls: calls.clone(),
    }) as Rc<dyn IStyle>);
    let target = Class1::new();
    // Collection validation also calls as_object; count only index construction.
    calls.set(0);
    assert_eq!(styles.candidates(&target).count(), 1);
    assert_eq!(calls.get(), 0);
    target.styles().add(&styles);
    calls.set(0);
    target.apply_styling();
    assert!(calls.get() > 0);
}

struct MutableSelectorType(Rc<std::cell::Cell<&'static TypeInfo>>);
impl SelectorNode for MutableSelectorType {
    fn in_template(&self) -> bool {
        false
    }
    fn is_combinator(&self) -> bool {
        false
    }
    fn target_type(&self) -> Option<&'static TypeInfo> {
        Some(self.0.get())
    }
    fn to_string(&self, _: Option<&Style>) -> String {
        "custom".into()
    }
    fn evaluate(&self, _: &StyledElement, _: Option<&StyleBase>, _: bool) -> SelectorMatch {
        SelectorMatch::ALWAYS_THIS_TYPE
    }
    fn move_previous(&self) -> Option<&Selector> {
        None
    }
    fn move_previous_or_parent(&self) -> Option<&Selector> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn class_wrapped_custom_type_metadata_is_not_indexed() {
    let styles = styles();
    let type_ = Rc::new(std::cell::Cell::new(Class3::TYPE));
    let selector = testing::selector(MutableSelectorType(type_.clone())).class("active");
    styles.add(indexed(selector, "custom"));
    assert_eq!(selected(&styles, Class1::TYPE).len(), 1);
    type_.set(Class1::TYPE);
    let target = Class1::new();
    target.classes().add("active");
    target.styles().add(&styles);
    target.apply_styling();
    assert_eq!(target.foo(), "custom");
}

#[test]
fn custom_selector_before_explicit_type_keeps_full_matching() {
    let styles = styles();
    let type_ = Rc::new(std::cell::Cell::new(Class1::TYPE));
    let custom = testing::selector(MutableSelectorType(type_));
    let style = indexed(
        Selectors::of_type_info(Some(custom), Class3::TYPE),
        "custom prefix",
    );
    styles.add(style);
    // Even an explicit type cannot justify bypassing a custom node which is
    // evaluated first and may change the target before that type is checked.
    assert_eq!(selected(&styles, Class1::TYPE).len(), 1);
}

#[test]
fn reparenting_under_container_query_invalidates_exclusions() {
    let styles = styles();
    let style = indexed(Selectors::of_type::<Class3>(), "container");
    styles.add(style.clone());
    assert!(selected(&styles, Class1::TYPE).is_empty());
    let query = ContainerQuery::new();
    // Exercise direct parent mutation, in addition to collection mutation.
    style.set_parent(Some(query.upcast_ref::<StyleBase>()));
    assert_eq!(selected(&styles, Class1::TYPE).len(), 1);
    style.set_parent(None);
    assert!(selected(&styles, Class1::TYPE).is_empty());
}

#[test]
fn key_getter_reentrancy_rechecks_selector_epoch_before_skipping_entries() {
    let styles = styles();
    let first = indexed(Selectors::of_type::<Class1>(), "first");
    let last = indexed(Selectors::of_type::<Class1>(), "last");
    styles.insert(0, first.clone());
    styles.add(last);
    let target = instantiate(MutableKey {
        base: Class1::construct(),
        alternate: std::cell::Cell::new(false),
        on_key: Default::default(),
    });
    let mut candidates = styles.candidates(&target);
    assert!(candidates.next().unwrap() == first.into());
    let changed = styles.get(1);
    let callback_style = changed.clone();
    *target.on_key.borrow_mut() = Some(Box::new(move || {
        callback_style
            .as_object()
            .unwrap()
            .downcast_ref::<Style>()
            .unwrap()
            .set_selector(Some(Selectors::of_type::<Class1>()));
    }));
    assert!(candidates.next().unwrap() == changed);
}

fn plain(selector: Selector, value: &str) -> Ref<Style> {
    Style::with_setters(selector, [Setter::new(Class1::foo_property(), value.to_string())])
}

/// The number of candidates of `target` in `styles` and the scans the lookup took.
fn lookup(styles: &Styles, target: &StyledElement) -> (usize, usize) {
    let before = selector_candidates::scans();
    let count = styles.candidates(target).count();
    (count, selector_candidates::scans() - before)
}

#[test]
fn a_collection_without_index_enabled_styles_has_no_index_and_is_never_scanned() {
    let styles = Styles::new();
    for _ in 0..20 {
        styles.add(plain(Selectors::of_type::<Class3>(), "other"));
    }
    styles.add(plain(Selectors::of_type::<Class1>(), "plain"));
    // A style without a selector takes child styles with selectors of their own.
    let group = Style::new();
    for _ in 0..20 {
        group.children().add(plain(Selectors::of_type::<Class3>(), "other"));
    }
    styles.add(group.clone());

    let before = selector_candidates::scans();
    {
        // Styling an element and editing the collections under it.
        let target = Class1::new();
        assert!(styles.candidates(&target).unfiltered().is_some_and(|all| all.len() == 22));
        assert!(group.children().candidates(&target).unfiltered().is_some_and(|all| all.len() == 20));
        target.styles().add(&styles);
        target.apply_styling();
        assert_eq!(target.foo(), "plain");
        styles.remove_at(0);
        styles.move_item(0, 1);
        styles.set(0, plain(Selectors::of_type::<Class3>(), "replacement"));
        group.children().remove_at(0);
        target.apply_styling();
        assert_eq!(target.foo(), "plain");
        assert_eq!(lookup(&styles, &target), (21, 0));
        target.styles().clear();
    }
    assert_eq!(selector_candidates::scans(), before);
    // Nothing was allocated for either collection.
    assert_eq!(styles.candidate_stats(), None);
    assert_eq!(group.children().candidate_stats(), None);

    // With one index-enabled entry the collection is scanned once for the key...
    let target = Class1::new();
    let enabled = indexed(Selectors::of_type::<Class3>(), "enabled");
    styles.add(enabled.clone());
    assert_eq!(lookup(&styles, &target), (21, 1));
    assert_eq!(lookup(&styles, &target), (21, 0));
    assert_eq!(styles.candidate_stats(), Some((1, 1, 1)));
    // ...and when the selector of that entry is replaced, or the entry leaves, the collection
    // is back to the check: no scan and no list.
    enabled.set_selector(Some(Selectors::of_type::<Class3>()));
    assert_eq!(lookup(&styles, &target), (22, 0));
    assert_eq!(styles.candidate_stats(), Some((0, 0, 0)));
    enabled.enable_selector_index();
    assert_eq!(lookup(&styles, &target), (21, 1));
    styles.remove(enabled);
    assert_eq!(lookup(&styles, &target), (21, 0));
    assert!(styles.candidates(&target).unfiltered().is_some());
    assert_eq!(styles.candidate_stats(), Some((0, 0, 0)));
}

/// An element styled by a key that the test chooses.
#[repr(C)]
struct Keyed {
    base: Class1,
    key: std::cell::Cell<&'static TypeInfo>,
}
ferro_class!(Keyed: Class1);
ferro_impl_classes!(Keyed: FerroObjectImpl, VisualImpl, crate::layout::LayoutableImpl);
impl StyledElementImpl for Keyed {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        this.key.get()
    }
}

/// Style keys of types derived from [`Class1`].
fn style_keys(count: usize) -> Vec<&'static TypeInfo> {
    (0..count).map(|_| &*Box::leak(Box::new(TypeInfo::new("Keyed", Some(Class1::TYPE))))).collect()
}

#[test]
fn two_hundred_style_keys_in_rotation_are_scanned_once_each() {
    let keys = style_keys(200);
    let styles = styles();
    let derived = indexed(Selectors::is::<Class1>(), "derived");
    styles.add(derived.clone());
    let exact: Vec<Ref<Style>> = keys[..10]
        .iter()
        .map(|key| {
            let style = indexed(Selectors::of_type_info(None, *key), "exact");
            styles.add(style.clone());
            style
        })
        .collect();
    let target = instantiate(Keyed { base: Class1::construct(), key: std::cell::Cell::new(keys[0]) });
    let before = selector_candidates::scans();
    for pass in 0..3 {
        for (index, key) in keys.iter().enumerate() {
            target.key.set(*key);
            let found: Vec<Rc<dyn IStyle>> = styles.candidates(&target).collect();
            let mut expected: Vec<Rc<dyn IStyle>> = vec![derived.clone().into()];
            if let Some(exact) = exact.get(index) {
                expected.push(exact.clone().into());
            }
            assert!(found == expected, "pass {pass}, key {index}");
        }
        // The first pass scans the collection once for each key; later passes scan nothing.
        assert_eq!(selector_candidates::scans() - before, 200, "pass {pass}");
    }
    // 19 index-enabled entries; 200 keys remembered; the 190 keys that only the derived
    // style can match share one list, and each of the other ten has its own.
    assert_eq!(styles.candidate_stats(), Some((19, 200, 11)));
}

#[test]
fn keys_beyond_the_bound_are_computed_and_nothing_remembered_is_dropped() {
    let bound = selector_candidates::MAX_KEYS;
    let keys = style_keys(bound + 5);
    let styles = styles();
    let derived = indexed(Selectors::is::<Class1>(), "derived");
    styles.add(derived.clone());
    let target = instantiate(Keyed { base: Class1::construct(), key: std::cell::Cell::new(keys[0]) });
    let pass = || {
        let before = selector_candidates::scans();
        for key in &keys {
            target.key.set(*key);
            let expected: Vec<Rc<dyn IStyle>> = vec![derived.clone().into()];
            assert!(styles.candidates(&target).collect::<Vec<_>>() == expected);
        }
        selector_candidates::scans() - before
    };
    assert_eq!(pass(), bound + 5);
    // The keys within the bound are still remembered; the five beyond it are scanned again.
    assert_eq!(pass(), 5);
    assert_eq!(pass(), 5);
    assert_eq!(styles.candidate_stats(), Some((9, bound, 1)));
}

#[test]
fn a_change_invalidates_the_collection_that_holds_the_style_and_no_other() {
    let ours = styles();
    let entry = indexed(Selectors::of_type::<Class3>(), "entry");
    ours.add(entry.clone());
    let other = styles();
    let other_entry = indexed(Selectors::of_type::<Class3>(), "other");
    other.add(other_entry.clone());
    let target = Class1::new();
    assert_eq!(lookup(&ours, &target), (0, 1));
    assert_eq!(lookup(&other, &target), (0, 1));
    assert_eq!(lookup(&ours, &target), (0, 0));

    // Changes of another collection, of a style in it and of the children of such a style
    // leave the lists of this one in place.
    other.add(indexed(Selectors::of_type::<Class1>(), "added"));
    assert_eq!(lookup(&other, &target), (1, 1));
    other.remove_at(0);
    assert_eq!(lookup(&other, &target), (1, 1));
    other.move_item(0, 1);
    assert_eq!(lookup(&other, &target), (1, 1));
    other_entry.set_selector(Some(Selectors::of_type::<Class1>()));
    assert_eq!(lookup(&other, &target), (2, 1));
    other_entry.enable_selector_index();
    assert_eq!(lookup(&other, &target), (2, 1));
    other_entry.children().add(Styles::new());
    assert_eq!(lookup(&other, &target), (2, 1));
    other.clear();
    assert_eq!(lookup(&other, &target), (0, 0));
    assert_eq!(lookup(&ours, &target), (0, 0));

    // Changes of the collection itself are seen at the next lookup.
    let added = indexed(Selectors::of_type::<Class1>(), "added");
    ours.add(added.clone());
    assert_eq!(lookup(&ours, &target), (1, 1));
    ours.move_item(9, 0);
    assert_eq!(lookup(&ours, &target), (1, 1));
    ours.remove(added);
    assert_eq!(lookup(&ours, &target), (0, 1));
    // So are a new selector of a style it holds...
    entry.set_selector(Some(Selectors::of_type::<Class1>()));
    assert_eq!(lookup(&ours, &target), (1, 1));
    entry.enable_selector_index();
    assert_eq!(lookup(&ours, &target), (1, 1));
    entry.set_selector(Some(Selectors::of_type::<Class3>()));
    entry.enable_selector_index();
    assert_eq!(lookup(&ours, &target), (0, 1));
    // ...and the children of such a style.
    entry.children().add(Styles::new());
    assert_eq!(lookup(&ours, &target), (1, 1));
    entry.children().clear();
    assert_eq!(lookup(&ours, &target), (0, 1));
    assert_eq!(lookup(&ours, &target), (0, 0));
}

#[test]
fn a_change_of_a_nested_child_collection_invalidates_it_and_the_collection_of_its_owner() {
    let outer = styles();
    let owner = indexed(Selectors::of_type::<Class3>(), "owner");
    outer.add(owner.clone());
    let target = Class1::new();
    let nested = || {
        let before = selector_candidates::scans();
        let count = owner.children().candidates(&target).count();
        (count, selector_candidates::scans() - before)
    };
    assert_eq!(lookup(&outer, &target), (0, 1));

    // The first children: the owner is a candidate of every element from here on.
    for _ in 0..8 {
        owner.children().add(indexed(Selectors::nesting(None).child().of_type::<Class3>(), "nested"));
    }
    assert_eq!(lookup(&outer, &target), (1, 1));
    assert_eq!(nested(), (0, 1));
    assert_eq!(lookup(&outer, &target), (1, 0));
    assert_eq!(nested(), (0, 0));

    // One more child: its own collection and the collection that holds the owner both
    // compute their lists again.
    let hit = indexed(Selectors::nesting(None).child().of_type::<Class1>(), "hit");
    owner.children().add(hit.clone());
    assert_eq!(nested(), (1, 1));
    assert_eq!(lookup(&outer, &target), (1, 1));

    // A new selector of a nested style concerns the nested collection alone.
    hit.set_selector(Some(Selectors::nesting(None).child().of_type::<Class3>()));
    assert_eq!(nested(), (1, 1));
    hit.enable_selector_index();
    assert_eq!(nested(), (0, 1));
    assert_eq!(lookup(&outer, &target), (1, 0));

    // Without children the owner is left out again.
    owner.children().clear();
    assert_eq!(lookup(&outer, &target), (0, 1));
    assert_eq!(nested(), (0, 0));
    assert_eq!(owner.children().candidate_stats(), Some((0, 0, 0)));
}

#[test]
fn a_style_is_narrowed_by_the_collection_it_was_added_to_while_enabled() {
    let styles = styles();
    // Enabled after it was added: a candidate of every element until it is added again.
    let late = plain(Selectors::of_type::<Class3>(), "late");
    styles.add(late.clone());
    late.enable_selector_index();
    assert!(selected(&styles, Class1::TYPE) == vec![late.clone().into()]);
    styles.set(8, late.clone());
    assert!(selected(&styles, Class1::TYPE).is_empty());

    // In two collections: the first one narrows it, the second one keeps it.
    let second = self::styles();
    second.add(late.clone());
    assert!(selected(&second, Class1::TYPE) == vec![late.clone().into()]);
    styles.remove(late.clone());
    assert!(selected(&styles, Class1::TYPE).is_empty());
    assert!(selected(&second, Class1::TYPE) == vec![late.clone().into()]);
    // The collection that counted it is gone: the next one to add it narrows it.
    drop(styles);
    let third = self::styles();
    third.add(late.clone());
    assert!(selected(&third, Class1::TYPE).is_empty());
}

/// The styles of the order test: eight that cannot match a `Class1`, then a style that the index
/// cannot leave out (`fallback`) and one it can, both setting `Class1::foo` on a `Class1`.
fn fallback_and_indexed(fallback: usize, fallback_first: bool, enable: bool) -> Vec<Ref<Style>> {
    let rule = |selector: Selector, value: &str, enable: bool| {
        let style = plain(selector, value);
        if enable {
            style.enable_selector_index();
        }
        style
    };
    let mut entries: Vec<Ref<Style>> =
        (0..8).map(|_| rule(Selectors::of_type::<Class3>(), "irrelevant", enable)).collect();
    let fallback = match fallback {
        // Not index-enabled.
        0 => rule(Selectors::of_type::<Class1>(), "fallback", false),
        // Index-enabled, with a selector that gives no type to index by.
        1 => rule(Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class3>()]), "fallback", enable),
        2 => rule(
            testing::selector(MutableSelectorType(Rc::new(std::cell::Cell::new(Class1::TYPE)))),
            "fallback",
            enable,
        ),
        // The same, and a trigger: a selector with a class outranks one without.
        _ => rule(Selectors::class(None, "active"), "fallback", enable),
    };
    let indexed = rule(Selectors::of_type::<Class1>(), "indexed", enable);
    if fallback_first {
        entries.extend([fallback, indexed]);
    } else {
        entries.extend([indexed, fallback]);
    }
    entries
}

/// `Class1::foo` of an element styled by `entries`, held by a styles collection or, with
/// `nested`, by the children of a style. `narrowed` is what the traversal is expected to be.
fn applied(entries: Vec<Ref<Style>>, nested: bool, narrowed: bool) -> String {
    let target = Class1::new();
    target.classes().add("active");
    let candidates = if nested {
        let group = Style::new();
        for entry in entries {
            group.children().add(entry);
        }
        target.styles().add(group.clone());
        group.children().candidates(&target)
    } else {
        let styles = Styles::new();
        for entry in entries {
            styles.add(entry);
        }
        target.styles().add(&styles);
        styles.candidates(&target)
    };
    assert_eq!(candidates.unfiltered().is_none(), narrowed);
    assert_eq!(candidates.count(), if narrowed { 2 } else { 10 });
    target.apply_styling();
    target.foo()
}

#[test]
fn a_fallback_style_and_an_indexed_style_apply_in_source_order() {
    for fallback in 0..4 {
        for nested in [false, true] {
            for fallback_first in [true, false] {
                // The later style wins; a trigger wins over a style that is not one.
                let expected = if fallback_first && fallback != 3 { "indexed" } else { "fallback" };
                let case = format!("fallback {fallback}, nested {nested}, fallback first {fallback_first}");
                let with_index = applied(fallback_and_indexed(fallback, fallback_first, true), nested, true);
                let without_index = applied(fallback_and_indexed(fallback, fallback_first, false), nested, false);
                assert_eq!(with_index, expected, "{case}");
                assert_eq!(without_index, expected, "{case}");
            }
        }
    }
}
