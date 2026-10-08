//! Port of `Pages/NavigationPage/NavigationPageInteractiveHeaderPage.xaml.cs`: the class of the
//! document `Pages/NavigationPage/NavigationPageInteractiveHeaderPage.xaml`, and the record
//! `ContactItem` the file declares.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::data::model::BindableList;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::utilities::StringComparison;
use ferroui_base::{ferro_class_info, instantiate, Ref, Thickness};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    ColumnDefinitions, ContentPage, Control, Dock, DockPanel, Grid, ItemsSource, ListBox, NavigationPage, StackPanel,
    TextBlock, TextBox, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A contact of the list: its name and role.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactItem {
    name: String,
    role: String,
}

impl ContactItem {
    pub fn new(name: &str, role: &str) -> Rc<ContactItem> {
        Rc::new(Self { name: name.to_string(), role: role.to_string() })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn role(&self) -> &str {
        &self.role
    }
}

/// The static field `AllContacts`: the name and role of each contact.
const ALL_CONTACTS: [(&str, &str); 20] = [
    ("Alice Martin", "Engineering Lead"),
    ("Bob Chen", "Product Designer"),
    ("Carol White", "Frontend Developer"),
    ("David Kim", "Backend Developer"),
    ("Eva M\u{fc}ller", "UX Researcher"),
    ("Frank Lopez", "QA Engineer"),
    ("Grace Zhang", "Data Scientist"),
    ("Henry Brown", "DevOps Engineer"),
    ("Iris Patel", "Security Analyst"),
    ("Jack Robinson", "Mobile Developer"),
    ("Karen Lee", "Project Manager"),
    ("Liam Thompson", "Full-Stack Developer"),
    ("Maya Singh", "Backend Developer"),
    ("Noah Garcia", "iOS Developer"),
    ("Olivia Davis", "Android Developer"),
    ("Paul Wilson", "Systems Architect"),
    ("Quinn Adams", "Technical Writer"),
    ("Rachel Turner", "Data Engineer"),
    ("Samuel Hall", "Cloud Engineer"),
    ("Tina Scott", "UI Designer"),
];

/// `string.IsNullOrWhiteSpace(value)`.
fn is_null_or_white_space(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

#[repr(C)]
pub struct NavigationPageInteractiveHeaderPage {
    base: UserControl,
    filtered_items: Rc<BindableList<Rc<ContactItem>>>,
    initialized: Cell<bool>,
    search_text: RefCell<String>,
}

user_control_class!(NavigationPageInteractiveHeaderPage);
ferro_class_info!(NavigationPageInteractiveHeaderPage { new: NavigationPageInteractiveHeaderPage::new });
xaml_class!(NavigationPageInteractiveHeaderPage, "/Pages/NavigationPage/NavigationPageInteractiveHeaderPage.xaml");

impl NavigationPageInteractiveHeaderPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            filtered_items: BindableList::new(ALL_CONTACTS.into_iter().map(|(name, role)| ContactItem::new(name, role))),
            initialized: Cell::new(false),
            search_text: RefCell::new(String::new()),
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

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let header_grid = Grid::new();
        header_grid.set_column_definitions(match ColumnDefinitions::parse("*, Auto") {
            Ok(definitions) => definitions,
            Err(error) => panic!("{error}"),
        });
        header_grid.set_vertical_alignment(VerticalAlignment::Stretch);

        let title_block = TextBlock::new();
        title_block.set_text(Some("Contacts"));
        title_block.set_font_size(16.0);
        title_block.set_font_weight(FontWeight::SemiBold);
        title_block.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&title_block, 0);

        let search_box = TextBox::new();
        search_box.set_placeholder_text(Some("Search..."));
        search_box.set_width(140.0);
        search_box.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&search_box, 1);

        // The handler belongs to the search box, which ends up in a page of the navigation
        // page of this control: it holds the box and the control weakly.
        let weak = self.to_ref().downgrade();
        let weak_search_box = search_box.downgrade();
        search_box.text_changed(move |_, _| {
            let (Some(this), Some(search_box)) = (weak.upgrade(), weak_search_box.upgrade()) else {
                return;
            };
            *this.search_text.borrow_mut() = search_box.text().unwrap_or_default();
            this.apply_filter();
        });

        header_grid.children().add(title_block);
        header_grid.children().add(search_box);

        let result_label = TextBlock::new();
        result_label.set_text(Some(&format!("{} contacts", ALL_CONTACTS.len())));
        result_label.set_font_size(12.0);
        result_label.set_opacity(0.6);
        result_label.set_margin(Thickness::symmetric(16.0, 8.0));

        let list_box = ListBox::new();
        list_box.set_items_source(Some(ItemsSource::from(self.filtered_items.clone())));
        // Deviation (DEVIATIONS.md, Templates): the template of the original also matches a
        // null item, for which it builds an empty text block; here null is not a contact.
        list_box.set_item_template(Some(FuncDataTemplate::for_type::<Rc<ContactItem>>(
            |item, _| {
                let panel = StackPanel::new();
                panel.set_margin(Thickness::symmetric(4.0, 2.0));

                let name = TextBlock::new();
                name.set_text(Some(item.name()));
                name.set_font_size(14.0);
                name.set_font_weight(FontWeight::SemiBold);
                panel.children().add(name);

                let role = TextBlock::new();
                role.set_text(Some(item.role()));
                role.set_font_size(12.0);
                role.set_opacity(0.6);
                panel.children().add(role);
                Some(panel.upcast())
            },
            false,
        )));

        // The list belongs to this control: its handler holds the control weakly.
        let weak = self.to_ref().downgrade();
        let label = result_label.clone();
        self.filtered_items.items().add_collection_changed(Rc::new(move |_| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            label.set_text(Some(&if is_null_or_white_space(&this.search_text.borrow()) {
                format!("{} contacts", ALL_CONTACTS.len())
            } else {
                format!("{} of {} contacts", this.filtered_items.items().count(), ALL_CONTACTS.len())
            }));
        }));

        let content = DockPanel::new();
        DockPanel::set_dock(&result_label, Dock::Top);
        content.children().add(result_label);
        content.children().add(list_box);

        let page = ContentPage::new();
        page.set_header(Some(Control::boxed(header_grid)));
        page.set_content(Some(Control::boxed(content)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        drop(self.demo_nav().push_async_with_transition(page, None));
    }

    fn apply_filter(&self) {
        let search_text = self.search_text.borrow().clone();
        let items = self.filtered_items.items();
        items.clear();
        for (name, role) in ALL_CONTACTS {
            if search_text.is_empty()
                || StringComparison::OrdinalIgnoreCase.contains(name, &search_text)
                || StringComparison::OrdinalIgnoreCase.contains(role, &search_text)
            {
                items.add(ContactItem::new(name, role));
            }
        }
    }
}
