//! The people page: the tab "People (v12 XAML)" of `MainWindow.axaml` of the reference demo, a
//! grid of an `ItemsSource` whose columns are declared in markup, with the handler of its
//! selection changes (`People_SelectionChanged` of `MainWindow.axaml.cs`), as a view of its own
//! that the main window and the ControlCatalog both show.

use crate::markup::{user_control_class, xaml_class};
use crate::models::Person;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBlock, UserControl};
use ferroui_controls_treedatagrid::{TreeDataGrid, TreeDataGridSelectionChangedEventArgs};
use ferroui_treedatagrid_core::selection::{IReadOnlyList, ITreeSelectionModel};
use std::any::Any;
use std::rc::Rc;

#[repr(C)]
pub struct PeopleXamlPage {
    base: UserControl,
}

user_control_class!(PeopleXamlPage);
ferro_class_info!(PeopleXamlPage {
    new: PeopleXamlPage::new,
    markup: {
        methods: [
            fn People_SelectionChanged(Option<BoxedValue>, TreeDataGridSelectionChangedEventArgs) =>
                |this: &Ref<PeopleXamlPage>, sender: Option<BoxedValue>, e: TreeDataGridSelectionChangedEventArgs| {
                    this.people_selection_changed(&sender, &e)
                },
        ],
    },
});
xaml_class!(PeopleXamlPage, "/Views/PeopleXamlPage.xaml");

/// The person an item of the selection holds (`OfType<Person>()`): the models of the source the
/// grid generates from its `ItemsSource` are the untyped items of the list, which the selection
/// boxes once more.
fn person_of(item: &BoxedValue) -> Option<Rc<Person>> {
    if let Some(inner) = item.downcast_ref::<BoxedValue>() {
        return person_of(inner);
    }
    if let Some(person) = item.downcast_ref::<Rc<Person>>() {
        return Some(person.clone());
    }
    let any: Rc<dyn Any> = item.clone();
    any.downcast::<Person>().ok()
}

impl PeopleXamlPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn people_selection_changed(&self, sender: &Option<BoxedValue>, _e: &TreeDataGridSelectionChangedEventArgs) {
        let Some(text_block) = self.find_control::<TextBlock>("peopleSelectionText") else {
            return;
        };

        let current_selection: Option<Vec<String>> = from_markup_value::<Ref<TreeDataGrid>>(sender)
            .and_then(|tree_data_grid| tree_data_grid.row_selection())
            .map(|selection: Rc<dyn ITreeSelectionModel>| {
                let selected_items = selection.selected_items();
                let names: Vec<String> = selected_items
                    .iter()
                    .flatten()
                    .filter_map(|item| person_of(&item))
                    // `string.Join` writes a null name as an empty string.
                    .map(|x| x.name().unwrap_or_default())
                    .collect();
                names
            });

        let text = match current_selection {
            Some(current_selection) if !current_selection.is_empty() => {
                format!("Selection: {}", current_selection.join(", "))
            }
            _ => "Selection: none".to_string(),
        };
        text_block.set_text(Some(&text));
    }
}
