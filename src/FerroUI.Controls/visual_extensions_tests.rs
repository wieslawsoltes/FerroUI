//! Port of `VisualExtensionsTests.cs` (base unit tests). The trees are built
//! from controls under a test root, so the tests live with the controls.

use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Button, Control, Decorator, StackPanel};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::TranslateTransform;
use ferroui_base::{Point, Rect, Ref, Size, Visual};

fn decorator(child: Option<Ref<Decorator>>) -> Ref<Decorator> {
    let decorator = Decorator::new();
    if let Some(child) = child {
        decorator.set_child(child);
    }
    decorator
}

fn stack_panel(children: &[Ref<Control>]) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    for child in children {
        panel.children().add(child.clone());
    }
    panel
}

fn v(visual: &Visual) -> Ref<Visual> {
    visual.to_ref()
}

#[test]
fn find_ancestor_of_type_finds_direct_parent() {
    let _scope = test_scope();
    let target = StackPanel::new();
    let root = TestRoot::with_child(target.clone());

    assert_eq!(Some(root), target.find_ancestor_of_type::<TestRoot>(false));
}

#[test]
fn find_ancestor_of_type_finds_visible_parent() {
    let _scope = test_scope();
    let target = StackPanel::new();
    let inner = TestRoot::with_child(target.clone());
    inner.set_is_visible(false);
    let root = TestRoot::with_child(inner.clone());

    assert_eq!(Some(root), target.find_ancestor_of_type_where::<TestRoot>(false, |v| v.is_visible()));
}

#[test]
fn find_ancestor_of_type_finds_ancestor_of_nested_child() {
    let _scope = test_scope();
    let target = Button::new();
    let root = TestRoot::with_child(stack_panel(&[stack_panel(&[target.clone().upcast()]).upcast()]));

    assert_eq!(Some(root), target.find_ancestor_of_type::<TestRoot>(false));
}

#[test]
fn find_descendant_of_type_finds_direct_child() {
    let _scope = test_scope();
    let target = StackPanel::new();
    let root = TestRoot::with_child(target.clone());

    assert_eq!(Some(target), root.find_descendant_of_type::<StackPanel>(false));
}

#[test]
fn find_descendant_of_type_finds_nested_child() {
    let _scope = test_scope();
    let target = Button::new();
    let root = TestRoot::with_child(stack_panel(&[stack_panel(&[target.clone().upcast()]).upcast()]));

    assert_eq!(Some(target), root.find_descendant_of_type::<Button>(false));
}

#[test]
fn find_descendant_of_type_finds_nested_visible_child() {
    let _scope = test_scope();
    let hidden = Button::new();
    hidden.set_is_visible(false);
    let target = Button::new();
    let root = TestRoot::with_child(stack_panel(&[stack_panel(&[hidden.upcast(), target.clone().upcast()]).upcast()]));

    assert_eq!(Some(target), root.find_descendant_of_type_where::<Button>(false, |v| v.is_visible()));
}

#[test]
fn find_common_visual_ancestor_first_is_parent_of_second() {
    let _scope = test_scope();
    let right = decorator(None);
    let left = decorator(Some(right.clone()));
    let _root = TestRoot::with_child(left.clone());

    let ancestor = left.find_common_visual_ancestor(&right);
    assert_eq!(Some(v(&left)), ancestor);

    let ancestor = right.find_common_visual_ancestor(&left);
    assert_eq!(Some(v(&left)), ancestor);
}

#[test]
fn find_common_visual_ancestor_two_subtrees_uniform_height() {
    let _scope = test_scope();
    let left = decorator(None);
    let right = decorator(None);
    let root = TestRoot::with_child(stack_panel(&[
        decorator(Some(decorator(Some(left.clone())))).upcast(),
        decorator(Some(decorator(Some(right.clone())))).upcast(),
    ]));
    let root_child = root.child().map(|c| v(&c));

    let ancestor = left.find_common_visual_ancestor(&right);
    assert_eq!(root_child, ancestor);

    let ancestor = right.find_common_visual_ancestor(&left);
    assert_eq!(root_child, ancestor);
}

#[test]
fn find_common_visual_ancestor_two_subtrees_non_uniform_height() {
    let _scope = test_scope();
    let left = decorator(None);
    let right = decorator(None);
    let root = TestRoot::with_child(stack_panel(&[
        decorator(Some(decorator(Some(left.clone())))).upcast(),
        decorator(Some(decorator(Some(decorator(Some(right.clone())))))).upcast(),
    ]));
    let root_child = root.child().map(|c| v(&c));

    let ancestor = left.find_common_visual_ancestor(&right);
    assert_eq!(root_child, ancestor);

    let ancestor = right.find_common_visual_ancestor(&left);
    assert_eq!(root_child, ancestor);
}

#[test]
fn translate_point_should_respect_render_transforms() {
    let _scope = test_scope();
    let target = Border::new();
    let child = Decorator::new();
    child.set_width(50.0);
    child.set_height(50.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_render_transform(Some(TranslateTransform::with_offset(25.0, 25.0).into()));
    child.set_child(target.clone());
    let root = TestRoot::new();
    root.set_width(100.0);
    root.set_height(100.0);
    root.set_child(child);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_size(root.desired_size()));

    let result = target.translate_point(Point::new(0.0, 0.0), &root);

    assert_eq!(Some(Point::new(50.0, 50.0)), result);
}
