//! What interaction with a control retains, control by control: the control
//! is created, shown in the window of the tours under the Fluent theme,
//! driven through the raw input of the window and of its popups
//! (`interaction_tour.rs`), taken out of the tree and dropped, and nothing
//! of it may be alive afterwards: not the control, not an element of its
//! template, not a container it realized, not the content of a popup it
//! opened. The devices and the services of the window and of the
//! application (the element under the pointer and the captured one, the
//! focus manager, the tooltip service, the light dismiss of popups, the
//! overlay layer, the timers) then hold nothing of it.
//!
//! Not ports: the upstream controls have no such tests, and the runtime of
//! upstream collects what a control leaves behind.
//!
//! Each scenario runs twice: with the popups as popups of the platform, and
//! hosted in the overlay layer of the window (a platform without popup
//! windows). `interaction_control_survivors` prints what every scenario
//! leaves, with the allocation trace when it is asked for:
//!
//! ```sh
//! cargo test -p control-catalog --features count-allocations --lib interaction_control_survivors -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `INTERACTION_SCENARIO` names the scenarios to run (a part of the name);
//! `CATALOG_TOUR_TRACE` records the blocks each leaves and prints them as
//! the revisits of `catalog_tour.rs` print theirs.

use super::catalog_tour::{self as tour, Tour};
use super::interaction_tour::Driver;
use super::support::*;
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, Point, Ref, Vector, Visual, WeakRef};
use ferroui_controls::primitives::{OverlayPopupHost, ToggleButton};
use ferroui_controls::testing::MockWindowImpl;
use ferroui_controls::{
    AutoCompleteBox, Button, Carousel, ComboBox, ContentControl, Control, ItemsControl, ItemsSource, ListBox, ListBoxItem, MenuItem,
    RepeatButton, ScrollViewer, Slider, TabItem, TextBox, ToolTip, TreeViewItem, VirtualizingStackPanel,
};
use std::collections::HashSet;
use std::rc::Rc;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// The window of the tours with a driver of raw input.
struct Bench {
    // Declared before the tour: dropped before the window closes.
    driver: Driver,
    tour: Tour,
}

/// The elements of a scenario, held weakly.
#[derive(Default)]
struct Watch {
    seen: HashSet<usize>,
    elements: Vec<(&'static str, WeakRef<Visual>)>,
}

impl Watch {
    /// Adds `root` and its visual descendants.
    fn add(&mut self, root: &Ref<Visual>) {
        let mut pending = vec![root.clone()];
        while let Some(visual) = pending.pop() {
            pending.extend(visual.get_visual_children().iter().cloned());
            if self.seen.insert(&*visual as *const Visual as usize) {
                self.elements.push((visual.get_type().name(), visual.downgrade()));
            }
        }
    }

    /// The classes of the elements that are alive, each with how many.
    fn alive(&self) -> Vec<String> {
        let mut alive: Vec<(&str, usize)> = Vec::new();
        for (name, element) in &self.elements {
            if element.upgrade().is_some() {
                match alive.iter_mut().find(|(class, _)| class == name) {
                    Some((_, count)) => *count += 1,
                    None => alive.push((name, 1)),
                }
            }
        }
        alive.into_iter().map(|(class, count)| format!("{count} {class}")).collect()
    }
}

impl Bench {
    fn start(overlay: bool) -> Bench {
        let tour = Tour::start();
        let driver = Driver::with_popups(&tour, overlay);
        Bench { driver, tour }
    }

    fn window(&self) -> &Rc<MockWindowImpl> {
        self.tour.window_impl()
    }

    fn frame(&self) {
        self.tour.frame();
    }

    fn frames(&self, count: usize) {
        for _ in 0..count {
            self.tour.frame();
        }
    }

    /// Fires the timers of the dispatcher, between two frames.
    fn timers(&self) {
        self.frame();
        self.driver.fire_timers();
        self.frame();
    }

    fn move_to(&self, point: Point) {
        self.driver.pointer(self.window(), RawPointerEventType::Move, point, RawInputModifiers::NONE);
        self.frame();
    }

    fn press(&self, point: Point) {
        self.driver.pointer(self.window(), RawPointerEventType::Move, point, RawInputModifiers::NONE);
        self.driver.pointer(self.window(), RawPointerEventType::LeftButtonDown, point, RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.frame();
    }

    fn drag_to(&self, point: Point) {
        self.driver.pointer(self.window(), RawPointerEventType::Move, point, RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.frame();
    }

    fn release(&self, point: Point) {
        self.driver.pointer(self.window(), RawPointerEventType::LeftButtonUp, point, RawInputModifiers::NONE);
        self.frame();
    }

    fn click(&self, point: Point) {
        self.press(point);
        self.release(point);
    }

    fn click_with(&self, point: Point, modifiers: RawInputModifiers) {
        let window = self.window();
        self.driver.pointer(window, RawPointerEventType::Move, point, modifiers);
        self.driver.pointer(window, RawPointerEventType::LeftButtonDown, point, modifiers | RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.driver.pointer(window, RawPointerEventType::LeftButtonUp, point, modifiers);
        self.frame();
    }

    fn right_click(&self, point: Point) {
        let window = self.window();
        self.driver.pointer(window, RawPointerEventType::Move, point, RawInputModifiers::NONE);
        self.driver.pointer(window, RawPointerEventType::RightButtonDown, point, RawInputModifiers::RIGHT_MOUSE_BUTTON);
        self.driver.pointer(window, RawPointerEventType::RightButtonUp, point, RawInputModifiers::NONE);
        self.frame();
    }

    fn wheel(&self, point: Point, notches: f64) {
        self.driver.wheel(self.window(), point, Vector::new(0.0, notches));
        self.frame();
    }

    fn key(&self, key: Key, physical_key: PhysicalKey) {
        self.key_with(key, physical_key, RawInputModifiers::NONE);
    }

    fn key_with(&self, key: Key, physical_key: PhysicalKey, modifiers: RawInputModifiers) {
        self.driver.key(self.window(), key, physical_key, modifiers);
        self.frame();
    }

    /// Closes the popups that are open: Escape for every level, then a
    /// press in the corner of the window.
    fn close(&self) {
        self.driver.close_popups(&self.tour, &mut Default::default());
    }

    fn escape(&self) {
        self.key(Key::Escape, PhysicalKey::Escape);
    }

    fn text(&self, text: &str) {
        self.driver.text(self.window(), text);
    }

    /// The centre of `element` in the window.
    fn centre(&self, element: &Control) -> Point {
        let size = element.bounds().size();
        let root: &Visual = self.tour.window();
        element.translate_point(Point::new(size.width / 2.0, size.height / 2.0), root).expect("the element is in the window")
    }

    fn is_popup_open(&self) -> bool {
        self.driver.popup_is_open(&self.tour)
    }

    /// The popup hosts in the overlay layer of the window.
    fn overlay_hosts(&self) -> Vec<Ref<Visual>> {
        let root: &Visual = self.tour.window();
        root.get_visual_descendants().filter(|visual| visual.is::<OverlayPopupHost>()).collect()
    }

    /// Adds what the open popups show to `watch`.
    fn watch_popups(&self, watch: &mut Watch) {
        for popup in self.driver.open_popups() {
            if let Some(root) = popup.input_root().and_then(|root| root.try_root_element()) {
                watch.add(&root.upcast());
            }
        }
        for host in self.overlay_hosts() {
            watch.add(&host);
        }
    }

    /// Moves the pointer to, or (with `press`) presses and releases the left
    /// button at, the point of the popup opened last that is `x` and `y` of
    /// its width and height from its corner.
    fn in_popup(&self, x: f64, y: f64, press: bool) {
        if let Some(popup) = self.driver.open_popups().last() {
            let size = popup.client_size.get();
            let point = Point::new(size.width * x, size.height * y);
            self.driver.pointer(popup, RawPointerEventType::Move, point, RawInputModifiers::NONE);
            self.frame();
            if press {
                self.driver.pointer(popup, RawPointerEventType::LeftButtonDown, point, RawInputModifiers::LEFT_MOUSE_BUTTON);
                self.driver.pointer(popup, RawPointerEventType::LeftButtonUp, point, RawInputModifiers::NONE);
                self.frame();
            }
        } else if let Some(host) = self.overlay_hosts().last() {
            let size = host.bounds().size();
            let root: &Visual = self.tour.window();
            let Some(point) = host.translate_point(Point::new(size.width * x, size.height * y), root) else { return };
            self.move_to(point);
            if press {
                self.click(point);
            }
        }
    }

    /// Shows `control` in the window, runs `interact` on it, takes it out of
    /// the tree, drops it and returns the elements of it that are alive:
    /// of its visual tree when it was shown, of the trees `interact` added to
    /// the watch, of its tree and of the open popups when `interact` was
    /// over.
    fn run(&self, control: Ref<Control>, interact: impl FnOnce(&Bench, &Ref<Control>, &mut Watch)) -> Vec<String> {
        let mut watch = Watch::default();
        self.tour.set_content(Some(&control));
        self.tour.settle();
        watch.add(&control.clone().upcast());
        interact(self, &control, &mut watch);
        watch.add(&control.clone().upcast());
        self.watch_popups(&mut watch);
        self.tour.set_content(None);
        drop(control);
        self.tour.settle();
        // The timers the input started and that have not come due in the
        // time of the test fire: a timer that runs once holds what its
        // callback holds until then, here as upstream (the timer of the hold
        // gesture has the element that was pressed, for as long as a hold
        // takes to begin). A timer that repeats and holds an element is
        // still there afterwards, and the element with it.
        self.driver.fire_timers();
        self.tour.settle();
        // The mock of the window records the calls made on it.
        self.window().clear_calls();
        watch.alive()
    }
}

fn load(xaml: &str) -> Ref<Control> {
    from_markup_value::<Ref<Control>>(&Some(load_text(xaml))).expect("a control")
}

/// The first element of class `T` under `root`, itself included.
fn first<T: ferroui_base::ObjectType>(root: &Ref<Control>) -> Ref<T> {
    let root: &Visual = root;
    root.find_descendant_of_type::<T>(true).unwrap_or_else(|| panic!("an element of {}", std::any::type_name::<T>()))
}

/// The elements of class `T` under `root`, in the order of the visual tree.
fn all<T: ferroui_base::ObjectType>(root: &Ref<Control>) -> Vec<Ref<T>> {
    let mut found = Vec::new();
    let mut pending: Vec<Ref<Visual>> = vec![root.clone().upcast()];
    while let Some(visual) = pending.pop() {
        for child in visual.get_visual_children().iter().rev() {
            pending.push(child.clone());
        }
        if let Some(element) = visual.cast::<T>() {
            found.push(element);
        }
    }
    found
}

fn items(count: usize) -> ItemsSource {
    ItemsSource::from_values((0..count).map(|index| format!("Item {index}")))
}

type Scenario = fn(&Bench) -> Vec<String>;

/// A drop-down that is opened by a press, has an item selected by a press,
/// is opened again and closed with Escape.
fn combo_box(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<ComboBox {XMLNS} SelectedIndex='0' Width='200' HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <ComboBoxItem>one</ComboBoxItem><ComboBoxItem>two</ComboBoxItem><ComboBoxItem>three</ComboBoxItem>\
           <ComboBoxItem>four</ComboBoxItem><ComboBoxItem>five</ComboBoxItem>\
         </ComboBox>"
    ));
    bench.run(control, |bench, control, watch| {
        let combo_box = first::<ComboBox>(control);
        bench.click(bench.centre(control));
        assert!(combo_box.is_drop_down_open(), "the press opens the drop-down");
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.3, false);
        bench.in_popup(0.5, 0.7, true);
        assert!(!combo_box.is_drop_down_open(), "the press on an item closes the drop-down");
        assert_ne!(0, combo_box.selected_index(), "the press on an item selects it");
        bench.click(bench.centre(control));
        assert!(combo_box.is_drop_down_open());
        bench.watch_popups(watch);
        bench.escape();
        assert!(!combo_box.is_drop_down_open(), "Escape closes the drop-down");
    })
}

/// A drop-down that is open when its control leaves the tree.
fn combo_box_removed_while_open(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<ComboBox {XMLNS} SelectedIndex='0' Width='200' HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <ComboBoxItem>one</ComboBoxItem><ComboBoxItem>two</ComboBoxItem><ComboBoxItem>three</ComboBoxItem>\
         </ComboBox>"
    ));
    let alive = bench.run(control, |bench, control, watch| {
        bench.click(bench.centre(control));
        assert!(first::<ComboBox>(control).is_drop_down_open());
        bench.in_popup(0.5, 0.5, false);
        bench.watch_popups(watch);
    });
    assert!(!bench.is_popup_open(), "the drop-down closed with its control");
    alive
}

/// A combo box with 2 000 items from an items source: opened, scrolled with
/// the wheel, an item selected, opened again and closed by a press outside.
fn combo_box_with_many_items(bench: &Bench) -> Vec<String> {
    let control = load(&format!("<ComboBox {XMLNS} Width='200' HorizontalAlignment='Left' VerticalAlignment='Top'/>"));
    first::<ComboBox>(&control).set_items_source(Some(items(2000)));
    bench.run(control, |bench, control, watch| {
        let combo_box = first::<ComboBox>(control);
        bench.click(bench.centre(control));
        assert!(combo_box.is_drop_down_open());
        for _ in 0..40 {
            if let Some(popup) = bench.driver.open_popups().last() {
                let size = popup.client_size.get();
                bench.driver.wheel(popup, Point::new(size.width / 2.0, size.height / 2.0), Vector::new(0.0, -3.0));
            } else if let Some(host) = bench.overlay_hosts().last() {
                let host: Ref<Control> = host.clone().cast::<Control>().expect("a control");
                bench.driver.wheel(bench.window(), bench.centre(&host), Vector::new(0.0, -3.0));
            }
            bench.frame();
        }
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.5, true);
        assert!(!combo_box.is_drop_down_open());
        bench.click(bench.centre(control));
        bench.watch_popups(watch);
        bench.click(Point::new(1200.0, 700.0));
        bench.escape();
        assert!(!combo_box.is_drop_down_open());
    })
}

/// The drop-down calendar of a date picker: opened by its button, a day
/// pressed, opened again and closed with Escape.
fn calendar_date_picker(bench: &Bench) -> Vec<String> {
    let control =
        load(&format!("<CalendarDatePicker {XMLNS} Width='250' HorizontalAlignment='Left' VerticalAlignment='Top'/>"));
    bench.run(control, |bench, control, watch| {
        let button = first::<Button>(control);
        bench.click(bench.centre(&button));
        assert!(bench.is_popup_open(), "the button opens the calendar");
        bench.watch_popups(watch);
        bench.in_popup(0.3, 0.4, false);
        bench.in_popup(0.5, 0.6, true);
        bench.frames(2);
        if !bench.is_popup_open() {
            bench.click(bench.centre(&button));
        }
        assert!(bench.is_popup_open());
        bench.watch_popups(watch);
        bench.close();
        assert!(!bench.is_popup_open());
    })
}

/// The flyouts of the date and of the time picker: opened, the pointer over
/// the spinners, the wheel, closed with Escape.
fn date_and_time_picker(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top' Spacing='8'><DatePicker/><TimePicker/></StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        for picker in all::<Button>(control).into_iter().take(2) {
            bench.click(bench.centre(&picker));
            assert!(bench.is_popup_open(), "the picker opens its flyout");
            bench.watch_popups(watch);
            bench.in_popup(0.3, 0.3, false);
            bench.in_popup(0.5, 0.5, false);
            if let Some(popup) = bench.driver.open_popups().last() {
                let size = popup.client_size.get();
                for _ in 0..5 {
                    bench.driver.wheel(popup, Point::new(size.width * 0.2, size.height * 0.5), Vector::new(0.0, -1.0));
                    bench.frame();
                }
            }
            bench.watch_popups(watch);
            bench.close();
            assert!(!bench.is_popup_open());
        }
    })
}

/// A button with a flyout and one with a menu flyout: opened, an item of the
/// menu pressed, closed with Escape and by a press outside.
fn flyouts(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top' Spacing='8'>\
           <Button Content='Flyout'><Button.Flyout><Flyout><StackPanel><TextBlock Text='content'/><Button Content='inner'/></StackPanel></Flyout></Button.Flyout></Button>\
           <Button Content='Menu'><Button.Flyout><MenuFlyout><MenuItem Header='one'/><MenuItem Header='two'/><MenuItem Header='three'/></MenuFlyout></Button.Flyout></Button>\
           <SplitButton Content='Split'><SplitButton.Flyout><Flyout><TextBlock Text='split'/></Flyout></SplitButton.Flyout></SplitButton>\
           <DropDownButton Content='Drop'><DropDownButton.Flyout><Flyout><TextBlock Text='drop'/></Flyout></DropDownButton.Flyout></DropDownButton>\
         </StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        let buttons = all::<Button>(control);
        // The flyout: Escape.
        bench.click(bench.centre(&buttons[0]));
        assert!(bench.is_popup_open(), "the button opens its flyout");
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.7, true);
        bench.close();
        assert!(!bench.is_popup_open());
        // The flyout again: a press outside.
        bench.click(bench.centre(&buttons[0]));
        bench.watch_popups(watch);
        bench.click(Point::new(1200.0, 700.0));
        bench.close();
        assert!(!bench.is_popup_open());
        // The menu flyout: an item.
        bench.click(bench.centre(&buttons[1]));
        assert!(bench.is_popup_open(), "the button opens its menu");
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.2, false);
        bench.in_popup(0.5, 0.5, true);
        bench.close();
        assert!(!bench.is_popup_open());
        // The drop-down button.
        let drop_down = buttons.last().expect("the drop-down button");
        bench.click(bench.centre(drop_down));
        bench.watch_popups(watch);
        bench.close();
        assert!(!bench.is_popup_open());
    })
}

/// A menu: a top-level item opened by a press, the pointer over an item
/// with a sub menu until it opens, an item pressed; opened again and closed
/// with Escape.
fn menu(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<Menu {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <MenuItem Header='File'>\
             <MenuItem Header='Open'/>\
             <MenuItem Header='Recent'><MenuItem Header='one'/><MenuItem Header='two'/></MenuItem>\
             <MenuItem Header='Close'/>\
           </MenuItem>\
           <MenuItem Header='Edit'><MenuItem Header='Copy'/><MenuItem Header='Paste'/></MenuItem>\
         </Menu>"
    ));
    bench.run(control, |bench, control, watch| {
        let file = first::<MenuItem>(control);
        bench.click(bench.centre(&file));
        assert!(file.is_sub_menu_open(), "the press opens the menu");
        bench.watch_popups(watch);
        // The item with the sub menu, which opens after its delay.
        bench.in_popup(0.5, 0.5, false);
        bench.timers();
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.3, true);
        bench.frames(2);
        bench.close();
        assert!(!bench.is_popup_open());
        // Again, and to the menu beside it by the pointer.
        bench.click(bench.centre(&file));
        bench.watch_popups(watch);
        let edit = all::<MenuItem>(control).into_iter().nth(1).expect("the second menu");
        bench.move_to(bench.centre(&edit));
        bench.timers();
        bench.watch_popups(watch);
        bench.close();
        assert!(!bench.is_popup_open());
    })
}

/// A context menu and a context flyout: opened by the right button, an item
/// pressed; opened again and closed with Escape.
fn context_menu_and_flyout(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top' Spacing='8'>\
           <Border Background='Red' Width='200' Height='80'>\
             <Border.ContextMenu><ContextMenu><MenuItem Header='one'/><MenuItem Header='two'/><MenuItem Header='three'/></ContextMenu></Border.ContextMenu>\
           </Border>\
           <Border Background='Green' Width='200' Height='80'>\
             <Border.ContextFlyout><MenuFlyout><MenuItem Header='one'/><MenuItem Header='two'/></MenuFlyout></Border.ContextFlyout>\
           </Border>\
           <TextBox Width='200' Text='text'/>\
         </StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        let panel: &Visual = control;
        let targets: Vec<Ref<Control>> =
            panel.get_visual_children().iter().filter_map(|child| child.cast::<Control>()).collect();
        assert_eq!(3, targets.len());
        for target in &targets {
            // The flyout of the text box is the one of the theme, shared by
            // every text box: what it shows is not the text box's.
            let own = !target.is::<TextBox>();
            bench.right_click(bench.centre(target));
            assert!(bench.is_popup_open(), "the right button opens the menu of {}", target.get_type().name());
            if own {
                bench.watch_popups(watch);
            }
            bench.in_popup(0.5, 0.3, false);
            bench.in_popup(0.5, 0.5, true);
            bench.close();
            assert!(!bench.is_popup_open());
            bench.right_click(bench.centre(target));
            if own {
                bench.watch_popups(watch);
            }
            bench.close();
            assert!(!bench.is_popup_open());
        }
    })
}

/// A tooltip that opens under the resting pointer and closes when the
/// pointer leaves.
fn tool_tip(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<Border {XMLNS} Background='Red' Width='200' Height='80' HorizontalAlignment='Left' VerticalAlignment='Top' ToolTip.Tip='A tip'/>"
    ));
    bench.run(control, |bench, control, watch| {
        bench.move_to(bench.centre(control));
        bench.timers();
        assert!(ToolTip::get_is_open(control), "the tooltip opens");
        bench.watch_popups(watch);
        bench.move_to(Point::new(1200.0, 700.0));
        bench.timers();
        assert!(!ToolTip::get_is_open(control), "the tooltip closes");
        bench.move_to(bench.centre(control));
        bench.timers();
        bench.watch_popups(watch);
        bench.move_to(Point::new(1200.0, 700.0));
        bench.timers();
    })
}

/// A tooltip that is open when its control leaves the tree.
fn tool_tip_removed_while_open(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<Border {XMLNS} Background='Red' Width='200' Height='80' HorizontalAlignment='Left' VerticalAlignment='Top' ToolTip.Tip='A tip'/>"
    ));
    let alive = bench.run(control, |bench, control, watch| {
        bench.move_to(bench.centre(control));
        bench.timers();
        assert!(ToolTip::get_is_open(control), "the tooltip opens");
        bench.watch_popups(watch);
    });
    bench.timers();
    assert!(!bench.is_popup_open(), "the tooltip closed with its control");
    alive
}

/// A virtualising list of 10 000 items: scrolled from end to end by the
/// offset, by the wheel and by the keys, with items selected by presses, a
/// range among them. The containers it holds stay as few as the viewport
/// needs.
fn list_box_with_10_000_items(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<ListBox {XMLNS} Width='300' Height='400' HorizontalAlignment='Left' VerticalAlignment='Top' SelectionMode='Multiple'/>"
    ));
    first::<ListBox>(&control).set_items_source(Some(items(10_000)));
    bench.run(control, |bench, control, watch| {
        let list_box = first::<ListBox>(control);
        let scroll_viewer = first::<ScrollViewer>(control);
        let panel = first::<VirtualizingStackPanel>(control);
        let centre = bench.centre(control);
        let realized = |moment: &str| {
            let containers = all::<ListBoxItem>(control).len();
            let children = panel.children().count();
            assert!(containers <= 40 && children <= 40, "{moment}: {containers} containers, {children} children of the panel");
        };
        realized("shown");
        bench.click(Point::new(centre.x, centre.y - 150.0));
        assert_eq!(1, list_box.selection().count(), "a press selects an item");

        // From the first item to the last by the offset, a viewport at a time.
        let viewport = scroll_viewer.viewport().height;
        let mut step = 0;
        loop {
            let offset = scroll_viewer.offset();
            let end = scroll_viewer.extent().height - viewport;
            if offset.y >= end - 0.5 {
                break;
            }
            scroll_viewer.set_offset(Vector::new(offset.x, (offset.y + viewport * 0.9).min(end)));
            bench.frame();
            step += 1;
            if step % 100 == 0 {
                realized("scrolling down");
                watch.add(&control.clone().upcast());
                bench.click(centre);
            }
            assert!(step < 5000, "the list scrolls to its end");
        }
        assert!(step > 500, "the list has 10 000 items to scroll over: {step} steps");
        realized("at the end");
        watch.add(&control.clone().upcast());
        bench.click(centre);
        // A range: Shift and a press further up.
        bench.click_with(Point::new(centre.x, centre.y - 150.0), RawInputModifiers::SHIFT);
        assert!(list_box.selection().count() > 2, "the range is selected");

        // Back by the wheel and the keys.
        for _ in 0..60 {
            bench.wheel(centre, 3.0);
        }
        realized("after the wheel");
        watch.add(&control.clone().upcast());
        bench.key_with(Key::Home, PhysicalKey::Home, RawInputModifiers::CONTROL);
        bench.key(Key::Home, PhysicalKey::Home);
        bench.frames(2);
        for _ in 0..20 {
            bench.key(Key::PageDown, PhysicalKey::PageDown);
        }
        bench.key(Key::End, PhysicalKey::End);
        bench.frames(2);
        realized("after the keys");
        scroll_viewer.set_offset(Vector::new(0.0, 0.0));
        bench.frames(2);
        assert_eq!(0.0, scroll_viewer.offset().y);
        realized("back at the start");
    })
}

/// An items control with a virtualising panel in a scroll viewer, without
/// selection: scrolled from end to end.
fn items_control_with_10_000_items(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<ScrollViewer {XMLNS} Width='300' Height='400' HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <ItemsControl>\
             <ItemsControl.ItemsPanel><ItemsPanelTemplate><VirtualizingStackPanel/></ItemsPanelTemplate></ItemsControl.ItemsPanel>\
             <ItemsControl.ItemTemplate><DataTemplate><Border Padding='4'><TextBlock Text='{{Binding}}'/></Border></DataTemplate></ItemsControl.ItemTemplate>\
           </ItemsControl>\
         </ScrollViewer>"
    ));
    bench.run(control, |bench, control, watch| {
        first::<ItemsControl>(control).set_items_source(Some(items(10_000)));
        bench.frames(2);
        let scroll_viewer = first::<ScrollViewer>(control);
        let panel = first::<VirtualizingStackPanel>(control);
        let viewport = scroll_viewer.viewport().height;
        let mut step = 0;
        loop {
            let offset = scroll_viewer.offset();
            let end = scroll_viewer.extent().height - viewport;
            if offset.y >= end - 0.5 {
                break;
            }
            scroll_viewer.set_offset(Vector::new(offset.x, (offset.y + viewport * 0.9).min(end)));
            bench.frame();
            step += 1;
            if step % 100 == 0 {
                assert!(panel.children().count() <= 40, "{} children of the panel", panel.children().count());
                watch.add(&control.clone().upcast());
            }
            assert!(step < 5000, "the list scrolls to its end");
        }
        assert!(step > 300, "{step} steps");
        scroll_viewer.set_offset(Vector::new(0.0, 0.0));
        bench.frames(2);
        assert!(panel.children().count() <= 40);
    })
}

/// A text box that is typed in, with an undo history, undo and redo, a
/// selection, the caret timer, the context flyout; it has the focus when it
/// leaves the tree.
fn text_box_with_undo_history(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<TextBox {XMLNS} Width='300' Height='120' AcceptsReturn='True' TextWrapping='Wrap' HorizontalAlignment='Left' VerticalAlignment='Top'/>"
    ));
    bench.run(control, |bench, control, watch| {
        let text_box = first::<TextBox>(control);
        bench.click(bench.centre(control));
        assert!(text_box.is_focused(), "the press focuses the text box");
        for index in 0..300 {
            bench.text(if index % 7 == 6 { " " } else { "a" });
            if index % 10 == 9 {
                // The timers of the dispatcher: the caret, and the one that
                // ends an entry of the undo history.
                bench.timers();
            }
            if index % 60 == 59 {
                bench.key(Key::Enter, PhysicalKey::Enter);
            }
        }
        assert!(text_box.text().is_some_and(|text| text.len() >= 300), "the text is typed");
        assert!(text_box.can_undo(), "the text box has an undo history");
        for _ in 0..10 {
            text_box.undo();
            bench.frame();
        }
        text_box.redo();
        bench.frame();
        text_box.select_all();
        bench.frame();
        bench.key(Key::Back, PhysicalKey::Backspace);
        bench.text("b");
        bench.timers();
        // A selection by a drag, and the context flyout.
        let centre = bench.centre(control);
        bench.press(Point::new(centre.x - 100.0, centre.y - 40.0));
        bench.drag_to(Point::new(centre.x + 50.0, centre.y - 40.0));
        bench.release(Point::new(centre.x + 50.0, centre.y - 40.0));
        // The flyout is the one of the theme, which every text box shows
        // (`the_context_flyout_of_text_boxes_is_shared`): its presenter is
        // not the text box's and is not watched.
        bench.right_click(centre);
        assert!(bench.is_popup_open(), "the right button opens the context flyout");
        bench.in_popup(0.5, 0.5, false);
        bench.close();
        assert!(!bench.is_popup_open());
        let _ = watch;
        bench.click(centre);
        assert!(text_box.is_focused(), "the text box has the focus when it leaves the tree");
    })
}

/// A slider that is dragged, and that has the pointer captured (the button
/// is down) when it leaves the tree.
fn slider_removed_while_dragged(bench: &Bench) -> Vec<String> {
    let control =
        load(&format!("<Slider {XMLNS} Width='300' Maximum='100' Value='50' HorizontalAlignment='Left' VerticalAlignment='Top'/>"));
    let centre = std::cell::Cell::new(Point::default());
    let alive = bench.run(control, |bench, control, _| {
        let slider = first::<Slider>(control);
        let middle = bench.centre(control);
        centre.set(middle);
        bench.press(middle);
        bench.drag_to(Point::new(middle.x + 60.0, middle.y));
        bench.release(Point::new(middle.x + 60.0, middle.y));
        assert!(slider.value() > 50.0, "the drag moves the slider: {}", slider.value());
        bench.press(Point::new(middle.x + 60.0, middle.y));
        bench.drag_to(Point::new(middle.x - 40.0, middle.y));
    });
    bench.release(Point::new(centre.get().x - 40.0, centre.get().y));
    alive
}

/// A box that completes what is typed: the drop-down opens while typing and
/// is open when the control leaves the tree.
fn auto_complete_box(bench: &Bench) -> Vec<String> {
    let control = load(&format!("<AutoCompleteBox {XMLNS} Width='250' HorizontalAlignment='Left' VerticalAlignment='Top'/>"));
    first::<AutoCompleteBox>(&control).set_items_source(Some(ItemsSource::from_strs([
        "Alabama", "Alaska", "Arizona", "Arkansas", "California", "Colorado", "Connecticut", "Delaware",
    ])));
    bench.run(control, |bench, control, watch| {
        let auto_complete_box = first::<AutoCompleteBox>(control);
        bench.click(bench.centre(control));
        bench.text("a");
        bench.timers();
        bench.text("l");
        bench.timers();
        assert!(auto_complete_box.is_drop_down_open(), "typing opens the drop-down");
        bench.watch_popups(watch);
        bench.in_popup(0.5, 0.3, true);
        bench.frames(2);
        // The press on the item of the drop-down moved the focus to the item;
        // the box, which lost it, closed the drop-down while the focus was
        // moving, and the item left the tree as the focused element. The
        // keyboard device cleared the focus and, back in the change that was
        // under way, told the input method manager of the item (upstream's
        // KeyboardDevice.SetFocusedElement ends the same way, with
        // _textInputManager.SetFocusedElement(element) for the element of
        // the outer call). The manager has the item, here as upstream, until
        // the focus changes again: the press that follows focuses the box.
        assert!(!auto_complete_box.is_drop_down_open());
        bench.click(bench.centre(control));
        // The drop-down is open when the box leaves the tree.
        bench.key(Key::Back, PhysicalKey::Backspace);
        bench.text("c");
        bench.timers();
        bench.watch_popups(watch);
    })
}

/// Buttons the focus moves over with Tab; one has the focus, and the
/// pointer over it, when the panel leaves the tree, and Tab is pressed after.
fn focused_button(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top' Spacing='8'>\
           <Button Content='_One'/><Button Content='_Two'/><CheckBox Content='T_hree'/><RadioButton Content='Four'/><ToggleSwitch/>\
         </StackPanel>"
    ));
    let alive = bench.run(control, |bench, control, _| {
        let buttons = all::<Button>(control);
        bench.click(bench.centre(&buttons[0]));
        for _ in 0..7 {
            bench.key(Key::Tab, PhysicalKey::Tab);
        }
        bench.key_with(Key::Tab, PhysicalKey::Tab, RawInputModifiers::SHIFT);
        bench.key(Key::Space, PhysicalKey::Space);
        // An access key: Alt and the letter.
        bench.key_with(Key::LeftAlt, PhysicalKey::AltLeft, RawInputModifiers::ALT);
        bench.key_with(Key::T, PhysicalKey::T, RawInputModifiers::ALT);
        bench.click(bench.centre(&buttons[1]));
        assert!(buttons[1].is_focused(), "the press focuses the button");
        assert!(buttons[1].is_pointer_over());
    });
    bench.key(Key::Tab, PhysicalKey::Tab);
    alive
}

/// A repeat button that is held down (its timer repeats the click) when it
/// leaves the tree, in a spinner.
fn repeat_button_removed_while_pressed(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<NumericUpDown {XMLNS} Width='200' Value='5' Minimum='0' Maximum='1000' HorizontalAlignment='Left' VerticalAlignment='Top'/>"
    ));
    let point = std::cell::Cell::new(Point::default());
    let alive = bench.run(control, |bench, control, _| {
        let button = first::<RepeatButton>(control);
        let centre = bench.centre(&button);
        point.set(centre);
        bench.click(centre);
        bench.press(centre);
        bench.timers();
        bench.timers();
    });
    bench.timers();
    bench.release(point.get());
    alive
}

/// A content control with a transition, whose content changes; the
/// transition runs when the control leaves the tree.
fn transition_removed_while_running(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <TransitioningContentControl Content='one' Width='200' Height='60'/>\
           <Carousel Width='200' Height='60'>\
             <Carousel.PageTransition><PageSlide Duration='0:0:1' Orientation='Horizontal'/></Carousel.PageTransition>\
             <TextBlock Text='a'/><TextBlock Text='b'/><TextBlock Text='c'/>\
           </Carousel>\
           <Expander Header='header' Width='200'><TextBlock Text='content'/></Expander>\
         </StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        let panel: &Visual = control;
        let content = panel.get_visual_children()[0].cast::<ContentControl>().expect("the transitioning content control");
        let carousel = first::<Carousel>(control);
        let text = |text: &str| -> Option<BoxedValue> { Some(Rc::new(text.to_string())) };
        // Transitions that run to their end.
        content.set_content(text("two"));
        carousel.next();
        bench.click(bench.centre(&first::<ToggleButton>(control)));
        bench.frames(16);
        watch.add(&control.clone().upcast());
        // And ones that are running when the controls leave.
        content.set_content(text("three"));
        carousel.next();
        bench.click(bench.centre(&first::<ToggleButton>(control)));
        bench.frame();
    })
}

/// A tab control whose tabs are selected by presses and by the keys, and a
/// tree view whose items are expanded, selected and collapsed.
fn tab_control_and_tree_view(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top'>\
           <TabControl Width='400' Height='150'>\
             <TabItem Header='One'><Button Content='first'/></TabItem>\
             <TabItem Header='Two'><TextBox Text='second'/></TabItem>\
             <TabItem Header='Three'><ComboBox SelectedIndex='0'><ComboBoxItem>third</ComboBoxItem></ComboBox></TabItem>\
           </TabControl>\
           <TreeView Width='400' Height='200'>\
             <TreeViewItem Header='root' IsExpanded='True'>\
               <TreeViewItem Header='a'><TreeViewItem Header='a1'/><TreeViewItem Header='a2'/></TreeViewItem>\
               <TreeViewItem Header='b'><TreeViewItem Header='b1'/></TreeViewItem>\
             </TreeViewItem>\
           </TreeView>\
         </StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        let tabs = all::<TabItem>(control);
        assert_eq!(3, tabs.len());
        for tab in tabs.iter().rev() {
            bench.click(bench.centre(tab));
            bench.frames(2);
            watch.add(&control.clone().upcast());
        }
        bench.key(Key::Right, PhysicalKey::ArrowRight);
        bench.key(Key::Right, PhysicalKey::ArrowRight);
        watch.add(&control.clone().upcast());

        let root = first::<TreeViewItem>(control);
        let header = bench.centre(&root);
        bench.click(Point::new(header.x, header.y - root.bounds().size().height / 2.0 + 12.0));
        for _ in 0..3 {
            bench.key(Key::Down, PhysicalKey::ArrowDown);
            bench.key(Key::Right, PhysicalKey::ArrowRight);
        }
        watch.add(&control.clone().upcast());
        for _ in 0..6 {
            bench.key(Key::Left, PhysicalKey::ArrowLeft);
        }
        bench.key(Key::Right, PhysicalKey::ArrowRight);
        bench.frames(2);
    })
}

/// A scroll viewer that is scrolled by the wheel, by a drag of the thumb of
/// its scroll bar and by a press on the track.
fn scroll_viewer(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<ScrollViewer {XMLNS} Width='300' Height='200' HorizontalAlignment='Left' VerticalAlignment='Top' \
                       HorizontalScrollBarVisibility='Auto' AllowAutoHide='False'>\
           <Border Width='900' Height='3000' Background='Red'><TextBlock Text='content'/></Border>\
         </ScrollViewer>"
    ));
    bench.run(control, |bench, control, _| {
        use ferroui_controls::primitives::Thumb;
        let scroll_viewer = first::<ScrollViewer>(control);
        let centre = bench.centre(control);
        bench.move_to(centre);
        for _ in 0..10 {
            bench.wheel(centre, -1.0);
        }
        assert!(scroll_viewer.offset().y > 0.0, "the wheel scrolls");
        for thumb in all::<Thumb>(control) {
            if !thumb.is_effectively_visible() {
                continue;
            }
            let from = bench.centre(&thumb);
            bench.press(from);
            bench.drag_to(Point::new(from.x + 20.0, from.y + 20.0));
            bench.drag_to(Point::new(from.x + 40.0, from.y + 40.0));
            bench.release(Point::new(from.x + 40.0, from.y + 40.0));
        }
        // The track of the vertical bar, below its thumb: the repeat button
        // of the track pages while the button is down.
        bench.press(Point::new(centre.x + 144.0, centre.y + 80.0));
        bench.timers();
        bench.release(Point::new(centre.x + 144.0, centre.y + 80.0));
        bench.move_to(centre);
    })
}

/// The context flyout of the text boxes of the Fluent theme is one flyout, a
/// resource of the theme: the presenter it shows is created once and shown
/// by every text box, so what it holds is bounded and is not what a text box
/// leaves behind. (Upstream's theme has the same resource, and its flyout
/// keeps its presenter.)
#[test]
fn the_context_flyout_of_text_boxes_is_shared() {
    let bench = Bench::start(false);
    let presenter_of = |bench: &Bench| -> WeakRef<Visual> {
        let control = load(&format!("<TextBox {XMLNS} Width='200' Text='text' HorizontalAlignment='Left' VerticalAlignment='Top'/>"));
        let presenter = std::cell::RefCell::new(None);
        let alive = bench.run(control, |bench, control, _| {
            bench.right_click(bench.centre(control));
            assert!(bench.is_popup_open());
            let popup = bench.driver.open_popups().last().cloned().expect("the popup of the flyout");
            let root: Ref<Visual> = popup.input_root().and_then(|root| root.try_root_element()).expect("the root of the popup").upcast();
            let shown = root.get_visual_descendants().find(|visual| visual.get_type().name() == "MenuFlyoutPresenter");
            *presenter.borrow_mut() = Some(shown.expect("the presenter of the flyout").downgrade());
            bench.close();
        });
        assert!(alive.is_empty(), "the text box is freed: {alive:?}");
        presenter.into_inner().expect("a presenter")
    };
    let first = presenter_of(&bench);
    let second = presenter_of(&bench);
    assert!(first.upgrade().is_some(), "the flyout of the theme keeps its presenter");
    assert!(first.ptr_eq(&second), "every text box shows the presenter of the one flyout");
}

/// A scroll viewer that is scrolled by a finger, let go while it moves (the
/// scroll goes on by its inertia) and taken out of the tree while it does;
/// a button that is tapped, and an element with a context menu that is held
/// until the menu opens.
fn touch(bench: &Bench) -> Vec<String> {
    let control = load(&format!(
        "<StackPanel {XMLNS} HorizontalAlignment='Left' VerticalAlignment='Top' Spacing='8'>\
           <Button Content='tap'/>\
           <Border Background='Red' Width='200' Height='60'>\
             <Border.ContextMenu><ContextMenu><MenuItem Header='one'/><MenuItem Header='two'/></ContextMenu></Border.ContextMenu>\
           </Border>\
           <ScrollViewer Width='300' Height='300'><Border Height='6000' Background='Green'><TextBlock Text='content'/></Border></ScrollViewer>\
         </StackPanel>"
    ));
    bench.run(control, |bench, control, watch| {
        let window = bench.window();
        let finger = |kind: RawPointerEventType, point: Point, milliseconds: u64| {
            bench.driver.touch(window, kind, point, 1, milliseconds);
            bench.frame();
        };
        // A tap.
        let button = first::<Button>(control);
        let clicks = Rc::new(std::cell::Cell::new(0));
        let counted = clicks.clone();
        let token = button.click(move |_, _| counted.set(counted.get() + 1));
        let point = bench.centre(&button);
        finger(RawPointerEventType::TouchBegin, point, 1000);
        finger(RawPointerEventType::TouchEnd, point, 50);
        assert_eq!(1, clicks.get(), "the tap clicks the button");
        drop(token);

        // A hold: the timer of the gesture fires, and the context menu opens.
        let panel: &Visual = control;
        let held = panel.get_visual_children()[1].cast::<Control>().expect("the border");
        let point = bench.centre(&held);
        finger(RawPointerEventType::TouchBegin, point, 1000);
        bench.timers();
        bench.watch_popups(watch);
        finger(RawPointerEventType::TouchEnd, point, 1000);
        bench.frames(2);
        bench.watch_popups(watch);
        bench.close();
        assert!(!bench.is_popup_open());

        // A pan, let go while it moves.
        let scroll_viewer = first::<ScrollViewer>(control);
        let centre = bench.centre(&scroll_viewer);
        finger(RawPointerEventType::TouchBegin, Point::new(centre.x, centre.y + 100.0), 1000);
        for step in 1..=12 {
            finger(RawPointerEventType::TouchUpdate, Point::new(centre.x, centre.y + 100.0 - 15.0 * f64::from(step)), 16);
        }
        assert!(scroll_viewer.offset().y > 0.0, "the finger scrolls");
        finger(RawPointerEventType::TouchEnd, Point::new(centre.x, centre.y - 80.0), 16);
        bench.frames(2);
        // And once more, taken out of the tree with the finger down.
        finger(RawPointerEventType::TouchBegin, Point::new(centre.x, centre.y + 100.0), 1000);
        for step in 1..=6 {
            finger(RawPointerEventType::TouchUpdate, Point::new(centre.x, centre.y + 100.0 - 15.0 * f64::from(step)), 16);
        }
    })
}

const SCENARIOS: &[(&str, Scenario)] = &[
    ("combo_box", combo_box),
    ("combo_box_removed_while_open", combo_box_removed_while_open),
    ("combo_box_with_many_items", combo_box_with_many_items),
    ("calendar_date_picker", calendar_date_picker),
    ("date_and_time_picker", date_and_time_picker),
    ("flyouts", flyouts),
    ("menu", menu),
    ("context_menu_and_flyout", context_menu_and_flyout),
    ("tool_tip", tool_tip),
    ("tool_tip_removed_while_open", tool_tip_removed_while_open),
    ("list_box_with_10_000_items", list_box_with_10_000_items),
    ("items_control_with_10_000_items", items_control_with_10_000_items),
    ("text_box_with_undo_history", text_box_with_undo_history),
    ("slider_removed_while_dragged", slider_removed_while_dragged),
    ("auto_complete_box", auto_complete_box),
    ("focused_button", focused_button),
    ("repeat_button_removed_while_pressed", repeat_button_removed_while_pressed),
    ("transition_removed_while_running", transition_removed_while_running),
    ("tab_control_and_tree_view", tab_control_and_tree_view),
    ("scroll_viewer", scroll_viewer),
    ("touch", touch),
];

/// Runs the scenario `name` in a window of its own and asserts that nothing
/// of its control is alive afterwards.
fn freed(name: &str, overlay: bool) {
    let (_, scenario) = SCENARIOS.iter().find(|(scenario, _)| *scenario == name).expect("a scenario of the list");
    let bench = Bench::start(overlay);
    let alive = scenario(&bench);
    let hosting = if overlay { "popups in the overlay layer" } else { "popups of the platform" };
    assert!(alive.is_empty(), "{name} ({hosting}): alive after the control left the tree and was dropped: {alive:?}");
}

macro_rules! interaction_scenarios {
    ($($name:ident,)*) => {
        /// The popups are popups of the platform.
        mod with_popup_windows {
            $(
                #[test]
                fn $name() {
                    super::freed(stringify!($name), false);
                }
            )*
        }

        /// The popups are hosted in the overlay layer of the window.
        mod with_overlay_popups {
            $(
                #[test]
                fn $name() {
                    super::freed(stringify!($name), true);
                }
            )*
        }
    };
}

interaction_scenarios! {
    combo_box,
    combo_box_removed_while_open,
    combo_box_with_many_items,
    calendar_date_picker,
    date_and_time_picker,
    flyouts,
    menu,
    context_menu_and_flyout,
    tool_tip,
    tool_tip_removed_while_open,
    list_box_with_10_000_items,
    items_control_with_10_000_items,
    text_box_with_undo_history,
    slider_removed_while_dragged,
    auto_complete_box,
    focused_button,
    repeat_button_removed_while_pressed,
    transition_removed_while_running,
    tab_control_and_tree_view,
    scroll_viewer,
    touch,
}

/// What every scenario leaves, in both hostings of the popups, in one window
/// each: the elements that are alive, and what a second and a third run of
/// the scenario in the same window add to what is alive in the process.
#[test]
#[ignore = "measurement: run with --features count-allocations --ignored --nocapture --test-threads=1"]
fn interaction_control_survivors() {
    use super::allocation_trace as trace;
    use super::allocations;
    let wanted = tour::environment("INTERACTION_SCENARIO");
    let traced = tour::environment("CATALOG_TOUR_TRACE").is_some();
    if traced {
        trace::start();
    }
    for overlay in [false, true] {
        println!("{}:", if overlay { "popups in the overlay layer" } else { "popups of the platform" });
        println!("  {:<40} {:>10} {:>8} | {:>10} {:>8}", "scenario", "KB", "blocks", "KB", "blocks");
        let bench = Bench::start(overlay);
        for (name, scenario) in SCENARIOS {
            if wanted.as_ref().is_some_and(|wanted| !name.contains(wanted.as_str())) {
                continue;
            }
            let alive = scenario(&bench);
            let first = allocations::live();
            let epoch = if traced { trace::next_epoch() } else { 0 };
            scenario(&bench);
            let second = allocations::live();
            if traced {
                // What the second run left, before a third run replaces
                // what a holder of the last element holds.
                trace::next_epoch();
                tour::print_recording(name, epoch);
            }
            scenario(&bench);
            let third = allocations::live();
            println!(
                "  {:<40} {:>+10.1} {:>8} | {:>+10.1} {:>8}",
                name,
                (second.bytes - first.bytes) as f64 / 1024.0,
                second.allocations - first.allocations,
                (third.bytes - second.bytes) as f64 / 1024.0,
                third.allocations - second.allocations,
            );
            if !alive.is_empty() {
                println!("    alive: {}", alive.join(", "));
            }
        }
    }
}
