use super::{ContentPage, NavigationPage, Page, PageImpl, TabbedPage};
use crate::primitives::TemplatedControlImpl;
use crate::templates::{FuncDataTemplate, IDataTemplate};
use crate::test_support::{boxed_str, string_of, test_scope};
use crate::{Button, Control, ControlImpl, Image, TextBlock};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl, Thickness, VisualImpl,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Whether the untyped value holds exactly this control.
fn is_control(value: &Option<BoxedValue>, control: &Control) -> bool {
    value.as_ref().and_then(Control::from_boxed).is_some_and(|held| std::ptr::eq::<Control>(&*held, control))
}

fn logical_children_contain(page: &ContentPage, child: &Control) -> bool {
    let child: &StyledElement = child;
    StyledElement::logical_children(page).to_vec().iter().any(|c| std::ptr::eq::<StyledElement>(&**c, child))
}

// --- PageDefaults ---

#[test]
fn header_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.header().is_none());
}

#[test]
fn header_set_string_round_trips() {
    let _scope = test_scope();
    for title in ["Home", "Settings", ""] {
        let page = ContentPage::new();
        page.set_header(boxed_str(title));
        assert_eq!(page.header().as_ref().and_then(string_of).as_deref(), Some(title));
    }
}

#[test]
fn header_set_control_round_trips() {
    let _scope = test_scope();
    let label = TextBlock::new();
    label.set_text(Some("Custom Title"));
    let page = ContentPage::new();
    page.set_header(Some(Control::boxed(label.clone())));
    assert!(is_control(&page.header(), &label));
}

#[test]
fn icon_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.icon().is_none());
}

#[test]
fn icon_round_trips() {
    let _scope = test_scope();
    let icon = Image::new();
    let page = ContentPage::new();
    page.set_icon(Some(Control::boxed(icon.clone())));
    assert!(is_control(&page.icon(), &icon));
}

#[test]
fn safe_area_padding_default_is_zero() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert_eq!(Thickness::default(), page.safe_area_padding());
}

#[test]
fn safe_area_padding_round_trips() {
    let _scope = test_scope();
    let page = ContentPage::new();
    let padding = Thickness::new(10.0, 20.0, 10.0, 30.0);
    page.set_safe_area_padding(padding);
    assert_eq!(padding, page.safe_area_padding());
}

#[test]
fn is_in_navigation_page_default_is_false() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(!page.is_in_navigation_page());
}

#[test]
fn navigation_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.navigation().is_none());
}

#[test]
fn current_page_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.current_page().is_none());
}

// --- ContentPropertyTests ---

#[test]
fn content_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.content().is_none());
}

#[test]
fn content_set_string_round_trips() {
    let _scope = test_scope();
    let page = ContentPage::new();
    page.set_content(boxed_str("Hello"));
    assert_eq!(page.content().as_ref().and_then(string_of).as_deref(), Some("Hello"));
}

#[test]
fn content_set_control_round_trips() {
    let _scope = test_scope();
    let ctrl = Button::new();
    let page = ContentPage::new();
    page.set_content(Some(Control::boxed(ctrl.clone())));
    assert!(is_control(&page.content(), &ctrl));
}

#[test]
fn content_set_any_page_subtype_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let children: [Ref<Page>; 3] =
        [ContentPage::new().upcast(), NavigationPage::new().upcast(), TabbedPage::new().upcast()];
    for child in children {
        let host = ContentPage::new();
        let result = catch_unwind(AssertUnwindSafe(|| host.set_content(Some(Control::boxed(child.clone())))));
        assert!(result.is_err());
    }
}

#[test]
fn automatically_apply_safe_area_padding_default_is_true() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.automatically_apply_safe_area_padding());
}

#[test]
fn automatically_apply_safe_area_padding_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let page = ContentPage::new();
        page.set_automatically_apply_safe_area_padding(value);
        assert_eq!(value, page.automatically_apply_safe_area_padding());
    }
}

#[test]
fn horizontal_content_alignment_default_is_stretch() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert_eq!(HorizontalAlignment::Stretch, page.horizontal_content_alignment());
}

#[test]
fn horizontal_content_alignment_round_trips() {
    let _scope = test_scope();
    for value in [
        HorizontalAlignment::Left,
        HorizontalAlignment::Center,
        HorizontalAlignment::Right,
        HorizontalAlignment::Stretch,
    ] {
        let page = ContentPage::new();
        page.set_horizontal_content_alignment(value);
        assert_eq!(value, page.horizontal_content_alignment());
    }
}

#[test]
fn vertical_content_alignment_default_is_stretch() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert_eq!(VerticalAlignment::Stretch, page.vertical_content_alignment());
}

#[test]
fn vertical_content_alignment_round_trips() {
    let _scope = test_scope();
    for value in
        [VerticalAlignment::Top, VerticalAlignment::Center, VerticalAlignment::Bottom, VerticalAlignment::Stretch]
    {
        let page = ContentPage::new();
        page.set_vertical_content_alignment(value);
        assert_eq!(value, page.vertical_content_alignment());
    }
}

#[test]
fn content_template_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.content_template().is_none());
}

#[test]
fn content_template_round_trips() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(|_| true, |_, _| None, false);
    let page = ContentPage::new();
    page.set_content_template(Some(template.clone()));
    assert!(page.content_template().is_some_and(|t| Rc::ptr_eq(&t, &template)));
}

// --- LogicalChildrenTests ---

#[test]
fn content_set_control_adds_to_logical_children() {
    let _scope = test_scope();
    let page = ContentPage::new();
    let child = Button::new();
    page.set_content(Some(Control::boxed(child.clone())));
    assert!(logical_children_contain(&page, &child));
}

#[test]
fn content_replaced_old_child_removed_new_child_added() {
    let _scope = test_scope();
    let page = ContentPage::new();
    let first = Button::new();
    let second = TextBlock::new();
    page.set_content(Some(Control::boxed(first.clone())));
    page.set_content(Some(Control::boxed(second.clone())));

    assert!(!logical_children_contain(&page, &first));
    assert!(logical_children_contain(&page, &second));
}

#[test]
fn content_set_to_null_removes_old_child() {
    let _scope = test_scope();
    let page = ContentPage::new();
    let child = Button::new();
    page.set_content(Some(Control::boxed(child.clone())));
    page.set_content(None);

    assert!(!logical_children_contain(&page, &child));
}

#[test]
fn content_set_to_string_does_not_add_to_logical_children() {
    let _scope = test_scope();
    let page = ContentPage::new();
    page.set_content(boxed_str("hello"));
    assert_eq!(0, StyledElement::logical_children(&page).count());
}

// --- CommandBarPropertyTests ---

#[test]
fn top_command_bar_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.top_command_bar().is_none());
}

#[test]
fn top_command_bar_round_trips() {
    let _scope = test_scope();
    let bar = Button::new();
    let page = ContentPage::new();
    page.set_top_command_bar(Some(Control::boxed(bar.clone())));
    assert!(is_control(&page.top_command_bar(), &bar));
}

#[test]
fn bottom_command_bar_default_is_null() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(page.bottom_command_bar().is_none());
}

#[test]
fn bottom_command_bar_round_trips() {
    let _scope = test_scope();
    let bar = Button::new();
    let page = ContentPage::new();
    page.set_bottom_command_bar(Some(Control::boxed(bar.clone())));
    assert!(is_control(&page.bottom_command_bar(), &bar));
}

#[test]
fn top_command_bar_and_bottom_command_bar_are_independent() {
    let _scope = test_scope();
    let top = Button::new();
    let bottom = TextBlock::new();
    let page = ContentPage::new();
    page.set_top_command_bar(Some(Control::boxed(top.clone())));
    page.set_bottom_command_bar(Some(Control::boxed(bottom.clone())));
    assert!(is_control(&page.top_command_bar(), &top));
    assert!(is_control(&page.bottom_command_bar(), &bottom));
}

// --- SystemBackButtonTests ---

/// The upstream test class only exposes the protected virtual; here the
/// virtual is callable as it is.
#[repr(C)]
struct TestableContentPage {
    base: ContentPage,
}

ferro_class!(TestableContentPage: ContentPage);
ferro_impl_classes!(
    TestableContentPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);

impl TestableContentPage {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ContentPage::construct() })
    }
}

#[repr(C)]
struct BackButtonHandlingPage {
    base: ContentPage,
}

ferro_class!(BackButtonHandlingPage: ContentPage);
ferro_impl_classes!(
    BackButtonHandlingPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl PageImpl for BackButtonHandlingPage {
    fn on_system_back_button_pressed(_this: &Self) -> bool {
        true
    }
}

impl BackButtonHandlingPage {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ContentPage::construct() })
    }
}

#[test]
fn on_system_back_button_pressed_default_returns_false() {
    let _scope = test_scope();
    let page = TestableContentPage::new();
    assert!(!page.on_system_back_button_pressed());
}

#[test]
fn on_system_back_button_pressed_override_returns_true() {
    let _scope = test_scope();
    let page = BackButtonHandlingPage::new();
    assert!(page.on_system_back_button_pressed());
}
