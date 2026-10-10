//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.

use crate::embedding_page::EmbeddingPage;
use crate::markup::xaml_class;
use crate::models::Page;
use crate::pages::{
    AutomationPage, ButtonPage, CheckBoxPage, ComboBoxPage, ContextMenuPage, DesktopPage, DragDropPage, GesturesPage,
    KeyboardPage, ListBoxPage, MenuPage, PointerPage, PopupsPage, RadioButtonPage, ScreensPage, ScrollBarPage,
    SliderPage, WindowDecorationsPage, WindowPage,
};
use crate::view_models::MainWindowViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, PixelRect, PixelSize,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, Decorator, ListBox, MenuItemToggleType, NativeMenu, NativeMenuItem,
    PixelPointEventArgs, TextBlock, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
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
            fn Pager_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.pager_selection_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        // Set name in code behind, so source generator will ignore it.
        this.set_name(Some("MainWindow".to_string()));

        this.initialize_component();

        let view_model = MainWindowViewModel::new(Self::create_pages());
        this.initialize_view_menu(&view_model.pages().items().to_vec());

        this.set_data_context(Some(view_model as BoxedValue));
        this.app_overlay_popups()
            .set_text(Some(if crate::overlay_popups() { "Overlay Popups" } else { "Native Popups" }));
        // The window holds the handlers of its events: the handler holds the window weakly.
        let weak = this.downgrade();
        this.position_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_position_changed(e);
            }
        });
        this
    }

    fn pager(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("Pager")
    }

    fn pager_content(&self) -> Ref<Decorator> {
        self.get_control::<Decorator>("PagerContent")
    }

    fn app_overlay_popups(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("AppOverlayPopups")
    }

    /// `ViewModel`: the data context as the view model of the window.
    fn view_model(&self) -> Option<Rc<MainWindowViewModel>> {
        from_markup_value::<Rc<MainWindowViewModel>>(&self.data_context())
    }

    fn initialize_view_menu(&self, pages: &[Rc<Page>]) {
        let view_menu = NativeMenu::get_menu(self).and_then(|menu| menu.items().get(1).cast::<NativeMenuItem>());

        for page in pages {
            let menu_item = NativeMenuItem::new();
            menu_item.set_header(Some(page.name()));
            menu_item.set_tool_tip(Some(format!("Tip:{}", page.name())));
            menu_item.set_toggle_type(MenuItemToggleType::Radio);

            {
                // The item is held by the menu of the window: its handler holds the window
                // weakly.
                let weak = self.to_ref().downgrade();
                let page = page.clone();
                menu_item.click(move |_| {
                    if let Some(view_model) = weak.upgrade().and_then(|this| this.view_model()) {
                        view_model.set_selected_page(Some(page.clone()));
                    }
                });
            }

            if let Some(menu) = view_menu.as_ref().and_then(|view_menu| view_menu.menu()) {
                menu.items().add(menu_item.upcast());
            }
        }
    }

    fn pager_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(page) = from_markup_value::<Rc<Page>>(&self.pager().selected_item()) {
            let content: Ref<Control> = (page.create_content())();
            self.pager_content().set_child(Some(content));
        }
    }

    fn on_position_changed(&self, e: &PixelPointEventArgs) {
        // HACK: Toggling the window decorations can cause the window to be moved off screen,
        // causing test failures. Until this bug is fixed, detect this and move the window
        // to the screen origin. See #11411.
        if let Some(screen) = self.screens().screen_from_window(self) {
            let bounds =
                PixelRect::from_position_size(e.point(), PixelSize::from_size(self.client_size(), self.desktop_scaling()));

            if !screen.working_area().contains_rect(bounds) {
                self.set_position(screen.working_area().position());
            }
        }
    }

    fn create_pages() -> Vec<Rc<Page>> {
        vec![
            Page::new("Automation", || AutomationPage::new().upcast()),
            Page::new("Button", || ButtonPage::new().upcast()),
            Page::new("CheckBox", || CheckBoxPage::new().upcast()),
            Page::new("ComboBox", || ComboBoxPage::new().upcast()),
            Page::new("ContextMenu", || ContextMenuPage::new().upcast()),
            Page::new("DesktopPage", || DesktopPage::new().upcast()),
            Page::new("DragDrop", || DragDropPage::new().upcast()),
            Page::new("Embedding", || EmbeddingPage::new().upcast()),
            Page::new("Gestures", || GesturesPage::new().upcast()),
            Page::new("Keyboard", || KeyboardPage::new().upcast()),
            Page::new("ListBox", || ListBoxPage::new().upcast()),
            Page::new("Menu", || MenuPage::new().upcast()),
            Page::new("Pointer", || PointerPage::new().upcast()),
            Page::new("Popups", || PopupsPage::new().upcast()),
            Page::new("RadioButton", || RadioButtonPage::new().upcast()),
            Page::new("Screens", || ScreensPage::new().upcast()),
            Page::new("ScrollBar", || ScrollBarPage::new().upcast()),
            Page::new("Slider", || SliderPage::new().upcast()),
            Page::new("Window Decorations", || WindowDecorationsPage::new().upcast()),
            Page::new("Window", || WindowPage::new().upcast()),
        ]
    }
}
