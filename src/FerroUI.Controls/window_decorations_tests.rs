//! Port of the drawn decoration tests of the reference `WindowTests`.

use crate::chrome::{TitleBarDecorations, WindowDrawnDecorations, WindowDrawnDecorationsContent};
use crate::platform::{IWindowImpl, PlatformAllowedWindowActions, PlatformRequestedDrawnDecoration};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{
    decorations_template_theme, FuncWindowDrawnDecorationsTemplate, MockCall, MockWindowImpl, MockWindowingPlatform,
    TestServices, UnitTestApplication,
};
use crate::{
    Canvas, ContentControl, Control, ControlImpl, Panel, SizeToContent, StackPanel, Window,
    WindowResizeReason,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl, LayoutableImplExt};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::*;
use std::cell::RefCell;
use std::rc::Rc;

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

    fn measure_sizes(&self) -> Vec<Size> {
        self.measure_sizes.borrow().clone()
    }
}

fn window_with_impl(window_impl: &Rc<MockWindowImpl>) -> Ref<Window> {
    Window::with_impl(window_impl.clone())
}

fn resized(target: &Window, size: Size, reason: WindowResizeReason) {
    let platform_impl = target.platform_impl().expect("the window is open");
    platform_impl.resized().expect("the resized callback is set")(size, reason);
}

fn decorations(window: &Window) -> Ref<WindowDrawnDecorations> {
    window.top_level_host().decorations().expect("the window has drawn decorations")
}

mod forced_decoration_sizing_tests {
    use super::*;

    /// Creates a window implementation that simulates forced drawn
    /// decorations: the platform needs managed decorations and requests the
    /// title bar and the border, but the client area is not extended.
    fn create_forced_csd_window_mock() -> Rc<MockWindowImpl> {
        let window_impl = MockWindowingPlatform::create_window_mock_with_size(800.0, 600.0);

        window_impl.needs_managed_decorations.set(true);
        window_impl
            .requested_drawn_decorations
            .set(PlatformRequestedDrawnDecoration::TITLE_BAR | PlatformRequestedDrawnDecoration::BORDER);

        window_impl
    }

    #[test]
    fn client_size_should_exclude_decoration_inset() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::MANUAL);

        // Verify mock setup
        assert!(IWindowImpl::needs_managed_decorations(&*window_impl));

        target.show();

        let host = target.top_level_host();
        let decorations = host.decorations();

        // Verify decorations were created
        let decorations = decorations.expect("the decorations were created");
        assert!(decorations.title_bar_height() > 0.0, "TitleBarHeight was {}", decorations.title_bar_height());

        let inset = host.decoration_inset();
        assert_ne!(Thickness::default(), inset);

        let expected_client_size =
            Size::new(800.0 - inset.left - inset.right, 600.0 - inset.top - inset.bottom);
        assert_eq!(expected_client_size, target.client_size());
    }

    #[test]
    fn window_decoration_margin_should_be_zero_in_forced_mode() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::MANUAL);

        target.show();

        assert_eq!(Thickness::default(), target.window_decoration_margin());
    }

    #[test]
    fn handle_resized_should_subtract_inset_from_platform_size() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::MANUAL);

        target.show();

        let inset = target.top_level_host().decoration_inset();

        // Simulate a platform resize (e.g. user resize)
        resized(&target, Size::new(1000.0, 700.0), WindowResizeReason::User);

        let expected_client_size =
            Size::new(1000.0 - inset.left - inset.right, 700.0 - inset.top - inset.bottom);
        assert_eq!(expected_client_size, target.client_size());
    }

    #[test]
    fn setting_width_should_resize_window_impl_with_inset_added() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let target = window_with_impl(&window_impl);
        target.set_width(400.0);
        target.set_height(300.0);
        target.set_size_to_content(SizeToContent::MANUAL);

        target.show();

        let inset = target.top_level_host().decoration_inset();

        target.set_width(500.0);
        target.layout_manager().execute_layout_pass();

        // Platform should receive full frame size (content + inset)
        let expected_platform_size =
            Size::new(500.0 + inset.left + inset.right, 300.0 + inset.top + inset.bottom);
        assert!(window_impl.calls().contains(&MockCall::Resize(expected_platform_size, WindowResizeReason::Layout)));
    }

    #[test]
    fn child_should_be_measured_with_content_size() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let child = ChildControl::new();
        let target = window_with_impl(&window_impl);
        target.set_width(400.0);
        target.set_height(300.0);
        target.set_size_to_content(SizeToContent::MANUAL);
        target.set_content(Some(Control::boxed(child.clone())));

        target.show();

        assert_eq!(1, child.measure_sizes().len());
        assert_eq!(Size::new(400.0, 300.0), child.measure_sizes()[0]);
    }

    #[test]
    fn width_height_should_not_be_nan_after_show() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::MANUAL);

        target.show();

        assert!(!target.width().is_nan());
        assert!(!target.height().is_nan());

        let inset = target.top_level_host().decoration_inset();
        assert_eq!(800.0 - inset.left - inset.right, target.width());
        assert_eq!(600.0 - inset.top - inset.bottom, target.height());
    }

    #[test]
    fn size_to_content_should_work_in_forced_mode() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let child = Canvas::new();
        child.set_width(400.0);
        child.set_height(300.0);

        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
        target.set_content(Some(Control::boxed(child)));

        target.show();

        assert_eq!(400.0, target.width());
        assert_eq!(300.0, target.height());
        assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, target.size_to_content());
    }

    #[test]
    fn user_resize_should_reset_size_to_content() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_forced_csd_window_mock();
        let child = Canvas::new();
        child.set_width(400.0);
        child.set_height(300.0);

        let target = window_with_impl(&window_impl);
        target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
        target.set_content(Some(Control::boxed(child)));

        target.show();
        assert_eq!(400.0, target.width());
        assert_eq!(300.0, target.height());

        let inset = target.top_level_host().decoration_inset();
        // Platform fires resize with full frame size
        let new_platform_width = 500.0 + inset.left + inset.right;
        let new_platform_height = 300.0 + inset.top + inset.bottom;
        resized(&target, Size::new(new_platform_width, new_platform_height), WindowResizeReason::User);

        assert_eq!(500.0, target.width());
        assert_eq!(300.0, target.height());
        assert_eq!(SizeToContent::HEIGHT, target.size_to_content());
    }
}

#[test]
fn window_decorations_theme_should_apply_to_decorations() {
    fn create_theme() -> (Ref<ControlTheme>, Ref<WindowDrawnDecorationsContent>) {
        let content = WindowDrawnDecorationsContent::new();
        let theme = decorations_template_theme(FuncWindowDrawnDecorationsTemplate::from_content(&content));
        (theme, content)
    }

    let _app = UnitTestApplication::start(TestServices::styled_window());

    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.needs_managed_decorations.set(true);
    window_impl
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR | PlatformRequestedDrawnDecoration::BORDER);

    let window = window_with_impl(&window_impl);

    let (theme1, content1) = create_theme();
    window.set_window_decorations_theme(theme1.clone());
    window.show();

    let decorations = decorations(&window);
    assert_eq!(Some(theme1), decorations.theme());
    assert_eq!(Some(content1), decorations.content());

    let (theme2, content2) = create_theme();
    window.set_window_decorations_theme(theme2.clone());

    assert_eq!(Some(theme2), decorations.theme());
    assert_eq!(Some(content2), decorations.content());
}

/// A content control with a template (the reference test gets it from the
/// theme of the application).
fn templated_content_control(content: Ref<Control>) -> Ref<ContentControl> {
    let control = ContentControl::new();
    control.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        for property in [
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ] {
            presenter.bind_binding(property, &TemplateBinding::new(property));
        }
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    control.set_content(Some(Control::boxed(content)));
    control
}

#[test]
fn is_visible_setter_should_affect_measurements_inside_window_drawn_decorations_content() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.needs_managed_decorations.set(true);
    window_impl.requested_drawn_decorations.set(PlatformRequestedDrawnDecoration::TITLE_BAR);

    let window = window_with_impl(&window_impl);

    let stack_panel = StackPanel::new();
    stack_panel.set_width(32.0);
    stack_panel.set_spacing(2.0);
    let first = Control::new();
    first.set_height(32.0);
    stack_panel.children().add(first);
    let second = Control::new();
    second.set_height(32.0);
    second.classes().add("hidden-by-style");
    stack_panel.children().add(second);

    let hidden = Control::new();
    hidden.set_height(32.0);
    hidden.set_width(32.0);
    hidden.classes().add("hidden-by-style");
    let content_control = templated_content_control(hidden);

    let panel = Panel::new();
    panel.children().add(stack_panel.clone());
    panel.children().add(content_control.clone());

    let content = WindowDrawnDecorationsContent::new();
    content.set_overlay(templated_content_control(panel.upcast()).upcast::<Control>());

    let theme = decorations_template_theme(FuncWindowDrawnDecorationsTemplate::from_content(&content));

    let style = Style::with_setters(
        Selectors::is::<WindowDrawnDecorations>().template().of_type::<Control>().class("hidden-by-style"),
        [Setter::new(Visual::is_visible_property(), false)],
    );

    window.set_window_decorations_theme(theme);
    window.styles().add(style);
    window.show();
    window.measure(Size::INFINITY);

    assert_eq!(Size::default(), content_control.desired_size());
    assert_eq!(Size::new(32.0, 32.0), stack_panel.desired_size());
}

mod title_bar_decorations_tests {
    use super::*;

    fn create_window_with_drawn_decorations(allowed_actions: PlatformAllowedWindowActions) -> Ref<Window> {
        let window_impl = MockWindowingPlatform::create_window_mock();
        window_impl.needs_managed_decorations.set(true);
        window_impl
            .requested_drawn_decorations
            .set(PlatformRequestedDrawnDecoration::TITLE_BAR | PlatformRequestedDrawnDecoration::BORDER);
        window_impl.allowed_window_actions.set(allowed_actions);

        window_with_impl(&window_impl)
    }

    fn assert_class_decorations(drawn_decorations: &WindowDrawnDecorations, expected: TitleBarDecorations) {
        let assert_class_decoration = |decoration: TitleBarDecorations, class_name: &str| {
            assert_eq!(
                expected.contains(decoration),
                drawn_decorations.classes().contains(class_name),
                "{class_name}"
            );
        };

        assert_class_decoration(TitleBarDecorations::TITLE, ":has-title");
        assert_class_decoration(TitleBarDecorations::MINIMIZE_BUTTON, ":has-minimize");
        assert_class_decoration(TitleBarDecorations::MAXIMIZE_BUTTON, ":has-maximize");
        assert_class_decoration(TitleBarDecorations::CLOSE_BUTTON, ":has-close");
        assert_class_decoration(TitleBarDecorations::FULL_SCREEN_BUTTON, ":has-fullscreen");
    }

    #[test]
    fn all_decorations_should_be_visible_by_default() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        let window = create_window_with_drawn_decorations(PlatformAllowedWindowActions::ALL);
        window.show();

        let decorations = decorations(&window);
        assert_eq!(TitleBarDecorations::ALL, decorations.title_bar_decorations());
        assert_class_decorations(&decorations, TitleBarDecorations::ALL);
    }

    #[test]
    fn decorations_set_before_show_should_apply_correct_classes() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        let decorations = TitleBarDecorations::TITLE | TitleBarDecorations::CLOSE_BUTTON;

        let window = create_window_with_drawn_decorations(PlatformAllowedWindowActions::ALL);
        WindowDrawnDecorations::set_title_bar_decorations(&window, decorations);
        window.show();

        let drawn_decorations = super::decorations(&window);
        assert_eq!(decorations, drawn_decorations.title_bar_decorations());
        assert_class_decorations(&drawn_decorations, decorations);
    }

    #[test]
    fn decorations_set_after_show_should_apply_correct_classes() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        let window = create_window_with_drawn_decorations(PlatformAllowedWindowActions::ALL);
        window.show();

        let drawn_decorations = decorations(&window);

        WindowDrawnDecorations::set_title_bar_decorations(&window, TitleBarDecorations::NONE);
        assert_class_decorations(&drawn_decorations, TitleBarDecorations::NONE);

        WindowDrawnDecorations::set_title_bar_decorations(&window, TitleBarDecorations::MINIMIZE_BUTTON);
        assert_class_decorations(&drawn_decorations, TitleBarDecorations::MINIMIZE_BUTTON);
    }

    #[test]
    fn decorations_should_not_show_buttons_unsupported_by_the_platform() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        let window = create_window_with_drawn_decorations(PlatformAllowedWindowActions::MINIMIZE);
        window.show();

        let decorations = decorations(&window);

        // Maximize and fullscreen are requested, but not allowed by the platform.
        assert_class_decorations(
            &decorations,
            TitleBarDecorations::TITLE | TitleBarDecorations::MINIMIZE_BUTTON | TitleBarDecorations::CLOSE_BUTTON,
        );
    }
}

// Additional tests of members the reference suite does not cover.
mod additional_tests {
    use super::*;
    use crate::chrome::{DrawnWindowDecorationParts, ResizeGripLayer};
    use crate::platform::ITopLevelImpl;
    use crate::top_level_host::TopLevelHost;
    use crate::top_level_host_decorations::LayerWrapper;
    use crate::{TopLevel, WindowDecorations, WindowState};
    use ferroui_base::input::{
        IInputRoot, InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
        PointerUpdateKind, RawInputModifiers, WindowDecorationsElementRole,
    };
    use ferroui_base::threading::Dispatcher;
    use std::cell::Cell;

    fn create_window_mock(requested: PlatformRequestedDrawnDecoration) -> Rc<MockWindowImpl> {
        let window_impl = MockWindowingPlatform::create_window_mock();
        window_impl.needs_managed_decorations.set(true);
        window_impl.requested_drawn_decorations.set(requested);
        window_impl
    }

    fn extend(window_impl: &MockWindowImpl) {
        window_impl.is_client_area_extended_to_decorations.set(true);
        window_impl.extend_client_area_to_decorations_changed().expect("the callback is set")(true);
    }

    fn change_state(window_impl: &MockWindowImpl, state: WindowState) {
        window_impl.window_state.set(state);
        IWindowImpl::window_state_changed(window_impl).expect("the callback is set")(state);
    }

    fn pointer_moved(host: &Ref<TopLevelHost>, y: f64) {
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let visual: &Visual = host;
        let args = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            host.clone().upcast::<FerroObject>(),
            pointer,
            Some(visual),
            Point::new(10.0, y),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        );
        host.raise_event(&args);
    }

    #[test]
    fn extended_mode_uses_the_decoration_geometry_as_window_decoration_margin() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        extend(&window_impl);

        // Title bar 30, frame 1 and shadow 8 in the test theme.
        assert_eq!(Thickness::new(9.0, 39.0, 9.0, 9.0), target.window_decoration_margin());
        assert_eq!(Thickness::default(), target.top_level_host().decoration_inset());
        assert!(window_impl.calls().contains(&MockCall::SetShadowExtents(Thickness::uniform(8.0))));

        let decorations = decorations(&target);
        assert!(decorations.has_shadow() && decorations.has_border() && decorations.has_title_bar());
        for class in [":has-shadow", ":has-border", ":has-titlebar", ":normal"] {
            assert!(decorations.classes().contains(class), "{class}");
        }
        assert_eq!(
            Thickness::uniform(9.0),
            target.top_level_host().resize_grips().expect("the grips exist").grip_thickness()
        );
    }

    #[test]
    fn layers_are_inserted_around_the_top_level_and_removed_with_the_decorations() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::TITLE_BAR);
        let target = window_with_impl(&window_impl);
        let host = target.top_level_host().clone();
        assert_eq!(1, host.visual_children().count());

        target.show();

        let children = host.visual_children().snapshot();
        assert_eq!(5, children.len());
        assert!(children[0].is::<LayerWrapper>());
        assert!(children[1].is::<TopLevel>());
        assert!(children[2].is::<LayerWrapper>());
        assert!(children[3].is::<LayerWrapper>());
        assert!(!children[3].is_visible());
        assert!(children[4].is::<ResizeGripLayer>());
        assert!(!children[4].is_visible());

        let decorations = decorations(&target);
        let content = decorations.content().expect("the template was applied");
        assert_eq!(content.underlay(), host.underlay_layer().and_then(|layer| layer.inner()));
        assert_eq!(content.overlay(), host.overlay_layer().and_then(|layer| layer.inner()));
        assert_eq!(content.fullscreen_popover(), host.fullscreen_popover_layer().and_then(|layer| layer.inner()));
        assert_eq!(Some(host.clone().upcast::<StyledElement>()), decorations.parent());
        assert_eq!(
            Some(decorations.clone().upcast::<FerroObject>()),
            content.underlay().and_then(|underlay| underlay.templated_parent())
        );

        window_impl.needs_managed_decorations.set(false);
        window_impl.drawn_decorations_request_changed().expect("the callback is set")();

        assert!(host.decorations().is_none());
        assert_eq!(1, host.visual_children().count());
        assert!(decorations.parent().is_none());
        assert_eq!(Thickness::default(), host.decoration_inset());
    }

    #[test]
    fn window_state_selects_the_enabled_parts() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        let decorations = decorations(&target);
        assert_eq!(DrawnWindowDecorationParts::ALL, decorations.enabled_parts());

        change_state(&window_impl, WindowState::Maximized);
        assert_eq!(DrawnWindowDecorationParts::TITLE_BAR, decorations.enabled_parts());
        assert!(decorations.classes().contains(":maximized"));
        assert!(!decorations.classes().contains(":normal"));
        assert_eq!(Thickness::new(0.0, 30.0, 0.0, 0.0), target.top_level_host().decoration_inset());

        change_state(&window_impl, WindowState::FullScreen);
        assert_eq!(DrawnWindowDecorationParts::NONE, decorations.enabled_parts());
        assert!(decorations.classes().contains(":fullscreen"));
        assert_eq!(Thickness::default(), target.top_level_host().decoration_inset());

        change_state(&window_impl, WindowState::Normal);
        assert_eq!(DrawnWindowDecorationParts::ALL, decorations.enabled_parts());

        target.set_can_resize(false);
        target.set_window_decorations(WindowDecorations::BorderOnly);
        assert_eq!(
            DrawnWindowDecorationParts::SHADOW | DrawnWindowDecorationParts::BORDER,
            decorations.enabled_parts()
        );

        target.set_window_decorations(WindowDecorations::None);
        assert_eq!(DrawnWindowDecorationParts::NONE, decorations.enabled_parts());
        assert!(target.top_level_host().decorations().is_some());
    }

    #[test]
    fn fullscreen_popover_follows_the_pointer() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        let host = target.top_level_host().clone();
        let popover = host.fullscreen_popover_layer().expect("the popover layer exists");
        let overlay = host.overlay_layer().expect("the overlay layer exists");
        let underlay = host.underlay_layer().expect("the underlay layer exists");

        // Not fullscreen: the pointer does not show the popover.
        pointer_moved(&host, 0.0);
        assert!(!popover.is_visible());

        change_state(&window_impl, WindowState::FullScreen);
        assert!(!overlay.is_visible() && !underlay.is_visible() && !popover.is_visible());

        pointer_moved(&host, 20.0);
        assert!(!popover.is_visible());
        pointer_moved(&host, 1.0);
        assert!(popover.is_visible());
        // Still within the default title bar height.
        pointer_moved(&host, 30.0);
        assert!(popover.is_visible());
        pointer_moved(&host, 31.0);
        assert!(!popover.is_visible());

        pointer_moved(&host, 0.0);
        assert!(popover.is_visible());
        change_state(&window_impl, WindowState::Normal);
        assert!(overlay.is_visible() && underlay.is_visible() && !popover.is_visible());
    }

    #[test]
    fn title_bar_height_hint_and_scaling_are_forwarded() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl =
            create_window_mock(PlatformRequestedDrawnDecoration::TITLE_BAR | PlatformRequestedDrawnDecoration::BORDER);
        let target = window_with_impl(&window_impl);
        target.set_extend_client_area_title_bar_height_hint(40.0);
        target.show();
        let decorations = decorations(&target);

        assert_eq!(40.0, decorations.title_bar_height());
        assert_eq!(Thickness::new(1.0, 41.0, 1.0, 1.0), target.top_level_host().decoration_inset());

        target.set_extend_client_area_title_bar_height_hint(-1.0);
        assert_eq!(30.0, decorations.title_bar_height());
        assert_eq!(Thickness::new(1.0, 31.0, 1.0, 1.0), target.top_level_host().decoration_inset());

        window_impl.render_scaling.set(1.5);
        ITopLevelImpl::scaling_changed(&*window_impl).expect("the callback is set")(1.5);
        assert_eq!(1.5, decorations.render_scaling());
        // 1 rounds to two device pixels at 150 %.
        assert_eq!(Thickness::uniform(2.0 / 1.5), decorations.frame_thickness());
    }

    #[test]
    fn title_and_title_bar_decorations_mirror_the_window() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::TITLE_BAR);
        let target = window_with_impl(&window_impl);
        target.set_title(Some("First".to_string()));
        target.show();
        let decorations = decorations(&target);

        assert_eq!(Some("First".to_string()), decorations.title());
        target.set_title(Some("Second".to_string()));
        assert_eq!(Some("Second".to_string()), decorations.title());

        window_impl.allowed_window_actions.set(PlatformAllowedWindowActions::empty());
        window_impl.allowed_window_actions_changed().expect("the callback is set")(PlatformAllowedWindowActions::empty());
        assert!(!decorations.classes().contains(":has-minimize"));
        assert!(decorations.classes().contains(":has-close"));
    }

    #[test]
    fn overrides_take_precedence_over_the_theme_defaults() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        extend(&window_impl);
        let decorations = decorations(&target);

        let changes = Rc::new(Cell::new(0));
        let subscription = decorations.effective_geometry_changed({
            let changes = changes.clone();
            move || changes.set(changes.get() + 1)
        });

        decorations.set_frame_thickness_override(Some(Thickness::uniform(3.0)));
        decorations.set_shadow_thickness_override(Some(Thickness::uniform(5.0)));
        assert_eq!(Some(Thickness::uniform(3.0)), decorations.frame_thickness_override());
        assert_eq!(Some(Thickness::uniform(5.0)), decorations.shadow_thickness_override());
        assert_eq!(Thickness::uniform(3.0), decorations.frame_thickness());
        assert_eq!(Thickness::uniform(5.0), decorations.shadow_thickness());
        assert_eq!(2, changes.get());
        assert_eq!(Thickness::new(8.0, 38.0, 8.0, 8.0), target.window_decoration_margin());

        subscription.dispose();
        decorations.set_default_title_bar_height(20.0);
        assert_eq!(2, changes.get());
        assert_eq!(20.0, decorations.title_bar_height());
        assert_eq!(-1.0, decorations.title_bar_height_override());
    }

    #[test]
    fn resize_grips_answer_their_roles() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        extend(&window_impl);
        Dispatcher::ui_thread().run_jobs(None);
        target.layout_manager().execute_layout_pass();

        let source = target.presentation_source();
        let input_root: &dyn IInputRoot = &**source;
        let size = target.top_level_host().bounds().size();
        let (w, h) = (size.width, size.height);
        use WindowDecorationsElementRole as R;
        for (point, role) in [
            (Point::new(2.0, 2.0), R::ResizeNW),
            (Point::new(w / 2.0, 2.0), R::ResizeN),
            (Point::new(w - 2.0, 2.0), R::ResizeNE),
            (Point::new(2.0, h / 2.0), R::ResizeW),
            (Point::new(w - 2.0, h / 2.0), R::ResizeE),
            (Point::new(2.0, h - 2.0), R::ResizeSW),
            (Point::new(w / 2.0, h - 2.0), R::ResizeS),
            (Point::new(w - 2.0, h - 2.0), R::ResizeSE),
        ] {
            assert_eq!(Some(role), input_root.hit_test_chrome_element(point), "{point:?}");
        }
        // Inside the grips nothing has a resize role. (The managed hit tester of the tests hits the
        // grip layer itself by its bounds there, so the roles below it are not asserted here.)
        assert_eq!(None, input_root.hit_test_chrome_element(Point::new(w / 2.0, h / 2.0)));
    }

    /// The caption button called `name` of the applied decorations template.
    fn caption_button(window: &Window, name: &str) -> Ref<crate::Button> {
        let content = decorations(window).content().expect("the template was applied");
        [content.overlay(), content.fullscreen_popover()]
            .into_iter()
            .flatten()
            .flat_map(|layer| layer.get_visual_descendants())
            .find(|visual| visual.downcast_ref::<StyledElement>().and_then(|element| element.name()).as_deref() == Some(name))
            .and_then(|visual| visual.cast::<crate::Button>())
            .unwrap_or_else(|| panic!("the template has no button called {name}"))
    }

    fn shown_window_with_decorations() -> (Rc<MockWindowImpl>, Ref<Window>) {
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        let target = window_with_impl(&window_impl);
        target.show();
        (window_impl, target)
    }

    #[test]
    fn close_buttons_close_the_window() {
        for name in [WindowDrawnDecorations::PART_CLOSE_BUTTON, WindowDrawnDecorations::PART_POPOVER_CLOSE_BUTTON] {
            let _app = UnitTestApplication::start(TestServices::styled_window());
            let (window_impl, target) = shown_window_with_decorations();
            let closed = Rc::new(Cell::new(0));
            let _subscription = target.closed({
                let closed = closed.clone();
                move || closed.set(closed.get() + 1)
            });

            caption_button(&target, name).perform_click();

            assert_eq!(1, closed.get(), "{name}");
            assert_eq!(1, window_impl.count_of(&MockCall::Dispose), "{name}");
        }
    }

    #[test]
    fn minimize_button_minimizes_the_window() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (window_impl, target) = shown_window_with_decorations();

        caption_button(&target, WindowDrawnDecorations::PART_MINIMIZE_BUTTON).perform_click();

        assert_eq!(WindowState::Minimized, target.window_state());
        assert_eq!(1, window_impl.count_of(&MockCall::SetWindowState(WindowState::Minimized)));
    }

    #[test]
    fn maximize_button_toggles_between_maximized_and_normal() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (_window_impl, target) = shown_window_with_decorations();
        let button = caption_button(&target, WindowDrawnDecorations::PART_MAXIMIZE_BUTTON);

        button.perform_click();
        assert_eq!(WindowState::Maximized, target.window_state());

        button.perform_click();
        assert_eq!(WindowState::Normal, target.window_state());

        target.set_window_state(WindowState::FullScreen);
        button.perform_click();
        assert_eq!(WindowState::Maximized, target.window_state());
    }

    #[test]
    fn full_screen_buttons_toggle_between_full_screen_and_normal() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (_window_impl, target) = shown_window_with_decorations();

        caption_button(&target, WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON).perform_click();
        assert_eq!(WindowState::FullScreen, target.window_state());

        caption_button(&target, WindowDrawnDecorations::PART_POPOVER_FULL_SCREEN_BUTTON).perform_click();
        assert_eq!(WindowState::Normal, target.window_state());

        target.set_window_state(WindowState::Maximized);
        caption_button(&target, WindowDrawnDecorations::PART_POPOVER_FULL_SCREEN_BUTTON).perform_click();
        assert_eq!(WindowState::FullScreen, target.window_state());
    }

    #[test]
    fn caption_button_click_is_handled() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (_window_impl, target) = shown_window_with_decorations();
        let button = caption_button(&target, WindowDrawnDecorations::PART_MINIMIZE_BUTTON);
        let handled = Rc::new(Cell::new(false));
        let _token = button.add_handler_with(
            crate::Button::click_event(),
            {
                let handled = handled.clone();
                move |_, e| handled.set(e.handled())
            },
            ferroui_base::interactivity::RoutingStrategies::BUBBLE,
            true,
        );

        button.perform_click();

        assert!(handled.get());
    }

    #[test]
    fn caption_buttons_are_enabled_by_the_window_and_the_platform() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (window_impl, target) = shown_window_with_decorations();
        let minimize = caption_button(&target, WindowDrawnDecorations::PART_MINIMIZE_BUTTON);
        let maximize = caption_button(&target, WindowDrawnDecorations::PART_MAXIMIZE_BUTTON);
        let full_screen = caption_button(&target, WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON);
        let states = || (minimize.is_enabled(), maximize.is_enabled(), full_screen.is_enabled());
        assert_eq!((true, true, true), states());

        target.set_can_minimize(false);
        assert_eq!((false, true, true), states());
        target.set_can_minimize(true);

        target.set_can_maximize(false);
        assert_eq!((true, false, false), states());
        target.set_can_maximize(true);
        assert_eq!((true, true, true), states());

        // Maximized or full screen: restoring needs a resizable window.
        target.set_window_state(WindowState::Maximized);
        assert_eq!((true, true, true), states());
        target.set_can_resize(false);
        target.set_window_state(WindowState::Normal);
        target.set_window_state(WindowState::Maximized);
        assert_eq!((true, false, false), states());
        target.set_window_state(WindowState::FullScreen);
        assert_eq!((true, false, false), states());
        target.set_window_state(WindowState::Minimized);
        assert_eq!((true, true, false), states());
        target.set_can_resize(true);
        target.set_window_state(WindowState::Normal);
        assert_eq!((true, true, true), states());

        let allow = |actions: PlatformAllowedWindowActions| {
            window_impl.allowed_window_actions.set(actions);
            IWindowImpl::allowed_window_actions_changed(&*window_impl).expect("the callback is set")(actions);
        };
        allow(PlatformAllowedWindowActions::MINIMIZE);
        assert_eq!((true, false, false), states());
        allow(PlatformAllowedWindowActions::MAXIMIZE | PlatformAllowedWindowActions::FULLSCREEN);
        assert_eq!((false, true, true), states());
        allow(PlatformAllowedWindowActions::ALL);
        assert_eq!((true, true, true), states());
    }

    #[test]
    fn theme_hides_the_caption_buttons_that_are_not_requested() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (_window_impl, target) = shown_window_with_decorations();
        let visible = |name: &str| caption_button(&target, name).is_visible();
        assert!(visible(WindowDrawnDecorations::PART_MINIMIZE_BUTTON));
        assert!(visible(WindowDrawnDecorations::PART_MAXIMIZE_BUTTON));
        assert!(visible(WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON));
        assert!(visible(WindowDrawnDecorations::PART_CLOSE_BUTTON));

        WindowDrawnDecorations::set_title_bar_decorations(
            &target,
            TitleBarDecorations::TITLE | TitleBarDecorations::MINIMIZE_BUTTON,
        );

        assert!(visible(WindowDrawnDecorations::PART_MINIMIZE_BUTTON));
        assert!(!visible(WindowDrawnDecorations::PART_MAXIMIZE_BUTTON));
        assert!(!visible(WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON));
        assert!(!visible(WindowDrawnDecorations::PART_POPOVER_FULL_SCREEN_BUTTON));
        assert!(!visible(WindowDrawnDecorations::PART_CLOSE_BUTTON));
        assert!(!visible(WindowDrawnDecorations::PART_POPOVER_CLOSE_BUTTON));
    }

    #[test]
    fn replacing_the_template_detaches_the_old_caption_buttons() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (_window_impl, target) = shown_window_with_decorations();
        let old_button = caption_button(&target, WindowDrawnDecorations::PART_MINIMIZE_BUTTON);
        let decorations = decorations(&target);

        decorations.set_template(Some(crate::testing::window_drawn_decorations_template()));
        decorations.apply_template();
        let new_button = caption_button(&target, WindowDrawnDecorations::PART_MINIMIZE_BUTTON);
        assert_ne!(old_button, new_button);

        old_button.perform_click();
        assert_eq!(WindowState::Normal, target.window_state());

        new_button.perform_click();
        assert_eq!(WindowState::Minimized, target.window_state());
    }

    #[test]
    fn detached_decorations_keep_their_caption_buttons_enabled() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = WindowDrawnDecorations::new();
        target.set_template(Some(crate::testing::window_drawn_decorations_template()));
        target.apply_template();
        let content = target.content().expect("the template was applied");
        let buttons: Vec<Ref<crate::Button>> = content
            .overlay()
            .into_iter()
            .flat_map(|layer| layer.get_visual_descendants())
            .filter_map(|visual| visual.cast::<crate::Button>())
            .collect();
        assert_eq!(4, buttons.len());

        // Without a host window the clicks are handled and do nothing.
        for button in &buttons {
            assert!(button.is_enabled());
            button.perform_click();
        }
    }

    #[test]
    fn top_level_tells_when_it_has_been_closed() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = Window::new();
        let closeable = target.as_closeable();
        let closed = Rc::new(Cell::new(0));
        let subscription = closeable.closed(Rc::new({
            let closed = closed.clone();
            move || closed.set(closed.get() + 1)
        }));
        let disposed = closeable.closed(Rc::new(|| panic!("the handler was unsubscribed")));
        disposed.dispose();

        target.show();
        target.close();

        assert_eq!(1, closed.get());
        subscription.dispose();
    }

    #[test]
    fn title_bar_is_hit_through_the_empty_layers_with_a_compositing_renderer() {
        use crate::testing::CompositorTestServices;
        use ferroui_base::rendering::composition::CompositingRenderer;

        let services = CompositorTestServices::start(TestServices::styled_window());
        let window_impl = create_window_mock(PlatformRequestedDrawnDecoration::all());
        services.setup(&window_impl);
        let target = window_with_impl(&window_impl);
        target.show();
        extend(&window_impl);
        services.run_jobs();
        target.layout_manager().execute_layout_pass();
        services.run_jobs();

        let source = target.presentation_source();
        assert!(source.typed_renderer().as_any().is::<CompositingRenderer>());

        // The layers above the underlay: the top-level, the overlay, the (hidden) popover and the
        // resize grips.
        let host = target.top_level_host().clone();
        let children = host.visual_children().snapshot();
        assert!(children[0].is::<LayerWrapper>());
        assert!(children[1].is::<TopLevel>());
        assert!(children[2].is::<LayerWrapper>());
        assert!(children[4].is::<ResizeGripLayer>());
        assert!(children[4].is_visible());

        let input_root: &dyn IInputRoot = &**source;
        let size = host.bounds().size();
        let (w, h) = (size.width, size.height);
        use WindowDecorationsElementRole as R;

        // Shadow 8 and frame 1 in the test theme, then the 30 high title bar. Only what is drawn is
        // hit: the window without content, the overlay next to the caption buttons and the inside of
        // the grip layer let the point through to the title bar of the underlay.
        assert_eq!(Some(R::TitleBar), input_root.hit_test_chrome_element(Point::new(w / 2.0, 24.0)));
        assert_eq!(Some(R::TitleBar), input_root.hit_test_chrome_element(Point::new(20.0, 12.0)));
        // Below the title bar nothing is drawn that has a role.
        assert_eq!(None, input_root.hit_test_chrome_element(Point::new(w / 2.0, h / 2.0)));
        // The grips are still above everything else.
        assert_eq!(Some(R::ResizeN), input_root.hit_test_chrome_element(Point::new(w / 2.0, 2.0)));
        assert_eq!(Some(R::ResizeSE), input_root.hit_test_chrome_element(Point::new(w - 2.0, h - 2.0)));
        // The caption buttons of the overlay are above the title bar.
        let close = caption_button(&target, WindowDrawnDecorations::PART_CLOSE_BUTTON);
        let center = close
            .translate_point(Point::new(close.bounds().width / 2.0, close.bounds().height / 2.0), &**host)
            .expect("the button is in the tree of the host");
        assert_eq!(Some(R::CloseButton), input_root.hit_test_chrome_element(center));

        target.close();
    }
}
