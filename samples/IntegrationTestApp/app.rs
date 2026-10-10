//! Port of `App.xaml.cs`: the class of the document `App.xaml`.

use crate::main_window::MainWindow;
use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, ElementRef, FerroObjectImpl, Ref};
use ferroui_controls::{
    Application, ApplicationImpl, ApplicationImplExt, CheckBox, NativeDock, NativeMenuItem, NewApplication,
};
use mini_mvvm::MiniCommand;
use std::cell::RefCell;
use std::rc::Rc;

/// The main window of the application, which its commands look into (`_mainWindow`).
type MainWindowCell = Rc<RefCell<Option<Ref<MainWindow>>>>;

#[repr(C)]
pub struct App {
    base: Application,
    main_window: MainWindowCell,
    tray_icon_command: Rc<MiniCommand>,
    dock_menu_command: Rc<MiniCommand>,
}

ferro_class!(App: Application);
ferro_class_info!(App {
    new: App::new,
    markup: {
        properties: [
            TrayIconCommand: Rc<dyn ICommand> { get: |this: &Ref<App>| this.tray_icon_command().as_command() },
            DockMenuCommand: Rc<dyn ICommand> { get: |this: &Ref<App>| this.dock_menu_command().as_command() },
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
        crate::register_types();
        this.initialize_component();
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            let main_window = MainWindow::new();
            *this.main_window.borrow_mut() = Some(main_window.clone());
            desktop.set_main_window(Some(main_window.upcast()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        let main_window: MainWindowCell = Rc::new(RefCell::new(None));
        let tray_icon_command = {
            let main_window = main_window.clone();
            MiniCommand::create_with::<String>(move |name| {
                let main_window = main_window.borrow().clone().expect("the main window of the application");
                main_window.get_control::<CheckBox>(&name).set_is_checked(Some(true));
            })
        };
        let dock_menu_command = {
            let main_window = main_window.clone();
            MiniCommand::create_with::<String>(move |name| {
                // This is for the "Show Main Window" dock menu item in the test.
                // It doesn't actually show the main window, but sets the checkbox to true in the page.
                let main_window = main_window.borrow().clone().expect("the main window of the application");
                let checkbox = main_window
                    .get_logical_descendants()
                    .filter_map(|x| x.cast::<CheckBox>())
                    .find(|x| x.name().as_deref() == Some(name.as_str()));
                if let Some(checkbox) = checkbox {
                    checkbox.set_is_checked(Some(true));
                }
            })
        };
        Self { base: Application::construct(), main_window, tray_icon_command, dock_menu_command }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        // `DataContext = this;` in the managed original, where the collector frees an object
        // that holds itself. Here the application is its data context as an element
        // reference, which holds it weakly: the bindings of the document read the
        // application through it.
        ValueTypes::register_element_ref::<App>();
        this.set_data_context(Some(Rc::new(ElementRef::of(&this)) as BoxedValue));
        this
    }

    pub fn tray_icon_command(&self) -> Rc<MiniCommand> {
        self.tray_icon_command.clone()
    }

    pub fn dock_menu_command(&self) -> Rc<MiniCommand> {
        self.dock_menu_command.clone()
    }

    pub fn add_dock_menu_item(&self, header: &str) {
        let dock_menu = NativeDock::get_menu(self);
        if let Some(dock_menu) = dock_menu {
            dock_menu.items().insert(0, NativeMenuItem::with_header(header).upcast());
        }
    }

    pub fn get_dock_menu_item_count(&self) -> i32 {
        let dock_menu = NativeDock::get_menu(self);
        dock_menu.map_or(0, |dock_menu| dock_menu.items().count() as i32)
    }
}
