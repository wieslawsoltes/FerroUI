//! Tests of the position a native control host gives its native control
//! while the host is moved by render transforms. Upstream has no tests of
//! the native control host, so none of these is a port.
//!
//! The frames of these tests are the frames of an application: a render
//! pass of the media context (the pulse of the animation clock, then the
//! layout passes) followed by the jobs of the dispatcher, the jobs after
//! the render among them.

use crate::page::{ContentPage, NavigationPage, Page};
use crate::platform::INativeControlHostImpl;
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControlImpl;
use crate::storage_misc_tests::{TestAttachment, TestNativeControlHostImpl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::{Border, Button, Control, NativeControlHost, Panel, Window};
use ferroui_base::animation::{IGlobalClock, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{ITransform, MediaContext, TranslateTransform};
use ferroui_base::threading::{Dispatcher, DispatcherTask};
use ferroui_base::{FerroLocator, Point, Rect, Ref, Size, Thickness, Visual};
use std::rc::Rc;
use std::time::Duration;

fn named_presenter(name: &str) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some(name.to_string()));
    presenter
}

/// The parts of the template of a navigation page that a navigation with a
/// transition needs.
fn navigation_page_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let content_host = Panel::new();
        content_host.set_name(Some("PART_ContentHost".to_string()));
        content_host.children().add(named_presenter("PART_PageBackPresenter").register_in_name_scope(&**ns));
        content_host.children().add(named_presenter("PART_PagePresenter").register_in_name_scope(&**ns));
        let content_host = content_host.register_in_name_scope(&**ns);

        let back_button = Button::new();
        back_button.set_name(Some("PART_BackButton".to_string()));
        let navigation_bar = Border::new();
        navigation_bar.set_name(Some("PART_NavigationBar".to_string()));
        navigation_bar.set_child(back_button.register_in_name_scope(&**ns));
        navigation_bar.set_is_visible(false);

        let panel = Panel::new();
        panel.children().add(navigation_bar.register_in_name_scope(&**ns));
        panel.children().add(content_host);
        panel.children().add(named_presenter("PART_TopCommandBar").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalBackPresenter").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalPresenter").register_in_name_scope(&**ns));
        panel.upcast()
    })
}

/// A page whose template is `content` at the left top corner, 20 from the
/// left and 30 from the top.
fn page_with(content: Option<Ref<Control>>) -> Ref<Page> {
    let page = ContentPage::new();
    page.set_template(Some(FuncControlTemplate::new(move |_, ns| {
        let presenter = named_presenter("PART_ContentPresenter");
        presenter.set_content(content.clone().map(Control::boxed));
        let border = Border::new();
        border.set_padding(Thickness::new(20.0, 30.0, 0.0, 0.0));
        border.set_child(presenter.register_in_name_scope(&**ns));
        border.upcast()
    })));
    page.upcast()
}

fn native_control_host() -> Ref<NativeControlHost> {
    let host = NativeControlHost::new();
    host.set_width(100.0);
    host.set_height(50.0);
    host.set_horizontal_alignment(HorizontalAlignment::Left);
    host.set_vertical_alignment(VerticalAlignment::Top);
    host
}

struct Shell {
    window: Ref<Window>,
    nav: Ref<NavigationPage>,
    host_impl: Rc<TestNativeControlHostImpl>,
}

/// A window of 400 by 300 with a navigation page that slides its pages in
/// and a first page, on the animation clock of the media context.
fn shell() -> Shell {
    let clock: Rc<dyn IGlobalClock> = MediaContext::instance().clock();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(clock);

    let window_impl = MockWindowingPlatform::create_window_mock_with_size(400.0, 300.0);
    let host_impl = TestNativeControlHostImpl::new();
    window_impl.setup_feature::<dyn INativeControlHostImpl>(host_impl.clone());
    let window = Window::with_impl(window_impl);

    let nav = NavigationPage::new();
    let transition: Rc<dyn IPageTransition> =
        Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(60.0), SlideAxis::Horizontal));
    nav.set_page_transition(Some(transition));
    nav.set_template(Some(navigation_page_template()));
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    frame();

    // The first page is shown without a transition.
    let first = nav.push_async(page_with(None));
    frame();
    assert!(first.is_completed());

    Shell { window, nav, host_impl }
}

/// One frame of the application.
fn frame() {
    MediaContext::instance().render();
    Dispatcher::ui_thread().run_jobs(None);
}

/// Runs frames until `task` has completed, and a few more.
fn run_to_end(task: &DispatcherTask<()>) {
    for _ in 0..500 {
        if task.is_completed() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
        frame();
    }
    assert!(task.is_completed(), "the transition did not end");
    for _ in 0..3 {
        frame();
    }
}

fn position_in(host: &NativeControlHost, root: &Visual) -> Rect {
    let matrix = host.transform_to_visual(root).expect("the host is in the tree of the root");
    Rect::from_position_size(Point::default(), host.bounds().size()).transform_to_aabb(matrix)
}

fn last_shown(attachment: &TestAttachment) -> Rect {
    attachment.shown.borrow().last().copied().expect("the native control was shown")
}

#[test]
fn native_control_of_a_page_that_slid_in_is_at_the_position_of_its_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let shell = shell();
    let host = native_control_host();

    let navigation = shell.nav.push_async(page_with(Some(host.clone().upcast())));
    run_to_end(&navigation);

    let expected = Rect::from_position_size(Point::new(20.0, 30.0), Size::new(100.0, 50.0));
    assert_eq!(expected, position_in(&host, &shell.window));
    let attachment = shell.host_impl.attachments.borrow()[0].clone();
    assert_eq!(expected, last_shown(&attachment));
}

#[test]
fn native_control_follows_the_render_transform_of_an_ancestor() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(400.0, 300.0);
    let host_impl = TestNativeControlHostImpl::new();
    window_impl.setup_feature::<dyn INativeControlHostImpl>(host_impl.clone());
    let window = Window::with_impl(window_impl);
    let host = native_control_host();
    let border = Border::new();
    border.set_padding(Thickness::new(20.0, 30.0, 0.0, 0.0));
    border.set_child(host.clone());
    window.set_content(Some(Control::boxed(border.clone())));
    window.show();
    frame();

    let attachment = host_impl.attachments.borrow()[0].clone();
    let at = |x: f64, y: f64| Rect::from_position_size(Point::new(x, y), Size::new(100.0, 50.0));
    assert_eq!(at(20.0, 30.0), last_shown(&attachment));

    // A render transform is set on an ancestor.
    let transform = TranslateTransform::new();
    transform.set_x(200.0);
    let handle: Rc<dyn ITransform> = (&transform).into();
    border.set_render_transform(Some(handle));
    frame();
    assert_eq!(at(220.0, 30.0), last_shown(&attachment));

    // The transform object changes in place, as an animation changes it.
    transform.set_x(50.0);
    frame();
    assert_eq!(at(70.0, 30.0), last_shown(&attachment));

    // The transform is removed.
    border.set_render_transform(None);
    frame();
    assert_eq!(at(20.0, 30.0), last_shown(&attachment));

    // A host that does not move is not moved again.
    let shown = attachment.shown.borrow().len();
    for _ in 0..3 {
        frame();
    }
    assert_eq!(shown, attachment.shown.borrow().len());

    // A transform that was removed is not listened to anymore.
    transform.set_x(300.0);
    frame();
    assert_eq!(shown, attachment.shown.borrow().len());
}
