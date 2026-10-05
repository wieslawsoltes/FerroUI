//! Port of `Pages/CarouselPage/CarouselPageDataTemplatePage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageDataTemplatePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color};
use ferroui_base::data::model::BindableList;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, FontWeight, IBrush, SolidColorBrush, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{
    Border, CarouselPage, ContentPage, Control, ItemsSource, PageSelectionChangedEventArgs, Panel, StackPanel,
    TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A city: the data of a page.
struct CityViewModel {
    name: String,
    color: String,
    description: String,
}

impl PartialEq for CityViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CityViewModel {
    fn new(name: &str, color: &str, description: &str) -> Rc<CityViewModel> {
        Rc::new(Self { name: name.to_string(), color: color.to_string(), description: description.to_string() })
    }
}

/// The names, colors and descriptions of the cities the page starts with.
const INITIAL_DATA: [(&str, &str, &str); 4] = [
    ("Tokyo", "#1565C0", "The neon-lit capital of Japan, where ancient temples meet futuristic skylines."),
    ("Amsterdam", "#2E7D32", "A city of canals, bicycles, and world-class museums."),
    ("New York", "#6A1B9A", "The city that never sleeps \u{2014} a cultural and financial powerhouse."),
    ("Sydney", "#B71C1C", "Iconic harbour, golden beaches and the world-famous Opera House."),
];

/// The names, colors and descriptions of the cities the page adds.
const ADD_DATA: [(&str, &str, &str); 3] = [
    ("Paris", "#E65100", "The city of light, love, and the Eiffel Tower."),
    ("Barcelona", "#00695C", "Art, architecture, and vibrant street life on the Mediterranean coast."),
    ("Kyoto", "#880E4F", "Japan's ancient capital, a living museum of traditional culture."),
];

#[repr(C)]
pub struct CarouselPageDataTemplatePage {
    base: UserControl,
    items: Rc<BindableList<Rc<CityViewModel>>>,
    add_counter: Cell<i32>,
    use_card_template: Cell<bool>,
    carousel_page: RefCell<Option<Ref<CarouselPage>>>,
}

user_control_class!(CarouselPageDataTemplatePage);
ferro_class_info!(CarouselPageDataTemplatePage {
    new: CarouselPageDataTemplatePage::new,
    markup: {
        methods: [
            fn OnAddPage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_page(&sender, e.as_routed_event_args())
                },
            fn OnRemovePage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_page(&sender, e.as_routed_event_args())
                },
            fn OnSwitchTemplate(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_switch_template(&sender, e.as_routed_event_args())
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageDataTemplatePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselPageDataTemplatePage, "/Pages/CarouselPage/CarouselPageDataTemplatePage.xaml");

impl CarouselPageDataTemplatePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            items: BindableList::new([]),
            add_counter: Cell::new(0),
            use_card_template: Cell::new(true),
            carousel_page: RefCell::new(None),
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

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.carousel_page.borrow().is_some() {
            return;
        }

        for (name, color, description) in INITIAL_DATA {
            self.items.items().add(CityViewModel::new(name, color, description));
        }

        self.add_counter.set(INITIAL_DATA.len() as i32);
        self.use_card_template.set(true);

        let carousel_page = CarouselPage::new();
        carousel_page.set_items_source(Some(ItemsSource::from(self.items.clone())));
        carousel_page.set_page_template(Some(self.create_page_template()));
        *self.carousel_page.borrow_mut() = Some(carousel_page.clone());

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = self.to_ref().downgrade();
        carousel_page.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
        self.get_control::<Panel>("CarouselHost").children().add(carousel_page);

        self.update_status();
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &PageSelectionChangedEventArgs) {
        self.update_status();
    }

    fn on_add_page(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let add_counter = self.add_counter.get();
        let length = ADD_DATA.len() as i32;
        let (name, color, description) = ADD_DATA[(add_counter % length) as usize];
        let suffix = if add_counter >= length { format!(" {}", add_counter / length + 1) } else { String::new() };
        self.items.items().add(CityViewModel::new(&format!("{name}{suffix}"), color, description));
        self.add_counter.set(add_counter + 1);
        self.update_status();
    }

    fn on_remove_page(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let count = self.items.items().count();
        if count > 0 {
            self.items.items().remove_at(count - 1);
            self.update_status();
        }
    }

    fn on_switch_template(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let carousel_page = self.carousel_page.borrow().clone();
        let Some(carousel_page) = carousel_page else {
            return;
        };
        self.use_card_template.set(!self.use_card_template.get());
        carousel_page.set_page_template(Some(self.create_page_template()));
    }

    fn on_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let carousel_page = self.carousel_page.borrow().clone();
        let Some(carousel_page) = carousel_page else {
            return;
        };
        if carousel_page.selected_index() > 0 {
            carousel_page.set_selected_index(carousel_page.selected_index() - 1);
        }
    }

    fn on_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let carousel_page = self.carousel_page.borrow().clone();
        let Some(carousel_page) = carousel_page else {
            return;
        };
        if carousel_page.selected_index() < self.items.items().count() as i32 - 1 {
            carousel_page.set_selected_index(carousel_page.selected_index() + 1);
        }
    }

    fn update_status(&self) {
        let count = self.items.items().count();
        let index = self.carousel_page.borrow().as_ref().map_or(-1, |carousel_page| carousel_page.selected_index());
        self.get_control::<TextBlock>("StatusText").set_text(Some(&if count == 0 {
            String::from("No pages")
        } else {
            format!("Page {} of {count} (index {index})", index + 1)
        }));
    }

    fn create_page_template(&self) -> Rc<dyn IDataTemplate> {
        // The template is held by the carousel page, a child of this control: it holds the
        // control weakly. The kind of the template is read when a page is built.
        let weak = self.to_ref().downgrade();
        FuncDataTemplate::for_type::<Rc<CityViewModel>>(
            move |vm, _| {
                let use_card_template = weak.upgrade().is_some_and(|this| this.use_card_template.get());
                Some(Self::create_page(vm, use_card_template).upcast())
            },
            false,
        )
    }

    fn create_page(vm: &CityViewModel, use_card_template: bool) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_header(Some(boxed_text(&vm.name)));
        page.set_content(Some(Control::boxed(if use_card_template {
            Self::create_card_content(vm)
        } else {
            Self::create_feature_content(vm)
        })));
        page
    }

    fn create_card_content(vm: &CityViewModel) -> Ref<Control> {
        let title = TextBlock::new();
        title.set_text(Some(vm.name.as_str()));
        title.set_font_size(28.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_foreground(Some(SolidColorBrush::with_color(parse_color(&vm.color)).into()));
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let text = TextBlock::new();
        text.set_text(Some(vm.description.as_str()));
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

    fn create_feature_content(vm: &CityViewModel) -> Ref<Control> {
        let accent = parse_color(&vm.color);
        let white: Rc<dyn IBrush> = Brushes::white();

        let title = TextBlock::new();
        title.set_text(Some(&vm.name.to_uppercase()));
        title.set_font_size(34.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_foreground(Some(white.clone()));
        title.set_horizontal_alignment(HorizontalAlignment::Center);

        let text = TextBlock::new();
        text.set_text(Some(vm.description.as_str()));
        text.set_font_size(15.0);
        text.set_foreground(Some(white));
        text.set_opacity(0.88);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text_alignment(TextAlignment::Center);
        text.set_max_width(320.0);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(12.0);
        panel.children().add(title);
        panel.children().add(text);

        let border = Border::new();
        border.set_background(Some(SolidColorBrush::with_color(accent).into()));
        border.set_padding(Thickness::uniform(32.0));
        border.set_child(panel);
        border.upcast()
    }
}
