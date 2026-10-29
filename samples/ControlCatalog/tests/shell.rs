//! What the shell of the catalog shows: the main view under the application
//! of the catalog ([`App`]: `App.xaml`, the Fluent theme) in a window that
//! renders through the compositor of the test, with the input of a mouse.
//!
//! Not ports: the upstream sample has no tests. The tours and the comparison
//! of the documents visit every page without looking at what the page shows,
//! and a class of the sample that never populated itself from its document
//! (`SectionControl`, the cards of the home page and of the section pages)
//! went unseen by both: the comparison populates an instance the constructor
//! of the class did not run for, and the tours count the pages, not their
//! content. The tests here look at what the user sees: the cards under each
//! section title, the page a card opens, the pane, the search box, the
//! header and the settings page.

use super::frame_benchmark::RasterSurface;
use super::support::*;
use crate::controls::SectionControl;
use crate::models::{CatalogTheme, HomeSection, PageItem};
use crate::pages::{HomePage, SectionPage, SettingsPage};
use crate::view_models::MainWindowViewModel;
use crate::{register_types, App, MainView};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    IInputDevice, IInputRoot, IKeyboardDevice, IKeyboardNavigationHandler, InputElement, KeyboardDevice,
    KeyboardNavigationHandler, MouseDevice, Pointer, PointerType, RawInputModifiers,
};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::testing::ManualRenderLoop;
use ferroui_base::styling::{IThemeVariantHost, ThemeVariant};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroLocator, ObjectType, Point, Rect, Ref, Visual};
use ferroui_controls::presentation_source::IRendererFactory;
use ferroui_controls::testing::{
    CompositorTestServices, MockWindowImpl, MockWindowingPlatform, TestLogSink, UnitTestApplication,
    UnitTestApplicationScope,
};
use ferroui_controls::{
    Application, AutoSuggestBox, Button, ComboBox, Control, Frame, NavigationPage, NavigationView,
    NavigationViewDisplayMode, NavigationViewItem, Page, ScrollViewer, TextBlock, TextBox, Window,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The frames after an input or a navigation: the transition of the
/// navigation page ends within them.
const SETTLE_FRAMES: usize = 8;

/// The time of the global clock between two frames.
const FRAME_TIME_MS: f64 = 100.0;

/// The sizes the defect was seen at: the wide layout (the pane expanded) and
/// the compact one.
const SIZES: [(f64, f64); 2] = [(1400.0, 900.0), (800.0, 600.0)];

/// The main view of the catalog in a shown window of the application of the
/// catalog.
struct Shell {
    // Declared before the compositor and the application scope: closed and
    // dropped before them.
    window: Ref<Window>,
    main_view: Ref<MainView>,
    view_model: Rc<MainWindowViewModel>,
    clock: Rc<TestGlobalClock>,
    time: Cell<f64>,
    window_impl: Rc<MockWindowImpl>,
    _surface: Arc<RasterSurface>,
    mouse: Rc<MouseDevice>,
    keyboard: Rc<KeyboardDevice>,
    timestamp: Cell<u64>,
    /// What the framework logged at the level of a warning or above.
    log: Rc<RefCell<Vec<String>>>,
    _log_sink: Rc<dyn IDisposable>,
    _compositor: Rc<Compositor>,
    render_loop: Arc<ManualRenderLoop>,
    _app: UnitTestApplicationScope,
}

impl Shell {
    fn start(width: f64, height: f64) -> Shell {
        register_types();
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_sink = {
            let log = log.clone();
            TestLogSink::start(move |level, area, _, template, values| {
                if level >= LogEventLevel::Warning {
                    let values: Vec<String> = values.iter().map(|value| value.to_string()).collect();
                    log.borrow_mut().push(format!("[{area}] {template} {values:?}"));
                }
            })
        };

        // The services of `start_catalog_compositor_application`, under the application of the
        // catalog instead of the unit test application: the themes are the ones of `App.xaml`.
        let clock = Rc::new(TestGlobalClock::default());
        let keyboard = KeyboardDevice::new();
        // The keyboard device and the keyboard navigation are the ones an application registers,
        // and the application is the host of the theme variant of its windows, as
        // `Application.RegisterServices` binds it (the unit test application does not).
        let mut services = catalog_services_with_clock(clock.clone())
            .with_input_manager(Rc::new(ferroui_base::input::InputManager::new()))
            .with_keyboard_device({
                let keyboard = keyboard.clone();
                move || Some(keyboard.clone() as Rc<dyn IKeyboardDevice>)
            })
            .with_keyboard_navigation(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>));
        services.theme = None;
        let app = UnitTestApplication::start_with(services, || App::new().upcast());
        FerroLocator::current_mutable()
            .bind::<dyn IThemeVariantHost>()
            .to_constant(Application::current().expect("the application").as_theme_variant_host());
        FerroLocator::current_mutable().bind::<dyn IRendererFactory>().to_func(|| None);
        let render_loop = ManualRenderLoop::new();
        let compositor = CompositorTestServices::create_dummy_compositor(Some(render_loop.clone()));

        let window_impl = MockWindowingPlatform::create_window_mock_with_size(width, height);
        window_impl.setup_compositor(Some(compositor.clone()));
        let surface = RasterSurface::new();
        window_impl.setup_surfaces(vec![surface.clone() as Arc<dyn IPlatformRenderSurface>]);
        // The popups of the window (the suggestions of the search box, the flyout of a section of
        // the compact pane) render through the compositor of the window.
        {
            let weak_parent = Rc::downgrade(&window_impl);
            let compositor = compositor.clone();
            window_impl.setup_create_popup(move |_| {
                let parent: Rc<dyn ferroui_controls::platform::ITopLevelImpl> = weak_parent.upgrade()?;
                let popup = MockWindowImpl::popup(parent);
                popup.setup_compositor(Some(compositor.clone()));
                popup.setup_surfaces(vec![RasterSurface::new() as Arc<dyn IPlatformRenderSurface>]);
                Some(popup as Rc<dyn ferroui_controls::platform::IPopupImpl>)
            });
        }
        let window = Window::with_impl(window_impl.clone());
        window.set_width(width);
        window.set_height(height);
        let view_model = MainWindowViewModel::new();
        let main_view = MainView::new();
        main_view.set_data_context(Some(view_model.clone() as BoxedValue));
        window.set_content(Some(Control::boxed(&main_view)));
        window.show();
        let shell = Shell {
            window,
            main_view,
            view_model,
            clock,
            time: Cell::new(0.0),
            window_impl,
            _surface: surface,
            mouse: MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true)),
            keyboard,
            timestamp: Cell::new(0),
            log,
            _log_sink: log_sink,
            _compositor: compositor,
            render_loop,
            _app: app,
        };
        shell.settle();
        shell
    }

    fn frame(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        let time = self.time.get() + FRAME_TIME_MS;
        self.time.set(time);
        self.clock.pulse(TimeSpan::from_milliseconds(time));
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    fn settle(&self) {
        for _ in 0..SETTLE_FRAMES {
            self.frame();
        }
    }

    /// The navigation page of the entry of the frame: the catalog page is its root.
    fn navigation_page(&self) -> Ref<NavigationPage> {
        self.main_view.current_navigation_page().expect("the frame shows a navigation page")
    }

    fn navigation_view(&self) -> Ref<NavigationView> {
        self.main_view.navigation_view()
    }

    /// The page the navigation page shows, once no navigation is running.
    fn page(&self) -> Ref<Page> {
        let navigation_page = self.navigation_page();
        assert!(!navigation_page.is_navigating(), "the navigation is over");
        navigation_page.current_page().expect("the navigation page shows a page")
    }

    fn show(&self, item: &Rc<PageItem>) -> Ref<Page> {
        self.view_model.navigate_to_item(item);
        self.settle();
        assert!(self.is_current(item), "{} is the current page", item.header());
        self.page()
    }

    fn is_current(&self, item: &Rc<PageItem>) -> bool {
        self.view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, item))
    }

    /// The bounds of a visual in the coordinates of the window.
    fn bounds_of(&self, visual: &Visual) -> Rect {
        let origin = visual
            .translate_point(Point::new(0.0, 0.0), &self.window)
            .expect("the visual is in the tree of the window");
        let size = visual.bounds().size();
        Rect::new(origin.x, origin.y, size.width, size.height)
    }

    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.window_impl.input_root().expect("the input root of the window")
    }

    fn pointer(&self, type_: RawPointerEventType, position: Point, modifiers: RawInputModifiers) {
        self.timestamp.set(self.timestamp.get() + 16);
        let input =
            ferroui_controls::platform::ITopLevelImpl::input(&*self.window_impl).expect("the window handles input");
        input(Rc::new(RawPointerEventArgs::new(
            self.mouse.clone() as Rc<dyn IInputDevice>,
            self.timestamp.get(),
            self.input_root(),
            type_,
            position,
            modifiers,
        )));
    }

    fn pointer_move(&self, position: Point) {
        self.pointer(RawPointerEventType::Move, position, RawInputModifiers::NONE);
        self.frame();
    }

    fn pointer_down(&self, position: Point) {
        self.pointer(RawPointerEventType::LeftButtonDown, position, RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.frame();
    }

    fn pointer_up(&self, position: Point) {
        self.pointer(RawPointerEventType::LeftButtonUp, position, RawInputModifiers::NONE);
        self.frame();
    }

    /// Moves the mouse to the middle of `visual`, presses and releases the left button, and
    /// runs the frames of what the click started.
    fn click(&self, visual: &Visual) {
        let position = self.bounds_of(visual).center();
        self.pointer_move(position);
        self.pointer_down(position);
        self.pointer_up(position);
        self.settle();
    }

    /// Whether the element under `position` is `visual` or one of its descendants.
    fn is_hit(&self, visual: &Visual, position: Point) -> bool {
        self.window.input_hit_test(position).is_some_and(|hit: Ref<InputElement>| {
            hit.get_self_and_visual_ancestors().any(|ancestor| std::ptr::eq(&*ancestor as &Visual, visual))
        })
    }

    /// What the framework logged about a binding.
    fn binding_log(&self) -> Vec<String> {
        self.log.borrow().iter().filter(|entry| entry.starts_with("[Binding]")).cloned().collect()
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

/// The texts of the text blocks below `root` that are shown.
fn texts(root: &Visual) -> Vec<String> {
    descendants::<TextBlock>(root)
        .into_iter()
        .filter(|text_block| text_block.is_effectively_visible())
        .filter_map(|text_block| text_block.text())
        .collect()
}

/// The cards of a section control: the buttons of its items.
fn cards(section_control: &Ref<SectionControl>) -> Vec<Ref<Button>> {
    descendants::<Button>(section_control)
}

fn items(section: &HomeSection) -> Vec<Rc<PageItem>> {
    section.items().map(|items| items.to_vec()).unwrap_or_default()
}

/// The cards of `section_control` are the pages of `section`, in their order: each is laid
/// out with a size, inside the extent of the scroll viewer of the page and clear of the
/// cards before it, shows the header and the description of its page, and has the page as
/// its data context.
fn assert_cards(section_control: &Ref<SectionControl>, section: &Rc<HomeSection>, context: &str) {
    let title = section.title();
    let section_items = items(section);
    let cards = cards(section_control);
    assert_eq!(
        section_items.len(),
        cards.len(),
        "{context}: the section {title} has {} pages and its control shows {} cards (the logical children of the \
         section control: {})",
        section_items.len(),
        cards.len(),
        section_control.logical_children().count(),
    );

    let scroll_viewer =
        section_control.find_ancestor_of_type::<ScrollViewer>(false).expect("the page scrolls its content");
    let extent = scroll_viewer.extent();
    let offset = scroll_viewer.offset();
    let mut seen: Vec<Rect> = Vec::new();
    for (card, item) in cards.iter().zip(&section_items) {
        let header = item.header();
        let bounds = card.bounds();
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{context}: the card {header} has the bounds {bounds:?}");
        assert!(card.is_effectively_visible(), "{context}: the card {header} is visible");

        let origin = card.translate_point(Point::new(0.0, 0.0), &scroll_viewer).expect("the card is in the scroll viewer");
        let in_extent = Rect::new(origin.x + offset.x, origin.y + offset.y, bounds.width, bounds.height);
        assert!(
            Rect::new(0.0, 0.0, extent.width, extent.height).contains_rect(in_extent),
            "{context}: the card {header} at {in_extent:?} is inside the extent {extent:?} of the page",
        );
        assert!(
            seen.iter().all(|other| !other.intersects(in_extent)),
            "{context}: the card {header} at {in_extent:?} does not overlap another card",
        );
        seen.push(in_extent);

        let card_texts = texts(card);
        assert!(card_texts.contains(&header), "{context}: the card of {header} shows {card_texts:?}");
        if let Some(description) = item.description().filter(|description| !description.is_empty()) {
            assert!(card_texts.contains(&description), "{context}: the card of {header} shows {card_texts:?}");
        }
        let data_context = from_markup_value::<Rc<PageItem>>(&card.data_context());
        assert!(
            data_context.is_some_and(|data_context| Rc::ptr_eq(&data_context, item)),
            "{context}: the data context of the card {header} is its page",
        );
        assert!(card.command().is_some(), "{context}: the card {header} has the command that navigates");
    }
}

/// The home page shows, under the title of each section, the cards of the pages of the
/// section.
#[test]
fn the_home_page_shows_the_cards_of_every_section() {
    for (width, height) in SIZES {
        let context = format!("{width}x{height}");
        let shell = Shell::start(width, height);
        let page = shell.page();
        assert!(page.cast::<HomePage>().is_some(), "{context}: the catalog starts on the home page");

        let sections = shell.view_model.home_sections();
        let section_controls = descendants::<SectionControl>(&page);
        assert_eq!(sections.len(), section_controls.len(), "{context}: a section control for each section");
        for (section_control, section) in section_controls.iter().zip(sections.iter()) {
            // The title is the text block before the section control in the template of the section.
            let template_root = section_control.get_visual_parent().expect("the panel of the section");
            let title = descendants::<TextBlock>(&template_root).into_iter().next().and_then(|title| title.text());
            assert_eq!(Some(section.title()), title, "{context}: the title above the cards");
            assert_cards(section_control, section, &context);
        }

        // The first card is in the viewport and takes the input at its middle.
        let first = cards(&section_controls[0]).into_iter().next().expect("the first card");
        let bounds = shell.bounds_of(&first);
        assert!(
            Rect::new(0.0, 0.0, width, height).contains_rect(bounds),
            "{context}: the first card at {bounds:?} is in the window",
        );
        assert!(shell.is_hit(&first, bounds.center()), "{context}: the first card is hit at its middle");
        assert_eq!(Vec::<String>::new(), shell.binding_log(), "{context}: no binding of the home page reports");
    }
}

/// The page of a section shows the cards of the pages of the section.
#[test]
fn a_section_page_shows_the_cards_of_its_section() {
    for (width, height) in SIZES {
        let context = format!("{width}x{height}");
        let shell = Shell::start(width, height);
        for section in shell.view_model.home_sections().iter() {
            let page = shell.show(&section.page_item());
            assert!(page.cast::<SectionPage>().is_some(), "{context}: the page of the section {}", section.title());
            assert_eq!(
                Some(section.title()),
                from_markup_value::<String>(&page.header()),
                "{context}: the header of the page of the section",
            );
            let section_controls = descendants::<SectionControl>(&page);
            assert_eq!(1, section_controls.len(), "{context}: the section control of the page");
            assert_cards(&section_controls[0], section, &format!("{context}, the page of the section"));
        }
        assert_eq!(Vec::<String>::new(), shell.binding_log(), "{context}: no binding of a section page reports");
    }
}

/// Pressing a card of the home page, and one of a section page, opens the page of the card.
#[test]
fn pressing_a_card_opens_its_page() {
    for (width, height) in SIZES {
        let context = format!("{width}x{height}");
        let shell = Shell::start(width, height);
        let sections = shell.view_model.home_sections();
        let section = &sections[0];
        let section_items = items(section);

        // The second card of the first section of the home page.
        let page = shell.page();
        let section_control = descendants::<SectionControl>(&page).into_iter().next().expect("the first section");
        let card = cards(&section_control).into_iter().nth(1).expect("the second card of the home page");
        shell.click(&card);
        assert!(
            shell.is_current(&section_items[1]),
            "{context}: the card {} of the home page opened {:?}",
            section_items[1].header(),
            shell.view_model.current_page_item().map(|item| item.header()),
        );
        let opened = shell.page();
        assert!(opened.cast::<HomePage>().is_none(), "{context}: the navigation page left the home page");
        assert_eq!(1, shell.navigation_page().stack_depth(), "{context}: the page replaced the home page");

        // The first card of the page of the section.
        let page = shell.show(&section.page_item());
        let section_control = descendants::<SectionControl>(&page).into_iter().next().expect("the section control");
        let card = cards(&section_control).into_iter().next().expect("the first card of the section page");
        shell.click(&card);
        assert!(
            shell.is_current(&section_items[0]),
            "{context}: the card {} of the section page opened {:?}",
            section_items[0].header(),
            shell.view_model.current_page_item().map(|item| item.header()),
        );
        assert!(shell.page().cast::<SectionPage>().is_none(), "{context}: the navigation page left the section page");
    }
}

// --- the rest of the shell: what the tours do not look at ---------------------------------------

/// The items of the pane that are shown (the home item, the sections, the pages of an open
/// section and the settings item; not the items of a page the frame shows), each with the text it
/// shows, from the top of the pane to its bottom.
fn pane_entries(shell: &Shell) -> Vec<(String, Ref<NavigationViewItem>)> {
    let mut entries: Vec<(f64, String, Ref<NavigationViewItem>)> = descendants::<NavigationViewItem>(&shell.main_view)
        .into_iter()
        .filter(|item| item.is_effectively_visible() && shell.bounds_of(item).height > 0.0)
        // Shown: hit where its icon is (an item of a closed overlay pane is not).
        .filter(|item| {
            let bounds = shell.bounds_of(item);
            shell.is_hit(item, Point::new(bounds.x + 20.0, bounds.y + (bounds.height / 2.0).min(18.0)))
        })
        .filter(|item| item.find_ancestor_of_type::<Frame>(false).is_none())
        .filter_map(|item| from_markup_value::<String>(&item.content()).map(|text| (shell.bounds_of(&item).y, text, item)))
        .collect();
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    entries.into_iter().map(|(_, text, item)| (text, item)).collect()
}

fn pane_entry(shell: &Shell, text: &str) -> Ref<NavigationViewItem> {
    let entries = pane_entries(shell);
    let names: Vec<String> = entries.iter().map(|(name, _)| name.clone()).collect();
    entries
        .into_iter()
        .find(|(name, _)| name == text)
        .map(|(_, item)| item)
        .unwrap_or_else(|| panic!("the pane has no item {text}: it shows {names:?}"))
}

/// The texts of the items the pane lists, also those scrolled out of its view.
fn listed_texts(shell: &Shell) -> Vec<String> {
    descendants::<NavigationViewItem>(&shell.main_view)
        .into_iter()
        .filter(|item| item.is_effectively_visible() && item.find_ancestor_of_type::<Frame>(false).is_none())
        .filter_map(|item| from_markup_value::<String>(&item.content()))
        .collect()
}

fn pane_texts(shell: &Shell) -> Vec<String> {
    pane_entries(shell).into_iter().map(|(text, _)| text).collect()
}

fn selected_entries(shell: &Shell) -> Vec<String> {
    pane_entries(shell).into_iter().filter(|(_, item)| item.is_selected()).map(|(text, _)| text).collect()
}

/// A named part of the template of the navigation view.
fn pane_part(shell: &Shell, name: &str) -> Ref<Button> {
    descendants::<Button>(&shell.navigation_view())
        .into_iter()
        .find(|button| button.name().as_deref() == Some(name) && button.is_effectively_visible())
        .unwrap_or_else(|| panic!("the part {name} of the navigation view"))
}

/// The back button of the navigation view: it pops a sample a gallery pushed, and else goes back
/// in the journal of the frame.
fn back_button(shell: &Shell) -> Ref<Button> {
    pane_part(shell, "NavigationViewBackButton")
}

/// The button that opens and closes the pane.
fn pane_toggle_button(shell: &Shell) -> Ref<Button> {
    pane_part(shell, "TogglePaneButton")
}

/// The texts the bar of the navigation page shows: the visible text blocks of the navigation
/// page that are outside its pages.
fn bar_texts(shell: &Shell) -> Vec<String> {
    let navigation_page = shell.navigation_page().upcast::<Page>();
    descendants::<TextBlock>(&shell.navigation_page())
        .into_iter()
        .filter(|text| text.is_effectively_visible())
        .filter(|text| text.find_ancestor_of_type::<Page>(false).is_some_and(|page| page == navigation_page))
        .filter_map(|text| text.text())
        .collect()
}

/// Raises the text-changed event the search box raises 150 ms after the last change.
fn fire_search(shell: &Shell) {
    for timer in Dispatcher::timers_for_unit_tests()
        .into_iter()
        .filter(|timer| timer.interval() == std::time::Duration::from_millis(150))
    {
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
    }
    shell.settle();
}

/// The text box of the search box of the pane.
fn search_box(shell: &Shell) -> Ref<TextBox> {
    let search = descendants::<AutoSuggestBox>(&shell.main_view)
        .into_iter()
        .find(|search| search.name().as_deref() == Some("SearchBox"))
        .expect("the search box of the pane");
    descendants::<TextBox>(&search).into_iter().next().expect("the text box of the search box")
}

fn text(value: &str) -> String {
    value.to_string()
}

/// The wide pane lists the home item, the sections and the settings item, with the home item
/// selected; a section opens its page and lists its pages below it; a page item opens the page
/// and takes the selection.
#[test]
fn the_pane_opens_the_sections_and_their_pages() {
    let shell = Shell::start(1400.0, 900.0);
    let navigation = shell.navigation_view();
    assert_eq!(NavigationViewDisplayMode::Expanded, navigation.display_mode());
    assert!(navigation.is_pane_open(), "the wide pane is open");

    let sections = shell.view_model.home_sections();
    let mut expected = vec![text("Home")];
    expected.extend(sections.iter().map(|section| section.title()));
    expected.push(text("Settings"));
    assert_eq!(expected, pane_texts(&shell), "the items of the pane");
    assert_eq!(vec![text("Home")], selected_entries(&shell), "the home item is selected");
    assert!(sections.iter().all(|section| !section.is_expanded()), "no section is open");

    // The second section: its item opens the page of the section and the list of its pages.
    let section = &sections[1];
    shell.click(&pane_entry(&shell, &section.title()));
    assert!(shell.is_current(&section.page_item()), "the item of the section opens its page");
    assert!(shell.page().cast::<SectionPage>().is_some());
    assert!(section.is_expanded(), "the section is open");
    let section_items = items(section);
    let mut expected_open = vec![text("Home"), sections[0].title(), section.title()];
    expected_open.extend(section_items.iter().map(|item| item.header()));
    expected_open.extend(sections[2..].iter().map(|section| section.title()));
    expected_open.push(text("Settings"));
    assert_eq!(expected_open, pane_texts(&shell), "the open section lists its pages");
    assert_eq!(vec![section.title()], selected_entries(&shell), "the section carries the selection for its own page");
    assert!(bar_texts(&shell).contains(&section.title()), "the bar shows the title of the section: {:?}", bar_texts(&shell));

    // An item of the open section.
    let item = &section_items[1];
    shell.click(&pane_entry(&shell, &item.header()));
    assert!(shell.is_current(item), "the item {} opens its page", item.header());
    assert_eq!(vec![item.header()], selected_entries(&shell), "the item of the page carries the selection");
    let header = from_markup_value::<String>(&shell.page().header()).expect("the header of the page is a text");
    assert!(bar_texts(&shell).contains(&header), "the bar shows the header {header}: {:?}", bar_texts(&shell));

    // The home item.
    shell.click(&pane_entry(&shell, "Home"));
    assert!(shell.is_current(&shell.view_model.home_item()), "the home item opens the home page");
    assert!(shell.page().cast::<HomePage>().is_some());
    assert_eq!(vec![text("Home")], selected_entries(&shell));
    assert_eq!(Vec::<String>::new(), shell.binding_log(), "no binding of the pane reports");
}

/// The text of the search box is the query of the view model: the pane shows the sections with
/// a matching page, open, with the matching pages only, and everything again once the text is
/// cleared; a suggestion opens its page.
#[test]
fn the_search_box_filters_the_pane() {
    let shell = Shell::start(1400.0, 900.0);
    let search_box = search_box(&shell);
    assert!(search_box.is_effectively_visible(), "the open pane shows the search box");
    let bounds = shell.bounds_of(&search_box);
    assert!(shell.is_hit(&search_box, bounds.center()), "the search box is hit at its middle");
    let all = pane_texts(&shell);

    // The click gives the box the focus, as typing into it needs.
    shell.click(&search_box);
    assert!(search_box.is_keyboard_focus_within(), "a click focuses the search box");

    search_box.set_text(Some("slider"));
    fire_search(&shell);
    assert_eq!(Some(text("slider")), shell.view_model.query(), "the text of the box is the query");
    // The suggestions are the matching pages; their list covers the pane, so it is closed to look
    // at the pane below.
    let search = descendants::<AutoSuggestBox>(&shell.main_view).into_iter().next().expect("the search box");
    assert!(search.is_suggestion_list_open(), "the search box suggests the matching pages");
    search.set_is_suggestion_list_open(false);
    shell.settle();
    let sections = shell.view_model.home_sections();
    let key = PageItem::create_search_key(&["slider"]);
    let mut expected = vec![text("Home")];
    for section in sections.iter() {
        let matching: Vec<String> =
            items(section).iter().filter(|item| item.matches_search(&key)).map(|item| item.header()).collect();
        if !matching.is_empty() {
            expected.push(section.title());
            expected.extend(matching);
        }
    }
    expected.push(text("Settings"));
    assert!(expected.contains(&text("Slider")), "the query matches the Slider page: {expected:?}");
    assert!(expected.len() < all.len(), "the query leaves most sections out: {expected:?}");
    assert_eq!(expected, pane_texts(&shell), "the pane shows the matches of the query");

    // A result opens its page.
    shell.click(&pane_entry(&shell, "Slider"));
    assert_eq!(Some(text("Slider")), shell.view_model.current_page_item().map(|item| item.header()));

    // Cleared, the pane lists every section again, with the section of the current page open.
    search_box.set_text(Some(""));
    fire_search(&shell);
    let texts = listed_texts(&shell);
    for title in sections.iter().map(|section| section.title()) {
        assert!(texts.contains(&title), "the cleared search shows the section {title}: {texts:?}");
    }
    assert!(texts.contains(&text("Slider")) && texts.contains(&text("CheckBox")), "{texts:?}");
    assert_eq!(vec![text("Slider")], selected_entries(&shell));
    assert_eq!(Vec::<String>::new(), shell.binding_log(), "no binding of the search reports");
}

/// The toggle button opens and closes the pane; the back button pops a sample a gallery pushed,
/// then goes back through the journal of the frame, and the selection of the pane follows.
#[test]
fn the_pane_toggles_and_the_back_button_follows_the_journal() {
    let shell = Shell::start(1400.0, 900.0);
    let navigation = shell.navigation_view();
    assert!(navigation.is_pane_open());
    assert!(!navigation.is_back_enabled(), "nothing to go back to on the first page");
    shell.click(&pane_toggle_button(&shell));
    assert!(!navigation.is_pane_open(), "the toggle button closes the pane");
    shell.click(&pane_toggle_button(&shell));
    assert!(navigation.is_pane_open(), "the toggle button opens the pane again");

    // A gallery page pushes its sample: the back button then goes back to the gallery.
    let sections = shell.view_model.home_sections();
    let text_section = sections.iter().find(|section| section.title() == "Text").expect("the section Text");
    let text_box = items(text_section).into_iter().find(|item| item.header() == "TextBox").expect("the TextBox page");
    let page = shell.show(&text_box);
    assert!(navigation.is_back_enabled(), "the journal has the home page");
    let sample = descendants::<Button>(&page)
        .into_iter()
        .find(|card| texts(card).contains(&text("First Look")))
        .expect("the card of the sample First Look");
    shell.click(&sample);
    assert_eq!(2, shell.navigation_page().stack_depth(), "the card of a sample pushes the sample");
    assert!(bar_texts(&shell).contains(&text("First Look")), "the bar shows the sample: {:?}", bar_texts(&shell));
    shell.click(&back_button(&shell));
    assert_eq!(1, shell.navigation_page().stack_depth(), "the back button pops the sample");
    assert!(shell.is_current(&text_box));

    // Then it goes back in the journal: the home page, selected in the pane.
    shell.click(&back_button(&shell));
    assert!(shell.is_current(&shell.view_model.home_item()), "the back button goes back to the home page");
    assert!(shell.page().cast::<HomePage>().is_some());
    assert_eq!(vec![text("Home")], selected_entries(&shell), "the selection follows the journal");
    assert!(!navigation.is_back_enabled(), "nothing before the home page");
}

/// A card under the pointer has the border of the hovered card, and a pressed one is dimmed.
#[test]
fn a_card_shows_that_it_is_hovered_and_pressed() {
    use ferroui_base::media::{Color, IBrush};

    let shell = Shell::start(1400.0, 900.0);
    let page = shell.page();
    let section_control = descendants::<SectionControl>(&page).into_iter().next().expect("the first section");
    let card = cards(&section_control).into_iter().next().expect("the first card");
    let presenter = card.presenter().expect("the content presenter of the template of the card");
    let colour = |brush: Option<Rc<dyn IBrush>>| -> Option<Color> {
        brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    };
    let border = || colour(presenter.border_brush());
    let rest = Color::parse("#E5E5EA").ok();
    let hover = Color::parse("#3E6DF3").ok();
    assert!(!card.is_pointer_over());
    assert_eq!(rest, border(), "the border of a card at rest (CatalogCardBorderBrush)");
    assert_eq!(1.0, presenter.opacity());

    let position = shell.bounds_of(&card).center();
    shell.pointer_move(position);
    shell.settle();
    assert!(card.is_pointer_over(), "the card is under the pointer");
    assert_eq!(hover, border(), "the border of a hovered card (CatalogCardBorderBrushHover)");

    shell.pointer_down(position);
    assert!(card.is_pressed(), "the card is pressed");
    assert_eq!(0.8, presenter.opacity(), "a pressed card is dimmed");
    assert!(shell.is_current(&shell.view_model.home_item()), "the press alone does not navigate");

    // Released outside the card, the press is given up.
    let outside = Point::new(position.x, shell.bounds_of(&card).y - 20.0);
    shell.pointer_move(outside);
    shell.pointer_up(outside);
    shell.settle();
    assert!(!card.is_pressed() && !card.is_pointer_over());
    assert_eq!(1.0, presenter.opacity());
    assert_eq!(rest, border(), "the border of the card the pointer left");
    assert!(shell.is_current(&shell.view_model.home_item()), "a press released outside does not navigate");
}

/// The settings entry opens the settings page; the theme variant and the theme of the catalog
/// follow its combo boxes, and the home page shows its cards under each.
#[test]
fn the_settings_page_switches_the_theme_variant_and_the_theme() {
    use ferroui_base::media::Color;

    let shell = Shell::start(1400.0, 900.0);
    let app = Application::current().expect("the application");
    shell.click(&pane_entry(&shell, "Settings"));
    assert!(shell.is_current(&shell.view_model.settings_item()), "the settings item opens the settings page");
    let page = shell.page();
    assert!(page.cast::<SettingsPage>().is_some());
    assert_eq!(vec![text("Settings")], selected_entries(&shell));
    assert!(bar_texts(&shell).contains(&text("Settings")), "the bar shows Settings: {:?}", bar_texts(&shell));

    let settings_combos = |page: &ferroui_base::Visual| {
        descendants::<ComboBox>(page).into_iter()
            .filter(|control| !control.name().is_some_and(|name| name.starts_with("MacOS")))
            .collect::<Vec<_>>()
    };
    let combo_boxes = settings_combos(&page);
    assert_eq!(6, combo_boxes.len(), "the combo boxes of the settings page");
    for combo_box in &combo_boxes {
        let bounds = combo_box.bounds();
        assert!(bounds.width >= 200.0 && bounds.height > 0.0, "a combo box of the settings page has the bounds {bounds:?}");
    }
    // The port starts the catalog in its WinUI theme (the original in the Fluent theme). The list
    // of themes is the themes the catalog is built with: the WinUI theme alone, with the feature
    // `classic-themes` the Fluent and Simple themes before it. With the feature the checks of the
    // variants below are those of the original, in the Fluent theme; without it they are made in
    // the WinUI theme (the backgrounds of the cards are the catalog's own, `CustomThemes.xaml`).
    let theme_index = |theme: CatalogTheme| {
        App::themes().iter().position(|built| *built == theme).expect("a theme the catalog is built with") as i32
    };
    assert_eq!(App::themes().len(), combo_boxes[1].item_count() as usize, "the themes of the list");
    assert_eq!(theme_index(CatalogTheme::WinUI), combo_boxes[1].selected_index(), "the WinUI theme is selected");
    assert_eq!(CatalogTheme::WinUI, App::current_theme());
    assert_eq!(0, combo_boxes[2].selected_index(), "the default variant is selected");
    assert_eq!(Some(ThemeVariant::light()), shell.window.actual_theme_variant());
    #[cfg(feature = "classic-themes")]
    let combo_boxes = {
        combo_boxes[1].set_selected_index(theme_index(CatalogTheme::Fluent));
        shell.settle();
        assert_eq!(CatalogTheme::Fluent, App::current_theme(), "the theme the page set");
        let combo_boxes = settings_combos(&shell.show(&shell.view_model.settings_item()));
        assert_eq!(theme_index(CatalogTheme::Fluent), combo_boxes[1].selected_index(), "the Fluent theme is selected");
        combo_boxes
    };

    // The first section of the home page has its cards; the background of the first.
    let card_background = |shell: &Shell| {
        let page = shell.show(&shell.view_model.home_item());
        let sections = shell.view_model.home_sections();
        let section_controls = descendants::<SectionControl>(&page);
        assert_cards(&section_controls[0], &sections[0], "the home page after the settings");
        let card = cards(&section_controls[0]).into_iter().next().expect("the first card");
        card.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    };
    let settings_combo_boxes = |shell: &Shell| settings_combos(&shell.show(&shell.view_model.settings_item()));

    // The dark variant.
    combo_boxes[2].set_selected_index(2);
    shell.settle();
    assert_eq!(Some(ThemeVariant::dark()), app.requested_theme_variant(), "the variant the page asks for");
    assert_eq!(Some(ThemeVariant::dark()), shell.window.actual_theme_variant(), "the variant of the window");
    assert_eq!(Color::parse("#2B2B33").ok(), card_background(&shell), "the card of the dark variant");

    // The light variant.
    let combo_boxes = settings_combo_boxes(&shell);
    assert_eq!(2, combo_boxes[2].selected_index(), "the settings page shows the variant it set");
    combo_boxes[2].set_selected_index(1);
    shell.settle();
    assert_eq!(Some(ThemeVariant::light()), shell.window.actual_theme_variant());
    assert_eq!(Color::parse("#FFFFFF").ok(), card_background(&shell), "the card of the light variant");

    // The Simple theme, and back to the Fluent theme.
    #[cfg(feature = "classic-themes")]
    {
        let combo_boxes = settings_combo_boxes(&shell);
        combo_boxes[1].set_selected_index(theme_index(CatalogTheme::Simple));
        shell.settle();
        assert_eq!(CatalogTheme::Simple, App::current_theme(), "the theme the page set");
        card_background(&shell);
        let combo_boxes = settings_combo_boxes(&shell);
        assert_eq!(theme_index(CatalogTheme::Simple), combo_boxes[1].selected_index(), "the settings page shows the theme it set");
        combo_boxes[1].set_selected_index(theme_index(CatalogTheme::Fluent));
        shell.settle();
        assert_eq!(CatalogTheme::Fluent, App::current_theme());
        card_background(&shell);
    }
    // The WinUI theme of the port, the catalog's default (not in the original), which themes every
    // control alone (docs/porting/winui-theme/PROGRESS.md).
    let combo_boxes = settings_combo_boxes(&shell);
    combo_boxes[1].set_selected_index(theme_index(CatalogTheme::WinUI));
    shell.settle();
    assert_eq!(CatalogTheme::WinUI, App::current_theme(), "the theme the page set");
    card_background(&shell);
    let combo_boxes = settings_combo_boxes(&shell);
    assert_eq!(theme_index(CatalogTheme::WinUI), combo_boxes[1].selected_index(), "the settings page shows the theme it set");
}

/// The optional macOS controls modify the same theme instance that the whole catalog uses.
#[cfg(feature = "macos-theme")]
#[test]
fn the_macos_settings_apply_to_the_live_catalog_theme() {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::media::Color;
    use ferroui_controls::CheckBox;
    use ferroui_themes_macos::ControlSize;

    let shell = Shell::start(1400.0, 1100.0);
    let primary_label_color = || from_markup_value::<Color>(&Application::current().unwrap()
        .try_get_resource(&ResourceKey::from("TextFillColorPrimary"), Some(&ThemeVariant::light())).flatten())
        .expect("the current theme's label color");
    let winui_label = primary_label_color();
    App::set_catalog_themes(CatalogTheme::MacOS);
    shell.settle();
    let page = shell.show(&shell.view_model.settings_item());
    let theme = App::macos_theme().expect("the macOS theme is retained by the application");
    let combo = |name: &str| descendants::<ComboBox>(&page).into_iter()
        .find(|control| control.name().as_deref() == Some(name)).expect("a macOS settings combo");
    let check = |name: &str| descendants::<CheckBox>(&page).into_iter()
        .find(|control| control.name().as_deref() == Some(name)).expect("a macOS settings check box");

    assert!(combo("MacOSControlSize").is_effectively_visible());
    assert_eq!(2, combo("MacOSControlSize").selected_index());
    combo("MacOSControlSize").set_selected_index(3);
    combo("MacOSAccent").set_selected_index(2);
    check("MacOSReduceMotion").set_is_checked(Some(true));
    check("MacOSReduceTransparency").set_is_checked(Some(true));
    check("MacOSIncreaseContrast").set_is_checked(Some(true));
    check("MacOSWindowActive").set_is_checked(Some(false));
    shell.settle();
    assert_eq!(ControlSize::Large, theme.control_size());
    assert_eq!(Some(Color::from_rgb(175, 82, 222)), theme.accent_color());
    assert!(theme.reduce_motion());
    assert!(theme.reduce_transparency());
    assert!(theme.increase_contrast());
    assert!(!theme.window_active());

    // A theme round trip keeps the preview settings, without contaminating the WinUI theme.
    App::set_catalog_themes(CatalogTheme::WinUI);
    assert_eq!(winui_label, primary_label_color(), "macOS contrast settings do not change WinUI colors");
    assert!(Application::current().unwrap()
        .try_get_resource(&ResourceKey::from("MacOSControlHeight"), None).flatten().is_none(),
        "detaching macOS removes its dynamic resources from the application");
    App::set_catalog_themes(CatalogTheme::MacOS);
    shell.settle();
    assert_eq!(ControlSize::Large, App::macos_theme().unwrap().control_size());
    assert!(App::macos_theme().unwrap().reduce_motion());
    assert_eq!(Some(Color::from_rgb(175, 82, 222)), App::macos_theme().unwrap().accent_color());
}

/// Below the wide breakpoint the pane is compact: closed it is a strip of icons as wide as the
/// compact pane, and the toggle button opens it over the content with its texts and the search
/// box. Below the narrow breakpoint it is minimal: only the toggle button is shown.
#[test]
fn the_pane_is_compact_and_minimal_on_narrow_windows() {
    let shell = Shell::start(800.0, 600.0);
    let navigation = shell.navigation_view();
    assert_eq!(NavigationViewDisplayMode::Compact, navigation.display_mode());
    assert!(!navigation.is_pane_open(), "the compact pane starts closed");
    let frame = shell.main_view.content_frame();
    let left = shell.bounds_of(&frame).x;
    assert!(left <= navigation.compact_pane_length() + 1.0, "the closed pane is a strip of icons; the content starts at {left}");
    shell.click(&pane_toggle_button(&shell));
    assert!(navigation.is_pane_open(), "the toggle button opens the pane");
    assert!(search_box(&shell).is_effectively_visible(), "the open pane shows the search box");
    let texts = pane_texts(&shell);
    assert!(texts.contains(&text("Home")) && texts.contains(&text("Basic Input")), "{texts:?}");
    // A page chosen in the open compact pane closes it again.
    let section = &shell.view_model.home_sections()[0];
    shell.click(&pane_entry(&shell, &section.title()));
    assert!(shell.is_current(&section.page_item()));
    drop(shell);

    let shell = Shell::start(500.0, 600.0);
    assert_eq!(NavigationViewDisplayMode::Minimal, shell.navigation_view().display_mode());
    assert!(pane_entries(&shell).is_empty(), "the minimal pane shows no item: {:?}", pane_texts(&shell));
    shell.click(&pane_toggle_button(&shell));
    assert!(shell.navigation_view().is_pane_open(), "the toggle button opens the minimal pane");
    assert!(pane_texts(&shell).contains(&text("Home")));
}

/// The keyboard: an item of the pane takes the focus, the arrow keys move it to the next items,
/// and Enter or Space on an item opens its page as a click does.
#[test]
fn the_pane_is_reached_with_the_keyboard() {
    use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType};
    use ferroui_base::input::{Key, KeyDeviceType, PhysicalKey};

    let shell = Shell::start(1400.0, 900.0);
    let key = |key: Key, physical_key: PhysicalKey| {
        let input =
            ferroui_controls::platform::ITopLevelImpl::input(&*shell.window_impl).expect("the window handles input");
        for type_ in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
            input(Rc::new(RawKeyEventArgs::new(
                shell.keyboard.clone() as Rc<dyn IInputDevice>,
                0,
                shell.input_root(),
                type_,
                key,
                RawInputModifiers::NONE,
                physical_key,
                None,
                KeyDeviceType::Keyboard,
            )));
        }
        shell.settle();
    };
    // The element with the focus: its class, its name and the texts it shows.
    let focus = |shell: &Shell| -> String {
        match shell.window.focus_manager().get_focused_element() {
            Some(element) => {
                let path: Vec<&str> =
                    element.get_self_and_visual_ancestors().take(6).map(|visual| visual.get_type().name()).collect();
                format!("{path:?} {:?} {:?}", element.name(), texts(&element))
            }
            None => "nothing".to_string(),
        }
    };
    let focused = |shell: &Shell| -> Vec<String> {
        pane_entries(shell).into_iter().filter(|(_, item)| item.is_keyboard_focus_within()).map(|(text, _)| text).collect()
    };

    let home = pane_entry(&shell, "Home");
    assert!(home.focus(), "the home item takes the focus");
    assert_eq!(vec![text("Home")], focused(&shell));

    let sections = shell.view_model.home_sections();
    key(Key::Down, PhysicalKey::ArrowDown);
    assert_eq!(vec![sections[0].title()], focused(&shell), "Down moves the focus to the first section: {}", focus(&shell));
    key(Key::Down, PhysicalKey::ArrowDown);
    assert_eq!(vec![sections[1].title()], focused(&shell), "Down moves the focus to the second section");

    key(Key::Enter, PhysicalKey::Enter);
    assert!(shell.is_current(&sections[1].page_item()), "Enter on a section opens its page");
    assert!(sections[1].is_expanded(), "Enter on a section opens the section");

    // An item of the open section takes the focus, and Space opens its page.
    let first = items(&sections[1])[0].clone();
    assert!(pane_entry(&shell, &first.header()).focus(), "the item of a page takes the focus");
    key(Key::Space, PhysicalKey::Space);
    assert!(shell.is_current(&first), "Space on an item opens its page; the focus is on {}", focus(&shell));
}

/// Every class of the sample with a document populates itself from the document in its
/// constructor (`InitializeComponent()` in the constructor of the managed original; the
/// application calls it from `Initialize`): the source of a class that declares its document
/// calls `initialize_component`. The comparison of the documents populates instances the
/// constructor did not run for, so it does not see a constructor that leaves the call out.
#[test]
fn every_class_with_a_document_populates_itself() {
    fn visit(directory: &std::path::Path, missing: &mut Vec<String>, declared: &mut usize) {
        for entry in std::fs::read_dir(directory).expect("a directory of the sample") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "build" && name != "tests") {
                    visit(&path, missing, declared);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                // The class of the themes of the original is compiled with its feature only.
                if !cfg!(feature = "classic-themes") && path.file_name().is_some_and(|name| name == "classic_themes.rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("a source of the sample");
                let classes = source.lines().filter(|line| line.starts_with("xaml_class!(")).count();
                *declared += classes;
                if classes > source.matches(".initialize_component();").count().min(classes) {
                    missing.push(path.display().to_string());
                }
            }
        }
    }

    let mut missing = Vec::new();
    let mut declared = 0;
    visit(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), &mut missing, &mut declared);
    assert_eq!(crate::register_types::classes().count(), declared, "the sources that declare the document of a class");
    assert_eq!(Vec::<String>::new(), missing, "classes that never populate themselves from their document");
}

/// The icons of the pane (path icons of geometries of 24 units) are scaled into their
/// boxes, by the view box of an item of the navigation view or by the theme of path icons.
#[test]
fn the_icons_of_the_pane_are_scaled_into_their_boxes() {
    use ferroui_controls::shapes::Path;
    use ferroui_controls::PathIcon;

    let shell = Shell::start(1400.0, 900.0);
    let icons = descendants::<PathIcon>(&shell.main_view);
    assert!(!icons.is_empty(), "the pane has path icons");
    for icon in icons.iter().filter(|icon| icon.is_effectively_visible()) {
        let types: Vec<String> = icon.get_self_and_visual_descendants().map(|d| d.get_type().name().to_string()).collect();
        let path = descendants::<Path>(icon).into_iter().next().unwrap_or_else(|| panic!("the path of an icon: {types:?}"));
        // The icon of an item of the navigation view is scaled by the view box of the item.
        let in_viewbox = icon.get_self_and_visual_ancestors().take(4).any(|d| d.is::<ferroui_controls::Viewbox>());
        assert!(
            in_viewbox || path.stretch() == ferroui_base::media::Stretch::Uniform,
            "the geometry of an icon is scaled: {types:?}"
        );
    }
}

/// The title bar of the shell (as WinUI Gallery): shown in the title bar of the window while its
/// client area extends into it, with the search box and the back and toggle buttons the navigation
/// view then hides; collapsed when the client area no longer extends into the title bar and in
/// full screen, where the navigation view shows its own buttons and the search box again.
#[test]
fn the_title_bar_follows_the_window_chrome() {
    use ferroui_controls::platform::IWindowImpl;
    use ferroui_controls::{NavigationViewBackButtonVisible, WindowState};

    let shell = Shell::start(1400.0, 900.0);
    let navigation = shell.navigation_view();
    let title_bar = shell.main_view.title_bar();
    let extend = |extended: bool| {
        shell.window_impl.is_client_area_extended_to_decorations.set(extended);
        if let Some(changed) = shell.window_impl.extend_client_area_to_decorations_changed() {
            changed(extended);
        }
        shell.settle();
    };
    let search_in_title_bar = |shell: &Shell| {
        title_bar.content().is_some_and(|content| content.name().as_deref() == Some("SearchBox"))
    };

    assert!(!shell.main_view.is_title_bar_active(), "the mock window has its own title bar");
    assert!(!title_bar.is_visible() && navigation.auto_suggest_box().is_some());

    extend(true);
    assert!(shell.main_view.is_title_bar_active() && title_bar.is_visible(), "the title bar of the shell is shown");
    assert!(search_in_title_bar(&shell), "the search box is in the title bar");
    assert!(navigation.auto_suggest_box().is_none());
    assert_eq!(NavigationViewBackButtonVisible::Collapsed, navigation.is_back_button_visible());
    assert!(!navigation.is_pane_toggle_button_visible());
    assert!(title_bar.is_pane_toggle_button_visible());
    assert_eq!("FerroUI Control Gallery", title_bar.title());

    // Its toggle button opens and closes the pane, its back button follows the journal.
    let open = navigation.is_pane_open();
    let toggle = descendants::<Button>(&title_bar)
        .into_iter()
        .find(|button| button.name().as_deref() == Some("PART_PaneToggleButton"))
        .expect("the toggle button of the title bar");
    shell.click(&toggle);
    assert_eq!(!open, navigation.is_pane_open(), "the toggle button of the title bar toggles the pane");
    assert!(!title_bar.is_back_button_visible(), "nothing to go back to");
    let item = items(&shell.view_model.home_sections()[0])[0].clone();
    shell.show(&item);
    assert!(title_bar.is_back_button_visible(), "the frame can go back");

    // Full screen hides it; leaving full screen shows it again.
    shell.window.set_window_state(WindowState::FullScreen);
    shell.settle();
    assert!(!shell.main_view.is_title_bar_active() && !title_bar.is_visible(), "full screen hides the title bar");
    assert_eq!(NavigationViewBackButtonVisible::Visible, navigation.is_back_button_visible());
    shell.window.set_window_state(WindowState::Normal);
    shell.settle();
    assert!(shell.main_view.is_title_bar_active(), "the title bar is back after full screen");

    // A title bar of the window's own again: the navigation view has its buttons and the search box.
    extend(false);
    assert!(!shell.main_view.is_title_bar_active() && !title_bar.is_visible());
    assert!(navigation.auto_suggest_box().is_some(), "the search box is back in the pane");
    assert_eq!(NavigationViewBackButtonVisible::Visible, navigation.is_back_button_visible());
    assert!(navigation.is_pane_toggle_button_visible());
}
