//! The tests of the upstream `TemplateBindingTests` whose template has a
//! tooltip, a popup or a flyout: they need the controls and a window, which
//! the other tests of the file (`template_binding_tests`) do without.

use ferroui_base::data::TemplateBinding;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::Popup;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, ContentControl, Control, Decorator, Flyout, TextBlock, ToolTip, Window};
use std::rc::Rc;

fn string(value: &str) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(value.to_string());
    Some(value)
}

/// `new TextBlock { [~TextBlock.TextProperty] = new TemplateBinding(ContentControl.ContentProperty) }`.
fn bind_text_to_content(target: &TextBlock) {
    target.bind_binding(
        TextBlock::text_property().as_property(),
        &TemplateBinding::new(ContentControl::content_property().as_property()),
    );
}

/// Closes the window when the test ends, however it ends (the `finally` of the original).
struct CloseWindow(Ref<Window>);

impl Drop for CloseWindow {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn single_visual_child(source: &Button) -> Ref<Control> {
    let children = source.get_visual_children();
    assert_eq!(1, children.len());
    children[0].cast::<Control>().expect("the template child is a control")
}

#[test]
fn should_work_inside_of_tooltip() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let source = Button::new();
    source.set_template(Some(FuncControlTemplate::for_type::<Button>(|_parent, _| {
        let text_block = TextBlock::new();
        bind_text_to_content(&text_block);
        let decorator = Decorator::new();
        ToolTip::set_tip(&decorator, Some(Control::boxed(text_block)));
        decorator.upcast()
    })));

    window.set_content(Some(Control::boxed(source.clone())));
    window.show();
    let _close = CloseWindow(window.clone());

    let template_child = single_visual_child(&source).cast::<Decorator>().expect("a decorator");
    ToolTip::set_is_open(&template_child, true);

    let tip = ToolTip::get_tip(&template_child).expect("the tip");
    let target = Control::from_boxed(&tip).and_then(|tip| tip.cast::<TextBlock>()).expect("a text block");

    assert_eq!(None, target.text());
    source.set_content(string("foo"));
    assert_eq!(Some("foo".to_string()), target.text());
    source.set_content(string("bar"));
    assert_eq!(Some("bar".to_string()), target.text());
}

#[test]
fn should_work_inside_of_popup() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let source = Button::new();
    source.set_template(Some(FuncControlTemplate::for_type::<Button>(|_parent, _| {
        let text_block = TextBlock::new();
        bind_text_to_content(&text_block);
        let popup = Popup::new();
        popup.set_child(text_block);
        popup.upcast()
    })));

    window.set_content(Some(Control::boxed(source.clone())));
    window.show();
    let _close = CloseWindow(window.clone());

    let popup = single_visual_child(&source).cast::<Popup>().expect("a popup");
    popup.set_is_open(true);

    let target = popup.child().and_then(|child| child.cast::<TextBlock>()).expect("a text block");

    bind_text_to_content(&target);
    assert_eq!(None, target.text());
    source.set_content(string("foo"));
    assert_eq!(Some("foo".to_string()), target.text());
    source.set_content(string("bar"));
    assert_eq!(Some("bar".to_string()), target.text());
}

#[test]
fn should_work_inside_of_flyout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let source = Button::new();
    source.set_template(Some(FuncControlTemplate::for_type::<Button>(|_parent, _| {
        let text_block = TextBlock::new();
        bind_text_to_content(&text_block);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(text_block)));
        let button = Button::new();
        button.set_flyout(&flyout);
        button.upcast()
    })));

    window.set_content(Some(Control::boxed(source.clone())));
    window.show();
    let _close = CloseWindow(window.clone());

    let template_child = single_visual_child(&source).cast::<Button>().expect("a button");
    let flyout = template_child.flyout().expect("the flyout");
    flyout.show_at(&template_child);

    let content = flyout.cast::<Flyout>().expect("a flyout").content().expect("the content of the flyout");
    let target = Control::from_boxed(&content).and_then(|content| content.cast::<TextBlock>()).expect("a text block");

    bind_text_to_content(&target);
    assert_eq!(None, target.text());
    source.set_content(string("foo"));
    assert_eq!(Some("foo".to_string()), target.text());
    source.set_content(string("bar"));
    assert_eq!(Some("bar".to_string()), target.text());
}
