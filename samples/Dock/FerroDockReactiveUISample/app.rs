// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    behaviors::DockLayoutLifetimeBehavior,
    infrastructure::{
        DockTabThemeSelection, DockThemeSelectionService, LayoutJsonSerialization,
        NativeLayoutFileStorage,
    },
    view_models::{MainViewThemeViewModel, MainWindowViewModel},
    views::{MainView, MainWindow},
};
use ferrodock_ferro::themes::IDockThemeManager;
use ferrodock_ferro_themes_fluent::DockFluentThemeManager;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, styling::ThemeVariant,
    FerroObjectImpl, Ref,
};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
ferroui_base::ferro_thread_local! {static THEME_MANAGER:RefCell<Option<Rc<dyn IDockThemeManager>>>=RefCell::new(None);}
/// The original sample application and its single composition root.
#[repr(C)]
pub struct App {
    base: Application,
    tab_themes: OnceCell<Rc<DockTabThemeSelection>>,
}
ferro_class!(App:Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::set_theme_manager(Some(DockFluentThemeManager::new()));
        crate::register_types();
        crate::compiled_app::populate(None, &this.to_ref())
            .unwrap_or_else(|error| panic!("{error}"));
    }
    fn on_framework_initialization_completed(this: &Self) {
        let model = Self::create_main_view_model();
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let window = MainWindow::new();
            model.attach_layout_files(NativeLayoutFileStorage::new(window.clone().upcast()));
            window.set_data_context(Some(Rc::new(model.clone())));
            #[cfg(debug_assertions)]
            {
                let model = model.clone();
                ferrodock_ferro_diagnostics::controls::RootDockDebugExtensions::attach_dock_debug(
                    window.clone().upcast(),
                    Rc::new(move || {
                        model
                            .layout()
                            .map(ferrodock_ferro::model_projection::to_root_value)
                    }),
                    None,
                );
            }
            #[cfg(debug_assertions)]
            ferrodock_ferro_diagnostics::DockDebugOverlayExtensions::attach_dock_debug_overlay(
                window.clone().upcast(),
                None,
            );
            DockLayoutLifetimeBehavior::attach(&window, desktop, model);
            desktop.set_main_window(Some(window.upcast()));
        } else if let Some(single) = lifetime
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            let view = MainView::new();
            model.attach_layout_files(NativeLayoutFileStorage::new(view.clone().upcast()));
            view.set_data_context(Some(Rc::new(model)));
            single.set_main_view(Some(view.upcast()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
impl App {
    pub(crate) fn create_main_view_model() -> Rc<MainWindowViewModel> {
        let factory = crate::view_models::DockFactory::new(crate::models::DemoData::new());
        let serialization =
            LayoutJsonSerialization::new().expect("compiled sample JSON registrations are valid");
        MainWindowViewModel::with_layout_services(factory.clone(), serialization, factory)
    }
    /// Creates the original application object before framework initialization.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Creates native class storage for source application subclasses.
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
            tab_themes: OnceCell::new(),
        }
    }
    /// Gets the original nullable static theme-manager field.
    pub fn theme_manager() -> Option<Rc<dyn IDockThemeManager>> {
        THEME_MANAGER.with(|slot| slot.borrow().clone())
    }
    /// Sets the original public static theme-manager field.
    pub fn set_theme_manager(value: Option<Rc<dyn IDockThemeManager>>) {
        THEME_MANAGER.with(|slot| *slot.borrow_mut() = value);
    }
    pub(crate) fn create_theme_view_model(
        manager: Rc<dyn IDockThemeManager>,
    ) -> Rc<MainViewThemeViewModel> {
        let dark = Application::current().and_then(|app| app.requested_theme_variant())
            == Some(ThemeVariant::dark());
        let app =
            Application::current().expect("theme actions require the initialized application");
        let tab_themes = if let Some(sample) = app.cast::<Self>() {
            sample
                .tab_themes
                .get_or_init(|| DockTabThemeSelection::new(app.styles()))
                .clone()
        } else {
            DockTabThemeSelection::new(app.styles())
        };
        MainViewThemeViewModel::new(DockThemeSelectionService::new(manager, tab_themes), dark)
    }
}
