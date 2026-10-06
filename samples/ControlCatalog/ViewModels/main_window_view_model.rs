//! Port of `ViewModels/MainWindowViewModel.cs`. The list of the pages (the
//! other part of the partial class) is in
//! `main_window_view_model_page_list.rs`.
//!
//! Not ported, because what they name does not exist in the framework yet:
//! the `[Required]` validation attribute of `ValidatedDateExample` (data
//! annotations), `Win32WindowCornerPreferences` and
//! `Win32WindowCornerPreference` (`Win32Properties`).

use crate::icons::Icons;
use crate::models::{HomeSection, PageItem};
use crate::page_assets::PageAssets;
use crate::pages::{HomePage, SettingsPage};
use crate::view_models::main_window_view_model_page_list::{page_sections, UnavailablePage};
use crate::view_models::SettingsViewModel;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::ICommand;
use ferroui_base::media::StreamGeometry;
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_markup_type, Ref, Thickness};
use ferroui_controls::chrome::TitleBarDecorations;
use ferroui_controls::{Application, INavigation, ItemsSource, SplitViewDisplayMode};
use ferroui_dialogs::AboutFerroDialog;
use mini_mvvm::{start_async, MiniCommand, ViewModelBase};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// The view model of the main window and the main view: the pages, the
/// current page, the state of the drawer and the options of the window.
pub struct MainWindowViewModel {
    base: ViewModelBase,
    settings_view_model: Rc<SettingsViewModel>,
    home_item: Rc<PageItem>,
    settings_item: Rc<PageItem>,
    pub(super) page_sections: Vec<Rc<HomeSection>>,
    unavailable_pages: Vec<UnavailablePage>,
    home_sections: OnceCell<Rc<Vec<Rc<HomeSection>>>>,
    navigator: RefCell<Option<Rc<dyn INavigation>>>,
    is_searching: Cell<bool>,
    extend_client_area_enabled: Cell<bool>,
    title_bar_height: Cell<f64>,
    is_system_bar_visible: Cell<bool>,
    display_edge_to_edge: Cell<bool>,
    safe_area_padding: Cell<Thickness>,
    can_resize: Cell<bool>,
    can_minimize: Cell<bool>,
    can_maximize: Cell<bool>,
    decorations_theme: RefCell<Option<Ref<ControlTheme>>>,
    title_bar_decorations: Cell<TitleBarDecorations>,
    current_page_item: RefCell<Option<Rc<PageItem>>>,
    /// The number of navigations asked for, so that one that waited for the assets of its page
    /// can tell whether another one was asked for meanwhile (not in upstream).
    navigation_request: Cell<u64>,
    is_drawer_opened: Cell<bool>,
    display_mode: Cell<SplitViewDisplayMode>,
    query: RefCell<Option<String>>,
    validated_date_example: Cell<Option<DateTime>>,
    about_command: Rc<MiniCommand>,
    exit_command: Rc<MiniCommand>,
    navigate_to_page_command: Rc<MiniCommand>,
    settings_command: Rc<MiniCommand>,
    home_command: Rc<MiniCommand>,
    this: Weak<MainWindowViewModel>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

fn icon(path: &str) -> Ref<StreamGeometry> {
    StreamGeometry::parse(path).unwrap_or_else(|e| panic!("invalid icon path data: {e}"))
}

impl MainWindowViewModel {
    pub fn new() -> Rc<MainWindowViewModel> {
        let settings_view_model = SettingsViewModel::new();
        let (page_sections, unavailable_pages) = page_sections();

        let this = Rc::new_cyclic(|this: &Weak<MainWindowViewModel>| {
            let about_command = MiniCommand::create_from_task(|| async {
                let dialog = AboutFerroDialog::new();

                let lifetime = Application::current().and_then(|app| app.application_lifetime());
                let main_window =
                    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|d| d.main_window());
                if let Some(main_window) = main_window {
                    let _ = dialog.show_dialog(&main_window).await;
                }
            });
            let exit_command = MiniCommand::create(|| {
                let lifetime = Application::current().and_then(|app| app.application_lifetime());
                if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
                    desktop.shutdown(0);
                }
            });
            let settings_item = {
                let settings_view_model = settings_view_model.clone();
                PageItem::new(
                    "Settings",
                    move || SettingsPage::with_settings_view_model(settings_view_model.clone()).upcast(),
                    icon(Icons::SETTINGS),
                    "Theme, transparency and window options",
                    None,
                    None,
                )
            };
            let navigate_to_page_command = {
                let this = this.clone();
                MiniCommand::create_with::<Rc<PageItem>>(move |item| {
                    if let Some(this) = this.upgrade() {
                        this.navigate_to_item(&item);
                    }
                })
            };
            let settings_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if this.is_current_page_item(&this.settings_item) {
                        return;
                    }

                    if this.navigator().is_some() {
                        this.navigate_to_item(&this.settings_item);
                    }
                })
            };
            let home_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if this.is_current_page_item(&this.home_item) {
                        return;
                    }

                    if this.navigator().is_some() {
                        this.navigate_to_item(&this.home_item);
                    }
                })
            };

            MainWindowViewModel {
                base: ViewModelBase::new(),
                settings_view_model,
                home_item: PageItem::new(
                    "Home",
                    || HomePage::new().upcast(),
                    icon(Icons::HOME),
                    "Overview of everything in the catalog",
                    None,
                    None,
                ),
                settings_item,
                page_sections,
                unavailable_pages,
                home_sections: OnceCell::new(),
                navigator: RefCell::new(None),
                is_searching: Cell::new(false),
                extend_client_area_enabled: Cell::new(false),
                title_bar_height: Cell::new(-1.0),
                is_system_bar_visible: Cell::new(false),
                display_edge_to_edge: Cell::new(false),
                safe_area_padding: Cell::new(Thickness::default()),
                can_resize: Cell::new(true),
                can_minimize: Cell::new(true),
                can_maximize: Cell::new(true),
                decorations_theme: RefCell::new(None),
                title_bar_decorations: Cell::new(TitleBarDecorations::ALL),
                current_page_item: RefCell::new(None),
                navigation_request: Cell::new(0),
                is_drawer_opened: Cell::new(true),
                display_mode: Cell::new(SplitViewDisplayMode::default()),
                query: RefCell::new(Some(String::new())),
                validated_date_example: Cell::new(None),
                about_command,
                exit_command,
                navigate_to_page_command,
                settings_command,
                home_command,
                this: this.clone(),
            }
        });

        this.filter(Some(""));
        this
    }

    pub fn settings_view_model(&self) -> Rc<SettingsViewModel> {
        self.settings_view_model.clone()
    }

    pub fn home_item(&self) -> Rc<PageItem> {
        self.home_item.clone()
    }

    pub fn settings_item(&self) -> Rc<PageItem> {
        self.settings_item.clone()
    }

    /// The sections of the home page. The home page doesn't have a section
    /// title and is excluded from this list.
    pub fn home_sections(&self) -> Rc<Vec<Rc<HomeSection>>> {
        self.home_sections
            .get_or_init(|| Rc::new(self.page_sections.iter().filter(|s| !s.title().is_empty()).cloned().collect()))
            .clone()
    }

    /// The pages of the upstream list that are left out of the sections,
    /// with what they wait for (see `main_window_view_model_page_list.rs`).
    pub fn unavailable_pages(&self) -> &[UnavailablePage] {
        &self.unavailable_pages
    }

    pub fn navigator(&self) -> Option<Rc<dyn INavigation>> {
        self.navigator.borrow().clone()
    }

    pub(crate) fn set_navigator(&self, value: Option<Rc<dyn INavigation>>) {
        *self.navigator.borrow_mut() = value;
    }

    pub fn extend_client_area_enabled(&self) -> bool {
        self.extend_client_area_enabled.get()
    }

    pub fn set_extend_client_area_enabled(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.extend_client_area_enabled, value, "ExtendClientAreaEnabled");
    }

    pub fn title_bar_height(&self) -> f64 {
        self.title_bar_height.get()
    }

    pub fn set_title_bar_height(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.title_bar_height, value, "TitleBarHeight");
    }

    pub fn is_system_bar_visible(&self) -> bool {
        self.is_system_bar_visible.get()
    }

    pub fn set_is_system_bar_visible(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_system_bar_visible, value, "IsSystemBarVisible");
    }

    pub fn display_edge_to_edge(&self) -> bool {
        self.display_edge_to_edge.get()
    }

    pub fn set_display_edge_to_edge(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.display_edge_to_edge, value, "DisplayEdgeToEdge");
    }

    pub fn safe_area_padding(&self) -> Thickness {
        self.safe_area_padding.get()
    }

    pub fn set_safe_area_padding(&self, value: Thickness) {
        self.base.raise_and_set_if_changed_cell(&self.safe_area_padding, value, "SafeAreaPadding");
    }

    pub fn can_resize(&self) -> bool {
        self.can_resize.get()
    }

    pub fn set_can_resize(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.can_resize, value, "CanResize");
    }

    pub fn can_minimize(&self) -> bool {
        self.can_minimize.get()
    }

    pub fn set_can_minimize(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.can_minimize, value, "CanMinimize");
    }

    pub fn can_maximize(&self) -> bool {
        self.can_maximize.get()
    }

    pub fn set_can_maximize(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.can_maximize, value, "CanMaximize");
    }

    pub fn decorations_theme(&self) -> Option<Ref<ControlTheme>> {
        self.decorations_theme.borrow().clone()
    }

    pub fn set_decorations_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.base.raise_and_set_if_changed(&self.decorations_theme, value, "DecorationsTheme");
    }

    pub fn title_bar_decorations(&self) -> TitleBarDecorations {
        self.title_bar_decorations.get()
    }

    pub fn set_title_bar_decorations(&self, value: TitleBarDecorations) {
        self.base.raise_and_set_if_changed_cell(&self.title_bar_decorations, value, "TitleBarDecorations");
    }

    pub fn show_title(&self) -> bool {
        self.has_title_bar_decoration(TitleBarDecorations::TITLE)
    }

    pub fn set_show_title(&self, value: bool) {
        self.set_title_bar_decoration(TitleBarDecorations::TITLE, value, "ShowTitle");
    }

    pub fn show_full_screen_button(&self) -> bool {
        self.has_title_bar_decoration(TitleBarDecorations::FULL_SCREEN_BUTTON)
    }

    pub fn set_show_full_screen_button(&self, value: bool) {
        self.set_title_bar_decoration(TitleBarDecorations::FULL_SCREEN_BUTTON, value, "ShowFullScreenButton");
    }

    pub fn show_minimize_button(&self) -> bool {
        self.has_title_bar_decoration(TitleBarDecorations::MINIMIZE_BUTTON)
    }

    pub fn set_show_minimize_button(&self, value: bool) {
        self.set_title_bar_decoration(TitleBarDecorations::MINIMIZE_BUTTON, value, "ShowMinimizeButton");
    }

    pub fn show_maximize_button(&self) -> bool {
        self.has_title_bar_decoration(TitleBarDecorations::MAXIMIZE_BUTTON)
    }

    pub fn set_show_maximize_button(&self, value: bool) {
        self.set_title_bar_decoration(TitleBarDecorations::MAXIMIZE_BUTTON, value, "ShowMaximizeButton");
    }

    pub fn show_close_button(&self) -> bool {
        self.has_title_bar_decoration(TitleBarDecorations::CLOSE_BUTTON)
    }

    pub fn set_show_close_button(&self, value: bool) {
        self.set_title_bar_decoration(TitleBarDecorations::CLOSE_BUTTON, value, "ShowCloseButton");
    }

    pub fn current_page_item(&self) -> Option<Rc<PageItem>> {
        self.current_page_item.borrow().clone()
    }

    pub fn set_current_page_item(&self, value: Option<Rc<PageItem>>) {
        self.base.raise_and_set_if_changed(&self.current_page_item, value, "CurrentPageItem");
    }

    fn is_current_page_item(&self, item: &Rc<PageItem>) -> bool {
        self.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, item))
    }

    pub fn is_drawer_opened(&self) -> bool {
        self.is_drawer_opened.get()
    }

    pub fn set_is_drawer_opened(&self, value: bool) {
        if !self.base.raise_and_set_if_changed_cell(&self.is_drawer_opened, value, "IsDrawerOpened") {
            return;
        }

        // With the drawer shut the page list is hidden, so the section itself carries the marker.
        for section in &self.page_sections {
            section.set_is_expanded(
                value && if self.is_searching.get() { section.is_section_visible() } else { section.is_current() },
            );
        }
    }

    pub fn display_mode(&self) -> SplitViewDisplayMode {
        self.display_mode.get()
    }

    pub fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.base.raise_and_set_if_changed_cell(&self.display_mode, value, "DisplayMode");
    }

    pub fn query(&self) -> Option<String> {
        self.query.borrow().clone()
    }

    pub fn set_query(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.query, value.clone(), "Query");

        self.filter(value.as_deref());
    }

    fn has_title_bar_decoration(&self, decoration: TitleBarDecorations) -> bool {
        self.title_bar_decorations().intersects(decoration)
    }

    fn set_title_bar_decoration(&self, decoration: TitleBarDecorations, value: bool, property_name: &str) {
        let current = self.title_bar_decorations();
        let new_decorations = if value { current | decoration } else { current & !decoration };
        if new_decorations == current {
            return;
        }

        self.set_title_bar_decorations(new_decorations);
        self.base.raise_property_changed(property_name);
        self.base.raise_property_changed("TitleBarDecorations");
    }

    pub fn about_command(&self) -> Rc<MiniCommand> {
        self.about_command.clone()
    }

    pub fn exit_command(&self) -> Rc<MiniCommand> {
        self.exit_command.clone()
    }

    pub fn navigate_to_page_command(&self) -> Rc<MiniCommand> {
        self.navigate_to_page_command.clone()
    }

    pub fn settings_command(&self) -> Rc<MiniCommand> {
        self.settings_command.clone()
    }

    pub fn home_command(&self) -> Rc<MiniCommand> {
        self.home_command.clone()
    }

    /// A required date which demonstrates validation for the date picker.
    pub fn validated_date_example(&self) -> Option<DateTime> {
        self.validated_date_example.get()
    }

    pub fn set_validated_date_example(&self, value: Option<DateTime>) {
        self.base.raise_and_set_if_changed_cell(&self.validated_date_example, value, "ValidatedDateExample");
    }

    pub fn navigate_to_item(&self, item: &Rc<PageItem>) {
        if let Some(this) = self.this.upgrade() {
            drop(start_async(Self::navigate_to_async(this, Some(item.clone()))));
        }
    }

    async fn navigate_to_async(this: Rc<MainWindowViewModel>, item: Option<Rc<PageItem>>) {
        let (Some(item), Some(navigator)) = (item, this.navigator()) else { return };
        let request = this.navigation_request.get().wrapping_add(1);
        this.navigation_request.set(request);

        // A gallery may have pushed a sample on top of its page. Drop it first: replacing the top page
        // would leave the previous page below the new one, and the shell would show a back button that
        // leads to a page the drawer no longer selects.
        if navigator.stack_depth() > 1 {
            let _ = navigator.pop_to_root_async_with_transition(None).await;
        }

        if !this.is_current_page_item(&item) {
            // Not in upstream: the host may have to fetch the assets of the page before the page can
            // be created (see `PageAssets`). Of navigations asked for while one waits, the last wins.
            if let Some(assets) = PageAssets::ensure(&item.header()) {
                if let Err(error) = assets.await {
                    println!("The assets of the page {} are not available: {error}", item.header());
                    return;
                }
                if this.navigation_request.get() != request {
                    return;
                }
            }

            let page = item.create_page();
            this.set_current_page_item(Some(item.clone()));

            for section in &this.page_sections {
                // A section's own page counts too, so the section stays open when the drawer comes back.
                let is_of_section = Rc::ptr_eq(&section.page_item(), &item) || section.title() == item.section();
                section.set_current_page(if is_of_section { Some(item.clone()) } else { None });

                if section.is_current() && this.is_drawer_opened() {
                    section.set_is_expanded(true);
                }
            }

            let _ = navigator.replace_async(page).await;

            if matches!(this.display_mode(), SplitViewDisplayMode::CompactOverlay | SplitViewDisplayMode::Overlay) {
                this.set_is_drawer_opened(false);
            }
        }
    }

    /// Shows the pages that match `query` (all of them for an empty query)
    /// and opens the sections that hold them.
    ///
    /// The managed original visits the pages ordered by header with the
    /// comparison of the current culture; here the order is the ordinal one
    /// (it only decides the order of the change notifications).
    pub fn filter(&self, query: Option<&str>) {
        // Left panel items are sorted alphabetically
        let mut all_pages: Vec<Rc<PageItem>> =
            self.page_sections.iter().flat_map(|cat| cat.items().map(|items| items.to_vec()).unwrap_or_default()).collect();
        all_pages.sort_by_key(|p| p.header());

        let query_search_key = query.map(|query| PageItem::create_search_key(&[query])).unwrap_or_default();
        let is_default_visible =
            query.is_none_or(|query| query.trim().is_empty()) || query_search_key.trim().is_empty();
        self.is_searching.set(!is_default_visible);

        for page in &all_pages {
            page.set_is_visible(is_default_visible);
        }

        if !query_search_key.trim().is_empty() {
            for item in &all_pages {
                if item.matches_search(&query_search_key) {
                    item.set_is_visible(true);
                }
            }
        }

        for section in &self.page_sections {
            section.set_is_expanded(if is_default_visible { section.is_current() } else { section.is_section_visible() });
        }
    }
}

type Vm = Rc<MainWindowViewModel>;

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        SettingsViewModel: Rc<SettingsViewModel> { get: |this: &Vm| this.settings_view_model() },
        HomeItem: Rc<PageItem> { get: |this: &Vm| this.home_item() },
        SettingsItem: Rc<PageItem> { get: |this: &Vm| this.settings_item() },
        // A list a binding delivers to an items source property.
        HomeSections: ItemsSource { get: |this: &Vm| ItemsSource::from_values(this.home_sections().iter().cloned()) },
        ExtendClientAreaEnabled: bool {
            get: |this: &Vm| this.extend_client_area_enabled(),
            set: |this: &Vm, value: bool| this.set_extend_client_area_enabled(value)
        },
        TitleBarHeight: f64 {
            get: |this: &Vm| this.title_bar_height(),
            set: |this: &Vm, value: f64| this.set_title_bar_height(value)
        },
        IsSystemBarVisible: bool {
            get: |this: &Vm| this.is_system_bar_visible(),
            set: |this: &Vm, value: bool| this.set_is_system_bar_visible(value)
        },
        DisplayEdgeToEdge: bool {
            get: |this: &Vm| this.display_edge_to_edge(),
            set: |this: &Vm, value: bool| this.set_display_edge_to_edge(value)
        },
        SafeAreaPadding: Thickness {
            get: |this: &Vm| this.safe_area_padding(),
            set: |this: &Vm, value: Thickness| this.set_safe_area_padding(value)
        },
        CanResize: bool { get: |this: &Vm| this.can_resize(), set: |this: &Vm, value: bool| this.set_can_resize(value) },
        CanMinimize: bool {
            get: |this: &Vm| this.can_minimize(),
            set: |this: &Vm, value: bool| this.set_can_minimize(value)
        },
        CanMaximize: bool {
            get: |this: &Vm| this.can_maximize(),
            set: |this: &Vm, value: bool| this.set_can_maximize(value)
        },
        DecorationsTheme: Option<Ref<ControlTheme>> {
            get: |this: &Vm| this.decorations_theme(),
            set: |this: &Vm, value: Option<Ref<ControlTheme>>| this.set_decorations_theme(value)
        },
        TitleBarDecorations: TitleBarDecorations {
            get: |this: &Vm| this.title_bar_decorations(),
            set: |this: &Vm, value: TitleBarDecorations| this.set_title_bar_decorations(value)
        },
        ShowTitle: bool { get: |this: &Vm| this.show_title(), set: |this: &Vm, value: bool| this.set_show_title(value) },
        ShowFullScreenButton: bool {
            get: |this: &Vm| this.show_full_screen_button(),
            set: |this: &Vm, value: bool| this.set_show_full_screen_button(value)
        },
        ShowMinimizeButton: bool {
            get: |this: &Vm| this.show_minimize_button(),
            set: |this: &Vm, value: bool| this.set_show_minimize_button(value)
        },
        ShowMaximizeButton: bool {
            get: |this: &Vm| this.show_maximize_button(),
            set: |this: &Vm, value: bool| this.set_show_maximize_button(value)
        },
        ShowCloseButton: bool {
            get: |this: &Vm| this.show_close_button(),
            set: |this: &Vm, value: bool| this.set_show_close_button(value)
        },
        CurrentPageItem: Option<Rc<PageItem>> {
            get: |this: &Vm| this.current_page_item(),
            set: |this: &Vm, value: Option<Rc<PageItem>>| this.set_current_page_item(value)
        },
        IsDrawerOpened: bool {
            get: |this: &Vm| this.is_drawer_opened(),
            set: |this: &Vm, value: bool| this.set_is_drawer_opened(value)
        },
        DisplayMode: SplitViewDisplayMode {
            get: |this: &Vm| this.display_mode(),
            set: |this: &Vm, value: SplitViewDisplayMode| this.set_display_mode(value)
        },
        Query: Option<String> { get: |this: &Vm| this.query(), set: |this: &Vm, value: Option<String>| this.set_query(value) },
        ValidatedDateExample: Option<DateTime> {
            get: |this: &Vm| this.validated_date_example(),
            set: |this: &Vm, value: Option<DateTime>| this.set_validated_date_example(value)
        },
        AboutCommand: Rc<dyn ICommand> { get: |this: &Vm| -> Rc<dyn ICommand> { this.about_command() } },
        ExitCommand: Rc<dyn ICommand> { get: |this: &Vm| -> Rc<dyn ICommand> { this.exit_command() } },
        NavigateToPageCommand: Rc<dyn ICommand> {
            get: |this: &Vm| -> Rc<dyn ICommand> { this.navigate_to_page_command() }
        },
        SettingsCommand: Rc<dyn ICommand> { get: |this: &Vm| -> Rc<dyn ICommand> { this.settings_command() } },
        HomeCommand: Rc<dyn ICommand> { get: |this: &Vm| -> Rc<dyn ICommand> { this.home_command() } },
    ],
    methods: [
        fn NavigateToItem(Rc<PageItem>) => |this: &Vm, item: Rc<PageItem>| this.navigate_to_item(&item),
        fn Filter(Option<String>) => |this: &Vm, query: Option<String>| this.filter(query.as_deref()),
    ],
    notify_property_changed: MainWindowViewModel,
});
