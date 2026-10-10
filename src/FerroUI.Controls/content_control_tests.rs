//! The default data template creates a `TestTextBlock` in these tests (the
//! real text block); a canvas is replaced by a `Panel`.

use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, use_test_text_block, TestRoot, TestTextBlock};
use crate::{Border, ContentControl, Control, Panel};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::TemplateBinding;
use ferroui_base::media::Brushes;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{AnyValue, BoxedValue, FerroObject, Ref, StyledElement, Visual};
use std::cell::Cell;
use std::rc::Rc;

fn get_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        for property in [
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ] {
            presenter.bind_binding(property, &TemplateBinding::new(property));
        }
        let border = Border::new();
        border.set_background(Some(Brushes::white()));
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    }))
}

fn single(visual: &Visual) -> Ref<Visual> {
    let children = visual.visual_children().to_vec();
    assert_eq!(children.len(), 1);
    children[0].clone()
}

fn logical(children: &[&Ref<Control>]) -> Vec<Ref<StyledElement>> {
    children.iter().map(|c| (*c).clone().upcast()).collect()
}

fn track(target: &ContentControl, predicate: impl Fn(NotifyCollectionChangedAction) -> bool + 'static) -> Rc<Cell<bool>> {
    let called = Rc::new(Cell::new(false));
    let result = called.clone();
    StyledElement::logical_children(target).add_collection_changed(Rc::new(
        move |e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>| called.set(predicate(e.action)),
    ));
    result
}

#[test]
fn empty_pseudo_class_should_track_content() {
    let target = ContentControl::new();
    assert!(target.classes().contains(":empty"));
    target.set_content(boxed_str("Content"));
    assert!(!target.classes().contains(":empty"));
    target.set_content(None);
    assert!(target.classes().contains(":empty"));
}

#[test]
fn template_should_be_instantiated() {
    use_test_text_block();
    let target = ContentControl::new();
    target.set_content(boxed_str("Foo"));
    target.set_template(get_template());
    target.apply_template();
    target.presenter().unwrap().update_child();

    let child = single(&target);
    assert!(child.is::<Border>());
    let child = single(&child);
    assert!(child.is::<ContentPresenter>());
    let child = single(&child);
    assert!(child.is::<TestTextBlock>());
}

#[test]
fn templated_children_should_be_styled() {
    let _scope = test_scope();
    use_test_text_block();
    let root = TestRoot::new();
    let foo: BoxedValue = Rc::new("foo".to_string());
    root.styles().add(Style::with_setters(Selectors::is::<Control>(), [Setter::new(Control::tag_property(), Some(foo))]));

    let target = ContentControl::new();
    target.set_content(boxed_str("Foo"));
    target.set_template(get_template());
    root.set_child(&target);
    target.apply_template();
    target.presenter().unwrap().apply_template();

    let descendants = target.get_template_descendants();
    assert!(!descendants.is_empty());
    for child in descendants {
        let tag = child.cast::<Control>().unwrap().tag().unwrap();
        let tag: &dyn AnyValue = &*tag;
        assert_eq!(tag.downcast_ref::<String>().unwrap(), "foo");
    }
}

#[test]
fn content_presenter_should_have_templated_parent_set() {
    let target = ContentControl::new();
    let child = Border::new();

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(&child)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let content_presenter = child.get_visual_parent_of_type::<ContentPresenter>().unwrap();
    let expected: Option<Ref<FerroObject>> = Some(target.clone().upcast());
    assert_eq!(content_presenter.templated_parent(), expected);
}

#[test]
fn content_should_have_templated_parent_set_to_null() {
    let target = ContentControl::new();
    let child = Border::new();

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(&child)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    assert!(child.templated_parent().is_none());
}

#[test]
fn control_content_should_be_logical_child_before_apply_template() {
    let target = ContentControl::new();
    target.set_template(get_template());

    let child = Control::new();
    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(child.parent().unwrap(), target);
    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&child]));
}

#[test]
fn control_content_should_be_logical_child_after_apply_template() {
    let target = ContentControl::new();
    target.set_template(get_template());

    let child = Control::new();
    target.set_content(Some(Control::boxed(&child)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    assert_eq!(child.parent().unwrap(), target);
    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&child]));
}

#[test]
fn should_use_content_template_to_create_control() {
    let target = ContentControl::new();
    target.set_template(get_template());
    let content_template: Rc<dyn IDataTemplate> =
        FuncDataTemplate::for_type::<String>(|_, _| Some(Panel::new().upcast()), false);
    target.set_content_template(Some(content_template));

    target.set_content(boxed_str("Foo"));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let child = target.presenter().unwrap().child().unwrap();

    assert!(child.is::<Panel>());
}

#[test]
fn data_template_created_control_should_be_logical_child_after_apply_template() {
    use_test_text_block();
    let target = ContentControl::new();
    target.set_template(get_template());

    target.set_content(boxed_str("Foo"));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let child = target.presenter().unwrap().child().unwrap();

    assert_eq!(child.parent().unwrap(), target);
    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&child]));
}

#[test]
fn clearing_content_should_clear_logical_child() {
    let target = ContentControl::new();
    let child = Control::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&child]));

    target.set_content(None);

    assert!(child.parent().is_none());
    assert!(StyledElement::logical_children(&target).is_empty());
}

#[test]
fn setting_content_should_fire_logical_children_collection_changed() {
    let target = ContentControl::new();
    let child = Control::new();
    let called = track(&target, |action| action == NotifyCollectionChangedAction::Add);

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(&child)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    assert!(called.get());
}

#[test]
fn clearing_content_should_fire_logical_children_collection_changed() {
    let target = ContentControl::new();
    let child = Control::new();

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(&child)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let called = track(&target, |_| true);

    target.set_content(None);
    target.presenter().unwrap().update_child();

    assert!(called.get());
}

#[test]
fn changing_content_should_fire_logical_children_collection_changed() {
    let target = ContentControl::new();
    let child1 = Control::new();
    let child2 = Control::new();

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(&child1)));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let called = track(&target, |_| true);

    target.set_content(Some(Control::boxed(&child2)));
    target.presenter().unwrap().apply_template();

    assert!(called.get());
}

#[test]
fn changing_content_should_update_presenter() {
    use_test_text_block();
    let target = ContentControl::new();

    target.set_template(get_template());
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.update_child();

    target.set_content(boxed_str("Foo"));
    presenter.update_child();
    assert_eq!(presenter.child().unwrap().cast::<TestTextBlock>().unwrap().text().unwrap(), "Foo");
    target.set_content(boxed_str("Bar"));
    presenter.update_child();
    assert_eq!(presenter.child().unwrap().cast::<TestTextBlock>().unwrap().text().unwrap(), "Bar");
}

#[test]
fn data_context_should_be_set_for_data_template_created_content() {
    use_test_text_block();
    let target = ContentControl::new();

    target.set_template(get_template());
    target.set_content(boxed_str("Foo"));
    target.apply_template();
    target.presenter().unwrap().update_child();

    let data_context = target.presenter().unwrap().child().unwrap().data_context().unwrap();
    assert_eq!(string_of(&data_context).unwrap(), "Foo");
}

#[test]
fn data_context_should_not_be_set_for_control_content() {
    let target = ContentControl::new();

    target.set_template(get_template());
    target.set_content(Some(Control::boxed(TestTextBlock::new())));
    target.apply_template();
    target.presenter().unwrap().update_child();

    assert!(target.presenter().unwrap().child().unwrap().data_context().is_none());
}

#[test]
fn should_set_child_logical_parent_after_removing_and_adding_back_to_logical_tree() {
    let _scope = test_scope();
    use_test_text_block();
    let target = ContentControl::new();
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<ContentControl>(),
        [Setter::new(TemplatedControl::template_property(), get_template())],
    ));
    root.set_child(&target);

    target.set_content(boxed_str("Foo"));
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.apply_template();

    assert_eq!(presenter.child().unwrap().parent().unwrap(), target);
    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&presenter.child().unwrap()]));

    root.set_child(None);

    target.set_content(None);

    assert!(StyledElement::logical_children(&target).is_empty());

    root.set_child(&target);
    target.set_content(boxed_str("Bar"));

    assert_eq!(presenter.child().unwrap().parent().unwrap(), target);
    assert_eq!(StyledElement::logical_children(&target).to_vec(), logical(&[&presenter.child().unwrap()]));
}

#[test]
fn default_template_creates_the_presenter_bound_to_the_control() {
    let target = ContentControl::new();
    let child = Border::new();
    target.set_content(Some(Control::boxed(&child)));
    target.set_padding(ferroui_base::Thickness::uniform(3.0));

    target.apply_template();

    let presenter = target.presenter().unwrap();
    assert_eq!(single(&target), presenter);
    assert_eq!(presenter.padding(), ferroui_base::Thickness::uniform(3.0));
    presenter.update_child();
    assert_eq!(presenter.child().unwrap(), child);
}

/// Not from upstream's unit tests; the scene is the button template of
/// upstream's render test of the pips pager (`Controls/PipsPagerTests.cs`:
/// `[!TextBlock.TextProperty] = b[!ContentControl.ContentProperty]`). The
/// indexer binding of a property of type `object` to a property of another
/// type gives the property the value itself when it is of that type.
#[test]
fn indexer_binding_of_content_gives_text_its_string() {
    let _scope = test_scope();
    let target = ContentControl::new();
    let text = crate::TextBlock::new();

    target.set_content(boxed_str("<"));
    text.bind_indexer(
        &crate::TextBlock::text_property().as_property().bind(),
        &target.indexer(&ContentControl::content_property().as_property().bind()),
    );
    assert_eq!(Some("<".to_string()), text.text());

    target.set_content(boxed_str(">"));
    assert_eq!(Some(">".to_string()), text.text());

    // A value that is not text is not the text.
    target.set_content(Some(Rc::new(5i32) as BoxedValue));
    assert_eq!(None, text.text());

    target.set_content(None);
    assert_eq!(None, text.text());
}
