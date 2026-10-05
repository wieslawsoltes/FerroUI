//! Port of `ViewModels/FlexItemViewModel.cs`.

use super::random::Random;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{IBrush, SolidColorBrush};
use ferroui_controls::flex_panel::{FlexAlignItems, FlexBasis, FlexBasisKind};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    /// `Random.Shared`.
    static SHARED_RANDOM: RefCell<Random> = RefCell::new(Random::new());
}

/// An item of the flex page.
pub struct FlexItemViewModel {
    base: ViewModelBase,
    value: i32,
    color: Rc<dyn IBrush>,
    align_self: FlexAlignItems,
    is_selected: Cell<bool>,
    is_visible: Cell<bool>,
    align_self_item: Cell<Option<FlexAlignItems>>,
    order: Cell<i32>,
    shrink: Cell<f64>,
    grow: Cell<f64>,
    basis_value: Cell<f64>,
    basis_kind: Cell<FlexBasisKind>,
    horizontal_alignment: Cell<HorizontalAlignment>,
    vertical_alignment: Cell<VerticalAlignment>,
}

impl PartialEq for FlexItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FlexItemViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl FlexItemViewModel {
    /// The "auto" choice of the alignment of an item: the value of the
    /// managed original that is not a member of the enumeration (`-1`) is
    /// the absence of a value here.
    pub(crate) const ALIGN_SELF_AUTO: Option<FlexAlignItems> = None;

    pub fn new(value: i32) -> Rc<FlexItemViewModel> {
        let align_self_item = Self::ALIGN_SELF_AUTO;
        let align_self = align_self_item.unwrap_or_default();

        let color = SHARED_RANDOM.with(|random| random.borrow_mut().next());
        let color: Rc<dyn IBrush> = SolidColorBrush::from_uint32(color as u32).into();

        Rc::new(Self {
            base: ViewModelBase::new(),
            value,
            color,
            align_self,
            is_selected: Cell::new(false),
            is_visible: Cell::new(true),
            align_self_item: Cell::new(align_self_item),
            order: Cell::new(0),
            shrink: Cell::new(1.0),
            grow: Cell::new(0.0),
            basis_value: Cell::new(100.0),
            basis_kind: Cell::new(FlexBasisKind::Auto),
            horizontal_alignment: Cell::new(HorizontalAlignment::Stretch),
            vertical_alignment: Cell::new(VerticalAlignment::Stretch),
        })
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn color(&self) -> Rc<dyn IBrush> {
        self.color.clone()
    }

    pub fn is_selected(&self) -> bool {
        self.is_selected.get()
    }

    pub fn set_is_selected(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_selected, value, "IsSelected");
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_visible, value, "IsVisible");
    }

    pub fn align_self_item(&self) -> Option<FlexAlignItems> {
        self.align_self_item.get()
    }

    pub fn set_align_self_item(&self, value: Option<FlexAlignItems>) {
        self.base.raise_and_set_if_changed_cell(&self.align_self_item, value, "AlignSelfItem");
        self.base.raise_property_changed("AlignSelf");
    }

    /// The alignment the constructor computed (it does not follow
    /// `AlignSelfItem`, as in the managed original).
    pub fn align_self(&self) -> Option<FlexAlignItems> {
        Some(self.align_self)
    }

    pub fn order(&self) -> i32 {
        self.order.get()
    }

    pub fn set_order(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.order, value, "Order");
    }

    pub fn shrink(&self) -> f64 {
        self.shrink.get()
    }

    pub fn set_shrink(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.shrink, value, "Shrink");
    }

    pub fn grow(&self) -> f64 {
        self.grow.get()
    }

    pub fn set_grow(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.grow, value, "Grow");
    }

    pub fn basis_value(&self) -> f64 {
        self.basis_value.get()
    }

    pub fn set_basis_value(&self, value: f64) {
        self.base.raise_and_set_if_changed_cell(&self.basis_value, value, "BasisValue");
        self.base.raise_property_changed("Basis");
    }

    pub fn basis_kind(&self) -> FlexBasisKind {
        self.basis_kind.get()
    }

    pub fn set_basis_kind(&self, value: FlexBasisKind) {
        self.base.raise_and_set_if_changed_cell(&self.basis_kind, value, "BasisKind");
        self.base.raise_property_changed("Basis");
    }

    /// # Panics
    /// Panics if the basis value is negative or not finite (the argument
    /// exception of the managed original).
    pub fn basis(&self) -> FlexBasis {
        FlexBasis::new(self.basis_value.get(), self.basis_kind.get())
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment.get()
    }

    pub fn set_horizontal_alignment(&self, value: HorizontalAlignment) {
        self.base.raise_and_set_if_changed_cell(&self.horizontal_alignment, value, "HorizontalAlignment");
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment.get()
    }

    pub fn set_vertical_alignment(&self, value: VerticalAlignment) {
        self.base.raise_and_set_if_changed_cell(&self.vertical_alignment, value, "VerticalAlignment");
    }
}

ferro_markup_type!(class FlexItemViewModel {
    this: Rc<FlexItemViewModel>,
    handles: [FlexItemViewModel, Rc<FlexItemViewModel>, Option<Rc<FlexItemViewModel>>],
    constructors: [(i32) => FlexItemViewModel::new],
    properties: [
        Value: i32 { get: |this: &Rc<FlexItemViewModel>| this.value() },
        Color: Rc<dyn IBrush> { get: |this: &Rc<FlexItemViewModel>| this.color() },
        IsSelected: bool {
            get: |this: &Rc<FlexItemViewModel>| this.is_selected(),
            set: |this: &Rc<FlexItemViewModel>, value: bool| this.set_is_selected(value)
        },
        IsVisible: bool {
            get: |this: &Rc<FlexItemViewModel>| this.is_visible(),
            set: |this: &Rc<FlexItemViewModel>, value: bool| this.set_is_visible(value)
        },
        AlignSelfItem: Option<FlexAlignItems> {
            get: |this: &Rc<FlexItemViewModel>| this.align_self_item(),
            set: |this: &Rc<FlexItemViewModel>, value: Option<FlexAlignItems>| this.set_align_self_item(value)
        },
        AlignSelf: Option<FlexAlignItems> { get: |this: &Rc<FlexItemViewModel>| this.align_self() },
        Order: i32 {
            get: |this: &Rc<FlexItemViewModel>| this.order(),
            set: |this: &Rc<FlexItemViewModel>, value: i32| this.set_order(value)
        },
        Shrink: f64 {
            get: |this: &Rc<FlexItemViewModel>| this.shrink(),
            set: |this: &Rc<FlexItemViewModel>, value: f64| this.set_shrink(value)
        },
        Grow: f64 {
            get: |this: &Rc<FlexItemViewModel>| this.grow(),
            set: |this: &Rc<FlexItemViewModel>, value: f64| this.set_grow(value)
        },
        BasisValue: f64 {
            get: |this: &Rc<FlexItemViewModel>| this.basis_value(),
            set: |this: &Rc<FlexItemViewModel>, value: f64| this.set_basis_value(value)
        },
        BasisKind: FlexBasisKind {
            get: |this: &Rc<FlexItemViewModel>| this.basis_kind(),
            set: |this: &Rc<FlexItemViewModel>, value: FlexBasisKind| this.set_basis_kind(value)
        },
        Basis: FlexBasis { get: |this: &Rc<FlexItemViewModel>| this.basis() },
        HorizontalAlignment: HorizontalAlignment {
            get: |this: &Rc<FlexItemViewModel>| this.horizontal_alignment(),
            set: |this: &Rc<FlexItemViewModel>, value: HorizontalAlignment| this.set_horizontal_alignment(value)
        },
        VerticalAlignment: VerticalAlignment {
            get: |this: &Rc<FlexItemViewModel>| this.vertical_alignment(),
            set: |this: &Rc<FlexItemViewModel>, value: VerticalAlignment| this.set_vertical_alignment(value)
        },
    ],
    notify_property_changed: FlexItemViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_new_item_has_the_defaults_of_the_page() {
        let item = FlexItemViewModel::new(7);
        assert_eq!(7, item.value());
        assert!(!item.is_selected());
        assert!(item.is_visible());
        assert_eq!(FlexItemViewModel::ALIGN_SELF_AUTO, item.align_self_item());
        assert_eq!(Some(FlexAlignItems::FlexStart), item.align_self());
        assert_eq!(0, item.order());
        assert_eq!(1.0, item.shrink());
        assert_eq!(0.0, item.grow());
        assert_eq!(FlexBasis::AUTO, item.basis());
    }

    #[test]
    fn the_basis_follows_its_value_and_kind() {
        let item = FlexItemViewModel::new(1);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        item.property_changed().add(Rc::new(move |name: &str| log.borrow_mut().push(name.to_string())));

        item.set_basis_kind(FlexBasisKind::Absolute);
        item.set_basis_value(50.0);
        assert_eq!(FlexBasis::absolute(50.0), item.basis());
        assert_eq!(vec!["BasisKind", "Basis", "BasisValue", "Basis"], *changes.borrow());
    }

    #[test]
    fn the_alignment_of_the_item_does_not_follow_its_choice() {
        let item = FlexItemViewModel::new(1);
        item.set_align_self_item(Some(FlexAlignItems::Center));
        assert_eq!(Some(FlexAlignItems::Center), item.align_self_item());
        assert_eq!(Some(FlexAlignItems::FlexStart), item.align_self());
    }
}
