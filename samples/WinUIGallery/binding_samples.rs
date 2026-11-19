//! The code behind of the pinned Binding page in Rust: `BindingPage.xaml.cs` with its models
//! `ExampleViewModel` and `ListDetailItem`.
//!
//! A native page has no `x:Class`, so the source's x:Bind of the page's own members is the code
//! behind here: `GreetingMessage` is set once (a OneTime x:Bind), `FormatDate(DatePickerControl
//! .SelectedDate)` follows the date picker, and the view model is the page's `DataContext`, which
//! the page reads with `{Binding}` over the markup types declared here. The view model's values do
//! not change, so it raises no property changes.
//!
//! The code behind of the Templates page (`TemplatesPage.xaml.cs`) is here too: the data template
//! the constructor creates in code, and `LayoutSelector_SelectionChanged`, whose two
//! ItemsPanelTemplates are styles of the list that a class switches.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::layout::Orientation;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeOffset};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    ComboBox, Control, DatePicker, FontIcon, ItemsSource, ListBox, RadioButtons, StackPanel, TextBlock,
};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Binding" => {
            register();
            wire_binding(root);
        }
        "Templates" => wire_templates(root),
        _ => {}
    }
}

/// `ExampleViewModel`.
pub(crate) struct ExampleViewModel {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) null_string: String,
}

impl PartialEq for ExampleViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class ExampleViewModel {
    this: Rc<ExampleViewModel>,
    handles: [ExampleViewModel, Rc<ExampleViewModel>, Option<Rc<ExampleViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<ExampleViewModel>| this.title.clone() },
        Description: String { get: |this: &Rc<ExampleViewModel>| this.description.clone() },
        NullString: String { get: |this: &Rc<ExampleViewModel>| this.null_string.clone() },
    ],
});

/// `ListDetailItem`.
pub(crate) struct ListDetailItem {
    pub(crate) id: i32,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) date_created: DateTime,
}

impl PartialEq for ListDetailItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ListDetailItem {
    /// `DateCreatedFormatted`: `MMM d, yyyy h:mm tt` of the current culture.
    pub(crate) fn date_created_formatted(&self) -> String {
        self.date_created
            .to_string_format("MMM d, yyyy h:mm tt", &CultureInfo::current_formatting())
    }
}

ferro_markup_type!(class ListDetailItem {
    this: Rc<ListDetailItem>,
    handles: [ListDetailItem, Rc<ListDetailItem>, Option<Rc<ListDetailItem>>],
    properties: [
        Id: i32 { get: |this: &Rc<ListDetailItem>| this.id },
        Title: String { get: |this: &Rc<ListDetailItem>| this.title.clone() },
        Text: String { get: |this: &Rc<ListDetailItem>| this.text.clone() },
        DateCreatedFormatted: String { get: |this: &Rc<ListDetailItem>| this.date_created_formatted() },
    ],
});

/// The markup types of the page, for its `{Binding}` paths.
fn register() {
    MarkupType::register_all(&[
        <ExampleViewModel as MarkupTyped>::MARKUP,
        <ListDetailItem as MarkupTyped>::MARKUP,
    ]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<ExampleViewModel>();
    ValueTypes::register_reference::<ListDetailItem>();
}

/// `GreetingMessage`.
pub(crate) const GREETING_MESSAGE: &str = "Hello, WinUI 3!";

/// The constructor's `ViewModel`.
pub(crate) fn view_model() -> Rc<ExampleViewModel> {
    Rc::new(ExampleViewModel {
        title: "Welcome to WinUI 3".to_string(),
        description: "This is an example of binding to a view model.".to_string(),
        null_string: String::new(),
    })
}

/// The constructor's `Items`.
pub(crate) fn items() -> Vec<Rc<ListDetailItem>> {
    [
        (
            "Item 1",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer id facilisis lectus. Cras nec convallis ante, quis pulvinar tellus.",
            (2025, 6, 15, 9, 30),
        ),
        (
            "Item 2",
            "Quisque accumsan pretium ligula in faucibus. Mauris sollicitudin augue vitae lorem cursus condimentum quis ac mauris.",
            (2025, 7, 22, 14, 15),
        ),
        (
            "Item 3",
            "Ut consequat magna luctus justo egestas vehicula. Integer pharetra risus libero, et posuere justo mattis et.",
            (2025, 8, 3, 11, 0),
        ),
        (
            "Item 4",
            "Duis facilisis, quam ut laoreet commodo, elit ex aliquet massa, non varius tellus lectus et nunc.",
            (2025, 9, 10, 16, 45),
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (title, text, (year, month, day, hour, minute)))| {
        Rc::new(ListDetailItem {
            id: id as i32,
            title: title.to_string(),
            text: text.to_string(),
            date_created: DateTime::new_with_time(year, month, day, hour, minute, 0),
        })
    })
    .collect()
}

/// `FormatDate`: the selected date in the long form, or "No date selected".
pub(crate) fn format_date(date: Option<DateTimeOffset>) -> String {
    match date {
        Some(date) => format!(
            "Selected date is: {}",
            date.to_string_format("dddd, MMMM d, yyyy", &CultureInfo::current_formatting())
        ),
        None => "No date selected".to_string(),
    }
}

fn wire_binding(root: &Ref<Control>) {
    // The constructor: the view model is the page's DataContext.
    root.set_data_context(Some(Rc::new(view_model()) as BoxedValue));
    root.get_control::<TextBlock>("GreetingMessageText")
        .set_text(Some(GREETING_MESSAGE));
    // `{x:Bind FormatDate(DatePickerControl.SelectedDate), Mode=OneWay}`.
    let picker = root.get_control::<DatePicker>("DatePickerControl");
    let output = root.get_control::<TextBlock>("FormattedDate");
    output.set_text(Some(&format_date(picker.selected_date())));
    let weak_picker = picker.downgrade();
    let weak_output = output.downgrade();
    picker.property_changed(move |change| {
        if change.property() != DatePicker::selected_date_property().as_property() {
            return;
        }
        if let (Some(picker), Some(output)) = (weak_picker.upgrade(), weak_output.upgrade()) {
            output.set_text(Some(&format_date(picker.selected_date())));
        }
    });
    // `ItemsSource="{x:Bind Items}"` and `SelectedIndex="0"`.
    let list = root.get_control::<ListBox>("ListDetailListView");
    list.set_items_source(Some(ItemsSource::from_items(
        items().into_iter().map(|item| Some(Rc::new(item) as BoxedValue)),
    )));
    list.set_selected_index(0);
}

/// The constructor of `TemplatesPage`: `new DataTemplate(() => ...)`, a glyph and the item's text
/// (`SetBinding(TextBlock.TextProperty, new Binding())`).
pub(crate) fn code_template() -> Rc<FuncDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let icon = FontIcon::new();
            icon.set_glyph("\u{E734}");
            icon.set_font_size(14.0);
            let text = TextBlock::new();
            text.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("."));
            let panel = StackPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            panel.set_spacing(8.0);
            panel.children().add(icon);
            panel.children().add(text);
            Some(panel.upcast())
        },
        false,
    )
}

fn wire_templates(root: &Ref<Control>) {
    root.get_control::<ComboBox>("CodeTemplateComboBox")
        .set_item_template(Some(code_template()));
    // `LayoutSelector_SelectionChanged`: the Tag of the selected option names the template.
    let selector = root.get_control::<RadioButtons>("LayoutSelector");
    let weak_selector = selector.downgrade();
    let weak_list = root.get_control::<ListBox>("MyListView").downgrade();
    selector.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(selector), Some(list)) = (weak_selector.upgrade(), weak_list.upgrade()) else {
            return;
        };
        let tag = selector
            .selected_item()
            .and_then(|item| Control::from_boxed(&item))
            .and_then(|item| item.tag())
            .and_then(|tag| tag.downcast_ref::<String>().cloned());
        match tag.as_deref() {
            Some("WrapGrid") => list.classes().set("StackPanel", false),
            Some("StackPanel") => list.classes().set("StackPanel", true),
            _ => {}
        }
    });
}
