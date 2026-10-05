//! Port of `App.xaml.cs`: the class of the document `App.xaml`.
//!
//! Not ported: the branches of `OnFrameworkInitializationCompleted` and
//! `SetCatalogThemes` for the activity and single-view lifetimes (they
//! create a `MainView`, whose base class `DrawerPage` is not ported).
//!
//! TEMPORARY, each part marked where it is: while `App.xaml`,
//! `MainWindow.xaml` or the Fluent theme do not load, the application
//! starts with the largest subset that does (see [`crate::temporary`]).

use crate::main_window::MainWindow;
use crate::markup::{describe, try_load_document_group, xaml_class};
use crate::models::CatalogTheme;
use crate::temporary;
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
    Application, ApplicationImpl, ApplicationImplExt, NativeDock, NativeMenuItem, NewApplication, Window,
};
use ferroui_themes_fluent::FluentTheme;
use ferroui_themes_simple::SimpleTheme;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The application of the catalog.
#[repr(C)]
pub struct App {
    base: Application,
    theme_styles_container: Ref<Styles>,
    fluent_theme: RefCell<Option<Ref<FluentTheme>>>,
    simple_theme: RefCell<Option<Ref<SimpleTheme>>>,
    color_picker_fluent: RefCell<Option<Rc<dyn IStyle>>>,
    color_picker_simple: RefCell<Option<Rc<dyn IStyle>>>,
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
        fields: [CurrentTheme: CatalogTheme => App::current_theme],
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

        *this.fluent_theme.borrow_mut() = temporary::fluent_theme(this);
        *this.simple_theme.borrow_mut() = from_markup_value::<Ref<SimpleTheme>>(&this.resource("SimpleTheme"));
        *this.color_picker_fluent.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&this.resource("ColorPickerFluent"));
        *this.color_picker_simple.borrow_mut() = from_markup_value::<Rc<dyn IStyle>>(&this.resource("ColorPickerSimple"));

        // TEMPORARY: the managed original starts with the Fluent theme; while it does not load
        // the catalog starts with the Simple theme (`FERROUI_CATALOG_THEME=fluent|simple` asks for
        // one of them).
        let initial_theme = temporary::initial_theme(this.fluent_theme.borrow().is_some());
        this.prev_theme.set(initial_theme);
        Self::set_catalog_themes(initial_theme);
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop_lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            desktop_lifetime.set_main_window(Some(Self::create_main_window()));
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
            fluent_theme: RefCell::new(None),
            simple_theme: RefCell::new(None),
            color_picker_fluent: RefCell::new(None),
            color_picker_simple: RefCell::new(None),
            dock_menu_item_count: Cell::new(0),
            prev_theme: Cell::new(CatalogTheme::Fluent),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_data_context(Some(ApplicationViewModel::new() as BoxedValue));
        this
    }

    /// `FerroXamlLoader.Load(this)`: populates the application from
    /// `App.xaml`, with the dictionary it includes (`CustomThemes.xaml`).
    ///
    /// TEMPORARY: while `App.xaml` or `CustomThemes.xaml` is listed in
    /// `excluded.txt` the subset of them that loads is loaded instead.
    fn load_document(&self) {
        crate::register_types();
        let root: BoxedValue = Rc::new(self.to_ref());
        let result = if crate::excluded(Self::DOCUMENT_PATH).is_some()
            || crate::excluded(temporary::CUSTOM_THEMES_PATH).is_some()
        {
            temporary::load_app_document_subset(root)
        } else {
            try_load_document_group(Self::DOCUMENT_PATH, Some(root), &[temporary::CUSTOM_THEMES_PATH]).map(|_| ())
        };
        if let Err(error) = result {
            panic!("{}: {}", Self::DOCUMENT_PATH, describe(&error));
        }
    }

    /// `Resources[key]`.
    pub(crate) fn resource(&self, key: &str) -> Option<BoxedValue> {
        self.resources().try_get_value(&ResourceKey::String(key.into())).flatten()
    }

    /// `new MainWindow { DataContext = new MainWindowViewModel() }`.
    ///
    /// TEMPORARY: while `MainWindow.xaml` is listed in `excluded.txt` the
    /// window is the code-built shell of [`crate::temporary`].
    fn create_main_window() -> Ref<Window> {
        let view_model = MainWindowViewModel::new();
        if crate::excluded(MainWindow::DOCUMENT_PATH).is_some() {
            return temporary::shell::create_shell_window(view_model);
        }
        let window = MainWindow::new();
        window.set_data_context(Some(view_model as BoxedValue));
        window.upcast()
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
        }

        // A theme or colour picker style that is not available leaves its slot as it is
        // (TEMPORARY: the managed original has all four; see `crate::temporary`).
        if theme == CatalogTheme::Fluent {
            if let Some(fluent_theme) = app.fluent_theme.borrow().clone() {
                app.theme_styles_container.set(0, fluent_theme.as_style());
            }
            if let Some(color_picker_fluent) = app.color_picker_fluent.borrow().clone() {
                app.theme_styles_container.set(1, color_picker_fluent);
            }
        } else if theme == CatalogTheme::Simple {
            if let Some(simple_theme) = app.simple_theme.borrow().clone() {
                app.theme_styles_container.set(0, simple_theme.as_style());
            }
            if let Some(color_picker_simple) = app.color_picker_simple.borrow().clone() {
                app.theme_styles_container.set(1, color_picker_simple);
            }
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
            }
        }
    }
}
