//! Ported from the upstream `UserControlTests`.

use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, ContentControl, UserControl};
use ferroui_base::data::BindingMode;
use ferroui_base::media::SolidColorBrush;
use ferroui_base::styling::{Selectors, Setter, Style};
use std::rc::Rc;

#[test]
fn should_be_styled_as_user_control() {
    let _scope = test_scope();
    let target = UserControl::new();
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<UserControl>(),
        [Setter::new(TemplatedControl::template_property(), get_template())],
    ));
    root.set_child(&target);

    assert!(target.template().is_some());
}

fn get_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<UserControl>(|parent, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().bind().with_mode(BindingMode::TwoWay),
            &parent.indexer(&ContentControl::content_property().bind().with_mode(BindingMode::TwoWay)),
        );
        let border = Border::new();
        border.set_background(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    }))
}
