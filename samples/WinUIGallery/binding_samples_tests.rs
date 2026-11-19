//! The Binding and Templates pages: the source's examples with their code behind
//! (`binding_samples.rs`). The x:Bind of the Binding page's elements is in `x_bind_tests.rs`.

use crate::binding_samples::{code_template, format_date, items, ExampleViewModel, ListDetailItem, GREETING_MESSAGE};
use crate::{examples, tests::start};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ComboBox, Control, DatePicker, ListBox, RadioButtons, StackPanel, TextBlock, TextBox};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

#[test]
fn binding_page_code_behind_sets_the_greeting_the_date_and_the_view_model() {
    let _application = start();
    let root = page("Binding");
    assert_eq!(text(&root, "GreetingMessageText").as_deref(), Some(GREETING_MESSAGE));
    // FormatDate of the date picker's SelectedDate.
    assert_eq!(text(&root, "FormattedDate").as_deref(), Some("No date selected"));
    let date = DateTimeOffset::now();
    root.get_control::<DatePicker>("DatePickerControl")
        .set_selected_date(Some(date));
    assert_eq!(text(&root, "FormattedDate"), Some(format_date(Some(date))));
    assert!(format_date(Some(date)).starts_with("Selected date is: "));
    // The view model is the page's DataContext.
    let model = root
        .data_context()
        .and_then(|data| data.downcast_ref::<Rc<ExampleViewModel>>().cloned());
    let model = model.expect("the view model");
    assert_eq!(model.title, "Welcome to WinUI 3");
    assert_eq!(model.null_string, "");
    let _ = root.get_control::<TextBox>("InputTextBox");
}

#[test]
fn binding_page_list_shows_the_items_and_the_detail_of_the_selection() {
    let _application = start();
    let root = page("Binding");
    let titles: Vec<_> = items().iter().map(|item| item.title.clone()).collect();
    assert_eq!(titles, ["Item 1", "Item 2", "Item 3", "Item 4"]);
    let list = root.get_control::<ListBox>("ListDetailListView");
    assert_eq!(list.selected_index(), 0);
    // `DataContext="{x:Bind ListDetailListView.SelectedItem, Mode=OneWay}"`.
    let detail = root.get_control::<StackPanel>("ListDetail");
    let selected = |detail: &StackPanel| {
        detail
            .data_context()
            .and_then(|data: BoxedValue| data.downcast_ref::<Rc<ListDetailItem>>().map(|item| item.title.clone()))
    };
    assert_eq!(selected(&detail).as_deref(), Some("Item 1"));
    list.set_selected_index(2);
    assert_eq!(selected(&detail).as_deref(), Some("Item 3"));
    assert!(!items()[0].date_created_formatted().is_empty());
}

#[test]
fn templates_page_code_template_and_items_panels_follow_the_source() {
    use ferroui_controls::templates::IDataTemplate;
    let _application = start();
    let root = page("Templates");
    // The constructor's DataTemplate in code: a glyph and the bound text.
    assert!(root
        .get_control::<ComboBox>("CodeTemplateComboBox")
        .item_template()
        .is_some());
    let template: Rc<dyn IDataTemplate> = code_template();
    let panel = template
        .build(&Some(Rc::new("Option 2".to_string()) as BoxedValue))
        .and_then(|panel| panel.cast::<StackPanel>())
        .expect("the template's panel");
    assert_eq!(panel.children().count(), 2);
    assert!(template.match_(None));
    // LayoutSelector_SelectionChanged.
    let list = root.get_control::<ListBox>("MyListView");
    assert!(!list.classes().contains("StackPanel"));
    root.get_control::<RadioButtons>("LayoutSelector").set_selected_index(1);
    assert!(list.classes().contains("StackPanel"));
    root.get_control::<RadioButtons>("LayoutSelector").set_selected_index(0);
    assert!(!list.classes().contains("StackPanel"));
    let _ = root.get_control::<TextBox>("CustomTemplateTextBox");
}
