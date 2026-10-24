//! Port of `App.xaml.cs`: the class of the document `App.xaml`.

use crate::main_view::MainView;
use crate::main_window::MainWindow;
use crate::markup::xaml_class;
use crate::models::CatalogTheme;
use crate::view_models::{ApplicationViewModel, MainWindowViewModel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{IStyle, Style, Styles};
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
};
use ferroui_controls::application_lifetimes::IActivatableLifetime;
use ferroui_controls::{
    Application, ApplicationImpl, ApplicationImplExt, Control, ItemsSource, NativeDock, NativeMenuItem, NewApplication,
    Page, PageNavigationHost, Window,
};
#[cfg(feature = "classic-themes")]
use ferroui_themes_fluent::FluentTheme;
#[cfg(feature = "classic-themes")]
use ferroui_themes_simple::SimpleTheme;
use ferroui_themes_winui::WinUITheme;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The application of the catalog.
#[repr(C)]
pub struct App {
    base: Application,
    theme_styles_container: Ref<Styles>,
    /// The themes of the original, with the feature `classic-themes` only (`ClassicThemes.xaml`).
    #[cfg(feature = "classic-themes")]
    fluent_theme: RefCell<Option<Ref<FluentTheme>>>,
    #[cfg(feature = "classic-themes")]
    simple_theme: RefCell<Option<Ref<SimpleTheme>>>,
    /// The WinUI theme of the port (`CatalogTheme::WinUI`), not in the original.
    winui_theme: RefCell<Option<Ref<WinUITheme>>>,
    color_picker_fluent: RefCell<Option<Rc<dyn IStyle>>>,
    #[cfg(feature = "classic-themes")]
    color_picker_simple: RefCell<Option<Rc<dyn IStyle>>>,
    tree_data_grid_fluent: RefCell<Option<Rc<dyn IStyle>>>,
    /// The WinUI styles of the inking controls (`ferroui-controls-inking`), not in the original.
    inking_winui: RefCell<Option<Rc<dyn IStyle>>>,
    #[cfg(feature = "classic-themes")]
    tree_data_grid_generic: RefCell<Option<Rc<dyn IStyle>>>,
    dock_menu_item_count: Cell<i32>,
    prev_theme: Cell<CatalogTheme>,
}

ferro_class!(App: Application);
ferro_class_info!(App {
    new: App::new,
    markup: {
        methods: [
            fn OnDockNewWindowClicked(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<App>, sender: Option<BoxedValue>, e: EventArgs| this.on_dock_new_window_clicked(&sender, &e),
            fn OnDockShowMainWindowClicked(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<App>, sender: Option<BoxedValue>, e: EventArgs| {
                    this.on_dock_show_main_window_clicked(&sender, &e)
                },
            fn OnDockAddItemClicked(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<App>, sender: Option<BoxedValue>, e: EventArgs| this.on_dock_add_item_clicked(&sender, &e),
        ],
        fields: [
            CurrentTheme: CatalogTheme => App::current_theme,
            AvailableThemes: ItemsSource => App::available_themes,
        ],
    },
});
ferro_impl_classes!(App: FerroObjectImpl);
xaml_class!(App, "/App.xaml");

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        this.styles().add(this.theme_styles_container.clone());

        this.load_document();

        // The resources of the themes of the original (`ClassicThemes.xaml`), which `App.xaml` of
        // the original declares itself: with the feature `classic-themes` only. The dictionary is
        // merged into the resources of the application, and its entries are read from it
        // (`Resources[key]` reads the entries of a dictionary itself, not of the ones it merges).
        #[cfg(feature = "classic-themes")]
        {
            let classic_themes = crate::ClassicThemes::new().upcast::<ferroui_base::controls::ResourceDictionary>();
            let resource = |key: &str| classic_themes.try_get_value(&ResourceKey::String(key.into())).flatten();
            *this.fluent_theme.borrow_mut() = from_markup_value::<Ref<FluentTheme>>(&resource("FluentTheme"));
            *this.simple_theme.borrow_mut() = from_markup_value::<Ref<SimpleTheme>>(&resource("SimpleTheme"));
            *this.color_picker_simple.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&resource("ColorPickerSimple"));
            *this.tree_data_grid_generic.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&resource("TreeDataGridGeneric"));
            this.resources().merged_dictionaries().add(classic_themes.into());
        }
        *this.winui_theme.borrow_mut() = from_markup_value::<Ref<WinUITheme>>(&this.resource("WinUITheme"));
        *this.color_picker_fluent.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&this.resource("ColorPickerFluent"));
        *this.tree_data_grid_fluent.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&this.resource("TreeDataGridFluent"));
        *this.inking_winui.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&this.resource("InkingWinUI"));

        // The port starts the catalog in its WinUI theme (not in the original, which starts in the
        // Fluent theme); with the feature `classic-themes` the settings page and
        // `FERROUI_CATALOG_THEME` select the others.
        Self::set_catalog_themes(CatalogTheme::WinUI);
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop_lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            desktop_lifetime.set_main_window(Some(Self::create_main_window()));
        } else if let Some(single_view_factory_application_lifetime) =
            lifetime.as_ref().and_then(|l| l.as_activity_application_lifetime())
        {
            single_view_factory_application_lifetime.set_main_view_factory(Some(Rc::new(Self::create_main_view_host)));
        } else if let Some(single_view_lifetime) = lifetime.as_ref().and_then(|l| l.as_single_view_application_lifetime())
        {
            single_view_lifetime.set_main_view(Some(Self::create_main_view_host()));
        }

        if let Some(activatable_application_lifetime) = this.try_get::<dyn IActivatableLifetime>() {
            activatable_application_lifetime
                .activated(Rc::new(|args| println!("App activated: {:?}", args.kind())));
            activatable_application_lifetime
                .deactivated(Rc::new(|args| println!("App deactivated: {:?}", args.kind())));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
            theme_styles_container: Styles::new(),
            #[cfg(feature = "classic-themes")]
            fluent_theme: RefCell::new(None),
            #[cfg(feature = "classic-themes")]
            simple_theme: RefCell::new(None),
            winui_theme: RefCell::new(None),
            color_picker_fluent: RefCell::new(None),
            #[cfg(feature = "classic-themes")]
            color_picker_simple: RefCell::new(None),
            tree_data_grid_fluent: RefCell::new(None),
            inking_winui: RefCell::new(None),
            #[cfg(feature = "classic-themes")]
            tree_data_grid_generic: RefCell::new(None),
            dock_menu_item_count: Cell::new(0),
            prev_theme: Cell::new(CatalogTheme::WinUI),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_data_context(Some(ApplicationViewModel::new() as BoxedValue));
        this
    }

    /// The rooted asset path of the dictionary `App.xaml` merges.
    pub const CUSTOM_THEMES_PATH: &'static str = "/CustomThemes.xaml";

    /// `FerroXamlLoader.Load(this)`: populates the application from the
    /// compiled markup of `App.xaml`, which the build links to the dictionary
    /// it includes (`CustomThemes.xaml`) and to the styles of the colour picker.
    #[cfg(not(feature = "runtime-markup"))]
    pub(crate) fn load_document(&self) {
        crate::register_types();
        self.initialize_component();
    }

    /// `FerroXamlLoader.Load(this)`: populates the application from
    /// `App.xaml`, with the dictionary it includes (`CustomThemes.xaml`), with
    /// the run-time loader.
    #[cfg(feature = "runtime-markup")]
    pub(crate) fn load_document(&self) {
        use crate::markup::{describe, try_load_document_group};
        crate::register_types();
        // The styles of the colour picker `App.xaml` includes stay run-time includes of a
        // document loaded at run time, so the application makes the run-time loader the
        // loader of such documents; the include then loads the compiled markup of the
        // library through its loader table.
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
        let root: BoxedValue = Rc::new(self.to_ref());
        let result = try_load_document_group(Self::DOCUMENT_PATH, Some(root), &[Self::CUSTOM_THEMES_PATH]).map(|_| ());
        if let Err(error) = result {
            panic!("{}: {}", Self::DOCUMENT_PATH, describe(&error));
        }
    }

    /// `Resources[key]`.
    pub(crate) fn resource(&self, key: &str) -> Option<BoxedValue> {
        self.resources().try_get_value(&ResourceKey::String(key.into())).flatten()
    }

    /// `new MainWindow { DataContext = new MainWindowViewModel() }`.
    fn create_main_window() -> Ref<Window> {
        let window = MainWindow::new();
        window.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
        window.upcast()
    }

    /// `new PageNavigationHost { Page = new MainView { DataContext = new MainWindowViewModel() } }`.
    fn create_main_view_host() -> Ref<Control> {
        let page = MainView::new();
        page.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
        let host = PageNavigationHost::new();
        host.set_page(page.upcast::<Page>());
        host.upcast()
    }

    pub fn on_dock_new_window_clicked(&self, _sender: &Option<BoxedValue>, _e: &EventArgs) {
        let lifetime = self.application_lifetime();
        if lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).is_some() {
            let window = MainWindow::new();
            window.show();
        }
    }

    pub fn on_dock_show_main_window_clicked(&self, _sender: &Option<BoxedValue>, _e: &EventArgs) {
        let lifetime = self.application_lifetime();
        if let Some(desktop_lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            if let Some(main_window) = desktop_lifetime.main_window() {
                main_window.activate();
            }
        }
    }

    pub fn on_dock_add_item_clicked(&self, _sender: &Option<BoxedValue>, _e: &EventArgs) {
        let dock_menu = NativeDock::get_menu(self);
        if let Some(dock_menu) = dock_menu {
            self.dock_menu_item_count.set(self.dock_menu_item_count.get() + 1);
            let item = NativeMenuItem::with_header(&format!("New item {}", self.dock_menu_item_count.get()));
            {
                let dock_menu = dock_menu.downgrade();
                item.click(move |item| {
                    if let Some(dock_menu) = dock_menu.upgrade() {
                        dock_menu.items().remove(&item.to_ref().upcast());
                    }
                });
            }
            dock_menu.items().insert(0, item.upcast());
        }
    }

    /// The current application as the application of the catalog.
    fn current_app() -> Ref<App> {
        Application::current()
            .and_then(|application| application.cast::<App>())
            .expect("the current application is the application of the catalog")
    }

    /// `App.CurrentTheme`.
    pub fn current_theme() -> CatalogTheme {
        Self::current_app().prev_theme.get()
    }

    /// Not in the original: the themes the catalog is built with, the items of the theme list
    /// of the settings page. The WinUI theme alone; with the feature `classic-themes` the Fluent
    /// and Simple themes of the original before it, in the order of the original.
    pub fn available_themes() -> ItemsSource {
        ItemsSource::from_values(Self::themes().iter().copied())
    }

    /// The themes of [`available_themes`](Self::available_themes).
    pub fn themes() -> &'static [CatalogTheme] {
        #[cfg(feature = "classic-themes")]
        return &[CatalogTheme::Fluent, CatalogTheme::Simple, CatalogTheme::WinUI];
        #[cfg(not(feature = "classic-themes"))]
        return &[CatalogTheme::WinUI];
    }

    /// Not in the original: selects the theme of the catalog before its main window exists (the
    /// theme a desktop run asks for with `FERROUI_CATALOG_THEME`), so that no window is reopened.
    pub fn select_initial_catalog_theme(theme: CatalogTheme) {
        Self::current_app().prev_theme.set(theme);
        Self::set_catalog_themes(theme);
    }

    /// `App.SetCatalogThemes(theme)`.
    pub fn set_catalog_themes(theme: CatalogTheme) {
        let app = Self::current_app();
        let prev_theme = app.prev_theme.get();
        app.prev_theme.set(theme);
        let should_reopen_window = prev_theme != theme;

        if app.theme_styles_container.count() == 0 {
            app.theme_styles_container.add(Style::new());
            app.theme_styles_container.add(Style::new());
            app.theme_styles_container.add(Style::new());
            // Not in the original: the styles of the TreeDataGrid of the port (the page of the
            // TreeDataGrid demo).
            app.theme_styles_container.add(Style::new());
            // Not in the original: the WinUI styles of the inking controls of the port (the page
            // Inking); the inking controls have no Fluent or Simple styles.
            app.theme_styles_container.add(Style::new());
        }

        // The TreeDataGrid has Fluent styles, whose brushes come from resources the Fluent and the
        // WinUI themes define, and the generic styles they build on, which the Simple theme takes.
        let tree_data_grid_fluent = || app.tree_data_grid_fluent.borrow().clone().expect("the TreeDataGrid Fluent styles of App.xaml");
        #[cfg(feature = "classic-themes")]
        let tree_data_grid_generic =
            || app.tree_data_grid_generic.borrow().clone().expect("the TreeDataGrid generic styles of ClassicThemes.xaml");

        match theme {
            #[cfg(feature = "classic-themes")]
            CatalogTheme::Fluent => {
                let fluent_theme = app.fluent_theme.borrow().clone().expect("the Fluent theme of ClassicThemes.xaml");
                app.theme_styles_container.set(0, fluent_theme.as_style());
                let color_picker_fluent =
                    app.color_picker_fluent.borrow().clone().expect("the colour picker Fluent styles of App.xaml");
                app.theme_styles_container.set(1, color_picker_fluent);
                app.theme_styles_container.set(2, Style::new());
                app.theme_styles_container.set(3, tree_data_grid_fluent());
                app.theme_styles_container.set(4, Style::new());
            }
            #[cfg(feature = "classic-themes")]
            CatalogTheme::Simple => {
                let simple_theme = app.simple_theme.borrow().clone().expect("the Simple theme of ClassicThemes.xaml");
                app.theme_styles_container.set(0, simple_theme.as_style());
                let color_picker_simple =
                    app.color_picker_simple.borrow().clone().expect("the colour picker Simple styles of ClassicThemes.xaml");
                app.theme_styles_container.set(1, color_picker_simple);
                app.theme_styles_container.set(2, Style::new());
                app.theme_styles_container.set(3, tree_data_grid_generic());
                app.theme_styles_container.set(4, Style::new());
            }
            CatalogTheme::WinUI => {
                // Not in the original: the WinUI theme of the port (docs/porting/winui-theme.md),
                // which themes every control, with the Fluent styles of the colour picker (the
                // colour picker library has no WinUI styles; WinUI's ColorPicker is waived,
                // docs/porting/winui-theme.md, section 5.1, and the Fluent styles take their
                // brushes from the resources both themes define).
                let winui_theme = app.winui_theme.borrow().clone().expect("the WinUI theme of App.xaml");
                app.theme_styles_container.set(0, winui_theme.as_style());
                let color_picker_fluent =
                    app.color_picker_fluent.borrow().clone().expect("the colour picker Fluent styles of App.xaml");
                app.theme_styles_container.set(1, color_picker_fluent);
                app.theme_styles_container.set(2, Style::new());
                app.theme_styles_container.set(3, tree_data_grid_fluent());
                let inking_winui = app.inking_winui.borrow().clone().expect("the inking WinUI styles of App.xaml");
                app.theme_styles_container.set(4, inking_winui);
            }
            // Not in the original: the catalog is built without the themes of the original.
            #[cfg(not(feature = "classic-themes"))]
            CatalogTheme::Fluent | CatalogTheme::Simple => panic!(
                "the theme {theme:?} is not built into the catalog: it needs the feature `classic-themes` of control-catalog"
            ),
        }

        if should_reopen_window {
            let lifetime = app.application_lifetime();
            if let Some(desktop_lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime())
            {
                let old_window = desktop_lifetime.main_window();
                let new_window = Self::create_main_window();
                desktop_lifetime.set_main_window(Some(new_window.clone()));
                new_window.show();
                if let Some(old_window) = old_window {
                    old_window.close();
                }
            } else if let Some(single_view_factory_application_lifetime) =
                lifetime.as_ref().and_then(|l| l.as_activity_application_lifetime())
            {
                single_view_factory_application_lifetime
                    .set_main_view_factory(Some(Rc::new(Self::create_main_view_host)));
            } else if let Some(single_view_lifetime) =
                lifetime.as_ref().and_then(|l| l.as_single_view_application_lifetime())
            {
                single_view_lifetime.set_main_view(Some(Self::create_main_view_host()));
            }
        }
    }
}
