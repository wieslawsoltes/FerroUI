//! Port of `ViewModels/ListBoxPageViewModel.cs`.

use super::random::Random;
use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{IDisposable, IObservable};
use ferroui_controls::selection::{ISelectionModel, SelectionModel};
use ferroui_controls::{ItemsSource, SelectionMode};
use mini_mvvm::{MiniCommand, PropertyChangedExtensions, ViewModelBase};
use std::cell::{Cell, OnceCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// The view model of the list box page.
pub struct ListBoxPageViewModel {
    base: ViewModelBase,
    multiple: Cell<bool>,
    toggle: Cell<bool>,
    always_selected: Cell<bool>,
    auto_scroll_to_selected_item: Cell<bool>,
    wrap_selection: Cell<bool>,
    counter: Cell<i32>,
    selection_mode: OnceCell<Rc<dyn IObservable<SelectionMode>>>,
    items: Rc<BindableList<Rc<ItemModel>>>,
    selection: Rc<SelectionModel<Rc<ItemModel>>>,
    add_item_command: Rc<MiniCommand>,
    remove_item_command: Rc<MiniCommand>,
    select_random_item_command: Rc<MiniCommand>,
}

impl PartialEq for ListBoxPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ListBoxPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ListBoxPageViewModel {
    pub fn new() -> Rc<ListBoxPageViewModel> {
        let this = Rc::new_cyclic(|this: &Weak<ListBoxPageViewModel>| {
            let counter = Cell::new(0);
            let items = BindableList::new((1..=10000).map(|_| {
                let id = counter.get();
                counter.set(id + 1);
                ItemModel::new(id)
            }));

            let selection = SelectionModel::<Rc<ItemModel>>::new();
            selection.select(1);

            let add_item_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    let item = this.generate_item();
                    this.items.items().add(item);
                    this.selection.clear();
                    this.selection.select(this.items.items().count() as i32 - 1);
                })
            };

            let remove_item_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    let items: Vec<Option<Rc<ItemModel>>> = this.selection.selected_items().iter().collect();

                    for item in items {
                        // A selected item of this list is never null.
                        this.items.items().remove(&item.expect("a selected item"));
                    }
                })
            };

            let select_random_item_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    let mut random = Random::new();

                    let update = this.selection.batch_update();
                    this.selection.clear();
                    this.selection.select(random.next_max(this.items.items().count() as i32 - 1));
                    update.dispose();
                })
            };

            Self {
                base: ViewModelBase::new(),
                multiple: Cell::new(false),
                toggle: Cell::new(false),
                always_selected: Cell::new(false),
                auto_scroll_to_selected_item: Cell::new(true),
                wrap_selection: Cell::new(false),
                counter,
                selection_mode: OnceCell::new(),
                items,
                selection,
                add_item_command,
                remove_item_command,
                select_random_item_command,
            }
        });

        let selection_mode = PropertyChangedExtensions::when_any_value3(
            &this,
            ("Multiple", |x: &ListBoxPageViewModel| x.multiple()),
            ("Toggle", |x: &ListBoxPageViewModel| x.toggle()),
            ("AlwaysSelected", |x: &ListBoxPageViewModel| x.always_selected()),
            |m, t, a| {
                (if m { SelectionMode::MULTIPLE } else { SelectionMode::empty() })
                    | (if t { SelectionMode::TOGGLE } else { SelectionMode::empty() })
                    | (if a { SelectionMode::ALWAYS_SELECTED } else { SelectionMode::empty() })
            },
        );
        if this.selection_mode.set(selection_mode).is_err() {
            unreachable!("the selection mode is set once");
        }

        this
    }

    pub fn items(&self) -> Rc<BindableList<Rc<ItemModel>>> {
        self.items.clone()
    }

    pub fn selection(&self) -> Rc<SelectionModel<Rc<ItemModel>>> {
        self.selection.clone()
    }

    pub fn selection_mode(&self) -> Rc<dyn IObservable<SelectionMode>> {
        self.selection_mode.get().expect("the selection mode is set by the constructor").clone()
    }

    pub fn multiple(&self) -> bool {
        self.multiple.get()
    }

    pub fn set_multiple(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.multiple, value, "Multiple");
    }

    pub fn toggle(&self) -> bool {
        self.toggle.get()
    }

    pub fn set_toggle(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.toggle, value, "Toggle");
    }

    pub fn always_selected(&self) -> bool {
        self.always_selected.get()
    }

    pub fn set_always_selected(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.always_selected, value, "AlwaysSelected");
    }

    pub fn auto_scroll_to_selected_item(&self) -> bool {
        self.auto_scroll_to_selected_item.get()
    }

    pub fn set_auto_scroll_to_selected_item(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.auto_scroll_to_selected_item, value, "AutoScrollToSelectedItem");
    }

    pub fn wrap_selection(&self) -> bool {
        self.wrap_selection.get()
    }

    pub fn set_wrap_selection(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.wrap_selection, value, "WrapSelection");
    }

    pub fn add_item_command(&self) -> Rc<MiniCommand> {
        self.add_item_command.clone()
    }

    pub fn remove_item_command(&self) -> Rc<MiniCommand> {
        self.remove_item_command.clone()
    }

    pub fn select_random_item_command(&self) -> Rc<MiniCommand> {
        self.select_random_item_command.clone()
    }

    fn generate_item(&self) -> Rc<ItemModel> {
        let id = self.counter.get();
        self.counter.set(id + 1);
        ItemModel::new(id)
    }
}

ferro_markup_type!(class ListBoxPageViewModel {
    this: Rc<ListBoxPageViewModel>,
    handles: [ListBoxPageViewModel, Rc<ListBoxPageViewModel>, Option<Rc<ListBoxPageViewModel>>],
    constructors: [() => ListBoxPageViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        Items: ItemsSource { get: |this: &Rc<ListBoxPageViewModel>| ItemsSource::from(this.items()) },
        Selection: Rc<dyn ISelectionModel> {
            get: |this: &Rc<ListBoxPageViewModel>| this.selection() as Rc<dyn ISelectionModel>
        },
        SelectionMode: ObservableValue {
            get: |this: &Rc<ListBoxPageViewModel>| ObservableValue::new(this.selection_mode())
        },
        Multiple: bool {
            get: |this: &Rc<ListBoxPageViewModel>| this.multiple(),
            set: |this: &Rc<ListBoxPageViewModel>, value: bool| this.set_multiple(value)
        },
        Toggle: bool {
            get: |this: &Rc<ListBoxPageViewModel>| this.toggle(),
            set: |this: &Rc<ListBoxPageViewModel>, value: bool| this.set_toggle(value)
        },
        AlwaysSelected: bool {
            get: |this: &Rc<ListBoxPageViewModel>| this.always_selected(),
            set: |this: &Rc<ListBoxPageViewModel>, value: bool| this.set_always_selected(value)
        },
        AutoScrollToSelectedItem: bool {
            get: |this: &Rc<ListBoxPageViewModel>| this.auto_scroll_to_selected_item(),
            set: |this: &Rc<ListBoxPageViewModel>, value: bool| this.set_auto_scroll_to_selected_item(value)
        },
        WrapSelection: bool {
            get: |this: &Rc<ListBoxPageViewModel>| this.wrap_selection(),
            set: |this: &Rc<ListBoxPageViewModel>, value: bool| this.set_wrap_selection(value)
        },
        AddItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<ListBoxPageViewModel>| this.add_item_command().as_command()
        },
        RemoveItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<ListBoxPageViewModel>| this.remove_item_command().as_command()
        },
        SelectRandomItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<ListBoxPageViewModel>| this.select_random_item_command().as_command()
        },
    ],
    notify_property_changed: ListBoxPageViewModel,
});

/// An item model for the list box page.
pub struct ItemModel {
    id: i32,
}

impl PartialEq for ItemModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ItemModel {
    /// Creates a new item model with the given ID.
    pub fn new(id: i32) -> Rc<ItemModel> {
        Rc::new(Self { id })
    }

    /// The ID of this item.
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// `ToString()`.
impl fmt::Display for ItemModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Item {}", self.id)
    }
}

ferro_markup_type!(class ItemModel {
    this: Rc<ItemModel>,
    handles: [ItemModel, Rc<ItemModel>, Option<Rc<ItemModel>>],
    constructors: [(i32) => ItemModel::new],
    properties: [ID: i32 { get: |this: &Rc<ItemModel>| this.id() }],
    methods: [fn ToString() -> String => |this: &Rc<ItemModel>| this.to_string()],
});


#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::reactive::AnonymousObserver;
    use std::cell::RefCell;

    fn with_source(view_model: &Rc<ListBoxPageViewModel>) {
        // What the list box does with the `ItemsSource` and `Selection` bindings.
        view_model.selection().set_source(Some(ItemsSource::from(view_model.items())));
    }

    #[test]
    fn the_list_starts_with_ten_thousand_items_and_the_second_selected() {
        let view_model = ListBoxPageViewModel::new();
        with_source(&view_model);
        let items = view_model.items();
        assert_eq!(10000, items.items().count());
        assert_eq!(0, items.items().get(0).id());
        assert_eq!("Item 9999", items.items().get(9999).to_string());
        assert_eq!(1, view_model.selection().selected_index());
        assert!(view_model.auto_scroll_to_selected_item());
    }

    #[test]
    fn add_appends_an_item_and_selects_it() {
        let view_model = ListBoxPageViewModel::new();
        with_source(&view_model);
        view_model.add_item_command().execute(None);
        assert_eq!(10001, view_model.items().items().count());
        assert_eq!(10000, view_model.items().items().get(10000).id());
        assert_eq!(10000, view_model.selection().selected_index());
    }

    #[test]
    fn remove_removes_the_selected_items() {
        let view_model = ListBoxPageViewModel::new();
        with_source(&view_model);
        view_model.remove_item_command().execute(None);
        assert_eq!(9999, view_model.items().items().count());
        assert_eq!(2, view_model.items().items().get(1).id());
    }

    #[test]
    fn select_random_selects_one_item() {
        let view_model = ListBoxPageViewModel::new();
        with_source(&view_model);
        view_model.select_random_item_command().execute(None);
        assert_eq!(1, view_model.selection().selected_indexes().count());
        assert!((0..9999).contains(&view_model.selection().selected_index()));
    }

    #[test]
    fn the_selection_mode_follows_the_three_switches() {
        let view_model = ListBoxPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let subscription =
            view_model.selection_mode().subscribe(Rc::new(AnonymousObserver::new(move |mode| sink.borrow_mut().push(mode))));

        view_model.set_multiple(true);
        view_model.set_always_selected(true);
        view_model.set_toggle(true);
        view_model.set_multiple(false);
        assert_eq!(
            vec![
                SelectionMode::SINGLE,
                SelectionMode::MULTIPLE,
                SelectionMode::MULTIPLE | SelectionMode::ALWAYS_SELECTED,
                SelectionMode::MULTIPLE | SelectionMode::ALWAYS_SELECTED | SelectionMode::TOGGLE,
                SelectionMode::ALWAYS_SELECTED | SelectionMode::TOGGLE,
            ],
            *seen.borrow()
        );
        subscription.dispose();
    }
}
