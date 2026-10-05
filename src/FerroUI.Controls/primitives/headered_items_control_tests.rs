use super::{HeaderedItemsControl, TemplatedControl};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, test_scope};
use crate::{Border, Control};
use ferroui_base::data::TemplateBinding;
use ferroui_base::{Ref, StyledElement};
use std::rc::Rc;

fn get_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<HeaderedItemsControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_HeaderPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(HeaderedItemsControl::header_property().as_property()),
        );
        let border = Border::new();
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    }))
}

fn logical(children: &[&Ref<Control>]) -> Vec<Ref<StyledElement>> {
    children.iter().map(|c| (*c).clone().upcast()).collect()
}

#[test]
fn control_header_should_be_logical_child_before_apply_template() {
    let target = HeaderedItemsControl::new();
    TemplatedControl::set_template(&target, get_template());

    let child = Control::new();
    target.set_header(Some(Control::boxed(child.clone())));

    assert_eq!(child.parent(), Some(target.clone().upcast()));
    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));
}

#[test]
fn data_template_created_control_should_be_logical_child_after_apply_template() {
    let _scope = test_scope();
    let target = HeaderedItemsControl::new();
    TemplatedControl::set_template(&target, get_template());

    target.set_header(boxed_str("Foo"));
    target.apply_template();
    target.header_presenter().unwrap().update_child();

    let child = target.header_presenter().unwrap().child().unwrap();

    assert_eq!(child.parent(), Some(target.clone().upcast()));
    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));
}

#[test]
fn clearing_content_should_clear_logical_child() {
    let target = HeaderedItemsControl::new();
    let child = Control::new();

    target.set_header(Some(Control::boxed(child.clone())));
    target.set_header(None);

    assert!(child.parent().is_none());
    assert!(target.logical_children().is_empty());
}
