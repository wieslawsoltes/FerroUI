//! Port of the tests of `StyledElementTests_Resources.cs` (base unit tests)
//! that are not ported elsewhere. The finding of control, parent, style and
//! styles resources is ported in `ferroui-base` (`controls/resource_tests.rs`).
//! The trees use applications, decorators, content controls and a test root,
//! so these tests live with the controls.
//!
//! The settable `StylingParent` of the upstream test root is the
//! `StylingRoot` of the styled element tests.

use crate::presenters::ContentPresenter;
use crate::styled_element_tests::StylingRoot;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{string_of, test_scope};
use crate::{Application, Border, ContentControl, Control, Decorator};
use ferroui_base::controls::ResourceValue;
use ferroui_base::data::TemplateBinding;
use ferroui_base::styling::{Style, StyleHostRef};
use ferroui_base::{Size, StyledElement};
use std::cell::Cell;
use std::rc::Rc;

fn string(value: ResourceValue) -> Option<String> {
    value.as_ref().and_then(string_of)
}

#[test]
fn find_resource_should_find_application_resource() {
    let _scope = test_scope();
    let app = Application::new();
    app.resources().add_value("foo", "foo-value".to_string());

    let target = Control::new();
    // Deviation (DEVIATIONS.md, Tests and test support): the test root of this crate has no settable
    // styling parent; a local derived root supplies it.
    let root = StylingRoot::new();
    root.set_child(target.clone());
    root.set_styling_parent(Some(StyleHostRef::Other(app.as_style_host())));

    assert_eq!(Some("foo-value".to_string()), string(target.find_resource(&"foo".into())));
}

#[test]
fn find_resource_should_find_application_style_resource() {
    let _scope = test_scope();
    let app = Application::new();
    let style = Style::new();
    style.resources().add_value("foo", "foo-value".to_string());
    app.styles().add(&style);
    app.resources().add_value("bar", "bar-value".to_string());

    let target = Control::new();
    // Deviation (DEVIATIONS.md, Tests and test support): the test root of this crate has no settable
    // styling parent; a local derived root supplies it.
    let root = StylingRoot::new();
    root.set_child(target.clone());
    root.set_styling_parent(Some(StyleHostRef::Other(app.as_style_host())));

    assert_eq!(Some("foo-value".to_string()), string(target.find_resource(&"foo".into())));
}

#[test]
fn adding_resource_should_call_raise_resource_changed_on_logical_children() {
    let _scope = test_scope();
    let child = Border::new();
    let target = ContentControl::new();
    target.set_content(Some(Control::boxed(child.clone())));
    target.set_template(content_control_template());

    let raised_on_target = Rc::new(Cell::new(false));
    let raised_on_child = Rc::new(Cell::new(false));

    target.measure(Size::INFINITY);
    let r = raised_on_target.clone();
    target.resources_changed(move |_| r.set(true));
    let r = raised_on_child.clone();
    child.resources_changed(move |_| r.set(true));

    target.resources().add_value("foo", "bar".to_string());

    assert!(raised_on_target.get());
    assert!(raised_on_child.get());
}

#[test]
fn adding_resource_to_styles_should_raise_resource_changed() {
    let _scope = test_scope();
    let target = Decorator::new();
    let raised = Rc::new(Cell::new(false));

    let r = raised.clone();
    target.resources_changed(move |_| r.set(true));
    target.styles().resources().add_value("foo", "bar".to_string());

    assert!(raised.get());
}

#[test]
fn adding_resource_to_nested_style_should_raise_resource_changed() {
    let _scope = test_scope();
    let style = Style::new();
    let target = StyledElement::new();
    target.styles().add(&style);

    let raised = Rc::new(Cell::new(false));

    let r = raised.clone();
    target.resources_changed(move |_| r.set(true));
    style.resources().add_value("foo", "bar".to_string());

    assert!(raised.get());
}

fn content_control_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let property = ContentPresenter::content_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(ContentControl::content_property().as_property()));
        presenter.register_in_name_scope(&**scope).upcast()
    }))
}
