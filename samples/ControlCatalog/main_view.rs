//! Port of `MainView.xaml.cs`: the class of the document `MainView.xaml`.

use crate::markup::xaml_class;
use crate::models::{HomeSection, PageItem};
use crate::view_models::MainWindowViewModel;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::{InputElementImpl, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, TypeInfo, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_base::Thickness;
use ferroui_controls::navigation::NavigationMode;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    box_item, unbox_item, Application, AutoSuggestBox, AutoSuggestionBoxTextChangeReason, ContentPage, Control,
    ControlImpl, Frame, FramePage, IconElement, ItemsSource, NavigationPage, NavigationView, NavigationViewDisplayMode,
    NavigationViewItem, Page,
    PageImpl, PathIcon, TopLevel,
};
use mini_mvvm::{delay, start_async};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The height of the row of the toggle and back buttons of a minimal pane, above the content.
const MINIMAL_CONTENT_TOP: f64 = 48.0;

/// The main view of the catalog: a page whose content is a navigation view. Its pane lists the
/// home page, the sections with their pages and the settings page; its frame shows them, each page
/// in a navigation page of its own so a gallery can push samples on it. Not as in upstream, whose
/// main view is a drawer page (see `MainView.xaml`).
#[repr(C)]
pub struct MainView {
    base: ContentPage,
    /// The items of the pane and the pages they stand for: the home page, the sections (their own
    /// pages) and the pages of the sections.
    entries: RefCell<Vec<(Ref<NavigationViewItem>, Rc<PageItem>)>>,
    /// The items of the sections, to open and hide them with the sections of the view model.
    section_items: RefCell<Vec<(Ref<NavigationViewItem>, Rc<HomeSection>)>>,
    /// The page item and the page the next navigation of the frame shows.
    pending: RefCell<Option<(Rc<PageItem>, Ref<Page>)>>,
    /// Set while the pane follows the view model, so that its selection does not navigate.
    following: Cell<bool>,
    /// Set while the pane navigates to a selected item: the navigation view toggles the expansion of
    /// the item after the selection, so the section is not opened for it first.
    selecting: Cell<bool>,
    built: Cell<bool>,
}

ferro_class!(MainView: ContentPage);
ferro_impl_classes!(MainView: FerroObjectImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, PageImpl);
ferro_class_info!(MainView {
    new: MainView::new,
    markup: {
        properties: [
            ViewModel: Rc<MainWindowViewModel> { get: |this: &Ref<MainView>| this.view_model() },
        ],
        methods: [
            fn FerroIcon_OnTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TappedEventArgs>() {
                        this.ferro_icon_on_tapped(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(MainView, "/MainView.xaml");

impl StyledElementImpl for MainView {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        MainView::TYPE
    }

    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        if let Some(view_model) = this.try_view_model() {
            this.set_up_shell(&view_model);
            view_model.navigate_to_item(&view_model.home_item());
        }
    }
}

impl VisualImpl for MainView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.data_context().is_none() {
            return;
        }

        let top_level = TopLevel::get_top_level(Some(this)).expect("the view is attached to a top level");

        let insets = top_level.insets_manager();
        if let Some(insets) = insets {
            let view_model = this.view_model();
            // In real life application these events should be unsubscribed to avoid memory leaks.
            view_model.set_safe_area_padding(insets.safe_area_padding());
            {
                let (view_model, insets_for_handler) = (view_model.clone(), insets.clone());
                insets.safe_area_changed(Rc::new(move |_| {
                    view_model.set_safe_area_padding(insets_for_handler.safe_area_padding());
                }));
            }

            view_model.set_display_edge_to_edge(insets.display_edge_to_edge_preference());
            view_model.set_is_system_bar_visible(insets.is_system_bar_visible().unwrap_or(true));

            let weak_view_model = Rc::downgrade(&view_model);
            view_model.property_changed().add(Rc::new(move |property_name: &str| {
                let Some(view_model) = weak_view_model.upgrade() else { return };
                let insets = insets.clone();
                let property_name = property_name.to_string();
                drop(start_async(async move {
                    if property_name == "DisplayEdgeToEdge" {
                        insets.set_display_edge_to_edge_preference(view_model.display_edge_to_edge());
                    } else if property_name == "IsSystemBarVisible" {
                        insets.set_is_system_bar_visible(Some(view_model.is_system_bar_visible()));
                    }

                    // Give the OS some time to apply new values and refresh the view model.
                    delay(Duration::from_millis(100)).await;
                    view_model.set_display_edge_to_edge(insets.display_edge_to_edge_preference());
                    view_model.set_is_system_bar_visible(insets.is_system_bar_visible().unwrap_or(true));
                }));
            }));
        }
    }
}

impl MainView {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            entries: RefCell::new(Vec::new()),
            section_items: RefCell::new(Vec::new()),
            pending: RefCell::new(None),
            following: Cell::new(false),
            selecting: Cell::new(false),
            built: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let view = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(view) = view.upgrade() {
                view.main_view_loaded();
            }
        });
        this
    }

    /// The navigation view of the shell.
    pub fn navigation_view(&self) -> Ref<NavigationView> {
        self.get_control::<NavigationView>("Navigation")
    }

    /// The frame that shows the pages.
    pub fn content_frame(&self) -> Ref<Frame> {
        self.get_control::<Frame>("ContentFrame")
    }

    /// The navigation page of the page the frame shows: its root is the catalog page.
    pub fn current_navigation_page(&self) -> Option<Ref<NavigationPage>> {
        let page = Control::from_boxed(&self.content_frame().content()?)?;
        let page = page.cast::<FramePage>()?;
        Control::from_boxed(&page.content()?)?.cast::<NavigationPage>()
    }

    /// The item of the pane for `item`.
    pub fn entry_of(&self, item: &Rc<PageItem>) -> Option<Ref<NavigationViewItem>> {
        if Rc::ptr_eq(item, &self.view_model().settings_item()) {
            return Some(self.navigation_view().settings_item());
        }
        self.entries.borrow().iter().find(|(_, page)| Rc::ptr_eq(page, item)).map(|(entry, _)| entry.clone())
    }

    fn page_item_of(&self, entry: &Ref<NavigationViewItem>, view_model: &MainWindowViewModel) -> Option<Rc<PageItem>> {
        if *entry == self.navigation_view().settings_item() {
            return Some(view_model.settings_item());
        }
        self.entries.borrow().iter().find(|(item, _)| item == entry).map(|(_, page)| page.clone())
    }

    fn page_item_by_header(&self, header: &str, view_model: &MainWindowViewModel) -> Option<Rc<PageItem>> {
        if header == view_model.settings_item().header() {
            return Some(view_model.settings_item());
        }
        self.entries.borrow().iter().find(|(_, page)| page.header() == header).map(|(_, page)| page.clone())
    }

    fn entry(item: &Rc<PageItem>, icon: Option<Ref<ferroui_base::media::StreamGeometry>>) -> Ref<NavigationViewItem> {
        let entry = NavigationViewItem::new();
        entry.set_content(Some(Rc::new(item.header()) as BoxedValue));
        if let Some(icon) = icon {
            let path = PathIcon::new();
            path.set_data(icon);
            entry.set_icon(Some(path.upcast::<IconElement>()));
        }
        entry
    }

    /// Builds the items of the pane from the page list of the view model and connects the pane, the
    /// search box and the frame with it.
    fn set_up_shell(&self, view_model: &Rc<MainWindowViewModel>) {
        let navigation = self.navigation_view();
        let frame = self.content_frame();
        if !self.built.replace(true) {
            let home = view_model.home_item();
            let home_entry = Self::entry(&home, home.icon_data());
            navigation.menu_items().add(Some(Control::boxed(&home_entry)));
            self.entries.borrow_mut().push((home_entry, home));
            for section in view_model.home_sections().iter() {
                let section_entry = Self::entry(&section.page_item(), Some(section.icon_data()));
                for item in section.items().unwrap_or_default().iter() {
                    let entry = Self::entry(item, item.icon_data());
                    entry.set_is_visible(item.is_visible());
                    let weak_entry = entry.downgrade();
                    let weak_item = Rc::downgrade(item);
                    item.property_changed().add(Rc::new(move |name: &str| {
                        if let (Some(entry), Some(item), "IsVisible") = (weak_entry.upgrade(), weak_item.upgrade(), name) {
                            entry.set_is_visible(item.is_visible());
                        }
                    }));
                    section_entry.menu_items().add(Some(Control::boxed(&entry)));
                    self.entries.borrow_mut().push((entry, item.clone()));
                }
                section_entry.set_is_visible(section.is_section_visible());
                section_entry.set_is_expanded(section.is_expanded());
                let weak_entry = section_entry.downgrade();
                let weak_section = Rc::downgrade(section);
                let weak = self.to_ref().downgrade();
                section.property_changed().add(Rc::new(move |name: &str| {
                    let (Some(entry), Some(section)) = (weak_entry.upgrade(), weak_section.upgrade()) else { return };
                    match name {
                        "IsSectionVisible" => entry.set_is_visible(section.is_section_visible()),
                        "IsExpanded" if !weak.upgrade().is_some_and(|this| this.selecting.get()) => {
                            entry.set_is_expanded(section.is_expanded())
                        }
                        _ => {}
                    }
                }));
                navigation.menu_items().add(Some(Control::boxed(&section_entry)));
                self.section_items.borrow_mut().push((section_entry.clone(), section.clone()));
                self.entries.borrow_mut().push((section_entry, section.page_item()));
            }
            let settings = navigation.settings_item();
            settings.set_content(Some(Rc::new(view_model.settings_item().header()) as BoxedValue));

            // The pane: a selection navigates to the page of its item.
            let weak = self.to_ref().downgrade();
            navigation.selection_changed(move |args| {
                let Some(this) = weak.upgrade() else { return };
                if this.following.get() {
                    return;
                }
                let view_model = this.view_model();
                let item = if args.is_settings_selected() {
                    Some(view_model.settings_item())
                } else {
                    args.selected_item_container()
                        .and_then(|container| container.cast::<NavigationViewItem>())
                        .and_then(|entry| this.page_item_of(&entry, &view_model))
                };
                if let Some(item) = item {
                    this.selecting.set(true);
                    view_model.navigate_to_item(&item);
                    this.selecting.set(false);
                }
            });
            // The view model opens the section of the current page while the pane is open.
            let weak = self.to_ref().downgrade();
            navigation.pane_opened(move || {
                if let Some(this) = weak.upgrade() {
                    this.open_current_section();
                }
            });
            let weak = self.to_ref().downgrade();
            navigation.pane_closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.view_model().set_is_drawer_opened(false);
                }
            });
            // An opened or closed section is the state of its section.
            let weak = self.to_ref().downgrade();
            navigation.expanding(move |args| {
                let Some(this) = weak.upgrade() else { return };
                let container = args.expanding_item_container();
                let sections = this.section_items.borrow().clone();
                for (entry, section) in sections {
                    if entry == container {
                        section.set_is_expanded(true);
                    }
                }
            });
            let weak = self.to_ref().downgrade();
            navigation.collapsed(move |args| {
                let Some(this) = weak.upgrade() else { return };
                let container = args.collapsed_item_container();
                let sections = this.section_items.borrow().clone();
                for (entry, section) in sections {
                    if entry == container {
                        section.set_is_expanded(false);
                    }
                }
            });
            // The back button: a sample pushed on the page first, then the journal of the frame.
            let weak = self.to_ref().downgrade();
            navigation.back_requested(move |_| {
                let Some(this) = weak.upgrade() else { return };
                if let Some(page) = this.current_navigation_page().filter(|page| page.stack_depth() > 1) {
                    drop(page.pop_async());
                } else {
                    let _ = this.content_frame().go_back();
                }
            });
            // The search box filters the pages and opens the one chosen.
            let search = self.get_control::<AutoSuggestBox>("SearchBox");
            let weak = self.to_ref().downgrade();
            search.text_changed(move |args| {
                let Some(this) = weak.upgrade() else { return };
                if args.reason() != AutoSuggestionBoxTextChangeReason::UserInput {
                    return;
                }
                let search = this.get_control::<AutoSuggestBox>("SearchBox");
                let view_model = this.view_model();
                let text = search.text();
                view_model.set_query(Some(text.clone()));
                let matches: Vec<String> = if text.trim().is_empty() {
                    Vec::new()
                } else {
                    view_model
                        .home_sections()
                        .iter()
                        .flat_map(|section| section.items().unwrap_or_default().to_vec())
                        .filter(|item| item.is_visible())
                        .map(|item| item.header())
                        .collect()
                };
                search.set_items_source(Some(ItemsSource::from_values(matches)));
            });
            let weak = self.to_ref().downgrade();
            search.query_submitted(move |args| {
                let Some(this) = weak.upgrade() else { return };
                let view_model = this.view_model();
                let header = unbox_item::<String>(&args.chosen_suggestion()).or_else(|| {
                    // Enter without a chosen suggestion opens the first page that matches.
                    view_model
                        .home_sections()
                        .iter()
                        .flat_map(|section| section.items().unwrap_or_default().to_vec())
                        .find(|item| item.is_visible() && !args.query_text().trim().is_empty())
                        .map(|item| item.header())
                });
                if let Some(item) = header.and_then(|header| this.page_item_by_header(&header, &view_model)) {
                    view_model.navigate_to_item(&item);
                }
            });

            // The frame: a new entry shows the page the view model made, an entry of the journal
            // gets a new page of its item.
            let weak = self.to_ref().downgrade();
            frame.navigated(move |args| {
                let Some(this) = weak.upgrade() else { return };
                let view_model = this.view_model();
                let Some(frame_page) = args.content() else { return };
                let header = unbox_item::<String>(&args.parameter()).unwrap_or_default();
                let page = match this.pending.borrow_mut().take() {
                    Some((item, page)) if args.navigation_mode() == NavigationMode::New => Some((item, page)),
                    _ => this.page_item_by_header(&header, &view_model).map(|item| {
                        view_model.note_current_page(&item);
                        let page = item.create_page();
                        (item, page)
                    }),
                };
                let Some((item, page)) = page else { return };
                let navigation_page = NavigationPage::new();
                drop(navigation_page.push_async(page));
                frame_page.set_content(Some(Control::boxed(&navigation_page)));
                let weak = this.downgrade();
                navigation_page.pushed(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.update_back_button();
                    }
                });
                let weak = this.downgrade();
                navigation_page.popped(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.update_back_button();
                    }
                });
                view_model.set_navigator(Some(navigation_page.as_navigation()));
                this.select(&item);
                this.update_back_button();
            });
        }

        view_model.set_is_drawer_opened(navigation.is_pane_open());
        let weak = self.to_ref().downgrade();
        navigation.display_mode_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.view_model().set_is_drawer_opened(this.navigation_view().is_pane_open());
                this.update_content_margin();
            }
        });
        self.update_content_margin();
        // The view model shows its pages in the frame.
        let weak = self.to_ref().downgrade();
        view_model.set_page_host(Some(Rc::new(move |item: &Rc<PageItem>, page: Ref<Page>| {
            let Some(this) = weak.upgrade() else { return };
            *this.pending.borrow_mut() = Some((item.clone(), page));
            let _ = this.content_frame().navigate(FramePage::TYPE, box_item(&item.header()));
        })));
        // A page opened elsewhere (a card of the home page, a smoke run) selects its item.
        let weak = self.to_ref().downgrade();
        view_model.property_changed().add(Rc::new(move |name: &str| {
            if name != "CurrentPageItem" {
                return;
            }
            let Some(this) = weak.upgrade() else { return };
            if let Some(item) = this.view_model().current_page_item() {
                this.select(&item);
            }
        }));
    }

    /// Selects the item of `item` in the pane without navigating; the open pane shows the section
    /// of the page open.
    fn select(&self, item: &Rc<PageItem>) {
        let Some(entry) = self.entry_of(item) else { return };
        let navigation = self.navigation_view();
        if navigation.is_pane_open() && !self.selecting.get() {
            self.open_current_section();
        }
        self.following.set(true);
        navigation.set_selected_item(Some(Control::boxed(&entry)));
        self.following.set(false);
    }

    /// Opens the section of the current page, as the open pane shows it.
    fn open_current_section(&self) {
        let view_model = self.view_model();
        view_model.set_is_drawer_opened(true);
        let sections = self.section_items.borrow().clone();
        for (entry, section) in sections {
            if section.is_current() && section.items().is_some_and(|items| !items.is_empty()) {
                section.set_is_expanded(true);
                entry.set_is_expanded(true);
            }
        }
    }

    /// The minimal pane leaves the toggle and back buttons over the top left of the content (the shell
    /// shows no header of the navigation view): the frame starts below them.
    fn update_content_margin(&self) {
        let minimal = self.navigation_view().display_mode() == NavigationViewDisplayMode::Minimal;
        let top = if minimal { MINIMAL_CONTENT_TOP } else { 0.0 };
        self.content_frame().set_margin(Thickness::new(0.0, top, 0.0, 0.0));
    }

    fn update_back_button(&self) {
        let pushed = self.current_navigation_page().is_some_and(|page| page.stack_depth() > 1);
        self.navigation_view().set_is_back_enabled(pushed || self.content_frame().can_go_back());
    }

    fn main_view_loaded(&self) {
        if self.data_context().is_none() {
            return;
        }

        self.view_model().set_is_drawer_opened(self.navigation_view().is_pane_open());
        if let Some(app) = Application::current() {
            // Not in the original: a start option can select another variant than Default.
            let variant = app.cast::<crate::App>().map_or_else(ThemeVariant::default, |_| crate::App::initial_theme_variant());
            app.set_requested_theme_variant(Some(variant));
        }
    }

    fn try_view_model(&self) -> Option<Rc<MainWindowViewModel>> {
        from_markup_value::<Rc<MainWindowViewModel>>(&self.data_context())
    }

    /// `ViewModel`: the data context as the view model of the main window.
    ///
    /// # Panics
    /// Panics if the data context is not that view model (the invalid cast
    /// of the managed original).
    pub(crate) fn view_model(&self) -> Rc<MainWindowViewModel> {
        self.try_view_model().expect("the data context of the main view is a MainWindowViewModel")
    }

    fn ferro_icon_on_tapped(&self, _sender: &Option<BoxedValue>, _e: &TappedEventArgs) {
        let top_level = TopLevel::get_top_level(Some(self)).expect("the view is attached to a top level");
        let launcher = top_level.launcher();
        drop(start_async(async move {
            if let Ok(uri) = Uri::absolute("https://github.com/wieslawsoltes/FerroUI") {
                launcher.launch_uri_async(&uri).await;
            }
        }));
    }
}

