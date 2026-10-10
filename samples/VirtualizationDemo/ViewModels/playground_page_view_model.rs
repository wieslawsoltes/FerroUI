//! Port of `ViewModels/PlaygroundPageViewModel.cs`.

use super::random::Random;
use super::PlaygroundItemViewModel;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::selection::{ISelectionModel, SelectionModel};
use ferroui_controls::SelectionMode;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_controls::ferro_markup_list!(pub PlaygroundItemList: Rc<PlaygroundItemViewModel>);

pub struct PlaygroundPageViewModel {
    base: ViewModelBase,
    selection_mode: Cell<SelectionMode>,
    scroll_to_index: Cell<i32>,
    new_item_header: RefCell<Option<String>>,
    items: FerroList<Rc<PlaygroundItemViewModel>>,
    selection: Rc<SelectionModel<Rc<PlaygroundItemViewModel>>>,
}

impl PartialEq for PlaygroundPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PlaygroundPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PlaygroundPageViewModel {
    pub fn new() -> Rc<PlaygroundPageViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            selection_mode: Cell::new(SelectionMode::MULTIPLE),
            scroll_to_index: Cell::new(500),
            new_item_header: RefCell::new(Some(String::from("New Item 1"))),
            items: FerroList::from_items((0..1000).map(PlaygroundItemViewModel::new)),
            selection: SelectionModel::new(),
        })
    }

    pub fn items(&self) -> FerroList<Rc<PlaygroundItemViewModel>> {
        self.items.clone()
    }

    pub fn multiple(&self) -> bool {
        self.selection_mode.get().contains(SelectionMode::MULTIPLE)
    }

    pub fn set_multiple(&self, value: bool) {
        self.set_selection_mode_flag(SelectionMode::MULTIPLE, value);
    }

    pub fn toggle(&self) -> bool {
        self.selection_mode.get().contains(SelectionMode::TOGGLE)
    }

    pub fn set_toggle(&self, value: bool) {
        self.set_selection_mode_flag(SelectionMode::TOGGLE, value);
    }

    pub fn always_selected(&self) -> bool {
        self.selection_mode.get().contains(SelectionMode::ALWAYS_SELECTED)
    }

    pub fn set_always_selected(&self, value: bool) {
        self.set_selection_mode_flag(SelectionMode::ALWAYS_SELECTED, value);
    }

    pub fn selection(&self) -> Rc<SelectionModel<Rc<PlaygroundItemViewModel>>> {
        self.selection.clone()
    }

    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode.get()
    }

    pub fn set_selection_mode(&self, value: SelectionMode) {
        self.base.raise_and_set_if_changed_cell(&self.selection_mode, value, "SelectionMode");
    }

    pub fn scroll_to_index(&self) -> i32 {
        self.scroll_to_index.get()
    }

    pub fn set_scroll_to_index(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.scroll_to_index, value, "ScrollToIndex");
    }

    pub fn new_item_header(&self) -> Option<String> {
        self.new_item_header.borrow().clone()
    }

    pub fn set_new_item_header(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.new_item_header, value, "NewItemHeader");
    }

    pub fn execute_scroll_to_index(&self) {
        self.selection.select(self.scroll_to_index());
    }

    pub fn randomize_scroll_to_index(&self) {
        let mut rnd = Random::new();
        self.set_scroll_to_index(rnd.next_max(self.items.count() as i32));
    }

    pub fn add_at_selected_index(&self) {
        if self.selection.selected_index() == -1 {
            return;
        }
        self.items.insert(self.selection.selected_index() as usize, PlaygroundItemViewModel::with_header(self.new_item_header()));
    }

    pub fn delete_selected_item(&self) {
        let count = self.selection.count();
        for i in (0..count).rev() {
            self.items.remove_at(self.selection.selected_indexes().get(i) as usize);
        }
    }

    /// `SetSelectionMode(mode, value)`.
    fn set_selection_mode_flag(&self, mode: SelectionMode, value: bool) {
        if value {
            self.set_selection_mode(self.selection_mode() | mode);
        } else {
            self.set_selection_mode(self.selection_mode() & !mode);
        }
    }
}

ferro_markup_type!(class PlaygroundPageViewModel {
    this: Rc<PlaygroundPageViewModel>,
    handles: [PlaygroundPageViewModel, Rc<PlaygroundPageViewModel>, Option<Rc<PlaygroundPageViewModel>>],
    constructors: [() => PlaygroundPageViewModel::new],
    properties: [
        Items: FerroList<Rc<PlaygroundItemViewModel>> { get: |this: &Rc<PlaygroundPageViewModel>| this.items() },
        Multiple: bool {
            get: |this: &Rc<PlaygroundPageViewModel>| this.multiple(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: bool| this.set_multiple(value)
        },
        Toggle: bool {
            get: |this: &Rc<PlaygroundPageViewModel>| this.toggle(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: bool| this.set_toggle(value)
        },
        AlwaysSelected: bool {
            get: |this: &Rc<PlaygroundPageViewModel>| this.always_selected(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: bool| this.set_always_selected(value)
        },
        Selection: Rc<dyn ISelectionModel> {
            get: |this: &Rc<PlaygroundPageViewModel>| this.selection() as Rc<dyn ISelectionModel>
        },
        SelectionMode: SelectionMode {
            get: |this: &Rc<PlaygroundPageViewModel>| this.selection_mode(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: SelectionMode| this.set_selection_mode(value)
        },
        ScrollToIndex: i32 {
            get: |this: &Rc<PlaygroundPageViewModel>| this.scroll_to_index(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: i32| this.set_scroll_to_index(value)
        },
        NewItemHeader: Option<String> {
            get: |this: &Rc<PlaygroundPageViewModel>| this.new_item_header(),
            set: |this: &Rc<PlaygroundPageViewModel>, value: Option<String>| this.set_new_item_header(value)
        },
    ],
    methods: [
        fn ExecuteScrollToIndex() => |this: &Rc<PlaygroundPageViewModel>| this.execute_scroll_to_index(),
        fn RandomizeScrollToIndex() => |this: &Rc<PlaygroundPageViewModel>| this.randomize_scroll_to_index(),
        fn AddAtSelectedIndex() => |this: &Rc<PlaygroundPageViewModel>| this.add_at_selected_index(),
        fn DeleteSelectedItem() => |this: &Rc<PlaygroundPageViewModel>| this.delete_selected_item(),
    ],
    notify_property_changed: PlaygroundPageViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::ItemsSource;

    /// The view model with the source of its selection, which the list box of the page sets
    /// (the `ItemsSource` and `Selection` bindings).
    fn view_model() -> Rc<PlaygroundPageViewModel> {
        let view_model = PlaygroundPageViewModel::new();
        view_model.selection().set_source(Some(ItemsSource::from(Rc::new(view_model.items()))));
        view_model
    }

    #[test]
    fn the_view_model_starts_with_a_thousand_items_and_a_multiple_selection() {
        let view_model = view_model();
        assert_eq!(1000, view_model.items().count());
        assert_eq!(Some(String::from("Item 0")), view_model.items().get(0).header());
        assert_eq!(Some(String::from("Item 999")), view_model.items().get(999).header());
        assert_eq!(SelectionMode::MULTIPLE, view_model.selection_mode());
        assert!(view_model.multiple() && !view_model.toggle() && !view_model.always_selected());
        assert_eq!(500, view_model.scroll_to_index());
        assert_eq!(Some(String::from("New Item 1")), view_model.new_item_header());
        assert_eq!(-1, view_model.selection().selected_index());
    }

    #[test]
    fn the_three_switches_change_the_selection_mode_and_notify_it() {
        let view_model = view_model();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_toggle(true);
        assert_eq!(SelectionMode::MULTIPLE | SelectionMode::TOGGLE, view_model.selection_mode());
        view_model.set_always_selected(true);
        view_model.set_multiple(false);
        assert_eq!(SelectionMode::TOGGLE | SelectionMode::ALWAYS_SELECTED, view_model.selection_mode());
        // A switch that is where it is asked to be changes nothing.
        view_model.set_multiple(false);
        assert_eq!(vec!["SelectionMode".to_string(); 3], *seen.borrow());
    }

    #[test]
    fn execute_selects_the_index_and_randomize_picks_one_of_the_list() {
        let view_model = view_model();
        view_model.execute_scroll_to_index();
        assert_eq!(500, view_model.selection().selected_index());
        for _ in 0..50 {
            view_model.randomize_scroll_to_index();
            assert!((0..1000).contains(&view_model.scroll_to_index()));
        }
    }

    #[test]
    fn add_inserts_at_the_selected_index_and_delete_removes_the_selected_items() {
        let view_model = view_model();
        // Nothing is selected: nothing is added.
        view_model.add_at_selected_index();
        assert_eq!(1000, view_model.items().count());

        view_model.selection().select(3);
        view_model.add_at_selected_index();
        assert_eq!(1001, view_model.items().count());
        assert_eq!(Some(String::from("New Item 1")), view_model.items().get(3).header());
        // The selected item moved down with its selection.
        assert_eq!(4, view_model.selection().selected_index());
        assert_eq!(Some(String::from("Item 3")), view_model.items().get(4).header());

        view_model.delete_selected_item();
        assert_eq!(1000, view_model.items().count());
        assert_eq!(Some(String::from("Item 4")), view_model.items().get(4).header());
    }
}
