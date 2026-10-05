//! Port of `MainWindow.xaml.cs`: the class of the document
//! `MainWindow.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::{InputElementImpl, Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, NativeMenu, NativeMenuItem, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::cell::RefCell;

/// The main window of the catalog.
#[repr(C)]
pub struct MainWindow {
    base: Window,
    recent_menu: RefCell<Option<Ref<NativeMenu>>>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow {
    new: MainWindow::new,
    markup: {
        methods: [
            fn OnOpenClicked(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, args: EventArgs| this.on_open_clicked(&sender, &args),
            fn OnCloseClicked(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, args: EventArgs| this.on_close_clicked(&sender, &args),
        ],
        fields: [
            MenuQuitHeader: String => MainWindow::menu_quit_header,
            MenuQuitGesture: KeyGesture => MainWindow::menu_quit_gesture,
        ],
    },
});
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()), recent_menu: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        // TEMPORARY: while `MainWindow.xaml` is listed in `excluded.txt` the subset of it that
        // loads is loaded instead.
        if crate::excluded(Self::DOCUMENT_PATH).is_some() {
            let root: BoxedValue = std::rc::Rc::new(this.clone());
            if let Err(error) = crate::temporary::load_main_window_subset(root) {
                panic!("{}: {}", Self::DOCUMENT_PATH, crate::markup::describe(&error));
            }
        } else {
            this.initialize_component();
        }

        let recent_menu = NativeMenu::get_menu(&this)
            .and_then(|menu| menu.items().get(0).cast::<NativeMenuItem>())
            .and_then(|item| item.menu())
            .and_then(|menu| menu.items().get(2).cast::<NativeMenuItem>())
            .and_then(|item| item.menu());
        *this.recent_menu.borrow_mut() = recent_menu;
        this
    }

    /// `MenuQuitHeader`.
    pub fn menu_quit_header() -> String {
        if cfg!(target_os = "macos") { "Quit FerroUI".to_string() } else { "E_xit".to_string() }
    }

    /// `MenuQuitGesture`.
    pub fn menu_quit_gesture() -> KeyGesture {
        if cfg!(target_os = "macos") {
            KeyGesture::new(Key::Q, KeyModifiers::META)
        } else {
            KeyGesture::new(Key::F4, KeyModifiers::ALT)
        }
    }

    pub fn on_open_clicked(&self, _sender: &Option<BoxedValue>, _args: &EventArgs) {
        let recent_menu = self.recent_menu.borrow().clone();
        if let Some(recent_menu) = recent_menu {
            let item = NativeMenuItem::with_header(&format!("Item {}", recent_menu.items().count() + 1));
            recent_menu.items().insert(0, item.upcast());
        }
    }

    pub fn on_close_clicked(&self, _sender: &Option<BoxedValue>, _args: &EventArgs) {
        self.close();
    }
}
