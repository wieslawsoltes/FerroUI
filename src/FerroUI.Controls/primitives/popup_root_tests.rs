//! Port of the reference `PopupRootTests`.

use super::popup_positioning::{IPopupPositioner, PopupPositionRequest, PopupPositionerParameters};
use super::{Popup, PopupRoot, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, VisualLayerManager};
use crate::platform::{IPopupImpl, ITopLevelImpl};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockCall, MockImplKind, MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::top_level_host::TopLevelHost;
use crate::{
    Border, Canvas, ContentControl, Control, ControlImpl, Decorator, LayoutTransformControl, Panel, PlacementMode,
    TopLevel, Window,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl, LayoutableImplExt};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn create_target(popup_parent: &Ref<TopLevel>, platform_impl: Option<Rc<dyn IPopupImpl>>) -> Ref<PopupRoot> {
    let platform_impl = platform_impl.unwrap_or_else(|| {
        popup_parent
            .platform_impl()
            .expect("the popup parent has a platform implementation")
            .create_popup()
            .expect("the platform creates popups")
    });

    let result = PopupRoot::new(popup_parent, platform_impl);
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    });
    result.set_template(Some(template));

    result.apply_template();

    result
}

fn new_window() -> Ref<TopLevel> {
    Window::new().upcast()
}

fn window_impl_of(window: &TopLevel) -> Rc<dyn ITopLevelImpl> {
    window.platform_impl().expect("the window has a platform implementation")
}

#[repr(C)]
struct TemplatedControlWithPopup {
    base: TemplatedControl,
    popup: RefCell<Option<Ref<Popup>>>,
}

ferro_class!(TemplatedControlWithPopup: TemplatedControl);
ferro_impl_classes!(
    TemplatedControlWithPopup: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for TemplatedControlWithPopup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|parent, _| {
            let popup = Popup::new();
            popup.bind_binding(
                Popup::child_property().as_property(),
                &TemplateBinding::new(TemplatedControlWithPopup::popup_content_property().as_property()),
            );
            popup.set_placement_target(parent.clone());
            popup.upcast()
        });
        this.set_template(Some(template));
    }
}

impl TemplatedControlImpl for TemplatedControlWithPopup {
    fn on_apply_template(this: &Self, _e: &TemplateAppliedEventArgs) {
        let children = this.get_visual_children();
        assert_eq!(1, children.len());
        *this.popup.borrow_mut() = children[0].cast::<Popup>();
    }
}

impl TemplatedControlWithPopup {
    ferro_property!(
        fn popup_content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TemplatedControlWithPopup, _>("PopupContent", None)
        }
    );

    fn new() -> Ref<Self> {
        instantiate(Self { base: TemplatedControl::construct(), popup: RefCell::new(None) })
    }

    fn popup(&self) -> Option<Ref<Popup>> {
        self.popup.borrow().clone()
    }

    fn set_popup_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::popup_content_property(), value)
    }
}

#[repr(C)]
struct ChildControl {
    base: Control,
    measure_sizes: RefCell<Vec<Size>>,
}

ferro_class!(ChildControl: Control);
ferro_impl_classes!(
    ChildControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for ChildControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_sizes.borrow_mut().push(available_size);
        Self::parent_measure_override(this, available_size)
    }
}

impl ChildControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), measure_sizes: RefCell::new(Vec::new()) })
    }
}

/// A positioner that records the parameters of its updates.
#[derive(Default)]
struct RecordingPositioner {
    updates: RefCell<Vec<PopupPositionerParameters>>,
}

impl IPopupPositioner for RecordingPositioner {
    fn update(&self, parameters: PopupPositionerParameters) {
        self.updates.borrow_mut().push(parameters);
    }
}

#[test]
fn popup_root_is_attached_to_logical_tree_is_true() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = create_target(&new_window(), None);

    assert!(target.is_attached_to_logical_tree());
}

#[test]
fn templated_child_is_attached_to_logical_tree_is_true() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = create_target(&new_window(), None);

    assert!(target.presenter().unwrap().is_attached_to_logical_tree());
}

#[test]
fn popup_root_forwards_initial_is_hit_test_visible_to_impl() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = MockWindowingPlatform::create_popup_mock(MockWindowImpl::bare(MockImplKind::Window));

    create_target(&new_window(), Some(platform_impl.clone()));

    assert!(platform_impl.count_of(&MockCall::SetHitTestVisible(true)) >= 1);
}

#[test]
fn popup_root_forwards_is_hit_test_visible_changes_to_impl() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = MockWindowingPlatform::create_popup_mock(MockWindowImpl::bare(MockImplKind::Window));
    let target = create_target(&new_window(), Some(platform_impl.clone()));

    target.set_is_hit_test_visible(false);

    assert!(platform_impl.count_of(&MockCall::SetHitTestVisible(false)) >= 1);
}

#[test]
fn popup_root_styling_parent_is_popup() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let target = TemplatedControlWithPopup::new();
    target.set_popup_content(Some(Canvas::new().upcast()));
    window.set_content(Some(Control::boxed(target.clone())));

    window.apply_styling();
    window.apply_template();
    window.presenter().unwrap().apply_template();
    target.apply_template();
    let popup = target.popup().unwrap();
    popup.open();

    let host = popup.host().unwrap().as_control();
    let styling_parent = host.styling_parent().and_then(|parent| parent.as_element().cloned());
    assert_eq!(Some(popup.upcast::<StyledElement>()), styling_parent);
}

#[test]
fn popup_root_should_have_template_applied() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let target = Popup::new();
    target.set_placement(PlacementMode::Pointer);

    window.set_content(Some(Control::boxed(target.clone())));
    window.apply_template();
    target.open();

    let host: Ref<Visual> = target.host().unwrap().as_control().upcast();
    assert_eq!(1, host.get_visual_children().len());

    let templated_child = host.get_visual_children()[0].clone();

    assert!(templated_child.get_type() == LayoutTransformControl::TYPE);

    let templated_child_children = templated_child.get_visual_children();
    assert_eq!(1, templated_child_children.len());
    let panel = templated_child_children[0].clone();

    assert!(panel.get_type() == Panel::TYPE);

    let panel_children = panel.get_visual_children();
    assert_eq!(2, panel_children.len());
    let visual_layer_manager = panel_children[1].clone();

    assert!(visual_layer_manager.get_type() == VisualLayerManager::TYPE);

    let visual_layer_manager_children = visual_layer_manager.visual_children().snapshot();
    assert_eq!(1, visual_layer_manager_children.len());
    let content_presenter = visual_layer_manager_children[0].clone();
    assert!(content_presenter.get_type() == ContentPresenter::TYPE);

    let host_object: Ref<FerroObject> = host.cast::<PopupRoot>().unwrap().upcast();
    assert_eq!(Some(host_object.clone()), templated_child.cast::<Control>().unwrap().templated_parent());
    assert_eq!(Some(host_object), content_presenter.cast::<Control>().unwrap().templated_parent());
}

#[test]
fn popup_root_should_have_top_level_host_visual_parent() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Popup::new();
    target.set_placement_target(Window::new());

    target.open();

    let host = target.host().unwrap().as_control();
    assert!(host.get_visual_parent().unwrap().get_type() == TopLevelHost::TYPE);
}

#[test]
fn attaching_popup_root_to_parent_logical_tree_raises_detached_from_logical_tree_and_attached_to_logical_tree() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let child = Decorator::new();
    let window = new_window();
    let target = create_target(&window, None);
    let detached_count = Rc::new(Cell::new(0));
    let attached_count = Rc::new(Cell::new(0));

    target.set_content(Some(Control::boxed(child.clone())));

    let count = detached_count.clone();
    let _s1 = target.detached_from_logical_tree(move |_| count.set(count.get() + 1));
    let count = detached_count.clone();
    let _s2 = child.detached_from_logical_tree(move |_| count.set(count.get() + 1));
    let count = attached_count.clone();
    let _s3 = target.attached_to_logical_tree(move |_| count.set(count.get() + 1));
    let count = attached_count.clone();
    let _s4 = child.attached_to_logical_tree(move |_| count.set(count.get() + 1));

    target.set_parent(window.clone());

    assert_eq!(2, detached_count.get());
    assert_eq!(2, attached_count.get());
}

#[test]
fn detaching_popup_root_from_parent_logical_tree_raises_detached_from_logical_tree_and_attached_to_logical_tree() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let child = Decorator::new();
    let window = new_window();
    let target = create_target(&window, None);
    let detached_count = Rc::new(Cell::new(0));
    let attached_count = Rc::new(Cell::new(0));

    target.set_content(Some(Control::boxed(child.clone())));
    target.set_parent(window.clone());

    let count = detached_count.clone();
    let _s1 = target.detached_from_logical_tree(move |_| count.set(count.get() + 1));
    let count = detached_count.clone();
    let _s2 = child.detached_from_logical_tree(move |_| count.set(count.get() + 1));
    let count = attached_count.clone();
    let _s3 = target.attached_to_logical_tree(move |_| count.set(count.get() + 1));
    let count = attached_count.clone();
    let _s4 = child.attached_to_logical_tree(move |_| count.set(count.get() + 1));

    target.set_parent(None);

    // Despite being detached from the parent logical tree, we're still attached to a
    // logical tree as PopupRoot itself is a logical tree root.
    assert!(target.is_attached_to_logical_tree());
    assert!(child.is_attached_to_logical_tree());
    assert_eq!(2, detached_count.get());
    assert_eq!(2, attached_count.get());
}

#[test]
fn clearing_content_of_popup_in_control_template_doesnt_crash() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let target = TemplatedControlWithPopup::new();
    target.set_popup_content(Some(Canvas::new().upcast()));
    window.set_content(Some(Control::boxed(target.clone())));

    window.apply_styling();
    window.apply_template();
    window.presenter().unwrap().apply_template();
    target.apply_template();
    target.popup().unwrap().open();
    target.set_popup_content(None);
}

#[test]
fn child_should_be_measured_with_max_auto_size_hint() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let child = ChildControl::new();
    let window = new_window();
    let popup_impl = MockWindowingPlatform::create_popup_mock(window_impl_of(&window));
    popup_impl.max_auto_size_hint.set(Size::new(1200.0, 1000.0));
    let target = create_target(&window, Some(popup_impl));

    target.set_content(Some(Control::boxed(child.clone())));
    target.show();

    assert_eq!(1, child.measure_sizes.borrow().len());
    assert_eq!(Size::new(1200.0, 1000.0), child.measure_sizes.borrow()[0]);
}

#[test]
fn child_should_be_measured_with_width_height_when_set() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let child = ChildControl::new();
    let window = new_window();
    let target = create_target(&window, None);

    target.set_width(500.0);
    target.set_height(600.0);
    target.set_content(Some(Control::boxed(child.clone())));
    target.show();

    assert_eq!(1, child.measure_sizes.borrow().len());
    assert_eq!(Size::new(500.0, 600.0), child.measure_sizes.borrow()[0]);
}

#[test]
fn child_should_be_measured_with_max_width_max_height_when_set() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let child = ChildControl::new();
    let window = new_window();
    let target = create_target(&window, None);

    target.set_max_width(500.0);
    target.set_max_height(600.0);
    target.set_content(Some(Control::boxed(child.clone())));
    target.show();

    assert_eq!(1, child.measure_sizes.borrow().len());
    assert_eq!(Size::new(500.0, 600.0), child.measure_sizes.borrow()[0]);
}

#[test]
fn should_not_have_offset_on_bounds_when_content_larger_than_max_window_size() {
    // Issue #3784.
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = new_window();
    let popup_impl = MockWindowingPlatform::create_popup_mock(window_impl_of(&window));

    let child = Canvas::new();
    child.set_width(400.0);
    child.set_height(1344.0);

    let target = create_target(&window, Some(popup_impl));
    target.set_content(Some(Control::boxed(child)));

    target.configure_position(PopupPositionRequest::new(window.clone().upcast(), PlacementMode::Top));
    target.show();

    assert_eq!(Size::new(400.0, 1024.0), target.bounds().size());

    // Issue #3784 causes this to be (0, 160) which makes no sense as Window has no
    // parent control to be offset against.
    assert_eq!(Point::new(0.0, 0.0), target.bounds().position());
}

#[test]
fn min_width_min_height_should_be_respected() {
    // Issue #3796
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = new_window();
    let popup_impl = MockWindowingPlatform::create_popup_mock(window_impl_of(&window));

    let target = create_target(&window, Some(popup_impl));
    target.set_min_width(400.0);
    target.set_min_height(800.0);
    let content = Border::new();
    content.set_width(100.0);
    content.set_height(100.0);
    target.set_content(Some(Control::boxed(content)));

    target.configure_position(PopupPositionRequest::new(window.clone().upcast(), PlacementMode::Top));
    target.show();

    assert_eq!(Rect::new(0.0, 0.0, 400.0, 800.0), target.bounds());
    assert_eq!(Size::new(400.0, 800.0), target.client_size());
    assert_eq!(Size::new(400.0, 800.0), target.platform_impl().unwrap().client_size());
}

#[test]
fn setting_width_should_resize_window_impl() {
    // Issue #3796
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = new_window();
    let popup_impl = MockWindowingPlatform::create_popup_mock(window_impl_of(&window));
    let positioner = Rc::new(RecordingPositioner::default());
    popup_impl.setup_popup_positioner(Some(positioner.clone()));

    let target = create_target(&window, Some(popup_impl));
    target.set_width(400.0);
    target.set_height(800.0);

    target.configure_position(PopupPositionRequest::new(window.clone().upcast(), PlacementMode::Top));
    target.show();

    assert_eq!(400.0, target.width());
    assert_eq!(800.0, target.height());

    target.set_width(410.0);
    target.layout_manager().execute_layout_pass();

    assert!(positioner.updates.borrow().iter().any(|x| x.size.width == 410.0));
    assert_eq!(410.0, target.width());
}

#[test]
fn popup_anchor_rect_should_account_for_decoration_inset() {
    use crate::platform::PlatformRequestedDrawnDecoration;
    use crate::SizeToContent;
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.needs_managed_decorations.set(true);
    window_impl
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR | PlatformRequestedDrawnDecoration::BORDER);

    let placement_target = Panel::new();
    placement_target.set_width(10.0);
    placement_target.set_height(10.0);
    placement_target.set_horizontal_alignment(HorizontalAlignment::Left);
    placement_target.set_vertical_alignment(VerticalAlignment::Top);

    let window = Window::with_impl(window_impl.clone());
    window.set_size_to_content(SizeToContent::MANUAL);
    window.set_content(Some(Control::boxed(&placement_target)));
    window.show();

    let inset = window.top_level_host().decoration_inset();
    assert!(inset.top > 0.0, "Expected non-zero decoration inset top (title bar)");

    let popup_impl = MockWindowingPlatform::create_popup_mock(window_impl.clone());
    let target = create_target(&window.clone().upcast(), Some(popup_impl.clone()));
    target.set_width(10.0);
    target.set_height(10.0);

    target.configure_position(PopupPositionRequest::new(placement_target.clone().upcast(), PlacementMode::Bottom));
    target.show();

    // The popup should be placed below the placement target.
    let popup_screen_pos = popup_impl.position.get();
    assert!(
        popup_screen_pos.y >= (inset.top + placement_target.bounds().height) as i32,
        "Popup Y ({}) should be >= inset.Top ({}) + target height ({}). If this fails, the anchor rect \
         calculation is not accounting for the decoration offset.",
        popup_screen_pos.y,
        inset.top,
        placement_target.bounds().height
    );
}
