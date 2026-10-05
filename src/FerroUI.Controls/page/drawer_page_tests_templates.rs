//! The drawer page tests, continued: the icon, header and footer template,
//! swipe gesture and detachment groups of the reference tests.

use super::drawer_page_tests::{boxed, pointer_pressed_args, same_template};
use super::navigation_page_tests::{page, same, wait};
use super::{DrawerPage, DrawerPlacement, NavigationPage};
use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::ContentPresenter;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{Border, Canvas, PathIcon, SplitViewDisplayMode, StackPanel, TextBlock};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{MouseButton, SwipeGestureEventArgs};
use ferroui_base::media::EllipseGeometry;
use ferroui_base::{BoxedValue, FerroObject, FerroObjectExtensions, Point, Rect, Ref, Size, Vector};
use std::cell::RefCell;
use std::rc::Rc;

// --- IconTests ---

/// `new FuncDataTemplate<object>((_, _) => new PathIcon())`.
fn path_icon_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, |_, _| Some(PathIcon::new().upcast()), false)
}

#[test]
fn drawer_icon_template_round_trips() {
    let _scope = test_scope();
    let template = path_icon_template();
    let dp = DrawerPage::new();
    dp.set_drawer_icon_template(Some(template.clone()));
    assert!(same_template(&template, &dp.drawer_icon_template()));
}

#[test]
fn drawer_icon_with_geometry_does_not_throw() {
    let _scope = test_scope();
    let geometry: BoxedValue = Rc::new(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
    let dp = DrawerPage::new();
    dp.set_drawer_icon(Some(geometry));
    dp.set_drawer_icon_template(Some(path_icon_template()));
    let _root = TestRoot::with_child(dp.clone());

    let geometry: BoxedValue = Rc::new(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 20.0, 20.0)));
    dp.set_drawer_icon(Some(geometry));
    assert!(dp.drawer_icon().is_some());
}

// --- HeaderFooterTemplateTests ---

fn named_presenter(name: &str) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some(name.to_string()));
    presenter
}

/// Wires the header and footer presenters to the properties of the drawer
/// page. Other parts are omitted; applying the template looks them up as
/// optional parts, so they are safe to skip.
fn minimal_pane_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|parent, scope| {
        let dp: &FerroObject = parent;

        let header = named_presenter("PART_DrawerHeader").register_in_name_scope(&**scope);
        header.bind(
            ContentPresenter::content_property(),
            dp.get_observable(DrawerPage::drawer_header_property()),
            BindingPriority::LocalValue,
        );
        header.bind(
            ContentPresenter::content_template_property(),
            dp.get_observable(DrawerPage::drawer_header_template_property()),
            BindingPriority::LocalValue,
        );

        let footer = named_presenter("PART_DrawerFooter").register_in_name_scope(&**scope);
        footer.bind(
            ContentPresenter::content_property(),
            dp.get_observable(DrawerPage::drawer_footer_property()),
            BindingPriority::LocalValue,
        );
        footer.bind(
            ContentPresenter::content_template_property(),
            dp.get_observable(DrawerPage::drawer_footer_template_property()),
            BindingPriority::LocalValue,
        );

        let panel = StackPanel::new();
        panel.children().add(header);
        panel.children().add(footer);
        panel.upcast()
    })
}

/// The drawer page, its header and footer presenters and the root that
/// keeps the drawer page attached.
struct Created {
    dp: Ref<DrawerPage>,
    header: Ref<ContentPresenter>,
    footer: Ref<ContentPresenter>,
    _root: Ref<TestRoot>,
}

fn create(
    drawer_header: Option<BoxedValue>,
    header_template: Option<Rc<dyn IDataTemplate>>,
    drawer_footer: Option<BoxedValue>,
    footer_template: Option<Rc<dyn IDataTemplate>>,
) -> Created {
    let dp = DrawerPage::new();
    dp.set_template(Some(minimal_pane_template()));
    dp.set_drawer_header(drawer_header);
    dp.set_drawer_header_template(header_template);
    dp.set_drawer_footer(drawer_footer);
    dp.set_drawer_footer_template(footer_template);
    let root = TestRoot::with_child(dp.clone());
    dp.apply_template();

    let presenter = |name: &str| {
        dp.get_visual_descendants()
            .filter_map(|visual| visual.cast::<ContentPresenter>())
            .find(|presenter| presenter.name().as_deref() == Some(name))
            .expect("the presenter is in the template")
    };
    let header = presenter("PART_DrawerHeader");
    let footer = presenter("PART_DrawerFooter");

    Created { dp, header, footer, _root: root }
}

fn create_header(drawer_header: Option<BoxedValue>, header_template: Rc<dyn IDataTemplate>) -> Created {
    create(drawer_header, Some(header_template), None, None)
}

fn create_footer(drawer_footer: Option<BoxedValue>, footer_template: Rc<dyn IDataTemplate>) -> Created {
    create(None, None, drawer_footer, Some(footer_template))
}

/// `new FuncDataTemplate<string>((_, _) => new T())`.
fn string_template(build: fn() -> Ref<crate::Control>) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<String>(move |_, _| Some(build()), false)
}

/// `new FuncDataTemplate<object>((_, _) => new Canvas())`.
fn object_canvas_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, |_, _| Some(Canvas::new().upcast()), false)
}

fn text_block() -> Ref<crate::Control> {
    TextBlock::new().upcast()
}

fn canvas() -> Ref<crate::Control> {
    Canvas::new().upcast()
}

fn border() -> Ref<crate::Control> {
    Border::new().upcast()
}

fn child_is<T: ferroui_base::ObjectType>(presenter: &ContentPresenter) -> bool {
    presenter.child().is_some_and(|child| child.is::<T>())
}

/// A string template that stores the data it is built for.
fn recording_template(received_data: &Rc<RefCell<Option<String>>>) -> Rc<dyn IDataTemplate> {
    let received_data = received_data.clone();
    FuncDataTemplate::for_type::<String>(
        move |data, _| {
            *received_data.borrow_mut() = Some(data.clone());
            let text_block = TextBlock::new();
            text_block.set_text(Some(data));
            Some(text_block.upcast())
        },
        false,
    )
}

#[test]
fn drawer_header_template_is_forwarded_to_content_presenter() {
    let _scope = test_scope();
    let template = string_template(text_block);
    let created = create_header(boxed_str("App"), template.clone());

    assert!(same_template(&template, &created.header.content_template()));
}

#[test]
fn drawer_footer_template_is_forwarded_to_content_presenter() {
    let _scope = test_scope();
    let template = string_template(text_block);
    let created = create_footer(boxed_str("v1.0"), template.clone());

    assert!(same_template(&template, &created.footer.content_template()));
}

#[test]
fn drawer_header_template_renders_control_produced_by_factory() {
    let _scope = test_scope();
    let created = create_header(boxed_str("App"), string_template(canvas));

    created.header.update_child();

    assert!(child_is::<Canvas>(&created.header));
}

#[test]
fn drawer_footer_template_renders_control_produced_by_factory() {
    let _scope = test_scope();
    let created = create_footer(boxed_str("v1.0"), string_template(canvas));

    created.footer.update_child();

    assert!(child_is::<Canvas>(&created.footer));
}

#[test]
fn drawer_header_template_receives_drawer_header_as_data() {
    let _scope = test_scope();
    let received_data = Rc::new(RefCell::new(None));
    let created = create_header(boxed_str("MyTitle"), recording_template(&received_data));

    created.header.update_child();

    assert_eq!(received_data.borrow().as_deref(), Some("MyTitle"));
}

#[test]
fn drawer_footer_template_receives_drawer_footer_as_data() {
    let _scope = test_scope();
    let received_data = Rc::new(RefCell::new(None));
    let created = create_footer(boxed_str("v2.0"), recording_template(&received_data));

    created.footer.update_child();

    assert_eq!(received_data.borrow().as_deref(), Some("v2.0"));
}

#[test]
fn drawer_header_template_swap_template_updates_content_presenter() {
    let _scope = test_scope();
    let second = string_template(border);
    let created = create_header(boxed_str("App"), string_template(canvas));

    created.header.update_child();
    assert!(child_is::<Canvas>(&created.header));

    created.dp.set_drawer_header_template(Some(second));
    created.header.update_child();

    assert!(child_is::<Border>(&created.header));
}

#[test]
fn drawer_footer_template_swap_template_updates_content_presenter() {
    let _scope = test_scope();
    let second = string_template(border);
    let created = create_footer(boxed_str("v1.0"), string_template(canvas));

    created.footer.update_child();
    assert!(child_is::<Canvas>(&created.footer));

    created.dp.set_drawer_footer_template(Some(second));
    created.footer.update_child();

    assert!(child_is::<Border>(&created.footer));
}

#[test]
fn drawer_header_template_clearing_template_falls_back_to_direct_content() {
    let _scope = test_scope();
    let direct_control = TextBlock::new();
    direct_control.set_text(Some("Direct"));
    let created = create_header(boxed(&direct_control), object_canvas_template());

    created.header.update_child();
    assert!(child_is::<Canvas>(&created.header));

    created.dp.set_drawer_header_template(None);
    created.header.update_child();

    assert!(same(&direct_control, &created.header.child()));
}

#[test]
fn drawer_footer_template_clearing_template_falls_back_to_direct_content() {
    let _scope = test_scope();
    let direct_control = TextBlock::new();
    direct_control.set_text(Some("Direct"));
    let created = create_footer(boxed(&direct_control), object_canvas_template());

    created.footer.update_child();
    assert!(child_is::<Canvas>(&created.footer));

    created.dp.set_drawer_footer_template(None);
    created.footer.update_child();

    assert!(same(&direct_control, &created.footer.child()));
}

// --- SwipeGestureTests ---

/// A left, overlay drawer page of 400 x 300 whose swipe recognizer accepts
/// the mouse, laid out in a root of the same size.
fn swipe_drawer_page() -> (Ref<DrawerPage>, Ref<TestRoot>) {
    let dp = DrawerPage::new();
    dp.set_drawer_placement(DrawerPlacement::Left);
    dp.set_display_mode(SplitViewDisplayMode::Overlay);
    dp.set_width(400.0);
    dp.set_height(300.0);
    dp.gesture_recognizers()
        .to_vec()
        .into_iter()
        .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>())
        .expect("the drawer page has a swipe gesture recognizer")
        .set_is_mouse_enabled(true);

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(dp.clone());
    root.execute_initial_layout_pass();
    (dp, root)
}

fn raise_handled_pointer_pressed(target: &DrawerPage, position: Point) {
    let args = pointer_pressed_args(&target.to_ref().upcast(), position);
    args.set_handled(true);

    target.raise_event(&args);
}

#[test]
fn handled_pointer_pressed_at_edge_allows_swipe_open() {
    let _scope = test_scope();
    let (dp, _root) = swipe_drawer_page();

    raise_handled_pointer_pressed(&dp, Point::new(5.0, 5.0));

    let swipe = SwipeGestureEventArgs::new(1, Vector::new(-20.0, 0.0), Vector::default());
    dp.raise_event(&swipe);

    assert!(swipe.handled());
    assert!(dp.is_open());
}

#[test]
fn mouse_edge_drag_allows_swipe_open() {
    let _scope = test_scope();
    let (dp, _root) = swipe_drawer_page();

    let mouse = MouseTestHelper::new();
    mouse.down_at(&dp, MouseButton::Left, Point::new(5.0, 5.0), 1);
    mouse.move_(&dp, Point::new(40.0, 5.0));
    mouse.up_at(&dp, MouseButton::Left, Point::new(40.0, 5.0));

    assert!(dp.is_open());
}

// --- DetachmentTests ---

#[test]
fn on_detached_clears_drawer_page_reference_on_navigation_page() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let nav = NavigationPage::new();
    let dp = DrawerPage::new();
    dp.set_content(boxed(&nav));
    root.set_child(dp.clone());

    // Detach: should clear the drawer page reference.
    root.set_child(None);

    // The navigation page should no longer reference the drawer page.
    // Pushing a page should not show a hamburger icon (which requires the drawer page).
    let page = page();
    wait(nav.push_async(&page));
    assert!(NavigationPage::get_back_button_content(&page).is_none());
}

#[test]
fn detach_and_reattach_restores_drawer_page_reference() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let dp = DrawerPage::new();
    dp.set_content(boxed(&nav));
    let root = TestRoot::with_child(dp.clone());
    let page = page();
    wait(nav.push_async(&page));

    assert!(nav.is_back_button_effectively_visible());

    root.set_child(None);
    assert!(!nav.is_back_button_effectively_visible());

    root.set_child(dp.clone());
    assert!(nav.is_back_button_effectively_visible());
}
