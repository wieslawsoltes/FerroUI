//! Tests of the logical tree extensions. Not from upstream: upstream has no
//! suite for `LogicalExtensions`; these state the order and the results of
//! each member on a small tree.

use crate::layout::Layoutable;
use crate::{Ref, StyledElement};

/// root
/// ├── a (Layoutable)
/// │   └── a1
/// └── b
///     └── b1 (Layoutable)
struct Tree {
    root: Ref<StyledElement>,
    a: Ref<StyledElement>,
    a1: Ref<StyledElement>,
    b: Ref<StyledElement>,
    b1: Ref<StyledElement>,
}

fn tree() -> Tree {
    let root = StyledElement::new();
    let a: Ref<StyledElement> = Layoutable::new().upcast();
    let a1 = StyledElement::new();
    let b = StyledElement::new();
    let b1: Ref<StyledElement> = Layoutable::new().upcast();
    root.logical_children().add(a.clone());
    root.logical_children().add(b.clone());
    a.logical_children().add(a1.clone());
    b.logical_children().add(b1.clone());
    Tree { root, a, a1, b, b1 }
}

#[test]
fn ancestors_run_from_the_parent_to_the_root() {
    let t = tree();

    assert_eq!(vec![t.a.clone(), t.root.clone()], t.a1.get_logical_ancestors().collect::<Vec<_>>());
    assert_eq!(
        vec![t.a1.clone(), t.a.clone(), t.root.clone()],
        t.a1.get_self_and_logical_ancestors().collect::<Vec<_>>()
    );
    assert_eq!(0, t.root.get_logical_ancestors().count());
}

#[test]
fn descendants_are_depth_first_pre_order() {
    let t = tree();

    assert_eq!(
        vec![t.a.clone(), t.a1.clone(), t.b.clone(), t.b1.clone()],
        t.root.get_logical_descendants().collect::<Vec<_>>()
    );
    assert_eq!(
        vec![t.root.clone(), t.a.clone(), t.a1.clone(), t.b.clone(), t.b1.clone()],
        t.root.get_self_and_logical_descendants().collect::<Vec<_>>()
    );
    assert_eq!(0, t.a1.get_logical_descendants().count());
}

#[test]
fn find_logical_ancestor_of_type_skips_self_unless_included() {
    let t = tree();

    assert_eq!(None, t.a.find_logical_ancestor_of_type::<Layoutable>(false));
    assert_eq!(Some(t.a.clone()), t.a.find_logical_ancestor_of_type::<Layoutable>(true).map(|x| x.upcast()));
    assert_eq!(Some(t.a.clone()), t.a1.find_logical_ancestor_of_type::<Layoutable>(false).map(|x| x.upcast()));
}

#[test]
fn find_logical_descendant_of_type_is_depth_first() {
    let t = tree();

    assert_eq!(Some(t.a.clone()), t.root.find_logical_descendant_of_type::<Layoutable>(false).map(|x| x.upcast()));
    assert_eq!(Some(t.b1.clone()), t.b.find_logical_descendant_of_type::<Layoutable>(false).map(|x| x.upcast()));
    assert_eq!(None, t.a.find_logical_descendant_of_type::<Layoutable>(false));
    assert_eq!(Some(t.a.clone()), t.a.find_logical_descendant_of_type::<Layoutable>(true).map(|x| x.upcast()));
}

#[test]
fn parent_children_and_siblings() {
    let t = tree();

    assert_eq!(Some(t.root.clone()), t.a.get_logical_parent());
    assert_eq!(Some(t.a.clone()), t.a1.get_logical_parent_of_type::<Layoutable>().map(|x| x.upcast()));
    assert_eq!(None, t.b1.get_logical_parent_of_type::<Layoutable>());
    assert_eq!(vec![t.a.clone(), t.b.clone()], *t.root.get_logical_children());
    assert_eq!(vec![t.a.clone(), t.b.clone()], *t.b.get_logical_siblings());
    assert!(t.root.get_logical_siblings().is_empty());
}

#[test]
fn is_logical_ancestor_of_is_strict() {
    let t = tree();

    assert!(t.root.is_logical_ancestor_of(Some(&t.a1)));
    assert!(t.a.is_logical_ancestor_of(Some(&t.a1)));
    assert!(!t.a.is_logical_ancestor_of(Some(&t.a)));
    assert!(!t.b.is_logical_ancestor_of(Some(&t.a1)));
    assert!(!t.a1.is_logical_ancestor_of(Some(&t.a)));
    assert!(!t.root.is_logical_ancestor_of(None));
}

/// Not from upstream (desktop-performance.md, section 9.63): the members that read the logical children make no
/// collection for an element that has none, so a leaf that was walked is as it was made.
#[test]
fn reading_the_children_of_a_leaf_makes_it_no_collection() {
    let t = tree();
    let leaf = t.a1.clone();
    assert!(leaf.logical_children_snapshot().is_none());
    assert_eq!(0, leaf.logical_children_count());

    assert!(leaf.get_logical_children().is_empty());
    assert_eq!(0, leaf.get_logical_descendants().count());
    assert_eq!(None, leaf.find_logical_descendant_of_type::<Layoutable>(false));
    assert_eq!(4, t.root.get_logical_descendants().count());
    assert_eq!(Some(t.a.clone()), t.root.find_logical_descendant_of_type::<Layoutable>(false).map(|x| x.upcast()));
    assert!(leaf.logical_children_snapshot().is_none());
    assert!(t.b1.logical_children_snapshot().is_none());

    // The first to ask for the collection makes it, and the reads see what it holds.
    let child = StyledElement::new();
    leaf.logical_children().add(child.clone());
    assert_eq!(1, leaf.logical_children_count());
    assert_eq!(Some(vec![child.clone()]), leaf.logical_children_snapshot().map(|children| children.to_vec()));
    assert_eq!(vec![child.clone()], *leaf.get_logical_children());
    assert_eq!(vec![child], leaf.get_logical_descendants().collect::<Vec<_>>());
}
