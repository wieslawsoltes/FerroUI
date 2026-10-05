use crate::presenters::ContentPresenter;
use crate::primitives::HeaderedContentControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{Control, Panel};
use ferroui_base::{Ref, StyledElement};

#[test]
fn control_header_should_be_logical_child() {
    let target = HeaderedContentControl::new();
    let child = Control::new();

    target.set_header(Some(Control::boxed(&child)));

    assert_eq!(child.parent().unwrap(), target);
    let expected: Vec<Ref<StyledElement>> = vec![child.clone().upcast()];
    assert_eq!(StyledElement::logical_children(&target).to_vec(), expected);

    target.set_header(None);

    assert!(child.parent().is_none());
    assert!(StyledElement::logical_children(&target).is_empty());
}

#[test]
fn registers_content_and_header_presenters_from_the_template() {
    let target = HeaderedContentControl::new();
    target.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let panel = Panel::new();
        for name in ["PART_HeaderPresenter", "PART_ContentPresenter", "Other"] {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some(name.to_string()));
            panel.children().add(presenter.register_in_name_scope(&**scope));
        }
        panel.upcast()
    })));

    target.apply_template();

    assert_eq!(target.header_presenter().unwrap().name().unwrap(), "PART_HeaderPresenter");
    assert_eq!(target.presenter().unwrap().name().unwrap(), "PART_ContentPresenter");
}
