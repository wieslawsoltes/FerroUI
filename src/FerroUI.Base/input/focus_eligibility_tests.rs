//! Regressions for focus queries inside dialogs.
//! The query may inspect detached scopes, but must not return disabled candidates.
use super::*;
fn scope(children: &[&Ref<TestControl>]) -> Ref<TestControl> {
    let scope = TestControl::scope("query");
    scope.set_is_tab_stop(false);
    for child in children {
        scope.add_child(child);
    }
    scope
}
fn first(scope: &TestControl) -> Option<Ref<InputElement>> {
    FocusManager::find_first_focusable_element_in(&scope.to_ref().upcast())
}
fn last(scope: &TestControl) -> Option<Ref<InputElement>> {
    FocusManager::find_last_focusable_element_in(&scope.to_ref().upcast())
}
#[test]
fn focus_eligibility_first_and_last_skip_disabled_siblings() {
    let a = TestControl::focusable("a");
    let b = TestControl::focusable("b");
    let c = TestControl::focusable("c");
    let query = scope(&[&a, &b, &c]);
    let _root = TestRoot::with_child(&query);
    a.set_is_enabled(false);
    c.set_is_enabled(false);
    assert_eq!(first(&query), Some(b.clone().upcast()));
    assert_eq!(last(&query), Some(b.clone().upcast()));
    a.set_is_enabled(true);
    c.set_is_enabled(true);
    assert_eq!(first(&query), Some(a.upcast()));
    assert_eq!(last(&query), Some(c.upcast()));
}
#[test]
fn focus_eligibility_disabled_ancestor_prunes_nested_candidates_and_recovers() {
    let a = TestControl::focusable("nested");
    let branch = scope(&[&a]);
    let b = TestControl::focusable("b");
    let query = scope(&[&branch, &b]);
    let _root = TestRoot::with_child(&query);
    branch.set_is_enabled(false);
    assert!(!a.is_effectively_enabled());
    assert!(!FocusHelpers::can_have_focusable_children(Some(&branch)));
    assert_eq!(first(&query), Some(b.clone().upcast()));
    query.set_is_enabled(false);
    assert_eq!(first(&query), None);
    assert_eq!(last(&query), None);
    query.set_is_enabled(true);
    branch.set_is_enabled(true);
    assert_eq!(first(&query), Some(a.upcast()));
}
#[test]
fn focus_eligibility_hidden_and_disabled_changes_preserve_tab_index_order() {
    let a = TestControl::focusable("a");
    let b = TestControl::focusable("b");
    let c = TestControl::focusable("c");
    KeyboardNavigation::set_tab_index(&a, 10);
    KeyboardNavigation::set_tab_index(&b, 0);
    KeyboardNavigation::set_tab_index(&c, 20);
    let query = scope(&[&a, &b, &c]);
    let _root = TestRoot::with_child(&query);
    b.set_is_enabled(false);
    c.set_is_visible(false);
    assert_eq!(first(&query), Some(a.clone().upcast()));
    assert_eq!(last(&query), Some(a.clone().upcast()));
    b.set_is_enabled(true);
    c.set_is_visible(true);
    assert_eq!(first(&query), Some(b.upcast()));
    assert_eq!(last(&query), Some(c.upcast()));
}
#[test]
fn focus_eligibility_detached_query_does_not_require_window_attachment() {
    let _services = FerroLocator::enter_scope();
    // The last-element API resolves the manager through the application service.
    FerroLocator::current_mutable()
        .bind::<dyn IFocusManager>()
        .to_constant(FocusManager::new());
    let a = TestControl::focusable("disabled");
    let b = TestControl::focusable("detached");
    a.set_is_enabled(false);
    let query = scope(&[&a, &b]);
    assert!(!b.is_attached_to_visual_tree());
    assert!(!FocusManager::can_focus(&b));
    assert_eq!(first(&query), Some(b.clone().upcast()));
    assert_eq!(last(&query), Some(b.upcast()));
}
#[test]
fn focus_eligibility_uses_virtual_effective_enabled_not_only_raw_property() {
    let a = TestControl::focusable("vetoed");
    let b = TestControl::focusable("b");
    let query = scope(&[&a, &b]);
    let _root = TestRoot::with_child(&query);
    a.set_should_enable(false);
    assert!(a.is_enabled());
    assert!(!a.is_effectively_enabled());
    assert!(!FocusHelpers::is_focusable(Some(&a)));
    assert_eq!(first(&query), Some(b.upcast()));
}
