//! Port of `Pages/ContentPage/ContentPageCommandBarPage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPageCommandBarPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight, IBrush, SolidColorBrush, StreamGeometry, TextAlignment, TextWrapping};
use ferroui_controls::{
    CheckBox, ComboBox, CommandBar, CommandBarButton, CommandBarSeparator, ContentPage, Control, ICommandBarElement,
    NavigationPage, PathIcon, StackPanel, TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The icons of the primary commands: a label and the data of its path.
const ICON_PRESETS: [(&str, &str); 5] = [
    ("Add", "M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"),
    (
        "Save",
        "M15,9H5V5H15M12,19A3,3 0 0,1 9,16A3,3 0 0,1 12,13A3,3 0 0,1 15,16A3,3 0 0,1 12,19M17,3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V7L17,3Z",
    ),
    (
        "Share",
        "M18,16.08C17.24,16.08 16.56,16.38 16.04,16.85L8.91,12.7C8.96,12.47 9,12.24 9,12C9,11.76 8.96,11.53 8.91,11.3L15.96,7.19C16.5,7.69 17.21,8 18,8A3,3 0 0,0 21,5A3,3 0 0,0 18,2A3,3 0 0,0 15,5C15,5.24 15.04,5.47 15.09,5.7L8.04,9.81C7.5,9.31 6.79,9 6,9A3,3 0 0,0 3,12A3,3 0 0,0 6,15C6.79,15 7.5,14.69 8.04,14.19L15.16,18.34C15.11,18.55 15.08,18.77 15.08,19C15.08,20.61 16.39,21.91 18,21.91C19.61,21.91 20.92,20.61 20.92,19C20.92,17.39 19.61,16.08 18,16.08Z",
    ),
    (
        "Favorite",
        "M12,21.35L10.55,20.03C5.4,15.36 2,12.27 2,8.5C2,5.41 4.42,3 7.5,3C9.24,3 10.91,3.81 12,5.08C13.09,3.81 14.76,3 16.5,3C19.58,3 22,5.41 22,8.5C22,12.27 18.6,15.36 13.45,20.03L12,21.35Z",
    ),
    ("Delete", "M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"),
];

/// `item as CommandBarButton`.
fn as_button(item: &Rc<dyn ICommandBarElement>) -> Option<Ref<CommandBarButton>> {
    item.as_object().and_then(|object| object.to_ref().cast::<CommandBarButton>())
}

fn text_content(text: &str) -> BoxedValue {
    Rc::new(text.to_string())
}

/// `new SolidColorBrush(Color.Parse(text))`.
///
/// # Panics
/// Panics if the text is not a color (the format exception of the original).
fn solid_color_brush(text: &str) -> Rc<dyn IBrush> {
    match Color::parse(text) {
        Ok(color) => SolidColorBrush::with_color(color).into(),
        Err(error) => panic!("{error}"),
    }
}

#[repr(C)]
pub struct ContentPageCommandBarPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
    item_counter: Cell<i32>,
    position: Cell<&'static str>,
    primary_items: RefCell<Vec<Rc<dyn ICommandBarElement>>>,
    secondary_items: RefCell<Vec<Rc<dyn ICommandBarElement>>>,
}

user_control_class!(ContentPageCommandBarPage);
ferro_class_info!(ContentPageCommandBarPage {
    new: ContentPageCommandBarPage::new,
    markup: {
        methods: [
            fn OnPositionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_position_changed(&sender, e.as_routed_event_args())
                },
            fn OnAddPrimary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_primary(&sender, e.as_routed_event_args())
                },
            fn OnAddSecondary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_secondary(&sender, e.as_routed_event_args())
                },
            fn OnAddSeparator(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_separator(&sender, e.as_routed_event_args())
                },
            fn OnClearAll(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_all(&sender, e.as_routed_event_args())
                },
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCommandBarPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPageCommandBarPage, "/Pages/ContentPage/ContentPageCommandBarPage.xaml");

impl ContentPageCommandBarPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            item_counter: Cell::new(0),
            position: Cell::new("Top"),
            primary_items: RefCell::new(Vec::new()),
            secondary_items: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn position_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("PositionCombo") } else { None }
    }

    fn demo_nav(&self) -> Option<Ref<NavigationPage>> {
        if self.initialized.get() { self.find_control::<NavigationPage>("DemoNav") } else { None }
    }

    fn use_icon_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("UseIconCheck")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let title = TextBlock::new();
        title.set_text(Some("Root Page"));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let body = TextBlock::new();
        body.set_text(Some("Add items using the panel on the right,\nthen change the position to Top or Bottom."));
        body.set_font_size(14.0);
        body.set_text_wrapping(TextWrapping::Wrap);
        body.set_horizontal_alignment(HorizontalAlignment::Center);
        body.set_text_alignment(TextAlignment::Center);
        body.set_opacity(0.7);

        let content = StackPanel::new();
        content.set_horizontal_alignment(HorizontalAlignment::Center);
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.set_spacing(12.0);
        content.children().add(title);
        content.children().add(body);

        let root_page = ContentPage::new();
        root_page.set_header(Some(text_content("CommandBar Demo")));
        root_page.set_background(Some(solid_color_brush("#E3F2FD")));
        root_page.set_content(Some(Control::boxed(content)));
        root_page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        root_page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        let _ = self.get_control::<NavigationPage>("DemoNav").push_async(root_page);
    }

    fn on_position_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(position_combo) = self.position_combo() else {
            return;
        };

        self.position.set(if position_combo.selected_index() == 1 { "Bottom" } else { "Top" });
        self.rebuild_command_bar();
    }

    fn on_add_primary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.item_counter.set(self.item_counter.get() + 1);
        let item_counter = self.item_counter.get();
        let btn = CommandBarButton::new();
        btn.set_label(Some(&format!("Action {item_counter}")));

        if self.use_icon_check().is_checked() == Some(true) {
            let (label, path_data) = ICON_PRESETS[((item_counter - 1) as usize) % ICON_PRESETS.len()];
            btn.set_label(Some(label));
            let icon = PathIcon::new();
            match StreamGeometry::parse(path_data) {
                Ok(data) => icon.set_data(data),
                Err(error) => panic!("{error}"),
            }
            btn.set_icon(Some(Control::boxed(icon)));
            btn.set_is_compact(true);
        }

        self.primary_items.borrow_mut().push(btn.as_command_bar_element());
        self.rebuild_command_bar();
    }

    fn on_add_secondary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.item_counter.set(self.item_counter.get() + 1);
        let btn = CommandBarButton::new();
        btn.set_label(Some(&format!("Item {}", self.item_counter.get())));
        self.secondary_items.borrow_mut().push(btn.as_command_bar_element());
        self.rebuild_command_bar();
    }

    fn on_add_separator(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.primary_items.borrow_mut().push(CommandBarSeparator::new().as_command_bar_element());
        self.rebuild_command_bar();
    }

    fn on_clear_all(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.primary_items.borrow_mut().clear();
        self.secondary_items.borrow_mut().clear();
        self.item_counter.set(0);
        self.clear_command_bar_from_active_page();
        self.status_text().set_text(Some("No items added"));
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_nav = self.get_control::<NavigationPage>("DemoNav");
        let next = demo_nav.stack_depth() + 1;

        let title = TextBlock::new();
        title.set_text(Some(&format!("Page {next}")));
        title.set_font_size(28.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let body = TextBlock::new();
        body.set_text(Some("New page: no CommandBar set"));
        body.set_font_size(14.0);
        body.set_opacity(0.6);
        body.set_horizontal_alignment(HorizontalAlignment::Center);

        let content = StackPanel::new();
        content.set_horizontal_alignment(HorizontalAlignment::Center);
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.set_spacing(8.0);
        content.children().add(title);
        content.children().add(body);

        let page = ContentPage::new();
        page.set_header(Some(text_content(&format!("Page {next}"))));
        page.set_background(Some(solid_color_brush("#E8F5E9")));
        page.set_content(Some(Control::boxed(content)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        let _ = demo_nav.push_async(page);
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let _ = self.get_control::<NavigationPage>("DemoNav").pop_async();
    }

    fn clear_command_bar_from_active_page(&self) {
        if let Some(active_page) = self.get_control::<NavigationPage>("DemoNav").current_page() {
            NavigationPage::set_top_command_bar(&active_page, None);
            NavigationPage::set_bottom_command_bar(&active_page, None);
        }
    }

    fn rebuild_command_bar(&self) {
        let Some(active_page) = self.demo_nav().and_then(|demo_nav| demo_nav.current_page()) else {
            return;
        };

        if self.primary_items.borrow().is_empty() && self.secondary_items.borrow().is_empty() {
            self.clear_command_bar_from_active_page();
            self.status_text().set_text(Some("No items added"));
            return;
        }

        NavigationPage::set_top_command_bar(&active_page, None);
        NavigationPage::set_bottom_command_bar(&active_page, None);

        let command_bar = CommandBar::new();
        command_bar.set_is_dynamic_overflow_enabled(true);

        let primary_items = self.primary_items.borrow().clone();
        for item in &primary_items {
            if let Some(btn) = as_button(item) {
                let icon = btn.icon().and_then(|icon| Control::from_boxed(&icon)).and_then(|icon| icon.cast::<PathIcon>());
                let copy = CommandBarButton::new();
                copy.set_label(btn.label().as_deref());
                if let Some(src) = icon {
                    let icon = PathIcon::new();
                    icon.set_data(src.data());
                    copy.set_icon(Some(Control::boxed(icon)));
                }
                copy.set_is_compact(btn.is_compact());
                command_bar.primary_commands().add(copy.as_command_bar_element());
            } else if item.as_object().is_some_and(|object| object.is::<CommandBarSeparator>()) {
                command_bar.primary_commands().add(CommandBarSeparator::new().as_command_bar_element());
            }
        }

        let secondary_items = self.secondary_items.borrow().clone();
        for item in &secondary_items {
            if let Some(btn) = as_button(item) {
                let copy = CommandBarButton::new();
                copy.set_label(btn.label().as_deref());
                command_bar.secondary_commands().add(copy.as_command_bar_element());
            }
        }

        let position = self.position.get();
        if position == "Top" {
            NavigationPage::set_top_command_bar(&active_page, command_bar);
        } else {
            NavigationPage::set_bottom_command_bar(&active_page, command_bar);
        }

        let primary_count = primary_items.iter().filter(|item| as_button(item).is_some()).count();
        let secondary_count = secondary_items.len();
        self.status_text().set_text(Some(&format!("{primary_count} primary, {secondary_count} secondary ({position})")));
    }
}
