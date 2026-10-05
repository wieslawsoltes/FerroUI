//! Tests of the focus adorner of [`Control`]. The reference suite has no
//! tests of it.

use crate::primitives::{AdornerLayer, VisualLayerManager};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_buttons::focus_scope;
use crate::{Border, Control, StackPanel};
use ferroui_base::input::{KeyModifiers, NavigationMethod};
use ferroui_base::{Ref, Visual};
use std::cell::Cell;
use std::rc::Rc;

type FocusAdornerTemplate = Rc<dyn ITemplateOf<Option<Ref<Control>>>>;

struct Tree {
    _root: Ref<TestRoot>,
    manager: Ref<VisualLayerManager>,
    target: Ref<Control>,
    other: Ref<Control>,
}

fn focusable() -> Ref<Control> {
    let control = Control::new();
    control.set_focusable(true);
    control
}

fn create_tree() -> Tree {
    let target = focusable();
    let other = focusable();
    let panel = StackPanel::new();
    panel.children().add(&target);
    panel.children().add(&other);
    let manager = VisualLayerManager::new();
    manager.set_child(panel);
    let root = TestRoot::with_child(&manager);
    root.execute_initial_layout_pass();
    Tree { _root: root, manager, target, other }
}

/// A template that builds a new border each time and counts the builds.
fn adorner_template() -> (FocusAdornerTemplate, Rc<Cell<i32>>) {
    let built = Rc::new(Cell::new(0));
    let template: FocusAdornerTemplate = FuncTemplate::new({
        let built = built.clone();
        move || -> Option<Ref<Control>> {
            built.set(built.get() + 1);
            Some(Border::new().upcast())
        }
    });
    (template, built)
}

fn adorners(tree: &Tree) -> Vec<Ref<Control>> {
    tree.manager.adorner_layer().expect("the layer manager has an adorner layer").children().snapshot().to_vec()
}

fn adorned(adorner: &Control) -> Option<Ref<Visual>> {
    AdornerLayer::get_adorned_element(adorner)
}

#[test]
fn focus_adorner_is_added_for_tab_and_directional_navigation() {
    for method in [NavigationMethod::Tab, NavigationMethod::Directional] {
        let _scope = test_scope();
        let _focus = focus_scope();
        let tree = create_tree();
        let (template, built) = adorner_template();
        tree.target.set_focus_adorner(Some(template));

        assert!(tree.target.focus_with(method, KeyModifiers::NONE));

        let shown = adorners(&tree);
        assert_eq!(1, shown.len(), "{method:?}");
        assert_eq!(1, built.get());
        assert!(shown[0].is::<Border>());
        assert_eq!(Some(tree.target.clone().upcast()), adorned(&shown[0]));
    }
}

#[test]
fn focus_adorner_is_removed_when_focus_is_lost() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let tree = create_tree();
    let (template, built) = adorner_template();
    tree.target.set_focus_adorner(Some(template));
    assert!(tree.target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    assert_eq!(1, adorners(&tree).len());

    // The other control has no adorner template: nothing replaces it.
    assert!(tree.other.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    assert!(adorners(&tree).is_empty());

    // Focusing again builds a new adorner.
    assert!(tree.target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    assert_eq!(1, adorners(&tree).len());
    assert_eq!(2, built.get());
}

#[test]
fn no_focus_adorner_for_pointer_or_unspecified_focus() {
    for method in [NavigationMethod::Pointer, NavigationMethod::Unspecified] {
        let _scope = test_scope();
        let _focus = focus_scope();
        let tree = create_tree();
        let (template, built) = adorner_template();
        tree.target.set_focus_adorner(Some(template));

        assert!(tree.target.focus_with(method, KeyModifiers::NONE));

        assert!(tree.target.is_focused());
        assert!(adorners(&tree).is_empty(), "{method:?}");
        assert_eq!(0, built.get());
    }
}

#[test]
fn default_focus_adorner_of_the_layer_is_used_when_the_control_has_none() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let tree = create_tree();
    let (template, built) = adorner_template();
    tree.manager.adorner_layer().unwrap().set_default_focus_adorner(Some(template));

    assert!(tree.target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    let shown = adorners(&tree);
    assert_eq!(1, shown.len());
    assert_eq!(Some(tree.target.clone().upcast()), adorned(&shown[0]));

    // The adorner moves with the focus: every control uses the default.
    assert!(tree.other.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    let shown = adorners(&tree);
    assert_eq!(1, shown.len());
    assert_eq!(Some(tree.other.clone().upcast()), adorned(&shown[0]));
    assert_eq!(2, built.get());
}

#[test]
fn focus_adorner_property_overrides_the_default_of_the_layer() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let tree = create_tree();
    let (default_template, default_built) = adorner_template();
    tree.manager.adorner_layer().unwrap().set_default_focus_adorner(Some(default_template));
    let (template, built) = adorner_template();
    tree.target.set_focus_adorner(Some(template));

    assert!(tree.target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    assert_eq!(1, adorners(&tree).len());
    assert_eq!((1, 0), (built.get(), default_built.get()));
}

#[test]
fn focus_adorner_set_to_none_suppresses_the_default_of_the_layer() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let tree = create_tree();
    let (default_template, default_built) = adorner_template();
    tree.manager.adorner_layer().unwrap().set_default_focus_adorner(Some(default_template));
    tree.target.set_focus_adorner(None);

    assert!(tree.target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    assert!(adorners(&tree).is_empty());
    assert_eq!(0, default_built.get());

    // A template that builds nothing shows nothing either.
    let empty: FocusAdornerTemplate = FuncTemplate::new(|| -> Option<Ref<Control>> { None });
    tree.other.set_focus_adorner(Some(empty));
    assert!(tree.other.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    assert!(adorners(&tree).is_empty());
}

#[test]
fn no_focus_adorner_without_an_adorner_layer() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = focusable();
    let (template, built) = adorner_template();
    target.set_focus_adorner(Some(template));
    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    assert!(target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    assert!(target.is_focused());
    assert_eq!(0, built.get());
}
