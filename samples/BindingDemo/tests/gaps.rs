//! The minimal reproductions of the gaps of the framework the sample found (`GAPS.md`): a
//! test of a gap that is fixed asserts what the framework does now, a test of an open gap
//! asserts what it does today and says what it should become.

use super::support::start_application;
use crate::view_models::MainWindowViewModel;
use crate::SAMPLE;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::{ContentControl, Control, DataValidationErrors, ListBox, StackPanel, TextBox, Window};
use sample_testing::documents::load_text;
use sample_testing::BindingReports;
use std::rc::Rc;

/// The children of a stack panel a document of the test declares, in a shown window whose
/// data context is a view model of the main window.
struct Fixture {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
    view_model: Rc<MainWindowViewModel>,
}

impl Fixture {
    /// `children` are the elements of the panel; its bindings are compiled against the
    /// view model of the main window.
    fn show(children: &str) -> Fixture {
        let root = load_text(
            &SAMPLE,
            &format!(
                r#"<StackPanel xmlns="https://github.com/ferroui"
                              xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                              xmlns:vm="using:BindingDemo.ViewModels"
                              x:DataType="vm:MainWindowViewModel">
                     {children}
                   </StackPanel>"#
            ),
        );
        let panel = from_markup_value::<Ref<StackPanel>>(&Some(root)).expect("a stack panel");
        let view_model = MainWindowViewModel::new();
        let window = Window::new();
        window.set_data_context(Some(view_model.clone() as BoxedValue));
        window.set_content(Some(Control::boxed(&panel)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        Fixture { window, panel, view_model }
    }

    fn child<T: ObjectType>(&self, index: usize) -> Ref<T> {
        self.panel.children().get(index).cast::<T>().expect("a child of the class")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn type_text(text_box: &Ref<TextBox>, text: &str) {
    text_box.set_current_value(TextBox::text_property(), Some(text.to_string()));
    Dispatcher::ui_thread().run_jobs(None);
}

/// B002 (open): a binding path cannot read a member of a selection model, which reaches
/// markup as its contract without members. The reflection binding of the content control of
/// the list box tab reports the error and delivers nothing. Closed, the content control
/// shows the first selected item (and reports that there is none while nothing is selected,
/// as the managed original does).
#[test]
fn b002_a_binding_path_cannot_read_the_selected_items_of_a_selection_model() {
    let reports = BindingReports::start();
    let _app = start_application();
    let fixture = Fixture::show(
        r#"<ListBox ItemsSource="{Binding Items}" SelectionMode="Multiple" Selection="{Binding Selection}"/>
           <ContentControl Content="{ReflectionBinding Selection.SelectedItems[0]}"/>"#,
    );
    fixture.view_model.selection().select(3);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(fixture.child::<ContentControl>(1).content().is_none(), "B002 is open: the path stops at the selection model");
    let lines: Vec<String> = reports.take().keys().map(|report| report.line()).collect();
    assert_eq!(
        vec![String::from(
            "ContentControl.Content <- Selection.SelectedItems[0] at SelectedItems: Could not find a matching property accessor for 'SelectedItems' on 'FerroUI.Controls.Selection.ISelectionModel'."
        )],
        lines
    );
}

/// B003 (fixed): the validation of a binding finds the error notifications of a view model
/// that is the value of a property of another one (a data context a binding delivered,
/// held as the handle of the view model).
#[test]
fn b003_a_view_model_read_from_a_property_reports_its_errors() {
    let _app = start_application();
    let fixture = Fixture::show(
        r#"<StackPanel DataContext="{Binding IndeiDataValidation}">
             <TextBox Text="{Binding Path=Maximum}"/>
             <TextBox Text="{Binding Path=Value}"/>
           </StackPanel>"#,
    );
    let inner = fixture.child::<StackPanel>(0);
    let value = inner.children().get(1).cast::<TextBox>().expect("the text box of the value");
    assert!(!DataValidationErrors::get_has_errors(&value));
    type_text(&value, "11");
    assert_eq!(11, fixture.view_model.indei_data_validation().value());
    assert!(DataValidationErrors::get_has_errors(&value), "the error of the view model is the error of the text box");
    assert_eq!(1, DataValidationErrors::get_errors(&value).unwrap_or_default().len());
    fixture.view_model.indei_data_validation().set_maximum(11);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!DataValidationErrors::get_has_errors(&value));
}

/// B010 (fixed): a binding path with an indexer over a list of a view model
/// (`Items[1].Value`) reads the item, writes to it, and follows the changes of the list.
/// This is the path the run-time loader builds; the main window is built from the compiled
/// markup of the same path (B008: `tests/shell.rs`, the basic tab, which presses "Shuffle").
#[test]
fn b010_an_indexer_of_a_binding_path_follows_the_changes_of_the_list() {
    let _app = start_application();
    let fixture = Fixture::show(r#"<TextBox Text="{Binding Path=Items[1].Value}"/>"#);
    let text_box = fixture.child::<TextBox>(0);
    assert_eq!(Some(String::from("Item 1")), text_box.text());

    let items = fixture.view_model.items();
    items.items().move_item(7, 1);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(Some(String::from("Item 7")), text_box.text(), "the item that moved to the index is shown");
    items.items().remove_at(0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(Some(String::from("Item 1")), text_box.text(), "a removal before the index moves the items");

    type_text(&text_box, "edited");
    assert_eq!(Some(String::from("edited")), items.items().get(1).value());
}

/// B009 (fixed): two list boxes bound to one list of a view model share its selection
/// model: the list is one collection to both, whichever binding delivered it.
#[test]
fn b009_two_list_boxes_over_one_list_share_a_selection_model() {
    let _app = start_application();
    let fixture = Fixture::show(
        r#"<ListBox ItemsSource="{Binding Items}" SelectionMode="Multiple" Selection="{Binding Selection}"/>
           <ListBox ItemsSource="{Binding Items}" SelectionMode="Multiple" Selection="{Binding Selection}"/>"#,
    );
    let (first, second) = (fixture.child::<ListBox>(0), fixture.child::<ListBox>(1));
    first.set_selected_index(2);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(2, fixture.view_model.selection().selected_index());
    assert_eq!(2, second.selected_index());
}
