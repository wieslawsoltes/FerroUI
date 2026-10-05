//! Port of `Pages/NavigationPage/NavigationPageToolbarPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageToolbarPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::NavigationDemoHelper;
use ferroui_base::controls::ResourceKey;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::media::Geometry;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    ComboBox, CommandBar, CommandBarButton, CommandBarSeparator, ContentPage, Control, NavigationPage, PathIcon,
    SelectionChangedEventArgs, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageToolbarPage {
    base: UserControl,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
    item_count: Cell<i32>,
    root_page: RefCell<Option<Ref<ContentPage>>>,
    root_command_bar: Ref<CommandBar>,
}

user_control_class!(NavigationPageToolbarPage);
ferro_class_info!(NavigationPageToolbarPage {
    new: NavigationPageToolbarPage::new,
    markup: {
        methods: [
            fn OnPositionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_position_changed(&sender, e)
                    }
                },
            fn OnAddPrimary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_primary(&sender, e.as_routed_event_args())
                },
            fn OnAddSecondary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_secondary(&sender, e.as_routed_event_args())
                },
            fn OnAddPrimarySeparator(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_primary_separator(&sender, e.as_routed_event_args())
                },
            fn OnAddSecondarySeparator(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_secondary_separator(&sender, e.as_routed_event_args())
                },
            fn OnClearAll(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_all(&sender, e.as_routed_event_args())
                },
            fn OnPushWithToolbar(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_with_toolbar(&sender, e.as_routed_event_args())
                },
            fn OnPushWithoutToolbar(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_without_toolbar(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageToolbarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageToolbarPage, "/Pages/NavigationPage/NavigationPageToolbarPage.xaml");

impl NavigationPageToolbarPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
            item_count: Cell::new(0),
            root_page: RefCell::new(None),
            root_command_bar: {
                let command_bar = CommandBar::new();
                command_bar.set_is_dynamic_overflow_enabled(true);
                command_bar
            },
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn position_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PositionCombo")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let this = self.to_ref();
        drop(start_async(async move {
            let root_page = NavigationDemoHelper::make_page(
                "CommandBar Demo",
                "Use the panel to add CommandBar items.\nTop items appear inside the navigation bar.\nBottom items appear as a separate bar.",
                0,
            );
            *this.root_page.borrow_mut() = Some(root_page.clone());
            this.apply_position();
            if this.demo_nav().push_async_with_transition(root_page, None).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    fn on_position_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        self.apply_position();
    }

    fn apply_position(&self) {
        let Some(root_page) = self.root_page.borrow().clone() else {
            return;
        };

        NavigationPage::set_top_command_bar(&root_page, None::<Ref<Control>>);
        NavigationPage::set_bottom_command_bar(&root_page, None::<Ref<Control>>);

        if self.position_combo().selected_index() == 1 {
            NavigationPage::set_bottom_command_bar(&root_page, self.root_command_bar.clone().upcast::<Control>());
        } else {
            NavigationPage::set_top_command_bar(&root_page, self.root_command_bar.clone().upcast::<Control>());
        }
    }

    /// `new PathIcon { Data = (Geometry)this.FindResource(key)! }`.
    ///
    /// # Panics
    /// Panics if the resource is not found or is not a geometry (the
    /// exceptions of the original).
    fn icon(&self, key: &str) -> BoxedValue {
        let data = from_markup_value::<Ref<Geometry>>(&self.find_resource(&ResourceKey::from(key)))
            .unwrap_or_else(|| panic!("The resource '{key}' is not a geometry."));
        let icon = PathIcon::new();
        icon.set_data(data);
        Control::boxed(icon)
    }

    fn on_add_primary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.item_count.set(self.item_count.get() + 1);
        let button = CommandBarButton::new();
        button.set_label(Some(&format!("Item {}", self.item_count.get())));
        button.set_icon(Some(self.icon("AddIcon")));
        self.root_command_bar.primary_commands().add(button.as_command_bar_element());
        self.update_status();
    }

    fn on_add_secondary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.item_count.set(self.item_count.get() + 1);
        let button = CommandBarButton::new();
        button.set_label(Some(&format!("Secondary {}", self.item_count.get())));
        self.root_command_bar.secondary_commands().add(button.as_command_bar_element());
        self.update_status();
    }

    fn on_add_primary_separator(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.root_command_bar.primary_commands().add(CommandBarSeparator::new().as_command_bar_element());
        self.update_status();
    }

    fn on_add_secondary_separator(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.root_command_bar.secondary_commands().add(CommandBarSeparator::new().as_command_bar_element());
        self.update_status();
    }

    fn on_clear_all(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.root_command_bar.primary_commands().clear();
        self.root_command_bar.secondary_commands().clear();
        self.item_count.set(0);
        self.update_status();
    }

    /// `async void`.
    fn on_push_with_toolbar(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();
            let page = NavigationDemoHelper::make_page(
                &format!("Page {page_count}"),
                "CommandBar is shown in the nav bar.",
                page_count,
            );
            let bar = this.build_preset_command_bar();
            if this.position_combo().selected_index() == 1 {
                NavigationPage::set_bottom_command_bar(&page, bar.upcast::<Control>());
            } else {
                NavigationPage::set_top_command_bar(&page, bar.upcast::<Control>());
            }
            if this.demo_nav().push_async(page).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_push_without_toolbar(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();
            let page = NavigationDemoHelper::make_page(
                &format!("Page {page_count}"),
                "No toolbar on this page.",
                page_count,
            );
            if this.demo_nav().push_async(page).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.demo_nav().pop_async().await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    fn update_status(&self) {
        self.status_text().set_text(Some(&format!("Depth: {}", self.demo_nav().stack_depth())));
    }

    fn build_preset_command_bar(&self) -> Ref<CommandBar> {
        let button = |label: &str, icon: &str| {
            let button = CommandBarButton::new();
            button.set_label(Some(label));
            button.set_icon(Some(self.icon(icon)));
            button.as_command_bar_element()
        };

        let command_bar = CommandBar::new();
        command_bar.primary_commands().add(button("Search", "SearchIcon"));
        command_bar.primary_commands().add(button("Share", "ShareIcon"));
        command_bar.primary_commands().add(button("Edit", "EditIcon"));
        command_bar.secondary_commands().add(button("Delete", "DeleteIcon"));
        command_bar
    }
}
