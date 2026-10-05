//! The carousel page tests, continued: the wheel, data template, swipe
//! gesture, interactive transition, carousel swipe and visual tree lifecycle
//! groups of the reference tests.
//!
//! The reference swipe tests run the continuations of asynchronous methods by
//! executing the callbacks posted to a test synchronization context; here
//! continuations are jobs of the dispatcher of the test, so running its jobs
//! stands for both steps.

use super::carousel_page_tests::{pages, simulate_key_down};
use super::navigation_page_tests::same;
use super::tabbed_page_tests::{header_of, hp, pages_of};
use super::{
    CarouselPage, ContentPage, MultiPage, NavigatedFromEventArgs, NavigatedToEventArgs, Page, PageImpl, PageImplExt,
    PageList,
};
use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{boxed_str, test_scope, TestRoot, TestScope};
use crate::text_box_tests_input::TouchTestHelper;
use crate::{
    Border, Canvas, Carousel, Control, ControlImpl, ItemsControl, ItemsSource, Panel, ScrollViewer,
    VirtualizingCarouselPanel,
};
use ferroui_base::animation::{
    CrossFade, IClock, IGlobalClock, IPageTransition, IProgressPageTransition, PageSlide, PlayState,
    Rotate3DTransition, SlideAxis, TimeSpan,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, Key, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
    PointerType, PointerUpdateKind, PointerWheelEventArgs, RawInputModifiers,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::TranslateTransform;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, Point, Rect, Ref, Size, StaticType,
    StyledElementImpl, Vector, Visual, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn simulate_wheel(cp: &Ref<CarouselPage>, delta: Vector) {
    simulate_wheel_returns_handled(cp, delta);
}

fn simulate_wheel_returns_handled(cp: &Ref<CarouselPage>, delta: Vector) -> bool {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let e = PointerWheelEventArgs::new(
        cp.clone(),
        pointer,
        cp,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
        delta,
    );
    cp.raise_event(&e);
    e.handled()
}

/// The template of a scroll viewer: a scroll content presenter, directly or
/// in a panel.
fn scroll_viewer_template(in_panel: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let presenter = presenter.register_in_name_scope(&**scope);
        if in_panel {
            let panel = Panel::new();
            panel.children().add(presenter);
            panel.upcast()
        } else {
            presenter.upcast()
        }
    })
}

/// The template of a carousel: a scroll viewer (with the given template, if
/// any) around an items presenter.
fn carousel_template(scroll_viewer_template: Option<Rc<dyn IControlTemplate>>) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, ns| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        presenter.bind_binding(
            ItemsPresenter::items_panel_property().as_property(),
            &TemplateBinding::new(ItemsControl::items_panel_property().as_property()),
        );

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        if let Some(template) = &scroll_viewer_template {
            scroll_viewer.set_template(Some(template.clone()));
        }
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**ns))));
        scroll_viewer.register_in_name_scope(&**ns).upcast()
    })
}

/// The template of a carousel page: a carousel with the given template.
fn create_carousel_page_template(carousel_template: Rc<dyn IControlTemplate>) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<CarouselPage>(move |_, scope| {
        let carousel = Carousel::new();
        carousel.set_name(Some("PART_Carousel".to_string()));
        carousel.set_template(Some(carousel_template.clone()));
        carousel.set_horizontal_alignment(HorizontalAlignment::Stretch);
        carousel.set_vertical_alignment(VerticalAlignment::Stretch);
        carousel.register_in_name_scope(&**scope).upcast()
    })
}

fn sized_border() -> Ref<Border> {
    let border = Border::new();
    border.set_width(400.0);
    border.set_height(300.0);
    border
}

// --- WheelBehavior ---

struct Hosted {
    cp: Ref<CarouselPage>,
    root: Ref<TestRoot>,
}

impl Hosted {
    fn layout(&self) {
        self.root.layout_manager().execute_layout_pass();
        run_jobs();
    }
}

fn make_carousel(count: usize, selected_index: i32) -> Hosted {
    let cp = CarouselPage::new();
    for i in 0..count {
        pages(&cp).add(hp(&format!("P{i}")));
    }
    cp.set_selected_index(selected_index);
    let root = TestRoot::with_child(cp.clone());
    Hosted { cp, root }
}

#[test]
fn wheel_down_navigates_forward() {
    let _scope = test_scope();
    let hosted = make_carousel(3, 0);
    simulate_wheel(&hosted.cp, Vector::new(0.0, -1.0));
    assert_eq!(1, hosted.cp.selected_index());
}

#[test]
fn wheel_up_navigates_backward() {
    let _scope = test_scope();
    let hosted = make_carousel(3, 2);
    simulate_wheel(&hosted.cp, Vector::new(0.0, 1.0));
    assert_eq!(1, hosted.cp.selected_index());
}

#[test]
fn wheel_down_at_last_page_does_not_handle_event() {
    let _scope = test_scope();
    let hosted = make_carousel(3, 2);
    let handled = simulate_wheel_returns_handled(&hosted.cp, Vector::new(0.0, -1.0));
    assert_eq!(2, hosted.cp.selected_index());
    assert!(!handled);
}

#[test]
fn wheel_up_at_first_page_does_not_handle_event() {
    let _scope = test_scope();
    let hosted = make_carousel(3, 0);
    let handled = simulate_wheel_returns_handled(&hosted.cp, Vector::new(0.0, 1.0));
    assert_eq!(0, hosted.cp.selected_index());
    assert!(!handled);
}

#[test]
fn wheel_when_gesture_disabled_does_not_handle_event() {
    let _scope = test_scope();
    let hosted = make_carousel(3, 0);
    hosted.cp.set_is_gesture_enabled(false);
    let handled = simulate_wheel_returns_handled(&hosted.cp, Vector::new(0.0, -1.0));
    assert_eq!(0, hosted.cp.selected_index());
    assert!(!handled);
}

#[test]
fn wheel_handled_by_child_does_not_navigate() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    cp.set_width(400.0);
    cp.set_height(300.0);
    cp.set_is_gesture_enabled(true);
    cp.set_template(Some(create_carousel_page_template(carousel_template(None))));

    let child = Border::new();
    child.add_handler(InputElement::pointer_wheel_changed_event(), |_, e: &PointerWheelEventArgs| e.set_handled(true));

    let page0 = ContentPage::new();
    page0.set_header(boxed_str("P0"));
    page0.set_content(Some(Control::boxed(child.clone())));
    pages(&cp).add(page0.upcast());
    pages(&cp).add(hp("P1"));
    pages(&cp).add(hp("P2"));

    let root = TestRoot::with_child(cp.clone());
    root.set_client_size(Size::new(400.0, 300.0));
    root.layout_manager().execute_initial_layout_pass();

    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let wheel_args = PointerWheelEventArgs::new(
        child.clone(),
        pointer,
        &root,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
        Vector::new(0.0, -1.0),
    );
    child.raise_event(&wheel_args);

    assert!(wheel_args.handled());
    assert_eq!(0, cp.selected_index());
}

// --- DataTemplateTests ---

#[derive(Clone, Debug, PartialEq)]
struct DataItem(String);

fn first_and_second() -> [DataItem; 2] {
    [DataItem("First".to_string()), DataItem("Second".to_string())]
}

/// A content page that counts its lifecycle notifications.
#[repr(C)]
struct TrackingPage {
    base: ContentPage,
    navigated_to_count: Cell<i32>,
    navigated_from_count: Cell<i32>,
}

ferro_class!(TrackingPage: ContentPage);
ferro_impl_classes!(
    TrackingPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl PageImpl for TrackingPage {
    fn on_navigated_to(this: &Self, args: &NavigatedToEventArgs) {
        this.navigated_to_count.set(this.navigated_to_count.get() + 1);
        Self::parent_on_navigated_to(this, args);
    }

    fn on_navigated_from(this: &Self, args: &NavigatedFromEventArgs) {
        this.navigated_from_count.set(this.navigated_from_count.get() + 1);
        Self::parent_on_navigated_from(this, args);
    }
}

impl TrackingPage {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: ContentPage::construct(),
            navigated_to_count: Cell::new(0),
            navigated_from_count: Cell::new(0),
        })
    }
}

/// `new ContentPage { Header = prefix + item.Name, Content = new Border { Width = 400, Height = 300 } }`.
fn detail_page(prefix: &'static str) -> impl Fn(&DataItem) -> Ref<Page> + 'static {
    move |item| {
        let page = ContentPage::new();
        page.set_header(boxed_str(&format!("{prefix}{}", item.0)));
        page.set_content(Some(Control::boxed(sized_border())));
        page.upcast()
    }
}

fn page_template(page_factory: impl Fn(&DataItem) -> Ref<Page> + 'static) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<DataItem>(move |item, _| Some(page_factory(item).upcast()), false)
}

fn create_templated_carousel_page(
    items: impl IntoIterator<Item = DataItem>,
    page_factory: impl Fn(&DataItem) -> Ref<Page> + 'static,
) -> Hosted {
    let cp = CarouselPage::new();
    cp.set_width(400.0);
    cp.set_height(300.0);
    cp.set_items_source(Some(ItemsSource::from_values(items)));
    cp.set_page_template(Some(page_template(page_factory)));
    cp.set_template(Some(create_carousel_page_template(carousel_template(Some(scroll_viewer_template(false))))));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(cp.clone());
    root.execute_initial_layout_pass();
    run_jobs();
    Hosted { cp, root }
}

#[test]
fn items_source_selected_page_is_resolved_after_layout() {
    let _scope = test_scope();
    let hosted = create_templated_carousel_page(first_and_second(), detail_page(""));

    let selected_page = hosted.cp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("First"));
    // AUTOMATION-SEAM: the automation name of the carousel page is "Page 1 of 2: First".
}

#[test]
fn keyboard_navigation_uses_items_source_count() {
    let _scope = test_scope();
    let hosted = create_templated_carousel_page(first_and_second(), detail_page(""));
    let cp = &hosted.cp;

    simulate_key_down(cp, Key::Right);
    hosted.layout();

    assert_eq!(1, cp.selected_index());
    let selected_page = cp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("Second"));
    // AUTOMATION-SEAM: the automation name of the carousel page is "Page 2 of 2: Second".
}

#[test]
fn wheel_navigation_uses_items_source_count() {
    let _scope = test_scope();
    let hosted = create_templated_carousel_page(first_and_second(), detail_page(""));
    let cp = &hosted.cp;

    let handled = simulate_wheel_returns_handled(cp, Vector::new(0.0, -1.0));
    hosted.layout();

    assert!(handled);
    assert_eq!(1, cp.selected_index());
    let selected_page = cp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("Second"));
}

#[test]
fn items_source_selection_changes_fire_lifecycle_on_generated_pages() {
    let _scope = test_scope();
    let hosted = create_templated_carousel_page(first_and_second(), |item| {
        let page = TrackingPage::new();
        page.set_header(boxed_str(&item.0));
        page.set_content(Some(Control::boxed(sized_border())));
        page.upcast()
    });
    let cp = &hosted.cp;

    let first_page =
        cp.selected_page().and_then(|page| page.cast::<TrackingPage>()).expect("a selected tracking page");
    assert_eq!(1, first_page.navigated_to_count.get());
    assert_eq!(0, first_page.navigated_from_count.get());

    simulate_key_down(cp, Key::Right);
    hosted.layout();

    let second_page =
        cp.selected_page().and_then(|page| page.cast::<TrackingPage>()).expect("a selected tracking page");
    assert_eq!(1, first_page.navigated_from_count.get());
    assert_eq!(1, second_page.navigated_to_count.get());
    assert!(same(&second_page, &cp.current_page()));
}

#[test]
fn page_template_changed_after_containers_realized_updates_selected_page() {
    let _scope = test_scope();
    let hosted = create_templated_carousel_page(first_and_second(), detail_page("Detail "));
    let cp = &hosted.cp;

    let original_page = cp.selected_page().expect("a selected page");
    assert!(std::ptr::eq(original_page.get_type(), <ContentPage as StaticType>::TYPE));
    assert_eq!(header_of(&original_page).as_deref(), Some("Detail First"));

    cp.set_page_template(Some(page_template(detail_page("Showcase "))));

    hosted.layout();

    let updated_page = cp.selected_page().expect("a selected page");
    assert!(std::ptr::eq(updated_page.get_type(), <ContentPage as StaticType>::TYPE));
    assert!(!updated_page.ptr_eq(&original_page));
    assert_eq!(header_of(&updated_page).as_deref(), Some("Showcase First"));
}

// --- SwipeGestureTests ---

/// The global clock of a test: pulses every subscriber.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    fn pulse(&self, time: TimeSpan) {
        self.subject.on_next(time);
    }
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// Starts a test whose animations run on a clock pulsed by the test.
fn start_with_clock() -> (TestScope, Rc<MockGlobalClock>) {
    let scope = test_scope();
    let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let service: Rc<dyn IGlobalClock> = clock.clone();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(service);
    (scope, clock)
}

struct SwipeReady {
    cp: Ref<CarouselPage>,
    carousel: Ref<Carousel>,
    panel: Ref<VirtualizingCarouselPanel>,
    /// The root owns the tree.
    _root: Ref<TestRoot>,
}

fn create_swipe_ready_carousel_page() -> SwipeReady {
    let content_page = |header: &str| -> Ref<Page> {
        let page = ContentPage::new();
        page.set_header(boxed_str(header));
        page.set_content(Some(Control::boxed(sized_border())));
        page.upcast()
    };

    let cp = CarouselPage::new();
    cp.set_width(400.0);
    cp.set_height(300.0);
    cp.set_is_gesture_enabled(true);
    let transition: Rc<dyn IPageTransition> =
        Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(1.0), SlideAxis::Horizontal));
    cp.set_page_transition(Some(transition));
    cp.set_pages(Some(pages_of([&content_page("A"), &content_page("B"), &content_page("C")])));
    cp.set_template(Some(FuncControlTemplate::for_type::<CarouselPage>(|_parent, scope| {
        let carousel = Carousel::new();
        carousel.set_name(Some("PART_Carousel".to_string()));
        carousel.set_template(Some(carousel_template(Some(scroll_viewer_template(true)))));
        carousel.set_horizontal_alignment(HorizontalAlignment::Stretch);
        carousel.set_vertical_alignment(VerticalAlignment::Stretch);
        for (target, source) in [
            (ItemsControl::items_source_property().as_property(), MultiPage::pages_property().as_property()),
            (ItemsControl::item_template_property().as_property(), MultiPage::page_template_property().as_property()),
            (ItemsControl::items_panel_property().as_property(), CarouselPage::items_panel_property().as_property()),
            (Carousel::page_transition_property().as_property(), CarouselPage::page_transition_property().as_property()),
        ] {
            carousel.bind_binding(target, &TemplateBinding::new(source));
        }
        carousel.register_in_name_scope(&**scope).upcast()
    })));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(cp.clone());
    root.execute_initial_layout_pass();
    run_jobs();

    let carousels: Vec<Ref<Carousel>> =
        cp.get_visual_descendants().filter_map(|visual| visual.cast::<Carousel>()).collect();
    assert_eq!(1, carousels.len());
    let carousel = carousels.into_iter().next().unwrap();
    let panel = carousel
        .presenter()
        .expect("the carousel has a presenter")
        .panel()
        .expect("the presenter has a panel")
        .cast::<VirtualizingCarouselPanel>()
        .expect("a virtualizing carousel panel");
    assert!(carousel.is_swipe_enabled());
    let recognizers: Vec<Ref<SwipeGestureRecognizer>> = panel
        .gesture_recognizers()
        .to_vec()
        .into_iter()
        .filter_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>())
        .collect();
    assert_eq!(1, recognizers.len());
    let recognizer = &recognizers[0];
    assert!(recognizer.is_enabled());
    assert!(recognizer.can_horizontally_swipe());
    recognizer.set_is_mouse_enabled(true);
    SwipeReady { cp, carousel, panel, _root: root }
}

#[test]
fn mouse_swipe_advances_page() {
    let (_scope, clock) = start_with_clock();

    let target = create_swipe_ready_carousel_page();
    let mouse = MouseTestHelper::new();

    mouse.down_at(&target.panel, MouseButton::Left, Point::new(200.0, 100.0), 1);
    mouse.move_(&target.panel, Point::new(40.0, 100.0));
    mouse.up_at(&target.panel, MouseButton::Left, Point::new(40.0, 100.0));
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(TimeSpan::from_seconds(1.0));
    run_jobs();

    assert_eq!(1, target.carousel.selected_index());
    assert_eq!(1, target.cp.selected_index());
    assert!(same(&pages(&target.cp).get(1), &target.cp.current_page()));
}

#[test]
fn touch_swipe_advances_page() {
    let (_scope, clock) = start_with_clock();

    let target = create_swipe_ready_carousel_page();
    let touch = TouchTestHelper::new();

    touch.down(&target.panel, Point::new(200.0, 100.0));
    touch.move_(&target.panel, Point::new(40.0, 100.0));
    touch.up(&target.panel, Point::new(40.0, 100.0));
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(TimeSpan::from_seconds(1.0));
    run_jobs();

    assert_eq!(1, target.carousel.selected_index());
    assert_eq!(1, target.cp.selected_index());
    assert!(same(&pages(&target.cp).get(1), &target.cp.current_page()));
}

// --- InteractiveTransitionTests ---

/// The horizontal offset of the translate transform that is the render
/// transform of the visual, if it has one.
fn translate_x(visual: &Visual) -> Option<f64> {
    let transform = visual.render_transform()?;
    let translate = transform.as_object()?.to_ref().cast::<TranslateTransform>()?;
    Some(translate.x())
}

fn page_slide_update() -> (Ref<Border>, Ref<Border>, Ref<Canvas>) {
    let parent = Canvas::new();
    parent.set_width(400.0);
    parent.set_height(300.0);
    let from = Border::new();
    let to = Border::new();
    parent.children().add(from.clone());
    parent.children().add(to.clone());
    parent.measure(Size::new(400.0, 300.0));
    parent.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));

    let slide = PageSlide::with_duration(TimeSpan::from_milliseconds(300.0), SlideAxis::Horizontal);
    slide.update(
        0.5,
        Some(&from.clone().upcast::<Visual>()),
        Some(&to.clone().upcast::<Visual>()),
        true,
        400.0,
        &[],
    );
    (from, to, parent)
}

#[test]
fn page_slide_update_applies_translate_transform_to_from() {
    let _scope = test_scope();
    let (from, _to, _parent) = page_slide_update();

    assert_eq!(Some(-200.0), translate_x(&from));
}

#[test]
fn page_slide_update_applies_translate_transform_to_to() {
    let _scope = test_scope();
    let (_from, to, _parent) = page_slide_update();

    assert_eq!(Some(200.0), translate_x(&to));
}

#[test]
fn cross_fade_update_sets_opacity() {
    let _scope = test_scope();
    let from = Border::new();
    let to = Border::new();

    let cross_fade = CrossFade::with_duration(TimeSpan::from_milliseconds(300.0));
    cross_fade.update(0.5, Some(&from.clone().upcast::<Visual>()), Some(&to.clone().upcast::<Visual>()), true, 0.0, &[]);

    assert!((from.opacity() - 0.5).abs() < 0.005);
    assert!((to.opacity() - 0.5).abs() < 0.005);
    assert!(to.is_visible());
}

// --- CarouselIsSwipeEnabledTests ---

#[test]
fn is_swipe_enabled_default_is_false() {
    let _scope = test_scope();
    let carousel = Carousel::new();
    assert!(!carousel.is_swipe_enabled());
}

#[test]
fn is_swipe_enabled_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let carousel = Carousel::new();
        carousel.set_is_swipe_enabled(value);
        assert_eq!(value, carousel.is_swipe_enabled());
    }
}

#[test]
fn get_transition_axis_returns_null_when_no_transition() {
    let _scope = test_scope();
    let carousel = Carousel::new();
    assert!(carousel.get_transition_axis().is_none());
}

#[test]
fn get_transition_axis_returns_null_when_non_page_slide_transition() {
    let _scope = test_scope();
    let carousel = Carousel::new();
    let transition: Rc<dyn IPageTransition> = Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(200.0)));
    carousel.set_page_transition(Some(transition));
    assert!(carousel.get_transition_axis().is_none());
}

#[test]
fn get_transition_axis_returns_vertical_when_page_slide_vertical() {
    let _scope = test_scope();
    let carousel = Carousel::new();
    let transition: Rc<dyn IPageTransition> =
        Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(200.0), SlideAxis::Vertical));
    carousel.set_page_transition(Some(transition));
    assert_eq!(Some(SlideAxis::Vertical), carousel.get_transition_axis());
}

#[test]
fn get_transition_axis_returns_vertical_when_rotate_3d_vertical() {
    let _scope = test_scope();
    let carousel = Carousel::new();
    let transition: Rc<dyn IPageTransition> =
        Rc::new(Rotate3DTransition::with_duration(TimeSpan::from_milliseconds(200.0), SlideAxis::Vertical, None));
    carousel.set_page_transition(Some(transition));
    assert_eq!(Some(SlideAxis::Vertical), carousel.get_transition_axis());
}

// --- VisualTreeLifecycleTests ---

#[test]
fn detach_and_reattach_collection_changed_still_updates_selection() {
    let _scope = test_scope();
    let pages = PageList::new();
    let cp = CarouselPage::new();
    cp.set_pages(Some(pages.clone()));
    let root = TestRoot::with_child(cp.clone());

    let page1 = hp("A");
    pages.add(page1.clone());
    assert!(same(&page1, &cp.selected_page()));

    root.set_child(None);
    root.set_child(cp.clone());

    let page2 = hp("B");
    pages.add(page2.clone());

    pages.remove(&page1);
    assert!(same(&page2, &cp.selected_page()));
}
