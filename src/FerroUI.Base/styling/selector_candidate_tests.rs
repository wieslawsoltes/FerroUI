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
