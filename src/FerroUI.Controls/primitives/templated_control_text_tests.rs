//! The text property tests of the templated control tests of the reference,
//! which were deferred until the text elements existed.

use crate::documents::TextElement;
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::ContentControl;
use ferroui_base::data::TemplateBinding;

#[test]
fn templated_control_letter_spacing_default_value_is_zero() {
    let target = TemplatedControl::new();
    assert_eq!(target.letter_spacing(), 0.0);
}

#[test]
fn templated_control_letter_spacing_uses_text_element_property() {
    assert!(std::ptr::eq(
        TextElement::letter_spacing_property().as_property(),
        TemplatedControl::letter_spacing_property().as_property()
    ));
}

#[test]
fn templated_control_letter_spacing_can_be_set_and_retrieved() {
    let target = TemplatedControl::new();
    target.set_letter_spacing(2.5);
    assert_eq!(target.letter_spacing(), 2.5);
}

#[test]
fn templated_control_letter_spacing_can_be_negative() {
    let target = TemplatedControl::new();
    target.set_letter_spacing(-1.5);
    assert_eq!(target.letter_spacing(), -1.5);
}

/// The reference test takes the template of the content control from the
/// theme; here it is the content presenter the theme template contains.
#[test]
fn templated_control_letter_spacing_inherits_to_content_presenter() {
    let _scope = test_scope();
    let target = ContentControl::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let property = ContentControl::content_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    target.set_letter_spacing(3.0);
    target.set_content(boxed_str("Test"));
    let _root = TestRoot::with_child(&target);

    target.apply_template();

    let presenter = target.presenter().unwrap();
    assert_eq!(presenter.letter_spacing(), 3.0);
}
