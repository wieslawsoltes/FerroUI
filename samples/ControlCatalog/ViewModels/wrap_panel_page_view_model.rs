//! Port of `ViewModels/WrapPanelPageViewModel.cs`.

use super::random::Random;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::layout::Orientation;
use ferroui_base::{ferro_markup_type, Thickness};
use ferroui_controls::{ItemsSource, WrapPanelItemsAlignment};
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

/// The view model of the wrap panel page.
pub struct WrapPanelPageViewModel {
    base: ViewModelBase,
    items_alignments: Rc<BindableList<WrapPanelItemsAlignment>>,
    orientations: Rc<BindableList<Orientation>>,
    items: Rc<BindableList<Rc<WrapPanelItemViewModel>>>,
    items_alignment: Cell<WrapPanelItemsAlignment>,
    orientation: Cell<Orientation>,
    item_spacing: Cell<f64>,
    line_spacing: Cell<f64>,
    is_item_width_enabled: Cell<bool>,
    item_width_value: Cell<f64>,
    is_item_height_enabled: Cell<bool>,
    item_height_value: Cell<f64>,
}

impl PartialEq for WrapPanelPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for WrapPanelPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl WrapPanelPageViewModel {
    pub fn new() -> Rc<WrapPanelPageViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            // The values of the enumerations, in the order of their declaration.
            items_alignments: BindableList::new([
                WrapPanelItemsAlignment::Start,
                WrapPanelItemsAlignment::Center,
                WrapPanelItemsAlignment::End,
                WrapPanelItemsAlignment::Justify,
                WrapPanelItemsAlignment::Stretch,
                WrapPanelItemsAlignment::StretchAll,
            ]),
            orientations: BindableList::new([Orientation::Horizontal, Orientation::Vertical]),
            items: BindableList::new(Self::create_items()),
            items_alignment: Cell::new(WrapPanelItemsAlignment::Start),
            orientation: Cell::new(Orientation::Horizontal),
            item_spacing: Cell::new(5.0),
            line_spacing: Cell::new(5.0),
            is_item_width_enabled: Cell::new(false),
            item_width_value: Cell::new(140.0),
            is_item_height_enabled: Cell::new(false),
            item_height_value: Cell::new(90.0),
        })
    }

    pub fn items_alignments(&self) -> Rc<BindableList<WrapPanelItemsAlignment>> {
        self.items_alignments.clone()
    }

    pub fn orientations(&self) -> Rc<BindableList<Orientation>> {
        self.orientations.clone()
    }

    pub fn items(&self) -> Rc<BindableList<Rc<WrapPanelItemViewModel>>> {
        self.items.clone()
    }

    pub fn items_alignment(&self) -> WrapPanelItemsAlignment {
        self.items_alignment.get()
    }

    pub fn set_items_alignment(&self, value: WrapPanelItemsAlignment) {
        self.base.raise_and_set_if_changed_cell(&self.items_alignment, value, "ItemsAlignment");
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation.get()
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.base.raise_and_set_if_changed_cell(&self.orientation, value, "Orientation");
    }

    pub fn item_spacing(&self) -> f64 {
        self.item_spacing.get()
    }

    pub fn set_item_spacing(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.item_spacing, value, "ItemSpacing");
    }

    pub fn line_spacing(&self) -> f64 {
        self.line_spacing.get()
    }

    pub fn set_line_spacing(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.line_spacing, value, "LineSpacing");
    }

    pub fn is_item_width_enabled(&self) -> bool {
        self.is_item_width_enabled.get()
    }

    pub fn set_is_item_width_enabled(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.is_item_width_enabled, value, "IsItemWidthEnabled") {
            self.base.raise_property_changed("EffectiveItemWidth");
        }
    }

    pub fn item_width_value(&self) -> f64 {
        self.item_width_value.get()
    }

    pub fn set_item_width_value(&self, value: f64) {
        if self.base.raise_and_set_if_changed_cell(&self.item_width_value, value, "ItemWidthValue") {
            self.base.raise_property_changed("EffectiveItemWidth");
        }
    }

    pub fn is_item_height_enabled(&self) -> bool {
        self.is_item_height_enabled.get()
    }

    pub fn set_is_item_height_enabled(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.is_item_height_enabled, value, "IsItemHeightEnabled") {
            self.base.raise_property_changed("EffectiveItemHeight");
        }
    }

    pub fn item_height_value(&self) -> f64 {
        self.item_height_value.get()
    }

    pub fn set_item_height_value(&self, value: f64) {
        if self.base.raise_and_set_if_changed_cell(&self.item_height_value, value, "ItemHeightValue") {
            self.base.raise_property_changed("EffectiveItemHeight");
        }
    }

    pub fn effective_item_width(&self) -> f64 {
        if self.is_item_width_enabled() { self.item_width_value() } else { f64::NAN }
    }

    pub fn effective_item_height(&self) -> f64 {
        if self.is_item_height_enabled() { self.item_height_value() } else { f64::NAN }
    }

    fn create_items() -> Vec<Rc<WrapPanelItemViewModel>> {
        let mut random = Random::with_seed(42);

        (0..50)
            .map(|i| {
                let horizontal = f64::from(random.next_range(15, 56));
                let vertical = f64::from(random.next_range(8, 31));
                WrapPanelItemViewModel::new(i + 1, Thickness::new(horizontal, vertical, horizontal, vertical))
            })
            .collect()
    }
}

ferro_markup_type!(class WrapPanelPageViewModel {
    this: Rc<WrapPanelPageViewModel>,
    handles: [WrapPanelPageViewModel, Rc<WrapPanelPageViewModel>, Option<Rc<WrapPanelPageViewModel>>],
    constructors: [() => WrapPanelPageViewModel::new],
    properties: [
        // Lists a binding delivers to an items source property.
        ItemsAlignments: ItemsSource {
            get: |this: &Rc<WrapPanelPageViewModel>| ItemsSource::from(this.items_alignments())
        },
        Orientations: ItemsSource { get: |this: &Rc<WrapPanelPageViewModel>| ItemsSource::from(this.orientations()) },
        Items: ItemsSource { get: |this: &Rc<WrapPanelPageViewModel>| ItemsSource::from(this.items()) },
        ItemsAlignment: WrapPanelItemsAlignment {
            get: |this: &Rc<WrapPanelPageViewModel>| this.items_alignment(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: WrapPanelItemsAlignment| this.set_items_alignment(value)
        },
        Orientation: Orientation {
            get: |this: &Rc<WrapPanelPageViewModel>| this.orientation(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: Orientation| this.set_orientation(value)
        },
        ItemSpacing: f64 {
            get: |this: &Rc<WrapPanelPageViewModel>| this.item_spacing(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: f64| this.set_item_spacing(value)
        },
        LineSpacing: f64 {
            get: |this: &Rc<WrapPanelPageViewModel>| this.line_spacing(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: f64| this.set_line_spacing(value)
        },
        IsItemWidthEnabled: bool {
            get: |this: &Rc<WrapPanelPageViewModel>| this.is_item_width_enabled(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: bool| this.set_is_item_width_enabled(value)
        },
        ItemWidthValue: f64 {
            get: |this: &Rc<WrapPanelPageViewModel>| this.item_width_value(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: f64| this.set_item_width_value(value)
        },
        IsItemHeightEnabled: bool {
            get: |this: &Rc<WrapPanelPageViewModel>| this.is_item_height_enabled(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: bool| this.set_is_item_height_enabled(value)
        },
        ItemHeightValue: f64 {
            get: |this: &Rc<WrapPanelPageViewModel>| this.item_height_value(),
            set: |this: &Rc<WrapPanelPageViewModel>, value: f64| this.set_item_height_value(value)
        },
        EffectiveItemWidth: f64 { get: |this: &Rc<WrapPanelPageViewModel>| this.effective_item_width() },
        EffectiveItemHeight: f64 { get: |this: &Rc<WrapPanelPageViewModel>| this.effective_item_height() },
    ],
    notify_property_changed: WrapPanelPageViewModel,
});

/// An item of the wrap panel: its number and its padding.
pub struct WrapPanelItemViewModel {
    number: i32,
    padding: Thickness,
}

impl PartialEq for WrapPanelItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl WrapPanelItemViewModel {
    pub fn new(number: i32, padding: Thickness) -> Rc<WrapPanelItemViewModel> {
        Rc::new(Self { number, padding })
    }

    pub fn number(&self) -> i32 {
        self.number
    }

    pub fn padding(&self) -> Thickness {
        self.padding
    }
}

ferro_markup_type!(class WrapPanelItemViewModel {
    this: Rc<WrapPanelItemViewModel>,
    handles: [WrapPanelItemViewModel, Rc<WrapPanelItemViewModel>, Option<Rc<WrapPanelItemViewModel>>],
    constructors: [(i32, Thickness) => WrapPanelItemViewModel::new],
    properties: [
        Number: i32 { get: |this: &Rc<WrapPanelItemViewModel>| this.number() },
        Padding: Thickness { get: |this: &Rc<WrapPanelItemViewModel>| this.padding() },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_fifty_items_have_paddings_of_the_ranges_of_the_page() {
        let view_model = WrapPanelPageViewModel::new();
        let items = view_model.items().items().to_vec();
        assert_eq!(50, items.len());
        assert_eq!(1, items[0].number());
        assert_eq!(50, items[49].number());
        for item in &items {
            let padding = item.padding();
            assert!((15.0..56.0).contains(&padding.left) && padding.left == padding.right);
            assert!((8.0..31.0).contains(&padding.top) && padding.top == padding.bottom);
        }
        // The seed is fixed: every instance has the same items.
        let other = WrapPanelPageViewModel::new().items().items().to_vec();
        assert!(items.iter().zip(&other).all(|(a, b)| a.padding() == b.padding()));
        // `new Random(42)`: 1434747710 and 302596119 scaled to 15..56 and 8..31.
        assert_eq!(Thickness::new(42.0, 11.0, 42.0, 11.0), items[0].padding());
    }

    #[test]
    fn the_effective_sizes_are_not_a_number_until_they_are_enabled() {
        let view_model = WrapPanelPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        assert!(view_model.effective_item_width().is_nan());
        assert!(view_model.effective_item_height().is_nan());
        view_model.set_is_item_width_enabled(true);
        assert_eq!(140.0, view_model.effective_item_width());
        view_model.set_item_width_value(150.0);
        view_model.set_item_width_value(150.0);
        view_model.set_item_height_value(100.0);
        assert!(view_model.effective_item_height().is_nan());
        assert_eq!(
            vec![
                "IsItemWidthEnabled",
                "EffectiveItemWidth",
                "ItemWidthValue",
                "EffectiveItemWidth",
                "ItemHeightValue",
                "EffectiveItemHeight"
            ],
            *seen.borrow()
        );
        assert_eq!(6, view_model.items_alignments().items().count());
        assert_eq!(vec![Orientation::Horizontal, Orientation::Vertical], view_model.orientations().items().to_vec());
    }
}
