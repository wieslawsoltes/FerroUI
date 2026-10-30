//! Port of `ViewModels/DraggableViewModel.cs`.

use super::items::{item_list, ItemList};
use super::ViewModelBase;
use crate::models::{DragItem, Tile};
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The view model of the page of the draggable behaviors: the items of the canvas, a long list of
/// texts and the tiles of the grid.
pub struct DraggableViewModel {
    base: ViewModelBase,
    items: ItemList,
    strings: ItemList,
    tiles: ItemList,
}
reactive_object!(DraggableViewModel, base);

impl PartialEq for DraggableViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DraggableViewModel {
    /// `DraggableViewModel()`.
    pub fn new() -> Rc<Self> {
        let items = item_list([
            DragItem::new("Item1", 30.0, 30.0),
            DragItem::new("Item2", 90.0, 30.0),
            DragItem::new("Item3", 120.0, 60.0),
            DragItem::new("Item4", 45.0, 90.0),
            DragItem::new("Item5", 60.0, 120.0),
            DragItem::new("Item6", 150.0, 180.0),
            DragItem::new("Item7", 250.0, 120.0),
            DragItem::new("Item8", 300.0, 150.0),
        ]);

        let strings: ItemList = Rc::new(FerroList::new());
        for i in 0..1_000 {
            strings.add(Rc::new(format!("Item {} / {}", i + 1, 1000)) as BoxedValue);
        }

        let tiles = item_list([
            Tile::new("Tile1", 0, 0, 1, 1, "Red"),
            Tile::new("Tile2", 0, 1, 1, 1, "Green"),
            Tile::new("Tile3", 1, 0, 1, 2, "Blue"),
            Tile::new("Tile4", 2, 0, 1, 2, "Cyan"),
        ]);

        Rc::new(Self {
            base: ViewModelBase::new(),
            items,
            strings,
            tiles,
        })
    }

    /// `Items`.
    pub fn items(&self) -> ItemList {
        self.items.clone()
    }

    /// `Strings`.
    pub fn strings(&self) -> ItemList {
        self.strings.clone()
    }

    /// `Tiles`.
    pub fn tiles(&self) -> ItemList {
        self.tiles.clone()
    }
}

type Vm = Rc<DraggableViewModel>;

ferro_markup_type!(class DraggableViewModel {
    this: Rc<DraggableViewModel>,
    handles: [DraggableViewModel, Rc<DraggableViewModel>, Option<Rc<DraggableViewModel>>],
    constructors: [() => DraggableViewModel::new],
    properties: [
        Items: ItemsSource { get: |this: &Vm| ItemsSource::from(this.items()) },
        Strings: ItemsSource { get: |this: &Vm| ItemsSource::from(this.strings()) },
        Tiles: ItemsSource { get: |this: &Vm| ItemsSource::from(this.tiles()) },
    ],
    notify_property_changed: DraggableViewModel,
});
