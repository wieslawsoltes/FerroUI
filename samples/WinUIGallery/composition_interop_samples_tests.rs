//! Exercises the handlers of the XamlCompInterop page on a window rendered by the test
//! compositor, and the ContentIsland page.

use crate::composition_interop_samples::{
    circle_expression, damping_ratio, period, DEFAULT_DAMPING_RATIO, NO_COMPOSITOR, POPUP_EXPRESSION,
    TOTAL_ELEMENTS,
};
use crate::{examples, tests::start};
use ferroui_base::input::{
    InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
};
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::CompositionVisual;
use ferroui_base::{Point, Ref, Vector3D, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::primitives::Popup;
use ferroui_controls::shapes::{Ellipse, Rectangle};
use ferroui_controls::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
use ferroui_controls::{
    Button, Control, Grid, RadioButtons, Slider, TextBlock, Window, WrapPanel,
};
use std::rc::Rc;
use std::time::Duration;

/// The XamlCompInterop page in a window that renders through the test compositor.
struct Host {
    services: CompositorTestServices,
    window: Ref<Window>,
    root: Ref<Control>,
}

impl Host {
    fn new() -> Host {
        ferroui_markup_xaml::register_types();
        let services = CompositorTestServices::start(TestServices::styled_window());
        let root = examples::create(examples::find("XamlCompInterop").unwrap()).unwrap();
        let platform = MockWindowingPlatform::create_window_mock_with_size(900.0, 1800.0);
        services.setup(&platform);
        let window = Window::with_impl(platform);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        services.run_jobs();
        services.run_jobs();
        Host { services, window, root }
    }

    /// Renders frames for `duration` of the compositor's clock.
    fn run_for(&self, duration: Duration) {
        let server = self.services.compositor().server().clone();
        let started = server.server_now();
        while server.server_now() < started + duration {
            self.services.run_jobs();
        }
        self.services.run_jobs();
    }

    /// Renders frames for `duration`, calling `sample` after each one.
    fn sample_for(&self, duration: Duration, mut sample: impl FnMut()) {
        let server = self.services.compositor().server().clone();
        let started = server.server_now();
        while server.server_now() < started + duration {
            self.services.run_jobs();
            sample();
        }
        self.services.run_jobs();
    }

    fn control<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        self.root.get_control::<T>(name)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn element_visual(element: &Visual) -> Rc<CompositionVisual> {
    ElementCompositionPreview::get_element_visual(element).expect("the element has a composition visual")
}

/// The values the compositor renders the composition visual of `element` with.
fn server(element: &Visual) -> Rc<ServerCompositionVisual> {
    let visual = element_visual(element);
    visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).expect("the server visual exists")
}

fn assert_near(actual: Vector3D, expected: Vector3D, what: &str) {
    let close = (actual.x - expected.x).abs() < 0.01
        && (actual.y - expected.y).abs() < 0.01
        && (actual.z - expected.z).abs() < 0.01;
    assert!(close, "{what}: {actual:?}, expected {expected:?}");
}

/// Raises `event` (PointerEntered or PointerExited) of a mouse on `element`.
fn pointer(element: &Control, event: &RoutedEvent<PointerEventArgs>) {
    element.raise_event(&PointerEventArgs::new(
        Some(event),
        element.to_ref(),
        Pointer::new(1, PointerType::Mouse, true),
        None,
        Point::new(0.0, 0.0),
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    ));
}

fn enter(element: &Control) {
    pointer(element, InputElement::pointer_entered_event());
}

fn exit(element: &Control) {
    pointer(element, InputElement::pointer_exited_event());
}

#[test]
fn xaml_comp_interop_spring_scales_the_hovered_button_with_the_options() {
    let host = Host::new();
    let button = host.control::<Control>("NaturalMotionButton");
    assert_eq!(server(&button).scale(), Vector3D::new(1.0, 1.0, 1.0));

    // `element_PointerEntered`: the spring to 1.5; `element_PointerExited`: back to 1.
    // The spring of damping 0.6 overshoots 1.5 before it comes to rest there.
    enter(&button);
    let mut largest: f64 = 1.0;
    host.sample_for(Duration::from_millis(1500), || largest = largest.max(server(&button).scale().x));
    assert!(largest > 1.5 + 0.01, "the underdamped spring overshoots: {largest}");
    assert_near(server(&button).scale(), Vector3D::new(1.5, 1.5, 1.5), "entered");
    exit(&button);
    host.run_for(Duration::from_millis(1500));
    assert_near(server(&button).scale(), Vector3D::new(1.0, 1.0, 1.0), "exited");

    // `GetDampingRatio` and `GetPeriod` of the options.
    let damping = host.control::<RadioButtons>("DampingStackPanel");
    damping.set_selected_index(2);
    assert_eq!(damping_ratio(&damping), 0.6);
    damping.set_selected_index(0);
    assert_eq!(damping_ratio(&damping), 0.2);
    damping.set_selected_index(-1);
    assert_eq!(damping_ratio(&damping), DEFAULT_DAMPING_RATIO);
    let slider = host.control::<Slider>("PeriodSlider");
    assert_eq!(period(&slider), Duration::from_millis(50));
    slider.set_range_value(125.0);
    assert_eq!(period(&slider), Duration::from_millis(125));
}

#[test]
fn xaml_comp_interop_ellipse_scale_is_the_inverse_of_the_hovered_rectangle() {
    let host = Host::new();
    let rectangle = host.control::<Rectangle>("rectangle");
    let ellipse = host.control::<Ellipse>("ellipse");
    host.run_for(Duration::from_millis(50));
    assert_near(server(&ellipse).scale(), Vector3D::new(1.0, 1.0, 1.0), "at rest");

    enter(&rectangle);
    host.run_for(Duration::from_millis(1500));
    assert_near(server(&rectangle).scale(), Vector3D::new(1.5, 1.5, 1.5), "the rectangle");
    assert_near(server(&ellipse).scale(), Vector3D::new(1.0 / 1.5, 1.0 / 1.5, 1.0), "the ellipse");
    exit(&rectangle);
    host.run_for(Duration::from_millis(1500));
    assert_near(server(&ellipse).scale(), Vector3D::new(1.0, 1.0, 1.0), "the ellipse at rest");
}

#[test]
fn xaml_comp_interop_stacked_buttons_follow_the_button_above() {
    let host = Host::new();
    let buttons: Vec<Ref<Button>> =
        (1..=4).map(|index| host.control::<Button>(&format!("ExpressionButton{index}"))).collect();
    host.run_for(Duration::from_millis(50));
    for button in &buttons[1..] {
        assert_near(server(button).translation(), Vector3D::new(0.0, 0.0, 0.0), "at rest");
    }

    // The first button at twice its size: (2 - 1) * 50 moves the second, and each one below by
    // the translation of the one above it (50 % 100, 50 % 150).
    element_visual(&buttons[0]).set_scale(Vector3D::new(2.0, 2.0, 1.0));
    host.run_for(Duration::from_millis(100));
    for button in &buttons[1..] {
        assert_near(server(button).translation(), Vector3D::new(0.0, 50.0, 0.0), "below the first");
    }

    // The hover handlers of the stacked buttons scale them too.
    enter(&buttons[2]);
    host.run_for(Duration::from_millis(1500));
    assert_near(server(&buttons[2]).scale(), Vector3D::new(1.5, 1.5, 1.5), "the hovered button");
    // The hovered third follows the second: (1 - 1) * 50 + 50 % 100; the fourth follows the
    // hovered third: (1.5 - 1) * 50 + 50 % 150.
    assert_near(server(&buttons[2]).translation(), Vector3D::new(0.0, 50.0, 0.0), "the hovered button");
    assert_near(server(&buttons[3]).translation(), Vector3D::new(0.0, 75.0, 0.0), "below it");
}

/// The translation of the button `index` (from 1) of the circle in a panel of `width`.
fn on_circle(width: f64, index: usize) -> Vector3D {
    let radius = width / 2.0;
    let theta = 0.02 * radius + (2.0 * std::f64::consts::PI / TOTAL_ELEMENTS as f64) * index as f64;
    Vector3D::new(radius * theta.cos() + radius, radius * theta.sin(), 0.0)
}

#[test]
fn xaml_comp_interop_buttons_lie_on_a_circle_of_the_panel_size() {
    let host = Host::new();
    let panel = host.control::<Grid>("LayoutPanel");
    host.run_for(Duration::from_millis(50));
    let buttons: Vec<Ref<Control>> = (0..panel.children().count()).map(|i| panel.children().get(i)).collect();
    assert_eq!(buttons.len(), TOTAL_ELEMENTS);
    for (i, button) in buttons.iter().enumerate() {
        assert_eq!(AutomationProperties::get_name(button).as_deref(), Some(format!("Button {i}").as_str()));
        assert_near(server(button).translation(), on_circle(200.0, i + 1), "radius 100");
    }

    // `RadiusSlider_ValueChanged`: the panel takes the slider's size, the circle follows it.
    host.control::<Slider>("RadiusSlider").set_range_value(300.0);
    assert_eq!((panel.width(), panel.height()), (300.0, 300.0));
    host.run_for(Duration::from_millis(100));
    for (i, button) in buttons.iter().enumerate() {
        assert_near(server(button).translation(), on_circle(300.0, i + 1), "radius 150");
    }

    // A page loaded again restarts the animations of the same buttons.
    host.window.set_content(None);
    host.services.run_jobs();
    host.window.set_content(Some(Control::boxed(host.root.clone())));
    host.run_for(Duration::from_millis(100));
    assert_eq!(panel.children().count(), TOTAL_ELEMENTS);
    assert_near(server(&buttons[0]).translation(), on_circle(300.0, 1), "loaded again");
}

#[test]
fn xaml_comp_interop_popup_stays_right_of_the_text_as_it_moves_and_grows() {
    let host = Host::new();
    let popup = host.control::<Popup>("Popup");
    let target = host.control::<TextBlock>("PopupTarget");
    host.run_for(Duration::from_millis(50));
    assert!(popup.is_open(), "ActualOffsetExample_Loaded opens the popup");
    // The source's OneWay x:Bind of the font size (the slider's minimum).
    assert_eq!(target.font_size(), 12.0);
    let child = popup.child().expect("the popup has its content");
    let expected = |target: &TextBlock| {
        let bounds = target.bounds();
        Vector3D::new(bounds.x + bounds.width, bounds.y + bounds.height / 2.0 - 25.0, 0.0)
    };
    assert_near(server(&child).translation(), expected(&target), "beside the text");

    // The margin of the slider (the source's doubleToThicknessConverter) and a larger font.
    host.control::<Slider>("MarginSlider").set_range_value(20.0);
    host.control::<Slider>("FontSizeSlider").set_range_value(20.0);
    assert_eq!(target.margin(), ferroui_base::Thickness::uniform(20.0));
    assert_eq!(target.font_size(), 20.0);
    host.run_for(Duration::from_millis(100));
    assert_eq!(target.bounds().x, 20.0);
    assert_near(server(&child).translation(), expected(&target), "after the change");
}

#[test]
fn xaml_comp_interop_expressions_are_the_source_expressions_over_size_and_offset() {
    let source = "Vector3({0}*cos({1})+{2}, {0}*sin({1})+{3},0)";
    let radius = "(source.ActualSize.X / 2)";
    let theta = format!(".02 * {radius} + ((2 * Pi)/total)*index");
    let expected = source
        .replace("{0}", radius)
        .replace("{1}", &theta)
        .replace("{2}", radius)
        .replace("{3}", "0")
        .replace("ActualSize", "Size");
    assert_eq!(circle_expression(), expected);
    assert_eq!(
        POPUP_EXPRESSION,
        "Vector3(source.ActualOffset.X + source.ActualSize.X, source.ActualOffset.Y + source.ActualSize.Y / 2 - 25, 0)"
            .replace("ActualOffset", "Offset")
            .replace("ActualSize", "Size")
    );
}

#[test]
fn xaml_comp_interop_reports_a_window_without_a_compositor() {
    let _application = start();
    let root = examples::create(examples::find("XamlCompInterop").unwrap()).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    let status = root.get_control::<TextBlock>("CompositionStatus");
    assert!(status.is_visible());
    assert_eq!(status.text().as_deref(), Some(NO_COMPOSITOR));
    // The hover handlers do nothing without the spring.
    enter(&root.get_control::<Control>("NaturalMotionButton"));
    window.close();
}

#[test]
fn content_island_page_keeps_the_cards_and_says_why_it_has_no_island() {
    let _application = start();
    let root = examples::create(examples::find("ContentIsland").unwrap()).unwrap();
    assert!(!root.get_control::<Button>("LoadModelButton").is_enabled());
    let diagnostic = root.get_control::<TextBlock>("ContentIslandDiagnostic").text().unwrap_or_default();
    assert!(diagnostic.starts_with("Not applicable: ContentIsland and ChildSiteLink"), "{diagnostic}");
    let cards = root.get_control::<WrapPanel>("_rectanglePanel");
    assert_eq!(cards.children().count(), 6);
    for i in 0..cards.children().count() {
        let card = cards.children().get(i).cast::<Rectangle>().expect("a card is a rectangle");
        assert_eq!((card.width(), card.height(), card.radius_x()), (400.0, 400.0, 8.0));
    }
}
