//! What interaction with a page of the catalog retains: the tours of
//! `catalog_tour.rs` with a page driven through raw input after it is shown,
//! as a platform backend delivers it (the input callback of the window and of
//! the popups), and what is alive read as the tours read it.
//!
//! Not ports: the upstream sample has no tests, and its runtime collects
//! what interaction leaves behind. The holders a visit without input never
//! fills are the ones measured here: the element under the pointer and the
//! captured one, the focus manager and the keyboard navigation, the tooltip
//! service, popups with their hosts and the light dismiss of the window, the
//! recycle pools of virtualising panels, selections, the undo history of text
//! boxes, the gesture recognisers.
//!
//! ```sh
//! cargo test -p control-catalog --features count-allocations --lib catalog_interaction_revisit_memory -- --ignored --nocapture --test-threads=1
//! cargo test -p control-catalog --features count-allocations --lib catalog_interaction_tour_memory -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The first prints, per page, what a second and a third visit with
//! interaction add; the second what every further tour with interaction
//! adds. The variables of `catalog_tour.rs` apply (the pages, the tours, the
//! recording and its target), and `CATALOG_TOUR_VERBOSE` prints what was
//! driven on each page.
//!
//! # What a visit drives
//!
//! In this order, each step with rendered frames, always the same for the
//! same page (the order is the one of the visual tree, the clock is the
//! manual one, a timer fires when the step fires it):
//!
//! 1. The pointer moves over the window in a grid of points 40 pixels apart
//!    (the drawer and the page).
//! 2. The pointer rests on the elements of the page with a tooltip (the
//!    first 8), and the timers of the dispatcher are fired: the tooltip
//!    opens; the pointer leaves and the timers fire again.
//! 3. Tab, 24 times, and Shift+Tab, 4 times.
//! 4. The wheel turns over the scroll viewers of the page (the first 6): 30
//!    notches down and 30 up.
//! 5. The focusable elements of the page that are visible, enabled and hit at
//!    their centre (the first 48): the pointer moves there, the left button
//!    is pressed, the pointer moves a few pixels and back, and the button is
//!    released (a click on a button, a drag on a slider, an open drop-down
//!    on a combo box, a picker, a flyout button or a menu item). A text box
//!    is typed in, with Backspace and the undo gesture. A popup that opened
//!    has the pointer moved over it, the item in its middle pressed (an item
//!    of a drop-down is selected, a sub menu opens), and what is still open
//!    is closed with Escape, and with a press in a corner of the window when
//!    Escape did not close it.
//! 6. The elements of the page with a context menu or a context flyout (the
//!    first 6): the right button is pressed and released over them, the
//!    pointer moves over the popup, Escape.
//! 7. Escape, and the timers once more.
//!
//! The pointer stays where the last step left it and the focus on the element
//! that had it: the page is navigated away from as a user leaves it.
//!
//! # What it does not drive
//!
//! - The buttons of [`NOT_PRESSED`], each with its reason, and every press
//!   on the pages of [`NO_PRESSES`] (none now): the pointer, the keyboard
//!   and the wheel are still driven there.
//! - A window a page opens is closed again as soon as the step is over.
//! - Touch, pen and gestures of more than one pointer, drag and drop between
//!   elements (the platform of the tests has no drag source), the input
//!   method client (the platform of the tests has none), real time: an
//!   animation runs by the frames of the manual clock only.
//! - More than one sample of a gallery: a press that pushes a page on the
//!   page of the catalog (a card of a gallery pushes its sample) ends the
//!   presses on the page, and the pushed page is driven by steps 2 to 6 in
//!   its place; a press there that changes the page again ends the presses
//!   of the visit, and the report counts it. The first card of a gallery is
//!   therefore the sample that is driven. A press that takes the elements
//!   still to be pressed out of the tree (a gallery that shows the sample in
//!   place of its cards) is followed by steps 2 to 6 once more, on what the
//!   page shows then.

use super::catalog_tour::{self as tour, Tour};
use super::frame_benchmark::{RasterSurface, HEIGHT, WIDTH};
use crate::models::PageItem;
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType,
    RawTextInputEventArgs,
};
use ferroui_base::input::{
    IInputDevice, IKeyboardDevice, InputElement, Key, KeyDeviceType, KeyboardDevice, MouseDevice, PhysicalKey, Pointer, PointerType,
    RawInputModifiers,
};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Point, Ref, Vector, Visual, WeakRef};
use ferroui_controls::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl, IWindowingPlatform};
use ferroui_controls::testing::{MockCall, MockWindowImpl, MockWindowingPlatform};
use ferroui_controls::{ContentControl, Control, ScrollViewer, TextBox, ToolTip, Window};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The pages no button is pressed on, each with its reason. Everything else
/// of a visit is driven on them.
const NO_PRESSES: &[(&str, &str)] = &[];

/// The buttons that are not pressed: the page, the text of the button and
/// the reason.
const NOT_PRESSED: &[(&str, &str, &str)] = &[(
    "TreeView",
    "Select Random",
    "the command walks down the tree by random indexes below ten, as upstream's does, and panics on a node that has \
     fewer children since Remove was pressed (upstream throws there)",
)];

const GRID: f64 = 40.0;
const TOOLTIPS: usize = 8;
const TAB_STEPS: usize = 24;
const SCROLL_VIEWERS: usize = 6;
const WHEEL_NOTCHES: usize = 30;
const PRESSES: usize = 48;
const CONTEXT_MENUS: usize = 6;

/// What a visit drove.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Driven {
    pub moves: usize,
    pub tooltips: usize,
    pub keys: usize,
    pub wheels: usize,
    pub presses: usize,
    pub typed: usize,
    pub context_menus: usize,
    /// The popups that opened, the tooltips among them.
    pub popups: usize,
    /// The popups Escape did not close.
    pub dismissed_by_press: usize,
    /// The popups neither closed.
    pub left_open: usize,
    /// The windows the page opened, closed again.
    pub windows: usize,
    /// The pages a press pushed on the page of the catalog (a sample of a
    /// gallery), driven as the page itself.
    pub pushed: usize,
    /// The visits in which a press replaced the content of the page (the
    /// elements still to be pressed left the tree): what is there instead
    /// was driven by the steps once more.
    pub replaced: usize,
    /// Whether a press on a pushed page changed the page again.
    pub left_the_page: bool,
}

/// How the steps on a page ended.
enum Outcome {
    /// Every element found was driven.
    Done,
    /// A press took elements that were still to be pressed out of the tree
    /// (a card of a gallery that shows its sample in place of the cards, a
    /// tab that replaces the content).
    Replaced,
    /// A press put another page on top of the stack of the navigation page.
    Pushed(Ref<Visual>),
}

type Impls = Rc<RefCell<Vec<Weak<MockWindowImpl>>>>;

/// The raw input of a tour.
pub(super) struct Driver {
    mouse: Rc<MouseDevice>,
    keyboard: Rc<KeyboardDevice>,
    timestamp: Cell<u64>,
    popups: Impls,
    windows: Impls,
    verbose: bool,
}

impl Drop for Driver {
    fn drop(&mut self) {
        crate::view_models::random::test_seed::set(None);
    }
}

/// Whether the platform implementation is shown: it was shown and neither
/// hidden nor disposed since.
fn is_shown(window_impl: &MockWindowImpl) -> bool {
    let mut shown = false;
    for call in window_impl.calls() {
        match call {
            MockCall::Show { .. } => shown = true,
            MockCall::Hide | MockCall::Dispose => shown = false,
            _ => {}
        }
    }
    shown
}

/// Makes the popups of `parent` popups of the mock platform that render
/// through `compositor`, as its window does, and records them in `popups`;
/// their own popups likewise.
fn popups_of(parent: &Rc<MockWindowImpl>, compositor: &Rc<Compositor>, popups: &Impls) {
    let weak_parent = Rc::downgrade(parent);
    let compositor = compositor.clone();
    let popups = popups.clone();
    parent.setup_create_popup(move |_| {
        let parent: Rc<dyn ITopLevelImpl> = weak_parent.upgrade()?;
        let popup = MockWindowImpl::popup(parent);
        popup.setup_compositor(Some(compositor.clone()));
        popup.setup_surfaces(vec![RasterSurface::new() as std::sync::Arc<dyn IPlatformRenderSurface>]);
        popups_of(&popup, &compositor, &popups);
        popups.borrow_mut().push(Rc::downgrade(&popup));
        Some(popup as Rc<dyn IPopupImpl>)
    });
}

impl Driver {
    /// The driver of `tour`: the popups of its window, and the windows a
    /// page creates, are recorded from here on.
    pub(super) fn new(tour: &Tour) -> Driver {
        let popups: Impls = Rc::new(RefCell::new(Vec::new()));
        let windows: Impls = Rc::new(RefCell::new(Vec::new()));
        let compositor = tour.compositor().clone();
        popups_of(tour.window_impl(), &compositor, &popups);
        if tour::environment("CATALOG_TOUR_POPUPS").as_deref() == Some("overlay") {
            // As a platform without popup windows (a browser): the popups
            // are hosted in the overlay layer of the window.
            tour.window_impl().setup_create_popup(|_| None);
        }
        let (created, created_popups) = (windows.clone(), popups.clone());
        let platform = MockWindowingPlatform::with_window_impl(move || {
            let window = MockWindowingPlatform::create_window_mock();
            window.setup_compositor(Some(compositor.clone()));
            window.setup_surfaces(vec![RasterSurface::new() as std::sync::Arc<dyn IPlatformRenderSurface>]);
            popups_of(&window, &compositor, &created_popups);
            created.borrow_mut().push(Rc::downgrade(&window));
            window as Rc<dyn IWindowImpl>
        });
        FerroLocator::current_mutable().bind::<dyn IWindowingPlatform>().to_constant(platform as Rc<dyn IWindowingPlatform>);
        // The services of the tours have no keyboard device: the one a
        // platform registers.
        let keyboard = KeyboardDevice::new();
        FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard.clone() as Rc<dyn IKeyboardDevice>);
        // The generators of the view models (a random selection, a random
        // colour) give the same numbers in every run.
        crate::view_models::random::test_seed::set(Some(42));
        Driver {
            mouse: MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true)),
            keyboard,
            timestamp: Cell::new(0),
            popups,
            windows,
            verbose: tour::environment("CATALOG_TOUR_VERBOSE").is_some(),
        }
    }

    fn next_timestamp(&self) -> u64 {
        // Further apart than a double click.
        self.timestamp.set(self.timestamp.get() + 1000);
        self.timestamp.get()
    }

    fn input(&self, target: &MockWindowImpl, args: Rc<dyn IRawInputEventArgs>) {
        if let Some(input) = ITopLevelImpl::input(target) {
            input(args);
        }
    }

    fn pointer(&self, target: &MockWindowImpl, kind: RawPointerEventType, position: Point, modifiers: RawInputModifiers) {
        let Some(root) = target.input_root() else { return };
        let device: Rc<dyn IInputDevice> = self.mouse.clone();
        self.input(target, Rc::new(RawPointerEventArgs::new(device, self.next_timestamp(), root, kind, position, modifiers)));
    }

    fn wheel(&self, target: &MockWindowImpl, position: Point, delta: Vector) {
        let Some(root) = target.input_root() else { return };
        let device: Rc<dyn IInputDevice> = self.mouse.clone();
        self.input(
            target,
            Rc::new(RawMouseWheelEventArgs::new(device, self.next_timestamp(), root, position, delta, RawInputModifiers::NONE)),
        );
    }

    fn key(&self, target: &MockWindowImpl, key: Key, physical_key: PhysicalKey, modifiers: RawInputModifiers) {
        let Some(root) = target.input_root() else { return };
        for kind in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
            let device: Rc<dyn IInputDevice> = self.keyboard.clone();
            self.input(
                target,
                Rc::new(RawKeyEventArgs::new(
                    device,
                    self.next_timestamp(),
                    root.clone(),
                    kind,
                    key,
                    modifiers,
                    physical_key,
                    None,
                    KeyDeviceType::Keyboard,
                )),
            );
        }
    }

    fn text(&self, target: &MockWindowImpl, text: &str) {
        let Some(root) = target.input_root() else { return };
        let device: Rc<dyn IInputDevice> = self.keyboard.clone();
        self.input(target, Rc::new(RawTextInputEventArgs::new(device, self.next_timestamp(), root, text)));
    }

    /// Fires every timer of the dispatcher once, as if its time had come.
    fn fire_timers(&self) {
        for timer in Dispatcher::timers_for_unit_tests() {
            Dispatcher::force_fire_timer_for_unit_tests(&timer);
        }
    }

    /// The popups that are shown, in the order they were created.
    fn open_popups(&self) -> Vec<Rc<MockWindowImpl>> {
        let mut popups = self.popups.borrow_mut();
        popups.retain(|popup| popup.strong_count() > 0);
        popups.iter().filter_map(Weak::upgrade).filter(|popup| is_shown(popup)).collect()
    }

    /// Whether a popup is open: a popup of the platform, or one hosted in
    /// the overlay layer of the window.
    fn popup_is_open(&self, tour: &Tour) -> bool {
        !self.open_popups().is_empty() || overlay_popups(tour) > 0
    }

    /// Closes the windows a page opened.
    fn close_windows(&self, driven: &mut Driven) {
        let windows: Vec<Rc<MockWindowImpl>> = {
            let mut windows = self.windows.borrow_mut();
            windows.retain(|window| window.strong_count() > 0);
            windows.iter().filter_map(Weak::upgrade).filter(|window| is_shown(window)).collect()
        };
        for window_impl in windows {
            let Some(root) = window_impl.input_root() else { continue };
            // The root element of a window is its host; the window is in it.
            let root_element: Ref<Visual> = root.root_element().upcast();
            if let Some(window) = root_element.find_descendant_of_type::<Window>(true) {
                window.close();
                driven.windows += 1;
            }
        }
    }

    /// After a press: the pointer moves over the popups that opened and
    /// presses the middle of the last, and what is still open is closed.
    fn popups_opened(&self, tour: &Tour, driven: &mut Driven, press_inside: bool) {
        tour.frame();
        let open = self.open_popups();
        let overlays = overlay_popups(tour);
        if open.is_empty() && overlays == 0 {
            return;
        }
        driven.popups += open.len() + overlays;
        for popup in &open {
            let size = popup.client_size.get();
            for (x, y) in [(0.5, 0.25), (0.5, 0.5), (0.5, 0.75)] {
                self.pointer(popup, RawPointerEventType::Move, Point::new(size.width * x, size.height * y), RawInputModifiers::NONE);
                tour.frame();
            }
        }
        if press_inside {
            if let Some(popup) = open.last() {
                let size = popup.client_size.get();
                let middle = Point::new(size.width * 0.5, size.height * 0.5);
                self.pointer(popup, RawPointerEventType::LeftButtonDown, middle, RawInputModifiers::LEFT_MOUSE_BUTTON);
                self.pointer(popup, RawPointerEventType::LeftButtonUp, middle, RawInputModifiers::NONE);
                tour.frame();
            }
        }
        self.close_popups(tour, driven);
    }

    /// Closes what is open: Escape, once for every level and once more; then
    /// a press in the corner of the window.
    fn close_popups(&self, tour: &Tour, driven: &mut Driven) {
        for _ in 0..4 {
            if !self.popup_is_open(tour) {
                return;
            }
            self.key(tour.window_impl(), Key::Escape, PhysicalKey::Escape, RawInputModifiers::NONE);
            driven.keys += 1;
            tour.frame();
        }
        if !self.popup_is_open(tour) {
            return;
        }
        driven.dismissed_by_press += 1;
        let corner = Point::new(WIDTH - 2.0, HEIGHT - 2.0);
        self.pointer(tour.window_impl(), RawPointerEventType::Move, corner, RawInputModifiers::NONE);
        self.pointer(tour.window_impl(), RawPointerEventType::LeftButtonDown, corner, RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.pointer(tour.window_impl(), RawPointerEventType::LeftButtonUp, corner, RawInputModifiers::NONE);
        tour.frame();
        self.fire_timers();
        tour.frame();
        if self.popup_is_open(tour) {
            driven.left_open += 1;
        }
    }

    /// Drives the page `item`, which the tour shows.
    pub(super) fn drive(&self, tour: &Tour, item: &Rc<PageItem>) -> Driven {
        let mut driven = Driven::default();
        let header = item.header();
        let window = tour.window_impl().clone();
        let Some(page) = top_page(tour) else { return driven };

        // 1. The pointer over the window.
        let mut y = GRID / 2.0;
        while y < HEIGHT {
            let mut x = GRID / 2.0;
            while x < WIDTH {
                self.pointer(&window, RawPointerEventType::Move, Point::new(x, y), RawInputModifiers::NONE);
                driven.moves += 1;
                x += GRID;
            }
            tour.frame();
            y += GRID;
        }

        // 2. to 6. The page, and the page a press on it pushed.
        let presses = !NO_PRESSES.iter().any(|(name, _)| *name == header);
        match self.drive_page(tour, &header, &page, presses, &mut driven) {
            Outcome::Done => {}
            Outcome::Replaced => {
                driven.replaced += 1;
                if matches!(self.drive_page(tour, &header, &page, presses, &mut driven), Outcome::Pushed(_)) {
                    driven.left_the_page = true;
                }
            }
            Outcome::Pushed(pushed) => {
                driven.pushed += 1;
                if matches!(self.drive_page(tour, &header, &pushed, presses, &mut driven), Outcome::Pushed(_)) {
                    driven.left_the_page = true;
                }
            }
        }

        // 7. The end of the visit.
        self.key(&window, Key::Escape, PhysicalKey::Escape, RawInputModifiers::NONE);
        driven.keys += 1;
        tour.frame();
        self.fire_timers();
        self.close_popups(tour, &mut driven);
        // A window may be shown a few frames after the press that asked for
        // it (the dialog with the snapshot of the OpenGL page, shown when
        // the snapshot is there).
        for _ in 0..2 {
            tour.settle();
            self.close_windows(&mut driven);
        }
        tour.settle();
        // The mock of the window records every call made on it (the cursor
        // of every pointer move): the record is not what a visit retains.
        window.clear_calls();
        if self.verbose {
            println!("    {header}: {driven:?}");
        }
        driven
    }

    /// Steps 2 to 6 on `page`, the page on top of the stack of the
    /// navigation page.
    fn drive_page(&self, tour: &Tour, header: &str, page: &Ref<Visual>, presses: bool, driven: &mut Driven) -> Outcome {
        let window = tour.window_impl().clone();

        // 2. Tooltips.
        for (element, _) in targets(tour, page, TOOLTIPS, &|control| ToolTip::get_tip(control).is_some(), false) {
            let Some(centre) = element.upgrade().and_then(|element| centre_in_window(tour, &element)) else { continue };
            self.pointer(&window, RawPointerEventType::Move, centre, RawInputModifiers::NONE);
            tour.frame();
            self.fire_timers();
            tour.frame();
            driven.tooltips += 1;
            driven.popups += self.open_popups().len() + overlay_popups(tour);
            self.pointer(&window, RawPointerEventType::Move, Point::new(WIDTH - 2.0, HEIGHT - 2.0), RawInputModifiers::NONE);
            tour.frame();
            self.fire_timers();
            tour.frame();
        }

        // 3. The keyboard navigation.
        for step in 0..TAB_STEPS + 4 {
            let modifiers = if step < TAB_STEPS { RawInputModifiers::NONE } else { RawInputModifiers::SHIFT };
            self.key(&window, Key::Tab, PhysicalKey::Tab, modifiers);
            driven.keys += 1;
            tour.frame();
        }

        // 4. The wheel over the scroll viewers.
        for (element, _) in targets(tour, page, SCROLL_VIEWERS, &|control| control.is::<ScrollViewer>(), false) {
            let Some(centre) = element.upgrade().and_then(|element| centre_in_window(tour, &element)) else { continue };
            self.pointer(&window, RawPointerEventType::Move, centre, RawInputModifiers::NONE);
            for direction in [-1.0, 1.0] {
                for notch in 0..WHEEL_NOTCHES {
                    self.wheel(&window, centre, Vector::new(0.0, direction));
                    driven.wheels += 1;
                    if notch % 3 == 2 {
                        tour.frame();
                    }
                }
            }
            tour.frame();
        }
        if !presses {
            return Outcome::Done;
        }
        let mut gone = 0;
        let on_top = |tour: &Tour| top_page(tour).filter(|top| !Ref::ptr_eq(top, page));

        // 5. The focusable elements.
        let focusable =
            |control: &Ref<Control>| control.focusable() && control.is_effectively_enabled() && !control.is::<ScrollViewer>();
        for (element, is_text_box) in targets(tour, page, PRESSES, &focusable, true) {
            if !element.upgrade().is_some_and(|element| element.is_attached_to_visual_tree()) {
                gone += 1;
                continue;
            }
            let Some(centre) = element.upgrade().and_then(|element| centre_in_window(tour, &element)) else { continue };
            // The element is still the one hit there: an earlier press may
            // have covered or moved it.
            if !element.upgrade().is_some_and(|element| is_hit_at(tour, &element, centre)) {
                continue;
            }
            if element.upgrade().is_some_and(|element| is_not_pressed(header, &element)) {
                continue;
            }
            let pressed = if self.verbose { element.upgrade().map(|element| path(&element)) } else { None };
            let nearby = Point::new(centre.x + 3.0, centre.y + 1.0);
            self.pointer(&window, RawPointerEventType::Move, centre, RawInputModifiers::NONE);
            self.pointer(&window, RawPointerEventType::LeftButtonDown, centre, RawInputModifiers::LEFT_MOUSE_BUTTON);
            self.pointer(&window, RawPointerEventType::Move, nearby, RawInputModifiers::LEFT_MOUSE_BUTTON);
            self.pointer(&window, RawPointerEventType::Move, centre, RawInputModifiers::LEFT_MOUSE_BUTTON);
            self.pointer(&window, RawPointerEventType::LeftButtonUp, centre, RawInputModifiers::NONE);
            driven.presses += 1;
            tour.frame();
            if is_text_box {
                self.text(&window, "a");
                self.text(&window, "b");
                self.key(&window, Key::Back, PhysicalKey::Backspace, RawInputModifiers::NONE);
                self.key(&window, Key::Z, PhysicalKey::Z, RawInputModifiers::CONTROL);
                self.key(&window, Key::Z, PhysicalKey::Z, RawInputModifiers::META);
                driven.typed += 1;
                driven.keys += 3;
            }
            self.popups_opened(tour, driven, true);
            // A navigation the press started runs to its end.
            tour.settle();
            self.close_windows(driven);
            if let Some(top) = on_top(tour) {
                if self.verbose {
                    println!("      {header}: another page after press {} on {}", driven.presses, pressed.unwrap_or_default());
                }
                return Outcome::Pushed(top);
            }
        }

        // 6. Context menus and context flyouts.
        let has_menu = |control: &Ref<Control>| control.context_menu().is_some() || control.context_flyout().is_some();
        for (element, _) in targets(tour, page, CONTEXT_MENUS, &has_menu, false) {
            let Some(centre) = element.upgrade().and_then(|element| centre_in_window(tour, &element)) else { continue };
            self.pointer(&window, RawPointerEventType::Move, centre, RawInputModifiers::NONE);
            self.pointer(&window, RawPointerEventType::RightButtonDown, centre, RawInputModifiers::RIGHT_MOUSE_BUTTON);
            self.pointer(&window, RawPointerEventType::RightButtonUp, centre, RawInputModifiers::NONE);
            driven.context_menus += 1;
            self.popups_opened(tour, driven, false);
            self.close_windows(driven);
            tour.settle();
            if let Some(top) = on_top(tour) {
                return Outcome::Pushed(top);
            }
        }
        if gone > 0 {
            Outcome::Replaced
        } else {
            Outcome::Done
        }
    }
}

/// Whether `element` is a button of [`NOT_PRESSED`] on the page `header`.
fn is_not_pressed(header: &str, element: &Ref<Control>) -> bool {
    let Some(content) = element.cast::<ContentControl>().and_then(|control| control.content()) else { return false };
    let Some(text) = ferroui_base::metadata::from_markup_value::<String>(&Some(content)) else { return false };
    NOT_PRESSED.iter().any(|(page, button, _)| *page == header && *button == text)
}

/// The page on top of the stack of the navigation page of the catalog.
fn top_page(tour: &Tour) -> Option<Ref<Visual>> {
    let stack = tour.view_model().navigator()?.navigation_stack();
    stack.last().map(|page| page.clone().upcast())
}

/// The classes of `element` and of its nearest ancestors, with the name of
/// the element.
fn path(element: &Ref<Control>) -> String {
    let visual: &Visual = element;
    let ancestors: Vec<&str> = visual.get_visual_ancestors().take(5).map(|ancestor| ancestor.get_type().name()).collect();
    format!("{} {:?} in {}", element.get_type().name(), element.name().unwrap_or_default(), ancestors.join(" in "))
}

/// The popups hosted in the overlay layer of the window of the tour.
fn overlay_popups(tour: &Tour) -> usize {
    use ferroui_controls::primitives::OverlayPopupHost;
    let root: &Visual = tour.window();
    root.get_visual_descendants().filter(|visual| visual.is::<OverlayPopupHost>()).count()
}

/// The centre of `element` in the window of the tour, when the element is
/// in its tree, visible, and the centre inside the window.
fn centre_in_window(tour: &Tour, element: &Ref<Control>) -> Option<Point> {
    if !element.is_attached_to_visual_tree() || !element.is_effectively_visible() {
        return None;
    }
    let size = element.bounds().size();
    if size.width < 1.0 || size.height < 1.0 {
        return None;
    }
    let root: &Visual = tour.window();
    let centre = element.translate_point(Point::new(size.width / 2.0, size.height / 2.0), root)?;
    (centre.x >= 0.0 && centre.y >= 0.0 && centre.x < WIDTH && centre.y < HEIGHT).then_some(centre)
}

/// Whether the element hit at `point` of the window is `element` or inside
/// it.
fn is_hit_at(tour: &Tour, element: &Ref<Control>, point: Point) -> bool {
    let root: &InputElement = tour.window();
    let visual: &Visual = element;
    root.input_hit_test(point).is_some_and(|hit| {
        let hit: Ref<Visual> = hit.upcast();
        std::ptr::eq::<Visual>(&*hit, visual) || visual.is_visual_ancestor_of(&hit)
    })
}

/// The controls under `page`, in the order of the visual tree, that `wanted`
/// accepts and that have their centre in the window (and, with `hit`, are
/// what is hit there), at most `count`, with whether each is a text box.
fn targets(
    tour: &Tour,
    page: &Ref<Visual>,
    count: usize,
    wanted: &dyn Fn(&Ref<Control>) -> bool,
    hit: bool,
) -> Vec<(WeakRef<Control>, bool)> {
    let mut found = Vec::new();
    let mut pending: Vec<Ref<Visual>> = vec![page.clone()];
    while let Some(visual) = pending.pop() {
        if found.len() >= count {
            break;
        }
        for child in visual.get_visual_children().iter().rev() {
            pending.push(child.clone());
        }
        let Some(control) = visual.cast::<Control>() else { continue };
        if !wanted(&control) {
            continue;
        }
        let Some(centre) = centre_in_window(tour, &control) else { continue };
        if hit && !is_hit_at(tour, &control, centre) {
            continue;
        }
        found.push((control.downgrade(), control.is::<TextBox>()));
    }
    found
}

/// The totals of what the visits of a measurement drove.
#[derive(Default)]
struct Totals {
    visits: Cell<usize>,
    driven: RefCell<Driven>,
    left: RefCell<Vec<String>>,
    open: RefCell<Vec<String>>,
}

impl Totals {
    fn add(&self, header: &str, driven: &Driven) {
        self.visits.set(self.visits.get() + 1);
        let mut total = self.driven.borrow_mut();
        total.moves += driven.moves;
        total.tooltips += driven.tooltips;
        total.keys += driven.keys;
        total.wheels += driven.wheels;
        total.presses += driven.presses;
        total.typed += driven.typed;
        total.context_menus += driven.context_menus;
        total.popups += driven.popups;
        total.dismissed_by_press += driven.dismissed_by_press;
        total.left_open += driven.left_open;
        total.windows += driven.windows;
        total.pushed += driven.pushed;
        total.replaced += driven.replaced;
        let note = |list: &RefCell<Vec<String>>| {
            let mut list = list.borrow_mut();
            if !list.iter().any(|name| name == header) {
                list.push(header.to_string());
            }
        };
        if driven.left_the_page {
            note(&self.left);
        }
        if driven.left_open > 0 {
            note(&self.open);
        }
    }

    fn print(&self) {
        let total = self.driven.borrow();
        println!(
            "  driven in {} visits: {} pointer moves over the grid, {} tooltips, {} keys, {} wheel notches, {} presses \
             ({} with typing), {} context menus; {} popups opened ({} closed by a press outside, {} left open), {} windows \
             opened and closed, {} pages pushed by a press and driven, {} \
             contents replaced by a press and driven",
            self.visits.get(),
            total.moves,
            total.tooltips,
            total.keys,
            total.wheels,
            total.presses,
            total.typed,
            total.context_menus,
            total.popups,
            total.dismissed_by_press,
            total.left_open,
            total.windows,
            total.pushed,
            total.replaced,
        );
        if !self.left.borrow().is_empty() {
            println!("  a press on a pushed page changed the page again (the presses of the visit ended there): {}", self.left.borrow().join(", "));
        }
        if !self.open.borrow().is_empty() {
            println!("  a popup stayed open: {}", self.open.borrow().join(", "));
        }
    }
}

#[test]
#[ignore = "measurement: run with --features count-allocations --ignored --nocapture --test-threads=1"]
fn catalog_interaction_revisit_memory() {
    let totals = Totals::default();
    tour::measure_revisits("catalog revisits with interaction", Driver::new, |tour, driver, page| {
        totals.add(&page.header(), &driver.drive(tour, page));
    });
    totals.print();
}

#[test]
#[ignore = "measurement: run with --features count-allocations --ignored --nocapture --test-threads=1"]
fn catalog_interaction_tour_memory() {
    let totals = Totals::default();
    tour::measure_tours("catalog tour with interaction", Driver::new, |tour, driver, page| {
        totals.add(&page.header(), &driver.drive(tour, page));
    });
    totals.print();
}

/// The elements of an interactive visit to `item` that are alive after the
/// catalog went back to the home page: of the visual trees of the pages on
/// the stack of the navigation page at the end of the visit, by class, each
/// with how many. The pointer is where the visit left it and the focus was
/// on the page: what the devices and the services of the window remember of
/// the page is among them.
fn survivors(tour: &Tour, driver: &Driver, item: &Rc<PageItem>) -> (Driven, usize, Vec<(String, usize)>) {
    let home = tour.view_model().home_item();
    tour.show(item);
    let driven = driver.drive(tour, item);
    let mut elements: Vec<(&'static str, WeakRef<Visual>)> = Vec::new();
    let stack = tour.view_model().navigator().expect("the navigator of the catalog").navigation_stack();
    for page in stack.iter() {
        let page: Ref<Visual> = page.clone().upcast();
        elements.push((page.get_type().name(), page.downgrade()));
        for visual in page.get_visual_descendants() {
            elements.push((visual.get_type().name(), visual.downgrade()));
        }
    }
    drop(stack);
    tour.show(&home);
    tour.settle();
    let mut alive: Vec<(String, usize)> = Vec::new();
    for (name, element) in &elements {
        if element.upgrade().is_some() {
            match alive.iter_mut().find(|(class, _)| class == name) {
                Some((_, count)) => *count += 1,
                None => alive.push((name.to_string(), 1)),
            }
        }
    }
    (driven, elements.len(), alive)
}

/// What the catalog keeps of a page it interacted with, page by page: the
/// elements that are alive on the home page afterwards.
#[test]
#[ignore = "measurement: run with --ignored --nocapture --test-threads=1"]
fn catalog_interaction_survivors() {
    let tour = Tour::start();
    let driver = Driver::new(&tour);
    let home = tour.view_model().home_item();
    let pages: Vec<Rc<PageItem>> = tour::selected_pages(&tour).into_iter().filter(|page| !Rc::ptr_eq(page, &home)).collect();
    println!("catalog survivors of an interactive visit: {} pages", pages.len());
    for page in &pages {
        let (_, elements, alive) = survivors(&tour, &driver, page);
        let total: usize = alive.iter().map(|(_, count)| count).sum();
        if total > 0 {
            let classes: Vec<String> = alive.iter().map(|(class, count)| format!("{count} {class}")).collect();
            println!("  {:<28} {total} of {elements} elements alive: {}", page.header(), classes.join(", "));
        }
    }
}

/// Nothing of a page the pointer moved over, that had the focus, whose
/// popups were opened and whose text boxes were typed in is alive once the
/// catalog shows another page: neither the devices (the element under the
/// pointer, the captured one, the focused one), nor the services of the
/// window (tooltips, access keys, the light dismiss of popups), nor a popup
/// keep an element of it.
#[test]
fn the_catalog_frees_the_page_it_interacted_with() {
    let tour = Tour::start();
    let driver = Driver::new(&tour);
    let pages = tour.pages();
    for name in [
        "Buttons", "ComboBox", "Slider", "AutoCompleteBox", "TextBox", "ListBox", "TreeView", "CalendarDatePicker",
        "Date/Time Picker", "ContextMenu", "Flyouts", "Menu", "TabControl", "ToolTip", "Focus",
    ] {
        let page = pages.iter().find(|page| page.header() == name).expect("a page of the list");
        let (driven, elements, alive) = survivors(&tour, &driver, page);
        assert!(elements > 10, "{name}");
        assert!(alive.is_empty(), "{name}: alive after the home page replaced it: {alive:?} ({driven:?})");
    }
}

/// The harness itself: a visit with interaction moves the pointer, presses,
/// types and opens popups, closes what it opened, and the page is still the
/// one shown.
#[test]
fn an_interactive_visit_drives_the_page() {
    let tour = Tour::start();
    let driver = Driver::new(&tour);
    let pages = tour.pages();
    let page = |name: &str| pages.iter().find(|page| page.header() == name).expect("a page of the list").clone();

    let combo_box = page("ComboBox");
    assert!(tour.show(&combo_box));
    let driven = driver.drive(&tour, &combo_box);
    assert!(driven.moves > 100 && driven.keys > 20 && driven.presses > 0, "{driven:?}");
    assert!(driven.popups > 0, "a drop-down opened: {driven:?}");
    assert_eq!(0, driven.left_open, "{driven:?}");
    assert!(!driver.popup_is_open(&tour));
    assert!(tour.is_shown(&combo_box));

    // A gallery: the first card pushes its sample, which is driven.
    let text_box = page("TextBox");
    assert!(tour.show(&text_box));
    let driven = driver.drive(&tour, &text_box);
    assert_eq!(1, driven.pushed, "{driven:?}");
    assert!(driven.typed > 0, "{driven:?}");
    assert!(tour.show(&tour.view_model().home_item()));
}
