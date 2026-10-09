//! The test of the upstream `ContentControlTests` of the controls that
//! binds with delayed bindings. `DelayedBinding` is of this library, which
//! the controls are not built on, so the test is here; the other tests of
//! the file are in the tests of the controls.
//!
//! The root of the original is the test root of the test library. The
//! test root of the port is private to the tests of the controls: the root
//! here is a window of the unit test application.

use crate::data::Binding;
use crate::markup::data::DelayedBinding;
use ferroui_base::data::{BindingPriority, RelativeSource, RelativeSourceMode};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{BoxedValue, FerroObject, FerroObjectExtensions, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Canvas, ContentControl, Control, TextBlock, Window};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn binding_content_template_after_content_does_not_leave_orpaned_text_block() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    // Test for the upstream issue 1271.
    let children: Rc<RefCell<Vec<Option<Ref<Control>>>>> = Rc::new(RefCell::new(Vec::new()));
    let presenter = ContentPresenter::new();

    // The content and then the content template property need to be bound with delayed bindings
    // as they are in the XAML library.
    DelayedBinding::add(
        &presenter,
        ContentPresenter::content_property().as_property(),
        Binding::with_path("Content")
            .with_priority(BindingPriority::Template)
            .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
    );

    DelayedBinding::add(
        &presenter,
        ContentPresenter::content_template_property().as_property(),
        Binding::with_path("ContentTemplate")
            .with_priority(BindingPriority::Template)
            .with_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent))),
    );

    let presenter_object: &FerroObject = &presenter;
    let _subscription = FerroObjectExtensions::get_observable(presenter_object, ContentPresenter::child_property())
        .subscribe_fn({
            let children = children.clone();
            move |child: Option<Ref<Control>>| children.borrow_mut().push(child)
        });

    let target = ContentControl::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ContentControl>({
        let presenter = presenter.clone();
        move |_, _| presenter.clone().upcast()
    })));
    target.set_content_template(Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Canvas::new().upcast()), false)));
    let content: BoxedValue = Rc::new("foo".to_string());
    target.set_content(Some(content));

    // The control must be rooted.
    let root = Window::new();
    root.set_content(Some(Control::boxed(target.clone())));
    root.show();

    target.apply_template();

    // When the template is applied, the Content property is bound before the ContentTemplate
    // property, causing a TextBlock to be created by the default template before ContentTemplate
    // is bound.
    let children = children.borrow();
    assert_eq!(3, children.len());
    assert!(children[0].is_none());
    assert!(children[1].as_ref().is_some_and(|child| child.is::<TextBlock>()));
    assert!(children[2].as_ref().is_some_and(|child| child.is::<Canvas>()));

    let text_block = children[1].clone().unwrap();

    // The leak in the upstream issue 1271 was caused by the TextBlock's logical parent not being
    // cleared when it is replaced by the Canvas.
    assert!(text_block.get_logical_parent().is_none());
}
