//! Port of `ViewModels/FlexViewModel.cs`.

use super::FlexItemViewModel;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_controls::flex_panel::{
    FlexAlignContent, FlexAlignItems, FlexBasisKind, FlexDirection, FlexJustifyContent, FlexWrap,
};
use ferroui_controls::ItemsSource;
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The view model of the flex page.
pub struct FlexViewModel {
    base: ViewModelBase,
    numbers: Rc<BindableList<Rc<FlexItemViewModel>>>,
    direction_values: Rc<BindableList<FlexDirection>>,
    justify_content_values: Rc<BindableList<FlexJustifyContent>>,
    align_items_values: Rc<BindableList<FlexAlignItems>>,
    align_content_values: Rc<BindableList<FlexAlignContent>>,
    wrap_values: Rc<BindableList<FlexWrap>>,
    flex_basis_kind_values: Rc<BindableList<FlexBasisKind>>,
    horizontal_alignment_values: Rc<BindableList<HorizontalAlignment>>,
    vertical_alignment_values: Rc<BindableList<VerticalAlignment>>,
    align_self_values: Rc<BindableList<Option<FlexAlignItems>>>,
    direction: Cell<FlexDirection>,
    justify_content: Cell<FlexJustifyContent>,
    align_items: Cell<FlexAlignItems>,
    align_content: Cell<FlexAlignContent>,
    wrap: Cell<FlexWrap>,
    column_spacing: Cell<i32>,
    row_spacing: Cell<i32>,
    current_number: Cell<i32>,
    selected_item: RefCell<Option<Rc<FlexItemViewModel>>>,
    add_item_command: Rc<MiniCommand>,
    remove_item_command: Rc<MiniCommand>,
}

impl PartialEq for FlexViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FlexViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

/// The values of the alignment of the items of a line, in the order of
/// their declaration.
const ALIGN_ITEMS_VALUES: [FlexAlignItems; 4] =
    [FlexAlignItems::FlexStart, FlexAlignItems::FlexEnd, FlexAlignItems::Center, FlexAlignItems::Stretch];

impl FlexViewModel {
    pub fn new() -> Rc<FlexViewModel> {
        Rc::new_cyclic(|this: &Weak<FlexViewModel>| {
            let add_item_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.add_item();
                    }
                })
            };
            let remove_item_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.remove_item();
                    }
                })
            };

            Self {
                base: ViewModelBase::new(),
                numbers: BindableList::new((1..=40).map(FlexItemViewModel::new)),
                // The values of the enumerations, in the order of their declaration.
                direction_values: BindableList::new([
                    FlexDirection::Row,
                    FlexDirection::RowReverse,
                    FlexDirection::Column,
                    FlexDirection::ColumnReverse,
                ]),
                justify_content_values: BindableList::new([
                    FlexJustifyContent::FlexStart,
                    FlexJustifyContent::FlexEnd,
                    FlexJustifyContent::Center,
                    FlexJustifyContent::SpaceBetween,
                    FlexJustifyContent::SpaceAround,
                    FlexJustifyContent::SpaceEvenly,
                ]),
                align_items_values: BindableList::new(ALIGN_ITEMS_VALUES),
                align_content_values: BindableList::new([
                    FlexAlignContent::FlexStart,
                    FlexAlignContent::FlexEnd,
                    FlexAlignContent::Center,
                    FlexAlignContent::Stretch,
                    FlexAlignContent::SpaceBetween,
                    FlexAlignContent::SpaceAround,
                    FlexAlignContent::SpaceEvenly,
                ]),
                wrap_values: BindableList::new([FlexWrap::NoWrap, FlexWrap::Wrap, FlexWrap::WrapReverse]),
                flex_basis_kind_values: BindableList::new([
                    FlexBasisKind::Auto,
                    FlexBasisKind::Absolute,
                    FlexBasisKind::Relative,
                ]),
                horizontal_alignment_values: BindableList::new([
                    HorizontalAlignment::Stretch,
                    HorizontalAlignment::Left,
                    HorizontalAlignment::Center,
                    HorizontalAlignment::Right,
                ]),
                vertical_alignment_values: BindableList::new([
                    VerticalAlignment::Stretch,
                    VerticalAlignment::Top,
                    VerticalAlignment::Center,
                    VerticalAlignment::Bottom,
                ]),
                align_self_values: BindableList::new(
                    std::iter::once(FlexItemViewModel::ALIGN_SELF_AUTO).chain(ALIGN_ITEMS_VALUES.into_iter().map(Some)),
                ),
                direction: Cell::new(FlexDirection::Row),
                justify_content: Cell::new(FlexJustifyContent::FlexStart),
                align_items: Cell::new(FlexAlignItems::FlexStart),
                align_content: Cell::new(FlexAlignContent::FlexStart),
                wrap: Cell::new(FlexWrap::Wrap),
                column_spacing: Cell::new(64),
                row_spacing: Cell::new(32),
                current_number: Cell::new(41),
                selected_item: RefCell::new(None),
                add_item_command,
                remove_item_command,
            }
        })
    }

    pub fn direction_values(&self) -> Rc<BindableList<FlexDirection>> {
        self.direction_values.clone()
    }

    pub fn justify_content_values(&self) -> Rc<BindableList<FlexJustifyContent>> {
        self.justify_content_values.clone()
    }

    pub fn align_items_values(&self) -> Rc<BindableList<FlexAlignItems>> {
        self.align_items_values.clone()
    }

    pub fn align_content_values(&self) -> Rc<BindableList<FlexAlignContent>> {
        self.align_content_values.clone()
    }

    pub fn wrap_values(&self) -> Rc<BindableList<FlexWrap>> {
        self.wrap_values.clone()
    }

    pub fn flex_basis_kind_values(&self) -> Rc<BindableList<FlexBasisKind>> {
        self.flex_basis_kind_values.clone()
    }

    pub fn horizontal_alignment_values(&self) -> Rc<BindableList<HorizontalAlignment>> {
        self.horizontal_alignment_values.clone()
    }

    pub fn vertical_alignment_values(&self) -> Rc<BindableList<VerticalAlignment>> {
        self.vertical_alignment_values.clone()
    }

    pub fn align_self_values(&self) -> Rc<BindableList<Option<FlexAlignItems>>> {
        self.align_self_values.clone()
    }

    pub fn direction(&self) -> FlexDirection {
        self.direction.get()
    }

    pub fn set_direction(&self, value: FlexDirection) {
        self.base.raise_and_set_if_changed_cell(&self.direction, value, "Direction");
    }

    pub fn justify_content(&self) -> FlexJustifyContent {
        self.justify_content.get()
    }

    pub fn set_justify_content(&self, value: FlexJustifyContent) {
        self.base.raise_and_set_if_changed_cell(&self.justify_content, value, "JustifyContent");
    }

    pub fn align_items(&self) -> FlexAlignItems {
        self.align_items.get()
    }

    pub fn set_align_items(&self, value: FlexAlignItems) {
        self.base.raise_and_set_if_changed_cell(&self.align_items, value, "AlignItems");
    }

    pub fn align_content(&self) -> FlexAlignContent {
        self.align_content.get()
    }

    pub fn set_align_content(&self, value: FlexAlignContent) {
        self.base.raise_and_set_if_changed_cell(&self.align_content, value, "AlignContent");
    }

    pub fn wrap(&self) -> FlexWrap {
        self.wrap.get()
    }

    pub fn set_wrap(&self, value: FlexWrap) {
        self.base.raise_and_set_if_changed_cell(&self.wrap, value, "Wrap");
    }

    pub fn column_spacing(&self) -> i32 {
        self.column_spacing.get()
    }

    pub fn set_column_spacing(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.column_spacing, value, "ColumnSpacing");
    }

    pub fn row_spacing(&self) -> i32 {
        self.row_spacing.get()
    }

    pub fn set_row_spacing(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.row_spacing, value, "RowSpacing");
    }

    pub fn numbers(&self) -> Rc<BindableList<Rc<FlexItemViewModel>>> {
        self.numbers.clone()
    }

    pub fn selected_item(&self) -> Option<Rc<FlexItemViewModel>> {
        self.selected_item.borrow().clone()
    }

    pub fn set_selected_item(&self, value: Option<Rc<FlexItemViewModel>>) {
        self.base.raise_and_set_if_changed(&self.selected_item, value, "SelectedItem");
    }

    pub fn add_item_command(&self) -> Rc<dyn ICommand> {
        self.add_item_command.as_command()
    }

    pub fn remove_item_command(&self) -> Rc<dyn ICommand> {
        self.remove_item_command.as_command()
    }

    fn add_item(&self) {
        let number = self.current_number.get();
        self.current_number.set(number + 1);
        self.numbers.items().add(FlexItemViewModel::new(number));
    }

    fn remove_item(&self) {
        let Some(selected_item) = self.selected_item() else {
            return;
        };

        self.numbers.items().remove(&selected_item);
        selected_item.set_is_selected(false);
        self.set_selected_item(None);
    }
}

ferro_markup_type!(class FlexViewModel {
    this: Rc<FlexViewModel>,
    handles: [FlexViewModel, Rc<FlexViewModel>, Option<Rc<FlexViewModel>>],
    constructors: [() => FlexViewModel::new],
    properties: [
        // Lists a binding delivers to an items source property.
        DirectionValues: ItemsSource { get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.direction_values()) },
        JustifyContentValues: ItemsSource {
            get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.justify_content_values())
        },
        AlignItemsValues: ItemsSource { get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.align_items_values()) },
        AlignContentValues: ItemsSource {
            get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.align_content_values())
        },
        WrapValues: ItemsSource { get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.wrap_values()) },
        FlexBasisKindValues: ItemsSource {
            get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.flex_basis_kind_values())
        },
        HorizontalAlignmentValues: ItemsSource {
            get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.horizontal_alignment_values())
        },
        VerticalAlignmentValues: ItemsSource {
            get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.vertical_alignment_values())
        },
        AlignSelfValues: ItemsSource { get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.align_self_values()) },
        Direction: FlexDirection {
            get: |this: &Rc<FlexViewModel>| this.direction(),
            set: |this: &Rc<FlexViewModel>, value: FlexDirection| this.set_direction(value)
        },
        JustifyContent: FlexJustifyContent {
            get: |this: &Rc<FlexViewModel>| this.justify_content(),
            set: |this: &Rc<FlexViewModel>, value: FlexJustifyContent| this.set_justify_content(value)
        },
        AlignItems: FlexAlignItems {
            get: |this: &Rc<FlexViewModel>| this.align_items(),
            set: |this: &Rc<FlexViewModel>, value: FlexAlignItems| this.set_align_items(value)
        },
        AlignContent: FlexAlignContent {
            get: |this: &Rc<FlexViewModel>| this.align_content(),
            set: |this: &Rc<FlexViewModel>, value: FlexAlignContent| this.set_align_content(value)
        },
        Wrap: FlexWrap {
            get: |this: &Rc<FlexViewModel>| this.wrap(),
            set: |this: &Rc<FlexViewModel>, value: FlexWrap| this.set_wrap(value)
        },
        ColumnSpacing: i32 {
            get: |this: &Rc<FlexViewModel>| this.column_spacing(),
            set: |this: &Rc<FlexViewModel>, value: i32| this.set_column_spacing(value)
        },
        RowSpacing: i32 {
            get: |this: &Rc<FlexViewModel>| this.row_spacing(),
            set: |this: &Rc<FlexViewModel>, value: i32| this.set_row_spacing(value)
        },
        Numbers: ItemsSource { get: |this: &Rc<FlexViewModel>| ItemsSource::from(this.numbers()) },
        SelectedItem: Option<Rc<FlexItemViewModel>> {
            get: |this: &Rc<FlexViewModel>| this.selected_item(),
            set: |this: &Rc<FlexViewModel>, value: Option<Rc<FlexItemViewModel>>| this.set_selected_item(value)
        },
        AddItemCommand: Rc<dyn ICommand> { get: |this: &Rc<FlexViewModel>| this.add_item_command() },
        RemoveItemCommand: Rc<dyn ICommand> { get: |this: &Rc<FlexViewModel>| this.remove_item_command() },
    ],
    notify_property_changed: FlexViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_page_starts_with_forty_numbers() {
        let view_model = FlexViewModel::new();
        let numbers = view_model.numbers().items().to_vec();
        assert_eq!(40, numbers.len());
        assert_eq!(1, numbers[0].value());
        assert_eq!(40, numbers[39].value());
        assert_eq!(5, view_model.align_self_values().items().count());
        assert_eq!(None, view_model.align_self_values().items().get(0));
    }

    #[test]
    fn adding_continues_the_numbering() {
        let view_model = FlexViewModel::new();
        view_model.add_item_command().execute(None);
        view_model.add_item_command().execute(None);
        let numbers = view_model.numbers().items().to_vec();
        assert_eq!(42, numbers.len());
        assert_eq!(41, numbers[40].value());
        assert_eq!(42, numbers[41].value());
    }

    #[test]
    fn removing_takes_the_selected_item_out() {
        let view_model = FlexViewModel::new();
        view_model.remove_item_command().execute(None);
        assert_eq!(40, view_model.numbers().items().count());

        let item = view_model.numbers().items().get(2);
        item.set_is_selected(true);
        view_model.set_selected_item(Some(item.clone()));
        view_model.remove_item_command().execute(None);

        assert_eq!(39, view_model.numbers().items().count());
        assert!(!view_model.numbers().items().contains(&item));
        assert!(!item.is_selected());
        assert!(view_model.selected_item().is_none());
    }
}
