//! Port of `Pages/CarouselPage/CarouselDataBindingPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselDataBindingPage.xaml`, and `CarouselCardItem`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::parse_color;
use crate::view_models::random::Random;
use ferroui_base::data::model::BindableList;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::media::{Brushes, IBrush, SolidColorBrush};
use ferroui_base::{ferro_class_info, ferro_markup_type, instantiate, Ref};
use ferroui_controls::{Button, Carousel, ItemsSource, SelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A card of the carousel of the data binding page.
pub struct CarouselCardItem {
    number: RefCell<String>,
    title: RefCell<String>,
    background: RefCell<Rc<dyn IBrush>>,
    accent: RefCell<Rc<dyn IBrush>>,
}

impl PartialEq for CarouselCardItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CarouselCardItem {
    pub fn new() -> Rc<CarouselCardItem> {
        let gray: Rc<dyn IBrush> = Brushes::gray();
        let white: Rc<dyn IBrush> = Brushes::white();
        Rc::new(Self {
            number: RefCell::new(String::new()),
            title: RefCell::new(String::new()),
            background: RefCell::new(gray),
            accent: RefCell::new(white),
        })
    }

    pub fn number(&self) -> String {
        self.number.borrow().clone()
    }

    pub fn set_number(&self, value: String) {
        *self.number.borrow_mut() = value;
    }

    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }

    pub fn set_title(&self, value: String) {
        *self.title.borrow_mut() = value;
    }

    pub fn background(&self) -> Rc<dyn IBrush> {
        self.background.borrow().clone()
    }

    pub fn set_background(&self, value: Rc<dyn IBrush>) {
        *self.background.borrow_mut() = value;
    }

    pub fn accent(&self) -> Rc<dyn IBrush> {
        self.accent.borrow().clone()
    }

    pub fn set_accent(&self, value: Rc<dyn IBrush>) {
        *self.accent.borrow_mut() = value;
    }
}

ferro_markup_type!(class CarouselCardItem {
    this: Rc<CarouselCardItem>,
    handles: [CarouselCardItem, Rc<CarouselCardItem>, Option<Rc<CarouselCardItem>>],
    constructors: [() => CarouselCardItem::new],
    properties: [
        Number: String {
            get: |this: &Rc<CarouselCardItem>| this.number(),
            set: |this: &Rc<CarouselCardItem>, value: String| this.set_number(value)
        },
        Title: String {
            get: |this: &Rc<CarouselCardItem>| this.title(),
            set: |this: &Rc<CarouselCardItem>, value: String| this.set_title(value)
        },
        Background: Rc<dyn IBrush> {
            get: |this: &Rc<CarouselCardItem>| this.background(),
            set: |this: &Rc<CarouselCardItem>, value: Rc<dyn IBrush>| this.set_background(value)
        },
        Accent: Rc<dyn IBrush> {
            get: |this: &Rc<CarouselCardItem>| this.accent(),
            set: |this: &Rc<CarouselCardItem>, value: Rc<dyn IBrush>| this.set_accent(value)
        },
    ],
});

/// The titles, colors and accents of the cards.
const PALETTE: [(&str, &str, &str); 6] = [
    ("Neon Pulse", "#3525CD", "#C3C0FF"),
    ("Ephemeral Blue", "#0891B2", "#BAF0FA"),
    ("Forest Forms", "#059669", "#A7F3D0"),
    ("Golden Hour", "#D97706", "#FDE68A"),
    ("Crimson Wave", "#BE185D", "#FBCFE8"),
    ("Stone Age", "#57534E", "#D6D3D1"),
];

#[repr(C)]
pub struct CarouselDataBindingPage {
    base: UserControl,
    items: Rc<BindableList<Rc<CarouselCardItem>>>,
    add_counter: Cell<i32>,
}

user_control_class!(CarouselDataBindingPage);
ferro_class_info!(CarouselDataBindingPage { new: CarouselDataBindingPage::new });
xaml_class!(CarouselDataBindingPage, "/Pages/CarouselPage/CarouselDataBindingPage.xaml");

impl CarouselDataBindingPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), items: BindableList::new([]), add_counter: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let demo_carousel = this.demo_carousel();
        demo_carousel.set_items_source(Some(ItemsSource::from(this.items.clone())));

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        demo_carousel.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });

        for _ in 0..4 {
            this.append_item();
        }

        let weak = this.downgrade();
        this.button("PreviousButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.demo_carousel().previous();
            }
        });
        let weak = this.downgrade();
        this.button("NextButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.demo_carousel().next();
            }
        });
        let weak = this.downgrade();
        this.button("AddButton").click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_add_item(sender, e);
            }
        });
        let weak = this.downgrade();
        this.button("RemoveButton").click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_remove_current(sender, e);
            }
        });
        let weak = this.downgrade();
        this.button("ShuffleButton").click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_shuffle(sender, e);
            }
        });
        this.update_status();
        this
    }

    fn button(&self, name: &str) -> Ref<Button> {
        self.get_control::<Button>(name)
    }

    fn demo_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    fn append_item(&self) {
        let add_counter = self.add_counter.get();
        let (title, color, accent) = PALETTE[add_counter as usize % PALETTE.len()];
        let item = CarouselCardItem::new();
        item.set_number(format!("{:02}", self.items.items().count() + 1));
        item.set_title(title.to_string());
        item.set_background(SolidColorBrush::with_color(parse_color(color)).into());
        item.set_accent(SolidColorBrush::with_color(parse_color(accent)).into());
        self.items.items().add(item);
        self.add_counter.set(add_counter + 1);
    }

    fn on_add_item(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_item();
        self.update_status();
    }

    fn on_remove_current(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let count = self.items.items().count() as i32;
        if count == 0 {
            return;
        }
        let idx = self.demo_carousel().selected_index().clamp(0, count - 1);
        self.items.items().remove_at(idx as usize);
        self.update_status();
    }

    fn on_shuffle(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let mut rng = Random::new();
        // `OrderBy(_ => rng.Next())`: a stable sort by a random key per item.
        let mut shuffled: Vec<(i32, Rc<CarouselCardItem>)> =
            self.items.items().to_vec().into_iter().map(|item| (rng.next(), item)).collect();
        shuffled.sort_by_key(|(key, _)| *key);
        self.items.items().clear();
        for (_, item) in shuffled {
            self.items.items().add(item);
        }
        self.update_status();
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        self.update_status();
    }

    fn update_status(&self) {
        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!(
            "Item: {} / {}",
            self.demo_carousel().selected_index() + 1,
            self.items.items().count()
        )));
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_new_card_has_empty_texts() {
        let item = CarouselCardItem::new();
        assert_eq!("", item.number());
        assert_eq!("", item.title());
        item.set_number(String::from("01"));
        item.set_title(String::from("Neon Pulse"));
        assert_eq!("01", item.number());
        assert_eq!("Neon Pulse", item.title());
    }
}
