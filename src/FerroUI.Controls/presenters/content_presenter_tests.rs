//! Ported from the standalone, in-template, layout and unrooted presenter
//! tests of the reference. The default data template creates a
//! `TestTextBlock` (the real text block); a canvas is replaced by a `Panel`.

use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IDataTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, use_test_text_block, TestRoot, TestScope, TestTextBlock};
use crate::{Border, ContentControl, Control, Decorator, Panel};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{BoxedValue, FerroObject, Rect, Ref, Size, StyledElement, Thickness, Visual};
use std::rc::Rc;

fn border_template(supports_recycling: bool) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<String>(|_, _| Some(Border::new().upcast()), supports_recycling)
}

/// A presenter in the template of a rooted content control.
fn create_target() -> (TestScope, Ref<TestRoot>, Ref<ContentPresenter>, Ref<ContentControl>) {
    let scope = test_scope();
    use_test_text_block();
    let templated_parent = ContentControl::new();
    templated_parent.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, s| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**s).upcast()
    })));
    let root = TestRoot::with_child(&templated_parent);
    templated_parent.apply_template();
    let presenter = templated_parent.presenter().unwrap();
    (scope, root, presenter, templated_parent)
}

fn sized_border(min: f64) -> Ref<Border> {
    let content = Border::new();
    content.set_min_width(min);
    content.set_min_height(min);
    content
}

fn layout(target: &ContentPresenter) {
    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
}

// --- layout -----------------------------------------------------------------

#[test]
fn content_alignment_is_applied_to_child_bounds() {
    use HorizontalAlignment as H;
    use VerticalAlignment as V;
    let cases = [
        (H::Stretch, V::Stretch, 0.0, 0.0, 100.0, 100.0),
        (H::Left, V::Stretch, 0.0, 0.0, 16.0, 100.0),
        (H::Right, V::Stretch, 84.0, 0.0, 16.0, 100.0),
        (H::Center, V::Stretch, 42.0, 0.0, 16.0, 100.0),
        (H::Stretch, V::Top, 0.0, 0.0, 100.0, 16.0),
        (H::Stretch, V::Bottom, 0.0, 84.0, 100.0, 16.0),
        (H::Stretch, V::Center, 0.0, 42.0, 100.0, 16.0),
    ];
    for (h, v, x, y, width, height) in cases {
        let content = sized_border(16.0);
        let target = ContentPresenter::new();
        target.set_horizontal_content_alignment(h);
        target.set_vertical_content_alignment(v);
        target.set_content(Some(Control::boxed(&content)));

        layout(&target);

        assert_eq!(content.bounds(), Rect::new(x, y, width, height), "{h:?} {v:?}");
    }
}

#[test]
fn content_alignment_and_padding_are_applied_to_child_bounds() {
    use HorizontalAlignment as H;
    use VerticalAlignment as V;
    let cases = [
        (H::Stretch, V::Stretch, 10.0, 10.0, 80.0, 80.0),
        (H::Left, V::Stretch, 10.0, 10.0, 16.0, 80.0),
        (H::Right, V::Stretch, 74.0, 10.0, 16.0, 80.0),
        (H::Center, V::Stretch, 42.0, 10.0, 16.0, 80.0),
        (H::Stretch, V::Top, 10.0, 10.0, 80.0, 16.0),
        (H::Stretch, V::Bottom, 10.0, 74.0, 80.0, 16.0),
        (H::Stretch, V::Center, 10.0, 42.0, 80.0, 16.0),
    ];
    for (h, v, x, y, width, height) in cases {
        let content = sized_border(16.0);
        let target = ContentPresenter::new();
        target.set_horizontal_content_alignment(h);
        target.set_vertical_content_alignment(v);
        target.set_padding(Thickness::uniform(10.0));
        target.set_content(Some(Control::boxed(&content)));

        layout(&target);

        assert_eq!(content.bounds(), Rect::new(x, y, width, height), "{h:?} {v:?}");
    }
}

#[test]
fn should_correctly_align_child_with_fixed_size() {
    let content = Border::new();
    content.set_horizontal_alignment(HorizontalAlignment::Left);
    content.set_vertical_alignment(VerticalAlignment::Bottom);
    content.set_width(16.0);
    content.set_height(16.0);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    layout(&target);

    assert_eq!(content.bounds(), Rect::new(0.0, 84.0, 16.0, 16.0));
}

#[test]
fn content_can_be_stretched() {
    let content = sized_border(16.0);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    layout(&target);

    assert_eq!(content.bounds(), Rect::new(0.0, 0.0, 100.0, 100.0));
}

#[test]
fn content_can_be_right_aligned() {
    let content = sized_border(16.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    layout(&target);

    assert_eq!(content.bounds(), Rect::new(84.0, 0.0, 16.0, 100.0));
}

#[test]
fn content_can_be_bottom_aligned() {
    let content = sized_border(16.0);
    content.set_vertical_alignment(VerticalAlignment::Bottom);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    layout(&target);

    assert_eq!(content.bounds(), Rect::new(0.0, 84.0, 100.0, 16.0));
}

#[test]
fn content_can_be_top_right_aligned() {
    let content = sized_border(16.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    content.set_vertical_alignment(VerticalAlignment::Top);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    layout(&target);

    assert_eq!(content.bounds(), Rect::new(84.0, 0.0, 16.0, 16.0));
}

#[test]
fn child_arrange_with_zero_height_when_padding_height_greater_than_child_height() {
    let content = Border::new();
    content.set_height(0.0);
    content.set_width(0.0);
    let target = ContentPresenter::new();
    target.set_padding(Thickness::uniform(32.0));
    target.set_max_height(32.0);
    target.set_max_width(32.0);
    target.set_horizontal_content_alignment(HorizontalAlignment::Center);
    target.set_vertical_content_alignment(VerticalAlignment::Center);
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();

    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(32.0, 32.0, 0.0, 0.0));
}

fn rounded_root(target: &Ref<ContentPresenter>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_layout_scaling(1.5);
    root.set_use_layout_rounding(true);
    root.set_child(target);
    root
}

fn sized_panel() -> Option<BoxedValue> {
    let panel = Panel::new();
    panel.set_width(101.0);
    panel.set_height(101.0);
    Some(Control::boxed(panel))
}

#[test]
fn measure_rounds_padding() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_padding(Thickness::uniform(1.0));
    target.set_content(sized_panel());

    let root = rounded_root(&target);
    root.execute_initial_layout_pass();

    // - 1 pixel padding is rounded up to 1.3333; for both sides it is 2.6666
    // - Size of 101 gets rounded up to 101.3333
    // - Desired size = 101.3333 + 2.6666 = 104
    assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
}

#[test]
fn measure_rounds_border_thickness() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_border_thickness(Thickness::uniform(1.0));
    target.set_content(sized_panel());

    let root = rounded_root(&target);
    root.execute_initial_layout_pass();

    assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
}

// --- unrooted ---------------------------------------------------------------

#[test]
fn setting_content_to_control_should_not_set_child_unless_update_child_called() {
    let target = ContentPresenter::new();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    assert!(target.child().is_none());

    target.apply_template();
    assert!(target.child().is_none());

    target.update_child();
    assert_eq!(target.child().unwrap(), child);
}

#[test]
fn setting_content_to_string_should_not_create_text_block_unless_update_child_called() {
    use_test_text_block();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().is_none());

    target.apply_template();
    assert!(target.child().is_none());

    target.update_child();
    let text_block = target.child().unwrap().cast::<TestTextBlock>().unwrap();
    assert_eq!(text_block.text().unwrap(), "Foo");
}

#[test]
fn clearing_control_content_should_remove_child_immediately() {
    let target = ContentPresenter::new();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    target.update_child();
    assert_eq!(target.child().unwrap(), child);

    target.set_content(None);
    assert!(target.child().is_none());
}

#[test]
fn clearing_string_content_should_remove_child_immediately() {
    use_test_text_block();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    target.update_child();
    assert!(target.child().unwrap().is::<TestTextBlock>());

    target.set_content(None);
    assert!(target.child().is_none());
}

#[test]
fn adding_to_logical_tree_should_reevaluate_data_templates() {
    let _scope = test_scope();
    use_test_text_block();
    let root = TestRoot::new();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().is_none());

    root.set_child(&target);
    target.apply_template();
    assert!(target.child().unwrap().is::<TestTextBlock>());

    root.set_child(None);
    let root = TestRoot::new();
    root.data_templates().add(FuncDataTemplate::for_type::<String>(|_, _| Some(Decorator::new().upcast()), false));

    root.set_child(&target);
    target.apply_template();
    assert!(target.child().unwrap().is::<Decorator>());
}

#[test]
fn should_reset_inheritance_parent_when_child_removed() {
    let logical_parent = Panel::new();
    let child = Control::new();
    let target = ContentPresenter::new();

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));
    target.update_child();
    target.set_content(None);
    target.update_child();

    let expected: Ref<FerroObject> = logical_parent.upcast();
    assert_eq!(child.inheritance_parent().unwrap(), expected);
}

// --- standalone -------------------------------------------------------------

#[test]
fn should_set_childs_parent_to_itself_standalone() {
    let _scope = test_scope();
    let content = Border::new();
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));
    let _root = TestRoot::with_child(&target);
    target.update_child();

    assert_eq!(content.parent().unwrap(), target);
}

#[test]
fn should_add_child_to_own_logical_children_standalone() {
    let content = Border::new();
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();

    let expected: Vec<Ref<StyledElement>> = vec![content.upcast()];
    assert_eq!(target.logical_children().to_vec(), expected);
}

#[test]
fn should_remove_old_child_from_logical_children_on_content_changed_standalone() {
    let _scope = test_scope();
    use_test_text_block();
    let target = ContentPresenter::new();
    let _root = TestRoot::with_child(&target);

    target.set_content(boxed_str("foo"));
    target.update_child();
    target.set_content(boxed_str("bar"));
    target.update_child();

    assert_eq!(target.logical_children().count(), 1);
    assert_eq!(target.visual_children().count(), 1);
}

#[test]
fn should_create_child_even_with_null_content_when_content_template_is_set() {
    let target = ContentPresenter::new();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(|_| true, |_, _| Some(TestTextBlock::new().upcast()), false);
    target.set_content_template(Some(template));
    target.set_content(None);

    target.update_child();

    assert!(target.child().unwrap().is::<TestTextBlock>());
}

#[test]
fn should_not_create_child_even_with_null_content_and_data_templates_instead_of_content_template() {
    let target = ContentPresenter::new();
    target.data_templates().add(FuncDataTemplate::new(|_| true, |_, _| Some(TestTextBlock::new().upcast()), false));
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

#[test]
fn should_not_create_child_when_content_and_template_are_null() {
    let target = ContentPresenter::new();
    target.set_content_template(None);
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

#[test]
fn should_not_create_child_when_content_is_null_but_expected_value_type_with_func_data_template() {
    let target = ContentPresenter::new();
    let template: Rc<dyn IDataTemplate> =
        FuncDataTemplate::for_type_with_match::<i32>(|_| true, |_, _| Some(TestTextBlock::new().upcast()), false);
    target.set_content_template(Some(template));
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

// --- in template ------------------------------------------------------------

#[test]
fn should_register_with_host_when_templated_parent_set() {
    let host = ContentControl::new();
    let target = ContentPresenter::new();
    target.set_name(Some("PART_ContentPresenter".to_string()));

    assert!(host.presenter().is_none());

    target.set_templated_parent(host.clone().upcast::<FerroObject>());

    assert_eq!(host.presenter().unwrap(), target);
    assert_eq!(target.host().unwrap(), host);
}

#[test]
fn setting_content_to_control_should_set_child() {
    let (_scope, _root, target, _) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(target.child().unwrap(), child);
}

#[test]
fn setting_content_to_control_should_update_logical_and_visual_tree() {
    let (_scope, _root, target, parent) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(child.parent().unwrap(), parent);
    let logical: Vec<Ref<StyledElement>> = vec![child.clone().upcast()];
    assert_eq!(StyledElement::logical_children(&parent).to_vec(), logical);
    assert_eq!(child.visual_parent().unwrap(), target);
    let visual: Vec<Ref<Visual>> = vec![child.upcast()];
    assert_eq!(target.visual_children().to_vec(), visual);
}

#[test]
fn setting_content_to_string_should_create_text_block_in_trees() {
    let (_scope, _root, target, parent) = create_target();

    target.set_content(boxed_str("Foo"));

    let child = target.child().unwrap();
    assert_eq!(child.cast::<TestTextBlock>().unwrap().text().unwrap(), "Foo");
    assert_eq!(child.parent().unwrap(), parent);
    assert_eq!(child.visual_parent().unwrap(), target);
}

#[test]
fn clearing_control_content_should_update_logical_and_visual_tree() {
    let (_scope, _root, target, parent) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    target.set_content(None);

    assert!(child.parent().is_none());
    assert!(StyledElement::logical_children(&parent).is_empty());
    assert!(child.visual_parent().is_none());
    assert!(target.visual_children().is_empty());
}

#[test]
fn assigning_control_to_content_should_not_set_data_context() {
    let (_scope, _root, target, _) = create_target();
    target.set_content(Some(Control::boxed(Border::new())));

    assert!(!target.is_set(StyledElement::data_context_property().as_property()));
}

#[test]
fn assigning_non_control_to_content_should_set_data_context_on_update_child() {
    let (_scope, _root, target, _) = create_target();
    target.set_content(boxed_str("foo"));

    assert_eq!(string_of(&target.data_context().unwrap()).unwrap(), "foo");
}

#[test]
fn should_use_content_template_if_specified() {
    let (_scope, _root, target, _) = create_target();

    target.set_content_template(Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Panel::new().upcast()), false)));
    target.set_content(boxed_str("Foo"));

    assert!(target.child().unwrap().is::<Panel>());
}

#[test]
fn should_update_if_content_template_changed() {
    let (_scope, _root, target, _) = create_target();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().unwrap().is::<TestTextBlock>());

    target.set_content_template(Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Panel::new().upcast()), false)));
    assert!(target.child().unwrap().is::<Panel>());

    target.set_content_template(None);
    assert!(target.child().unwrap().is::<TestTextBlock>());
}

#[test]
fn assigning_control_to_content_after_non_control_should_clear_data_context() {
    let (_scope, _root, target, _) = create_target();

    target.set_content(boxed_str("foo"));
    assert_eq!(string_of(&target.data_context().unwrap()).unwrap(), "foo");

    target.set_content(Some(Control::boxed(Border::new())));
    assert!(target.data_context().is_none());
}

#[test]
fn recycles_data_template() {
    let (_scope, _root, target, _) = create_target();
    target.data_templates().add(border_template(true));

    target.set_content(boxed_str("foo"));

    let control = target.child().unwrap();
    assert!(control.is::<Border>());

    target.set_content(boxed_str("bar"));
    assert_eq!(target.child().unwrap(), control);
}

#[test]
fn detects_data_template_doesnt_match_and_doesnt_recycle() {
    let (_scope, _root, target, _) = create_target();
    target.data_templates().add(FuncDataTemplate::for_type_with_match::<String>(
        |x| x == "foo",
        |_, _| Some(Border::new().upcast()),
        true,
    ));

    target.set_content(boxed_str("foo"));
    assert!(target.child().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert!(target.child().unwrap().is::<TestTextBlock>());
}

#[test]
fn detects_data_template_doesnt_support_recycling() {
    let (_scope, _root, target, _) = create_target();
    target.data_templates().add(border_template(false));

    target.set_content(boxed_str("foo"));

    let control = target.child().unwrap();
    assert!(control.is::<Border>());

    target.set_content(boxed_str("bar"));
    assert_ne!(target.child().unwrap(), control);
}

#[test]
fn reevaluates_data_templates_when_recycling() {
    let (_scope, _root, target, _) = create_target();
    target.data_templates().add(FuncDataTemplate::for_type_with_match::<String>(
        |x| x == "bar",
        |_, _| Some(Panel::new().upcast()),
        true,
    ));
    target.data_templates().add(border_template(true));

    target.set_content(boxed_str("foo"));
    assert!(target.child().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert!(target.child().unwrap().is::<Panel>());
}

#[test]
fn should_set_inheritance_parent_even_when_logical_parent_is_already_set() {
    let (_scope, _root, target, _) = create_target();
    let logical_parent = Panel::new();
    let child = Control::new();

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));

    let expected: Ref<FerroObject> = target.clone().upcast();
    assert_eq!(child.inheritance_parent().unwrap(), expected);
}

#[test]
fn should_clear_host_when_host_template_cleared() {
    let (_scope, _root, target, host) = create_target();

    assert_eq!(target.host().unwrap(), host);

    host.set_template(None);
    host.apply_template();

    assert!(target.host().is_none());
}

#[test]
fn set_content_with_data_context_overrides_the_data_context() {
    let (_scope, _root, target, _) = create_target();

    target.set_content_with_data_context(boxed_str("foo"), boxed_str("context"));
    assert_eq!(string_of(&target.data_context().unwrap()).unwrap(), "context");

    // Unchanged content: the data context is applied directly.
    target.set_content_with_data_context(boxed_str("foo"), boxed_str("other"));
    assert_eq!(string_of(&target.data_context().unwrap()).unwrap(), "other");
}
