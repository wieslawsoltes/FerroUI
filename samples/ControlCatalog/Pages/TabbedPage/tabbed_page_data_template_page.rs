//! Port of `Pages/TabbedPage/TabbedPageDataTemplatePage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageDataTemplatePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color};
use ferroui_base::data::model::BindableList;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, FontWeight, IBrush, SolidColorBrush, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref, Thickness};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{
    Border, ContentPage, Control, ItemsSource, PageSelectionChangedEventArgs, Panel, StackPanel, TabPlacement,
    TabbedPage, TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A category: the data of a tab.
struct CategoryViewModel {
    name: String,
    color: String,
}

impl PartialEq for CategoryViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CategoryViewModel {
    fn new(name: &str, color: &str) -> Rc<CategoryViewModel> {
        Rc::new(Self { name: name.to_string(), color: color.to_string() })
    }
}

/// The names and colors of the categories the page starts with.
const INITIAL_DATA: [(&str, &str); 3] = [("Electronics", "#1565C0"), ("Books", "#2E7D32"), ("Clothing", "#6A1B9A")];

/// The names and colors of the categories the page adds.
const ADD_DATA: [(&str, &str); 5] =
    [("Sports", "#E53935"), ("Music", "#F57C00"), ("Garden", "#00796B"), ("Toys", "#E91E63"), ("Food", "#3F51B5")];

#[repr(C)]
pub struct TabbedPageDataTemplatePage {
    base: UserControl,
    items: Rc<BindableList<Rc<CategoryViewModel>>>,
    add_counter: Cell<i32>,
    use_detail_template: Cell<bool>,
    tabbed_page: RefCell<Option<Ref<TabbedPage>>>,
}

user_control_class!(TabbedPageDataTemplatePage);
ferro_class_info!(TabbedPageDataTemplatePage {
    new: TabbedPageDataTemplatePage::new,
    markup: {
        methods: [
            fn OnAddCategory(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_category(&sender, e.as_routed_event_args())
                },
            fn OnRemoveCategory(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_category(&sender, e.as_routed_event_args())
                },
            fn OnSwitchTemplate(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_switch_template(&sender, e.as_routed_event_args())
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageDataTemplatePage, "/Pages/TabbedPage/TabbedPageDataTemplatePage.xaml");

impl TabbedPageDataTemplatePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            items: BindableList::new([]),
            add_counter: Cell::new(0),
            use_detail_template: Cell::new(true),
            tabbed_page: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn tabbed_page_host(&self) -> Ref<Panel> {
        self.get_control::<Panel>("TabbedPageHost")
    }

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.tabbed_page.borrow().is_some() {
            return;
        }

        for (name, color) in INITIAL_DATA {
            self.items.items().add(CategoryViewModel::new(name, color));
        }

        self.add_counter.set(INITIAL_DATA.len() as i32);
        self.use_detail_template.set(true);

        let tabbed_page = TabbedPage::new();
        tabbed_page.set_tab_placement(TabPlacement::Top);
        tabbed_page.set_items_source(Some(ItemsSource::from(self.items.clone())));
        tabbed_page.set_page_template(Some(self.create_page_template()));
        *self.tabbed_page.borrow_mut() = Some(tabbed_page.clone());

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = self.to_ref().downgrade();
        tabbed_page.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
        self.tabbed_page_host().children().add(tabbed_page);

        self.update_status();
    }

    fn on_add_category(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let add_counter = self.add_counter.get();
        let length = ADD_DATA.len() as i32;
        let (name, color) = ADD_DATA[(add_counter % length) as usize];
        let suffix = if add_counter >= length { format!(" {}", add_counter / length + 1) } else { String::new() };
        self.items.items().add(CategoryViewModel::new(&format!("{name}{suffix}"), color));
        self.add_counter.set(add_counter + 1);
        self.update_status();
    }

    fn on_remove_category(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let count = self.items.items().count();
        if count > 0 {
            self.items.items().remove_at(count - 1);
            self.update_status();
        }
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &PageSelectionChangedEventArgs) {
        self.update_status();
    }

    fn on_switch_template(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let tabbed_page = self.tabbed_page.borrow().clone();
        let Some(tabbed_page) = tabbed_page else {
            return;
        };
        self.use_detail_template.set(!self.use_detail_template.get());
        tabbed_page.set_page_template(Some(self.create_page_template()));
        self.update_status();
    }

    fn on_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let tabbed_page = self.tabbed_page.borrow().clone();
        let Some(tabbed_page) = tabbed_page else {
            return;
        };
        if tabbed_page.selected_index() > 0 {
            tabbed_page.set_selected_index(tabbed_page.selected_index() - 1);
        }
    }

    fn on_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let tabbed_page = self.tabbed_page.borrow().clone();
        let Some(tabbed_page) = tabbed_page else {
            return;
        };
        if tabbed_page.selected_index() < self.items.items().count() as i32 - 1 {
            tabbed_page.set_selected_index(tabbed_page.selected_index() + 1);
        }
    }

    fn update_status(&self) {
        let count = self.items.items().count();
        let index = self.tabbed_page.borrow().as_ref().map_or(-1, |tabbed_page| tabbed_page.selected_index());
        self.status_text().set_text(Some(&if count == 0 {
            String::from("No tabs")
        } else {
            format!("Tab {} of {count} (index {index})", index + 1)
        }));
    }

    fn create_page_template(&self) -> Rc<dyn IDataTemplate> {
        // The template is held by the tabbed page, a child of this control: it holds the
        // control weakly. The kind of the template is read when a page is built.
        let weak = self.to_ref().downgrade();
        FuncDataTemplate::for_type::<Rc<CategoryViewModel>>(
            move |vm, _| {
                let use_detail_template = weak.upgrade().is_some_and(|this| this.use_detail_template.get());
                Some(Self::create_page(vm, use_detail_template).upcast())
            },
            false,
        )
    }

    fn create_page(vm: &CategoryViewModel, use_detail_template: bool) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_header(Some(boxed_text(&vm.name)));
        page.set_content(Some(Control::boxed(if use_detail_template {
            Self::create_detail_content(vm)
        } else {
            Self::create_showcase_content(vm)
        })));
        page
    }

    fn create_detail_content(vm: &CategoryViewModel) -> Ref<Control> {
        let title = TextBlock::new();
        title.set_text(Some(vm.name.as_str()));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_foreground(Some(SolidColorBrush::with_color(parse_color(&vm.color)).into()));
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let text = TextBlock::new();
        text.set_text(Some(&format!("Tab for category: {}", vm.name)));
        text.set_font_size(13.0);
        text.set_opacity(0.7);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text_alignment(TextAlignment::Center);
        text.set_max_width(280.0);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(8.0);
        panel.children().add(title);
        panel.children().add(text);
        panel.upcast()
    }

    fn create_showcase_content(vm: &CategoryViewModel) -> Ref<Control> {
        let accent = parse_color(&vm.color);
        let white: Rc<dyn IBrush> = Brushes::white();

        let title = TextBlock::new();
        title.set_text(Some(vm.name.as_str()));
        title.set_font_size(28.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_foreground(Some(white.clone()));
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let text = TextBlock::new();
        text.set_text(Some("Template switched at runtime"));
        text.set_font_size(14.0);
        text.set_foreground(Some(white));
        text.set_opacity(0.9);
        text.set_horizontal_alignment(HorizontalAlignment::Center);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(10.0);
        panel.children().add(title);
        panel.children().add(text);

        let border = Border::new();
        border.set_margin(Thickness::uniform(24.0));
        border.set_corner_radius(CornerRadius::uniform(18.0));
        border.set_background(Some(SolidColorBrush::with_color(accent).into()));
        border.set_padding(Thickness::uniform(28.0));
        border.set_child(panel);
        border.upcast()
    }
}
