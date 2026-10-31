//! The pane of a SplitView in an overlay mode slides over the content with its background
//! (`Controls/SplitView.xaml`; DEVIATIONS.md, BTN-3, SPV-1, SPV-7): a NavigationView whose pane is
//! at the left in the minimal mode, and a DrawerPage in the overlay mode, change the background of
//! the pane from a transparent solid colour brush to an acrylic one when the pane opens. The
//! transitions of the template are stepped with the global clock of the test.

use super::support::*;
use ferroui_base::animation::TimeSpan;
use ferroui_base::media::IBrush;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{
    Border, Control, DrawerPage, NavigationView, NavigationViewItem, NavigationViewPaneDisplayMode, Panel,
    SplitView, SplitViewDisplayMode, TextBlock, Window,
};
use std::rc::Rc;

/// The named element `name` of the template of `control`.
fn part(control: &TemplatedControl, name: &str) -> Ref<Control> {
    control
        .get_template_descendants()
        .into_iter()
        .find(|element| element.name().as_deref() == Some(name))
        .and_then(|element| element.cast::<Control>())
        .unwrap_or_else(|| panic!("the part {name} of {}", control.get_type().name()))
}

/// The pane of a split view: `PART_PaneRoot`, which clips it, and the border `PaneRoot` with its
/// background.
struct Pane {
    root: Ref<Panel>,
    border: Ref<Border>,
}

impl Pane {
    fn of(split: &SplitView) -> Self {
        Self {
            root: part(split, "PART_PaneRoot").cast::<Panel>().expect("a panel"),
            border: part(split, "PaneRoot").cast::<Border>().expect("a border"),
        }
    }

    /// The background is the acrylic of an overlaying pane, not a solid colour brush.
    fn has_overlay_background(&self) -> bool {
        self.border.background().is_some_and(|brush| brush.as_solid_color_brush().is_none())
    }

    /// The background lets the content show: no brush or a transparent solid colour.
    fn has_transparent_background(&self) -> bool {
        self.border.background().map_or(true, |brush| {
            brush.as_solid_color_brush().is_some_and(|solid| solid.color().a == 0 || brush.opacity() == 0.0)
        })
    }

    /// The width the pane takes in the window.
    fn width(&self) -> f64 {
        self.root.bounds().width
    }
}

/// Steps the clock of the test: pulses at `now` milliseconds and lays the window out.
struct Steps {
    clock: Rc<MockGlobalClock>,
    window: Ref<Window>,
    now: f64,
}

impl Steps {
    fn to(&mut self, ms: f64) {
        assert!(ms >= self.now);
        self.now = ms;
        self.clock.pulse(TimeSpan::from_milliseconds(ms));
        self.window.update_layout();
    }

    fn by(&mut self, ms: f64) {
        let now = self.now + ms;
        self.to(now);
    }
}

fn navigation_view_in_the_minimal_mode() -> (Ref<Window>, Ref<NavigationView>) {
    let navigation = NavigationView::new();
    navigation.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    navigation.set_is_pane_open(false);
    for name in ["Standard", "Scientific", "Graphing"] {
        let item = NavigationViewItem::new();
        item.set_content(Some(Rc::new(name.to_string()) as BoxedValue));
        navigation.menu_items().add(Some(Control::boxed(&item)));
    }
    let content = TextBlock::new();
    content.set_text(Some("The content"));
    navigation.set_content(Some(Control::boxed(&content)));
    let window = Window::new();
    window.set_width(360.0);
    window.set_height(480.0);
    window.set_content(Some(Control::boxed(&navigation)));
    window.show();
    window.update_layout();
    (window, navigation)
}

/// The pane of a minimal NavigationView: closed when the window is shown (not laid out at its open
/// width while the width of `PART_PaneRoot` goes from not set to 0), with its acrylic background
/// on the first frame of the opening, and with it until the closing transition has ended.
#[test]
fn the_overlay_pane_of_a_navigation_view_slides_with_its_background() {
    let (_app, clock) = start_themed_application_with_clock();
    let (window, navigation) = navigation_view_in_the_minimal_mode();
    let split = part(&navigation, "RootSplitView").cast::<SplitView>().expect("a split view");
    let pane = Pane::of(&split);
    let mut steps = Steps { clock, window: window.clone(), now: 0.0 };
    assert_eq!(SplitViewDisplayMode::Overlay, split.display_mode());

    // Shown closed: the pane takes no room from the first frame on.
    for _ in 0..4 {
        steps.by(16.0);
        assert!(!pane.root.width().is_nan(), "the width of the closed pane is set at {} ms", steps.now);
        assert_eq!(0.0, pane.width(), "the closed pane at {} ms", steps.now);
    }
    assert!(pane.has_transparent_background());

    // Opening: the background is the acrylic one from the first frame, while the pane is revealed.
    navigation.set_is_pane_open(true);
    steps.by(0.0);
    assert!(pane.has_overlay_background(), "the background on the first frame of the opening");
    steps.by(16.0);
    let opening = pane.width();
    assert!(opening > 0.0 && opening < navigation.open_pane_length(), "the pane is opening: {opening}");
    assert!(pane.has_overlay_background(), "the background while the pane opens");
    steps.by(400.0);
    assert_eq!(navigation.open_pane_length(), pane.width());
    assert!(pane.has_overlay_background());

    // Closing: the pane keeps its background until the 120 ms of the closing transition have passed.
    navigation.set_is_pane_open(false);
    assert_closes_with_its_background(&mut steps, &pane);
    window.close();
}

/// Closes the pane (already asked for): it keeps its overlay background through the closing
/// transition, and is closed with the background of a closed pane after it.
fn assert_closes_with_its_background(steps: &mut Steps, pane: &Pane) {
    let start = steps.now;
    steps.to(start);
    for at in [16.0, 60.0, 100.0] {
        steps.to(start + at);
        assert!(pane.has_overlay_background(), "the background {at} ms into the closing");
    }
    steps.to(start + 130.0);
    assert_eq!(0.0, pane.width());
    assert!(pane.has_transparent_background(), "the background once the pane is closed");
}

/// The drawer of a DrawerPage in the overlay mode (DRP-6, the same SplitView template): opened and
/// closed by the page, it has its acrylic background from the first frame of the opening to the
/// end of the closing.
#[test]
fn the_overlay_drawer_of_a_drawer_page_slides_with_its_background() {
    let (_app, clock) = start_themed_application_with_clock();
    let page = DrawerPage::new();
    page.set_display_mode(SplitViewDisplayMode::Overlay);
    let drawer = TextBlock::new();
    drawer.set_text(Some("The drawer"));
    page.set_drawer(Some(Control::boxed(&drawer)));
    let content = TextBlock::new();
    content.set_text(Some("The content"));
    page.set_content(Some(Control::boxed(&content)));
    page.set_is_open(false);
    let window = Window::new();
    window.set_width(360.0);
    window.set_height(480.0);
    window.set_content(Some(Control::boxed(&page)));
    window.show();
    window.update_layout();
    let split = part(&page, "PART_SplitView").cast::<SplitView>().expect("a split view");
    let pane = Pane::of(&split);
    let mut steps = Steps { clock, window: window.clone(), now: 0.0 };
    steps.by(16.0);
    assert_eq!(0.0, pane.width());

    page.set_is_open(true);
    steps.by(0.0);
    assert!(pane.has_overlay_background(), "the background on the first frame of the opening");
    steps.by(400.0);
    assert!(pane.width() > 0.0);
    assert!(pane.has_overlay_background());

    page.set_is_open(false);
    assert_closes_with_its_background(&mut steps, &pane);
    window.close();
}
