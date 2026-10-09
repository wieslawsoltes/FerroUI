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
//! section title, the page a card opens, the drawer, the search box, the
//! header and the settings page.

use super::frame_benchmark::RasterSurface;
use super::support::*;
use crate::controls::{HomeItemExpander, SectionControl, SelectableButton};
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
    Application, Button, ComboBox, Control, NavigationPage, Page, ScrollViewer, SplitViewDisplayMode, TextBlock, TextBox,
    Window,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The frames after an input or a navigation: the transition of the
/// navigation page ends within them.
const SETTLE_FRAMES: usize = 8;

/// The time of the global clock between two frames.
const FRAME_TIME_MS: f64 = 100.0;

/// The sizes the defect was seen at: the wide layout (the drawer inline) and
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

    fn navigation_page(&self) -> Ref<NavigationPage> {
        self.main_view.get_control::<NavigationPage>("NavPage")
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

/// The entries of the drawer that are shown: the selectable buttons below the main view and
/// outside the page (the home entry, the headers of the sections, the entries of the pages of
/// an open section and the settings entry), each with the text it shows, from the top of the
/// drawer to its bottom.
fn drawer_entries(shell: &Shell) -> Vec<(String, Ref<SelectableButton>)> {
    let navigation_page = shell.navigation_page();
    let mut entries: Vec<(f64, String, Ref<SelectableButton>)> = descendants::<SelectableButton>(&shell.main_view)
        .into_iter()
        .filter(|button| button.is_effectively_visible())
        .filter(|button| button.find_ancestor_of_type::<NavigationPage>(false).is_none_or(|page| page != navigation_page))
        .filter_map(|button| texts(&button).into_iter().next().map(|text| (shell.bounds_of(&button).y, text, button)))
        .collect();
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    entries.into_iter().map(|(_, text, button)| (text, button)).collect()
}

fn drawer_entry(shell: &Shell, text: &str) -> Ref<SelectableButton> {
    let entries = drawer_entries(shell);
    let names: Vec<String> = entries.iter().map(|(name, _)| name.clone()).collect();
    entries
        .into_iter()
        .find(|(name, _)| name == text)
        .map(|(_, button)| button)
        .unwrap_or_else(|| panic!("the drawer has no entry {text}: it shows {names:?}"))
}

fn drawer_texts(shell: &Shell) -> Vec<String> {
    drawer_entries(shell).into_iter().map(|(text, _)| text).collect()
}

fn selected_entries(shell: &Shell) -> Vec<String> {
    drawer_entries(shell).into_iter().filter(|(_, button)| button.is_selected()).map(|(text, _)| text).collect()
}

/// The button of the bar of the navigation page: it toggles the drawer on a root page and
/// goes back on a pushed one.
fn back_button(shell: &Shell) -> Ref<Button> {
    descendants::<Button>(&shell.navigation_page())
        .into_iter()
        .find(|button| button.name().as_deref() == Some("PART_BackButton") && button.is_effectively_visible())
        .expect("the back button of the navigation page")
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

fn search_box(shell: &Shell) -> Ref<TextBox> {
    descendants::<TextBox>(&shell.main_view)
        .into_iter()
        .find(|text_box| text_box.name().as_deref() == Some("SearchBox"))
        .expect("the search box of the drawer")
}

fn text(value: &str) -> String {
    value.to_string()
}

/// The wide drawer lists the home entry, the sections and the settings entry, with the home
/// entry selected; a section opens its page and lists its pages below it; a page entry opens
/// the page and takes the selection.
#[test]
fn the_drawer_opens_the_sections_and_their_pages() {
    let shell = Shell::start(1400.0, 900.0);
    assert_eq!(SplitViewDisplayMode::Inline, shell.view_model.display_mode());
    assert!(shell.view_model.is_drawer_opened() && shell.main_view.is_open(), "the wide drawer is open");

    let sections = shell.view_model.home_sections();
    let mut expected = vec![text("Home")];
    expected.extend(sections.iter().map(|section| section.title()));
    expected.push(text("Settings"));
    assert_eq!(expected, drawer_texts(&shell), "the entries of the drawer");
    assert_eq!(vec![text("Home")], selected_entries(&shell), "the home entry is selected");
    let expanders = descendants::<HomeItemExpander>(&shell.main_view);
    assert_eq!(sections.len(), expanders.len());
    assert!(expanders.iter().all(|expander| !expander.is_effectively_expanded()), "no section is open");

    // The second section: its header opens the page of the section and the list of its pages.
    let section = &sections[1];
    shell.click(&drawer_entry(&shell, &section.title()));
    assert!(shell.is_current(&section.page_item()), "the header of the section opens its page");
    assert!(shell.page().cast::<SectionPage>().is_some());
    assert!(section.is_expanded() && expanders[1].is_effectively_expanded(), "the section is open");
    let section_items = items(section);
    let mut expected_open = vec![text("Home"), sections[0].title(), section.title()];
    expected_open.extend(section_items.iter().map(|item| item.header()));
    expected_open.extend(sections[2..].iter().map(|section| section.title()));
    expected_open.push(text("Settings"));
    assert_eq!(expected_open, drawer_texts(&shell), "the open section lists its pages");
    assert_eq!(vec![section.title()], selected_entries(&shell), "the section carries the selection for its own page");
    assert!(bar_texts(&shell).contains(&section.title()), "the bar shows the title of the section: {:?}", bar_texts(&shell));

    // An entry of the open section.
    let item = &section_items[1];
    shell.click(&drawer_entry(&shell, &item.header()));
    assert!(shell.is_current(item), "the entry {} opens its page", item.header());
    assert_eq!(vec![item.header()], selected_entries(&shell), "the entry of the page carries the selection");
    let header = from_markup_value::<String>(&shell.page().header()).expect("the header of the page is a text");
    assert!(bar_texts(&shell).contains(&header), "the bar shows the header {header}: {:?}", bar_texts(&shell));

    // The home entry.
    shell.click(&drawer_entry(&shell, "Home"));
    assert!(shell.is_current(&shell.view_model.home_item()), "the home entry opens the home page");
    assert!(shell.page().cast::<HomePage>().is_some());
    assert_eq!(vec![text("Home")], selected_entries(&shell));
    assert!(bar_texts(&shell).contains(&text("Home")), "the bar shows Home: {:?}", bar_texts(&shell));
    assert_eq!(Vec::<String>::new(), shell.binding_log(), "no binding of the drawer reports");
}

/// The text of the search box is the query of the view model: the drawer shows the sections
/// with a matching page, open, with the matching pages only, and everything again once the
/// text is cleared.
#[test]
fn the_search_box_filters_the_drawer() {
    let shell = Shell::start(1400.0, 900.0);
    let search_box = search_box(&shell);
    assert!(search_box.is_effectively_visible(), "the open drawer shows the search box");
    let bounds = shell.bounds_of(&search_box);
    assert!(shell.is_hit(&search_box, bounds.center()), "the search box is hit at its middle");
    let all = drawer_texts(&shell);

    // The click gives the box the focus, as typing into it needs.
    shell.click(&search_box);
    assert!(search_box.is_keyboard_focus_within(), "a click focuses the search box");

    search_box.set_text(Some("slider"));
    shell.settle();
    assert_eq!(Some(text("slider")), shell.view_model.query(), "the text of the box is the query");
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
    assert_eq!(expected, drawer_texts(&shell), "the drawer shows the matches of the query");

    // A result opens its page.
    shell.click(&drawer_entry(&shell, "Slider"));
    assert_eq!(Some(text("Slider")), shell.view_model.current_page_item().map(|item| item.header()));

    // Cleared, the drawer lists every section again, with the section of the current page open.
    search_box.set_text(Some(""));
    shell.settle();
    let texts = drawer_texts(&shell);
    for title in sections.iter().map(|section| section.title()) {
        assert!(texts.contains(&title), "the cleared search shows the section {title}: {texts:?}");
    }
    assert!(texts.contains(&text("Slider")) && texts.contains(&text("CheckBox")), "{texts:?}");
    assert_eq!(vec![text("Slider")], selected_entries(&shell));
    assert_eq!(Vec::<String>::new(), shell.binding_log(), "no binding of the search reports");
}

/// The button of the bar toggles the drawer on a root page, and goes back from a sample a
/// gallery pushed; the bar shows the header of the page it is on.
#[test]
fn the_button_of_the_bar_toggles_the_drawer_and_goes_back() {
    let shell = Shell::start(1400.0, 900.0);
    assert!(shell.view_model.is_drawer_opened());
    let button = back_button(&shell);
    let bounds = shell.bounds_of(&button);
    assert!(bounds.width > 0.0 && shell.is_hit(&button, bounds.center()), "the button of the bar at {bounds:?} is hit");

    shell.click(&button);
    assert!(!shell.view_model.is_drawer_opened() && !shell.main_view.is_open(), "the button closes the drawer");
    // Closed, the drawer shows its entries without their texts.
    assert_eq!(Vec::<String>::new(), drawer_texts(&shell), "the texts of the closed drawer");
    shell.click(&back_button(&shell));
    assert!(shell.view_model.is_drawer_opened() && shell.main_view.is_open(), "the button opens the drawer again");

    // A gallery page pushes its sample: the button then goes back to the gallery.
    let sections = shell.view_model.home_sections();
    let text_section = sections.iter().find(|section| section.title() == "Text").expect("the section Text");
    let text_box = items(text_section).into_iter().find(|item| item.header() == "TextBox").expect("the TextBox page");
    let page = shell.show(&text_box);
    let sample = descendants::<Button>(&page)
        .into_iter()
        .find(|card| texts(card).contains(&text("First Look")))
        .expect("the card of the sample First Look");
    shell.click(&sample);
    assert_eq!(2, shell.navigation_page().stack_depth(), "the card of a sample pushes the sample");
    assert!(bar_texts(&shell).contains(&text("First Look")), "the bar shows the sample: {:?}", bar_texts(&shell));
    shell.click(&back_button(&shell));
    assert_eq!(1, shell.navigation_page().stack_depth(), "the button of the bar goes back");
    assert!(shell.view_model.is_drawer_opened(), "going back leaves the drawer as it is");
    assert!(shell.is_current(&text_box));
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
    shell.click(&drawer_entry(&shell, "Settings"));
    assert!(shell.is_current(&shell.view_model.settings_item()), "the settings entry opens the settings page");
    let page = shell.page();
    assert!(page.cast::<SettingsPage>().is_some());
    assert_eq!(vec![text("Settings")], selected_entries(&shell));
    assert!(bar_texts(&shell).contains(&text("Settings")), "the bar shows Settings: {:?}", bar_texts(&shell));

    let combo_boxes = descendants::<ComboBox>(&page);
    assert_eq!(6, combo_boxes.len(), "the combo boxes of the settings page");
    for combo_box in &combo_boxes {
        let bounds = combo_box.bounds();
        assert!(bounds.width >= 200.0 && bounds.height > 0.0, "a combo box of the settings page has the bounds {bounds:?}");
    }
    assert_eq!(0, combo_boxes[1].selected_index(), "the Fluent theme is selected");
    assert_eq!(0, combo_boxes[2].selected_index(), "the default variant is selected");
    assert_eq!(Some(ThemeVariant::light()), shell.window.actual_theme_variant());

    // The first section of the home page has its cards; the background of the first.
    let card_background = |shell: &Shell| {
        let page = shell.show(&shell.view_model.home_item());
        let sections = shell.view_model.home_sections();
        let section_controls = descendants::<SectionControl>(&page);
        assert_cards(&section_controls[0], &sections[0], "the home page after the settings");
        let card = cards(&section_controls[0]).into_iter().next().expect("the first card");
        card.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    };
    let settings_combo_boxes = |shell: &Shell| descendants::<ComboBox>(&shell.show(&shell.view_model.settings_item()));

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

    // The Simple theme, and back.
    let combo_boxes = settings_combo_boxes(&shell);
    combo_boxes[1].set_selected_index(1);
    shell.settle();
    assert_eq!(CatalogTheme::Simple, App::current_theme(), "the theme the page set");
    card_background(&shell);
    let combo_boxes = settings_combo_boxes(&shell);
    assert_eq!(1, combo_boxes[1].selected_index(), "the settings page shows the theme it set");
    combo_boxes[1].set_selected_index(0);
    shell.settle();
    assert_eq!(CatalogTheme::Fluent, App::current_theme());
    card_background(&shell);
}

/// Below the wide breakpoint the drawer is compact: closed it shows the icons of its entries
/// without their texts and without the search box, and the button of the bar opens it.
#[test]
fn the_compact_drawer_shows_its_texts_when_it_is_open() {
    let shell = Shell::start(800.0, 600.0);
    assert_eq!(SplitViewDisplayMode::CompactInline, shell.view_model.display_mode());
    let search_box = search_box(&shell);
    // Upstream leaves the drawer of the compact layout as it was: open from the start.
    assert!(shell.view_model.is_drawer_opened() && shell.main_view.is_open(), "the drawer starts open");
    shell.click(&back_button(&shell));
    assert!(!shell.view_model.is_drawer_opened() && !shell.main_view.is_open(), "the button closes the drawer");
    assert!(!search_box.is_effectively_visible(), "the closed drawer hides the search box");
    assert_eq!(Vec::<String>::new(), drawer_texts(&shell), "the closed compact drawer shows no text");
    shell.click(&back_button(&shell));
    assert!(shell.view_model.is_drawer_opened() && shell.main_view.is_open(), "the button opens the drawer");
    assert!(search_box.is_effectively_visible(), "the open drawer shows the search box");
    let texts = drawer_texts(&shell);
    assert!(texts.contains(&text("Home")) && texts.contains(&text("Basic Input")), "{texts:?}");
}

/// The keyboard: Tab moves the focus from the home entry through the sections, Enter on a
/// section opens its page as a click does, and Space on the entry of a page opens the page.
#[test]
fn the_drawer_is_reached_with_the_keyboard() {
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
    let focused = |shell: &Shell| -> Vec<String> {
        drawer_entries(shell).into_iter().filter(|(_, button)| button.is_focused()).map(|(text, _)| text).collect()
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

    let home = drawer_entry(&shell, "Home");
    assert!(home.focus(), "the home entry takes the focus");
    assert_eq!(vec![text("Home")], focused(&shell));

    let sections = shell.view_model.home_sections();
    key(Key::Tab, PhysicalKey::Tab);
    assert_eq!(vec![sections[0].title()], focused(&shell), "Tab moves the focus to the first section");
    key(Key::Tab, PhysicalKey::Tab);
    assert_eq!(vec![sections[1].title()], focused(&shell), "Tab moves the focus to the second section");

    key(Key::Enter, PhysicalKey::Enter);
    assert!(shell.is_current(&sections[1].page_item()), "Enter on a section opens its page");
    assert!(sections[1].is_expanded(), "Enter on a section opens the section");
    // The command of the catalog cannot execute while it executes (`MiniCommand.Busy`, which
    // raises `CanExecuteChanged` around the call), so the button that ran it was disabled for
    // that moment, and an element that is disabled with the focus gives the focus up
    // (`InputElement.OnPropertyChanged` for `IsEffectivelyEnabled`): as upstream has it.
    assert_eq!(Vec::<String>::new(), focused(&shell), "the focus left the entry that ran the command: {}", focus(&shell));

    // An entry of the open section takes the focus, and Space opens its page.
    let first = items(&sections[1])[0].clone();
    assert!(drawer_entry(&shell, &first.header()).focus(), "the entry of a page takes the focus");
    assert_eq!(vec![first.header()], focused(&shell));
    key(Key::Space, PhysicalKey::Space);
    assert!(shell.is_current(&first), "Space on an entry opens its page; the focus is on {}", focus(&shell));
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
