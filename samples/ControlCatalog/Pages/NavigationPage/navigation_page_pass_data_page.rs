//! Port of `Pages/NavigationPage/NavigationPagePassDataPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPagePassDataPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, value_text};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref, Thickness};
use ferroui_controls::{
    Border, Button, ComboBox, ContentPage, Control, NavigationPage, Page, Panel, ScrollViewer,
    SelectionChangedEventArgs, StackPanel, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

/// The record `Contact` of the page: the data passed to the detail page.
#[derive(Clone, Debug, PartialEq)]
struct Contact {
    name: &'static str,
    occupation: &'static str,
    country: &'static str,
    color: Color,
}

/// The static field `Contacts`: name, occupation, country and color of each contact.
const CONTACTS: [(&str, &str, &str, &str); 5] = [
    ("Alice Johnson", "Software Engineer", "United States", "#4CAF50"),
    ("Bob Smith", "Product Designer", "Canada", "#2196F3"),
    ("Carol White", "Data Scientist", "United Kingdom", "#9C27B0"),
    ("David Lee", "DevOps Engineer", "Australia", "#FF9800"),
    ("Emma Brown", "UX Researcher", "Germany", "#F44336"),
];

fn contacts() -> impl Iterator<Item = Contact> {
    CONTACTS.into_iter().map(|(name, occupation, country, color)| Contact {
        name,
        occupation,
        country,
        color: parse_color(color),
    })
}

/// `$"{page?.Header}"`: the text of the header of a page, empty without a header.
fn header_text(page: &Page) -> String {
    value_text(&page.header()).unwrap_or_default()
}

/// `string.Concat(name.Split(' ')[0][0], name.Split(' ')[1][0])`.
///
/// # Panics
/// Panics if the name has fewer than two words or an empty word (the index out of range of
/// the managed original).
fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split(' ').collect();
    let first = words[0].chars().next().expect("the first letter of the first word");
    let second = words[1].chars().next().expect("the first letter of the second word");
    format!("{first}{second}")
}

fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

#[repr(C)]
pub struct NavigationPagePassDataPage {
    base: UserControl,
    initialized: Cell<bool>,
    is_loaded: Cell<bool>,
}

user_control_class!(NavigationPagePassDataPage);
ferro_class_info!(NavigationPagePassDataPage {
    new: NavigationPagePassDataPage::new,
    markup: {
        methods: [
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePassDataPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnMethodChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePassDataPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_method_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(NavigationPagePassDataPage, "/Pages/NavigationPage/NavigationPagePassDataPage.xaml");

impl NavigationPagePassDataPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false), is_loaded: Cell::new(false) }
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

    fn method_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("MethodCombo")
    }

    fn method_description(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("MethodDescription")
    }

    fn navigation_log(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("NavigationLog")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.is_loaded.set(true);

        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);

        // The handlers belong to a child of the control: they hold the control weakly.
        let demo_nav = self.demo_nav();
        let weak = self.to_ref().downgrade();
        demo_nav.pushed(move |ev| {
            if let Some(this) = weak.upgrade() {
                this.append_navigation_log(&format!("Pushed \u{2192} {}", header_text(&ev.page())));
            }
        });
        let weak = self.to_ref().downgrade();
        demo_nav.popped(move |ev| {
            if let Some(this) = weak.upgrade() {
                this.append_navigation_log(&format!("Popped \u{2190} {}", header_text(&ev.page())));
            }
        });

        drop(demo_nav.push_async_with_transition(self.create_contact_list_page(), None));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.demo_nav().pop_async());
    }

    fn on_method_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }

        if self.method_combo().selected_index() == 0 {
            self.method_description().set_text(Some("Data is passed as a constructor argument to the detail page. The page stores the contact and displays its properties directly."));
        } else {
            self.method_description().set_text(Some("Data is passed by setting the new page's DataContext. This enables data binding in XAML to display the data automatically."));
        }
    }

    fn create_contact_list_page(&self) -> Ref<ContentPage> {
        let list = StackPanel::new();
        list.set_spacing(8.0);
        list.set_margin(Thickness::uniform(16.0));

        let header = TextBlock::new();
        header.set_text(Some("Contacts"));
        header.set_font_size(20.0);
        header.set_font_weight(FontWeight::Bold);
        header.set_margin(Thickness::new(0.0, 0.0, 0.0, 4.0));
        list.children().add(header);

        let subtitle = TextBlock::new();
        subtitle.set_text(Some("Tap a contact to navigate and pass its data to the detail page."));
        subtitle.set_font_size(13.0);
        subtitle.set_opacity(0.6);
        subtitle.set_text_wrapping(TextWrapping::Wrap);
        subtitle.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        list.children().add(subtitle);

        for contact in contacts() {
            let card = self.create_contact_card(contact);
            list.children().add(card);
        }

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(list)));

        let page = ContentPage::new();
        page.set_header(Some(boxed_text("Contacts")));
        page.set_content(Some(Control::boxed(scroll_viewer)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }

    fn create_contact_card(&self, contact: Contact) -> Ref<Button> {
        let initials = initials(contact.name);

        let initials_text = TextBlock::new();
        initials_text.set_text(Some(&initials));
        initials_text.set_foreground(white());
        initials_text.set_font_size(16.0);
        initials_text.set_font_weight(FontWeight::Bold);
        initials_text.set_horizontal_alignment(HorizontalAlignment::Center);
        initials_text.set_vertical_alignment(VerticalAlignment::Center);

        let avatar = Border::new();
        avatar.set_width(44.0);
        avatar.set_height(44.0);
        avatar.set_corner_radius(CornerRadius::uniform(22.0));
        avatar.set_background(Some(SolidColorBrush::with_color(contact.color).into()));
        avatar.set_child(initials_text);

        let name = TextBlock::new();
        name.set_text(Some(contact.name));
        name.set_font_size(15.0);
        name.set_font_weight(FontWeight::SemiBold);

        let details = TextBlock::new();
        details.set_text(Some(&format!("{} \u{b7} {}", contact.occupation, contact.country)));
        details.set_font_size(12.0);
        details.set_opacity(0.6);

        let texts = StackPanel::new();
        texts.set_vertical_alignment(VerticalAlignment::Center);
        texts.set_spacing(2.0);
        texts.children().add(name);
        texts.children().add(details);

        let content = StackPanel::new();
        content.set_orientation(Orientation::Horizontal);
        content.set_spacing(12.0);
        content.children().add(avatar);
        content.children().add(texts);

        let card = Button::new();
        card.set_horizontal_alignment(HorizontalAlignment::Stretch);
        card.set_horizontal_content_alignment(HorizontalAlignment::Left);
        card.set_padding(Thickness::symmetric(12.0, 8.0));
        card.set_content(Some(Control::boxed(content)));

        // The card is a descendant of the control: its handler holds the control weakly.
        let weak = self.to_ref().downgrade();
        card.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.navigate_to_detail(contact.clone());
            }
        });
        card
    }

    /// `async Task NavigateToDetail(contact)`, started: the navigation is logged once the page
    /// is pushed.
    fn navigate_to_detail(&self, contact: Contact) {
        let page_bg = SolidColorBrush::with_color(Color::from_argb(30, contact.color.r, contact.color.g, contact.color.b));

        let detail_page = ContentPage::new();
        if self.method_combo().selected_index() == 1 {
            // Via DataContext
            detail_page.set_header(Some(boxed_text(contact.name)));
            detail_page.set_background(Some(page_bg.into()));
            detail_page.set_data_context(Some(Rc::new(contact.clone()) as BoxedValue));
            detail_page.set_content(Some(Control::boxed(Self::create_detail_content(&contact, "DataContext"))));
        } else {
            // Via Constructor argument
            detail_page.set_header(Some(boxed_text(contact.name)));
            detail_page.set_background(Some(page_bg.into()));
            detail_page.set_content(Some(Control::boxed(Self::create_detail_content(&contact, "Constructor"))));
        }

        detail_page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        detail_page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        let pushed = self.demo_nav().push_async(detail_page);

        // The continuation of the method holds the control until the push completes.
        let this = self.to_ref();
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }

            this.append_navigation_log(&format!(
                "Navigated to {} via {}",
                contact.name,
                if this.method_combo().selected_index() == 1 { "DataContext" } else { "Constructor" }
            ));
        }));
    }

    fn create_detail_content(contact: &Contact, method: &str) -> Ref<Panel> {
        let initials = initials(contact.name);

        let initials_text = TextBlock::new();
        initials_text.set_text(Some(&initials));
        initials_text.set_foreground(white());
        initials_text.set_font_size(28.0);
        initials_text.set_font_weight(FontWeight::Bold);
        initials_text.set_horizontal_alignment(HorizontalAlignment::Center);
        initials_text.set_vertical_alignment(VerticalAlignment::Center);

        let avatar = Border::new();
        avatar.set_width(80.0);
        avatar.set_height(80.0);
        avatar.set_corner_radius(CornerRadius::uniform(40.0));
        avatar.set_background(Some(SolidColorBrush::with_color(contact.color).into()));
        avatar.set_horizontal_alignment(HorizontalAlignment::Center);
        avatar.set_child(initials_text);

        let name = TextBlock::new();
        name.set_text(Some(contact.name));
        name.set_font_size(24.0);
        name.set_font_weight(FontWeight::Bold);
        name.set_horizontal_alignment(HorizontalAlignment::Center);

        let method_text = TextBlock::new();
        method_text.set_text(Some(&format!("Passed via {method}")));
        method_text.set_font_size(11.0);
        method_text.set_foreground(white());

        let method_badge = Border::new();
        method_badge.set_background(Some(SolidColorBrush::with_color(parse_color("#2196F3")).into()));
        method_badge.set_corner_radius(CornerRadius::uniform(4.0));
        method_badge.set_padding(Thickness::symmetric(8.0, 4.0));
        method_badge.set_horizontal_alignment(HorizontalAlignment::Center);
        method_badge.set_child(method_text);

        let occupation = TextBlock::new();
        occupation.set_text(Some(contact.occupation));
        occupation.set_font_size(14.0);
        occupation.set_opacity(0.7);
        occupation.set_horizontal_alignment(HorizontalAlignment::Center);

        let country = TextBlock::new();
        country.set_text(Some(contact.country));
        country.set_font_size(13.0);
        country.set_opacity(0.5);
        country.set_horizontal_alignment(HorizontalAlignment::Center);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(12.0);
        panel.children().add(avatar);
        panel.children().add(name);
        panel.children().add(method_badge);
        panel.children().add(occupation);
        panel.children().add(country);
        panel.upcast()
    }

    fn append_navigation_log(&self, message: &str) {
        let navigation_log = self.navigation_log();
        let current = navigation_log.text();
        navigation_log.set_text(Some(&match current.filter(|current| !current.is_empty()) {
            None => message.to_string(),
            Some(current) => format!("{current}\n{message}"),
        }));
    }
}
