//! Controls that do not exist yet are replaced by plain controls of this
//! crate where the test only needs "some control" (a text block by
//! `Control`, a canvas by `Panel`, a scroll viewer by `ContentControl`).

use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, ContentControl, Control, Decorator, Panel};
use ferroui_base::data::BindingPriority;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{AnyValue, BoxedValue, FerroObject, FerroObjectExtensions, Ref, Size, TypeInfo, Visual};
use std::cell::Cell;
use std::rc::Rc;

fn template(build: impl Fn() -> Ref<Control> + 'static) -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(move |_, _| build()))
}

fn decorator_panel_tree() -> Ref<Control> {
    let panel = Panel::new();
    panel.children().add(Control::new());
    panel.children().add(Border::new());
    let decorator = Decorator::new();
    decorator.set_child(panel);
    decorator.upcast()
}

fn decorator_with_border() -> Ref<Control> {
    let decorator = Decorator::new();
    decorator.set_child(Border::new());
    decorator.upcast()
}

fn single_visual_child(target: &Visual) -> Ref<Visual> {
    let children = target.visual_children().to_vec();
    assert_eq!(children.len(), 1);
    children[0].clone()
}

fn as_object(target: &Ref<TemplatedControl>) -> Option<Ref<FerroObject>> {
    Some(target.clone().upcast())
}

fn template_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TemplatedControl>(),
        [Setter::new(TemplatedControl::template_property(), template(decorator_with_border))],
    )
}

#[test]
fn template_doesnt_get_executed_on_set() {
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();

    let target = TemplatedControl::new();
    target.set_template(template(move || {
        flag.set(true);
        Control::new()
    }));

    assert!(!executed.get());
}

#[test]
fn template_gets_executed_on_measure() {
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();

    let target = TemplatedControl::new();
    target.set_template(template(move || {
        flag.set(true);
        Control::new()
    }));

    target.measure(Size::new(100.0, 100.0));

    assert!(executed.get());
}

#[test]
fn apply_template_should_create_visual_children() {
    let target = TemplatedControl::new();
    target.set_template(template(decorator_panel_tree));

    target.apply_template();

    let types: Vec<&'static TypeInfo> = target.get_visual_descendants().map(|x| x.get_type()).collect();

    assert_eq!(types, vec![Decorator::TYPE, Panel::TYPE, Control::TYPE, Border::TYPE]);
    assert!(target.logical_children().is_empty());
}

#[test]
fn templated_children_should_have_templated_parent_set() {
    let target = TemplatedControl::new();
    target.set_template(template(decorator_panel_tree));

    target.apply_template();

    let templated_parents: Vec<_> = target.get_visual_descendants().map(|x| x.templated_parent()).collect();

    assert_eq!(templated_parents.len(), 4);
    assert!(templated_parents.iter().all(|x| *x == as_object(&target)));
}

#[test]
fn templated_child_should_have_parent_set() {
    let target = TemplatedControl::new();
    target.set_template(template(|| Decorator::new().upcast()));

    target.apply_template();

    let child = single_visual_child(&target);

    assert_eq!(child.parent().unwrap(), target);
}

#[test]
fn changing_template_should_clear_old_templated_childs_parent() {
    let target = TemplatedControl::new();
    target.set_template(template(|| Decorator::new().upcast()));

    target.apply_template();

    let child = single_visual_child(&target);

    target.set_template(template(|| Panel::new().upcast()));
    target.apply_template();

    assert!(child.parent().is_none());
}

#[test]
fn nested_templated_control_should_not_have_template_applied() {
    let target = TemplatedControl::new();
    target.set_template(template(|| ContentControl::new().upcast()));

    target.apply_template();

    let child = single_visual_child(&target);
    assert!(child.is::<ContentControl>());
    assert!(child.visual_children().is_empty());
}

#[test]
fn templated_children_should_be_styled() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let foo: BoxedValue = Rc::new("foo".to_string());
    root.styles().add(Style::with_setters(Selectors::is::<Control>(), [Setter::new(Control::tag_property(), Some(foo))]));

    let target = TemplatedControl::new();
    target.set_template(template(|| {
        let panel = Panel::new();
        panel.children().add(Control::new());
        panel.upcast()
    }));
    root.set_child(&target);

    target.apply_template();

    let descendants = target.get_template_descendants();
    assert_eq!(descendants.len(), 2);
    for child in descendants {
        let tag = child.cast::<Control>().unwrap().tag().unwrap();
        let tag: &dyn AnyValue = &*tag;
        assert_eq!(tag.downcast_ref::<String>().unwrap(), "foo");
    }
}

#[test]
fn nested_templated_controls_have_correct_templated_parent() {
    let target = TemplatedControl::new();
    target.set_template(template(|| {
        let content_control = ContentControl::new();
        content_control.set_template(Some(FuncControlTemplate::new(|parent, _| {
            let presenter = ContentPresenter::new();
            let parent_object: &FerroObject = parent;
            FerroObject::bind(
                &presenter,
                ContentPresenter::content_property(),
                FerroObjectExtensions::get_observable(parent_object, ContentControl::content_property()),
                BindingPriority::LocalValue,
            );
            let border = Border::new();
            border.set_child(presenter);
            border.upcast()
        })));
        let decorator = Decorator::new();
        decorator.set_child(Control::new());
        content_control.set_content(Some(Control::boxed(decorator)));
        content_control.upcast()
    }));

    target.apply_template();

    let content_control = target.get_template_descendants()[0].cast::<ContentControl>().unwrap();
    content_control.apply_template();

    let descendants = content_control.get_template_descendants();
    let border = descendants.iter().find_map(|x| x.cast::<Border>()).unwrap();
    let presenter = descendants.iter().find_map(|x| x.cast::<ContentPresenter>()).unwrap();
    let decorator = Control::from_boxed(&presenter.content().unwrap()).unwrap().cast::<Decorator>().unwrap();
    let text_block = decorator.child().unwrap();

    let content_control_object: Option<Ref<FerroObject>> = Some(content_control.clone().upcast());
    assert_eq!(content_control.templated_parent(), as_object(&target));
    assert_eq!(border.templated_parent(), content_control_object);
    assert_eq!(presenter.templated_parent(), content_control_object);
    assert_eq!(decorator.templated_parent(), as_object(&target));
    assert_eq!(text_block.templated_parent(), as_object(&target));
}

#[test]
fn apply_template_should_raise_template_applied() {
    let target = TemplatedControl::new();
    target.set_template(template(|| Decorator::new().upcast()));

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    let expected_source = target.clone();

    target.template_applied(move |_, e| {
        assert_eq!(e.routed_event().unwrap(), *TemplatedControl::template_applied_event());
        assert_eq!(e.source().unwrap(), expected_source);
        flag.set(true);
    });

    target.apply_template();

    assert!(raised.get());
}

#[test]
fn applying_new_template_clears_templated_parent_of_old_template_children() {
    let _scope = test_scope();
    let target = TemplatedControl::new();
    target.set_template(template(decorator_with_border));

    target.apply_template();

    let decorator = single_visual_child(&target).cast::<Decorator>().unwrap();
    let border = decorator.child().unwrap();

    assert_eq!(decorator.templated_parent(), as_object(&target));
    assert_eq!(border.templated_parent(), as_object(&target));

    target.set_template(template(|| Panel::new().upcast()));

    // Templated children should not be removed here: the control may be
    // re-added somewhere with the same template, so they could still be of
    // use.
    assert_eq!(single_visual_child(&target), decorator);
    assert_eq!(decorator.templated_parent(), as_object(&target));
    assert_eq!(border.templated_parent(), as_object(&target));

    target.apply_template();

    assert!(decorator.templated_parent().is_none());
    assert!(border.templated_parent().is_none());
}

fn root_with_template_child(template_child: &Ref<Border>) -> (Ref<TestRoot>, Ref<TemplatedControl>) {
    let template_child = template_child.clone();
    let target = TemplatedControl::new();
    target.set_template(template(move || {
        let decorator = Decorator::new();
        decorator.set_child(&template_child);
        decorator.upcast()
    }));
    (TestRoot::with_child(&target), target)
}

#[test]
fn template_child_attached_to_logical_tree_should_be_raised() {
    let _scope = test_scope();
    let template_child = Border::new();
    let (_root, target) = root_with_template_child(&template_child);

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    template_child.attached_to_logical_tree(move |_| flag.set(true));

    target.apply_template();
    assert!(raised.get());
}

#[test]
fn template_child_detached_from_logical_tree_should_be_raised() {
    let _scope = test_scope();
    let template_child = Border::new();
    let (root, target) = root_with_template_child(&template_child);

    target.apply_template();

    let raised = Rc::new(Cell::new(false));
    let flag = raised.clone();
    template_child.detached_from_logical_tree(move |_| flag.set(true));

    root.set_child(None);
    assert!(raised.get());
}

#[test]
fn removing_from_logical_tree_should_not_remove_child() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.styles().add(template_style());
    let target = TemplatedControl::new();
    root.set_child(&target);

    assert!(target.template().is_some());
    target.apply_template();

    root.set_child(None);

    assert!(target.template().is_none());
    assert!(single_visual_child(&target).is::<Decorator>());
}

#[test]
fn re_adding_to_same_logical_tree_should_not_recreate_template() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.styles().add(template_style());
    let target = TemplatedControl::new();
    root.set_child(&target);

    assert!(target.template().is_some());
    target.apply_template();
    let expected = single_visual_child(&target);

    root.set_child(None);
    root.set_child(&target);
    target.apply_template();

    assert_eq!(single_visual_child(&target), expected);
}

#[test]
fn re_adding_to_different_logical_tree_should_recreate_template() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.styles().add(template_style());
    let target = TemplatedControl::new();
    root.set_child(&target);

    let root2 = TestRoot::new();
    root2.styles().add(template_style());

    assert!(target.template().is_some());
    target.apply_template();

    let expected = single_visual_child(&target);

    root.set_child(None);
    root2.set_child(&target);
    target.apply_template();

    let child = single_visual_child(&target);
    assert!(target.template().is_some());
    assert_ne!(child, expected);
}

#[test]
fn moving_to_new_logical_tree_should_detach_attach_template_child() {
    let _scope = test_scope();
    let target = TemplatedControl::new();
    target.set_template(template(|| Decorator::new().upcast()));
    let root = TestRoot::with_child(&target);

    assert!(target.template().is_some());
    target.apply_template();

    let template_child = single_visual_child(&target);
    assert!(template_child.is_attached_to_logical_tree());

    root.set_child(None);
    assert!(!template_child.is_attached_to_logical_tree());

    let _new_root = TestRoot::with_child(&target);
    assert!(template_child.is_attached_to_logical_tree());
}

#[test]
fn clip_to_bounds_defaults_to_true() {
    assert!(TemplatedControl::new().clip_to_bounds());
    assert!(!Control::new().clip_to_bounds());
}

#[test]
fn template_focus_target_is_the_marked_template_child() {
    let target = TemplatedControl::new();
    assert_eq!(target.get_template_focus_target().unwrap(), target);

    target.set_template(template(|| {
        let border = Border::new();
        TemplatedControl::set_is_template_focus_target(&border, true);
        let decorator = Decorator::new();
        decorator.set_child(border);
        decorator.upcast()
    }));
    target.apply_template();

    assert!(target.get_template_focus_target().unwrap().is::<Border>());
}

/// A content control with the resource "red" whose template is a content
/// presenter with the background bound to that resource.
fn content_control_with_resource_bound_presenter() -> Ref<ContentControl> {
    use crate::templates::FuncTemplateNameScopeExtensions;
    use ferroui_base::controls::ResourceHostRef;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::reactive::ObservableExt;

    let target = ContentControl::new();
    target.resources().add_value("red", Brushes::red());
    target.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|x, scope| {
        let result = ContentPresenter::new();
        result.set_name(Some("PART_ContentPresenter".to_string()));
        let parent_object: &FerroObject = x;
        FerroObject::bind(
            &result,
            ContentPresenter::content_property(),
            FerroObjectExtensions::get_observable(parent_object, ContentControl::content_property()),
            BindingPriority::LocalValue,
        );
        let result = result.register_in_name_scope(&**scope);

        // The observable of a resource is untyped; the brush is taken out of
        // the value for the typed binding.
        let background =
            ResourceHostRef::from(&result).resource_observable("red", None).select(|value: Option<BoxedValue>| {
                let brush = value.and_then(|value| value.downcast_ref::<Rc<ImmutableSolidColorBrush>>().cloned())?;
                Some(brush as Rc<dyn IBrush>)
            });
        result.bind(ContentPresenter::background_property(), background, BindingPriority::LocalValue);

        result.upcast()
    })));
    target
}

fn assert_same_brush(
    expected: &Rc<ferroui_base::media::immutable::ImmutableSolidColorBrush>,
    actual: Option<Rc<dyn ferroui_base::media::IBrush>>,
) {
    let actual = actual.expect("a background");
    assert_eq!(Rc::as_ptr(expected) as *const (), Rc::as_ptr(&actual) as *const ());
}

#[test]
fn templated_child_should_find_resource_in_templated_parent() {
    use ferroui_base::media::Brushes;

    let _scope = test_scope();
    let target = content_control_with_resource_bound_presenter();

    let _root = TestRoot::with_child(&target);
    target.apply_template();

    let content_presenter = single_visual_child(&target).cast::<ContentPresenter>().expect("a content presenter");
    assert_same_brush(&Brushes::red(), content_presenter.background());
}

#[test]
fn changing_resource_in_templated_parent_should_affect_templated_child() {
    use ferroui_base::media::Brushes;

    let _scope = test_scope();
    let target = content_control_with_resource_bound_presenter();

    let _root = TestRoot::with_child(&target);
    target.apply_template();

    let content_presenter = single_visual_child(&target).cast::<ContentPresenter>().expect("a content presenter");
    assert_same_brush(&Brushes::red(), content_presenter.background());

    target.resources().set("red", Some(Rc::new(Brushes::green())));

    assert_same_brush(&Brushes::green(), content_presenter.background());
}
