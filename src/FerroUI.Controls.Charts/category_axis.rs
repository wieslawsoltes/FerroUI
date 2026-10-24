use crate::{CartesianAxis, CategorySortKey, SortOrder};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::cmp::Ordering;

/// An axis whose categories retain source order or sort by their values.
#[repr(C)]
pub struct CategoryAxis {
    base: CartesianAxis,
}

ferro_class!(CategoryAxis: CartesianAxis);
ferro_class_info!(CategoryAxis {
    new: CategoryAxis::new
});
ferro_impl_classes!(CategoryAxis: FerroObjectImpl);

ferro_properties! { impl CategoryAxis {
    pub fn sort_key_property() -> StyledProperty<CategorySortKey> {
        FerroProperty::register::<CategoryAxis, _>("SortKey", CategorySortKey::Index)
    }
    pub fn sort_order_property() -> StyledProperty<SortOrder> {
        FerroProperty::register::<CategoryAxis, _>("SortOrder", SortOrder::Ascending)
    }
} }

impl CategoryAxis {
    pub fn construct() -> Self {
        Self {
            base: CartesianAxis::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn sort_key(&self) -> CategorySortKey {
        self.get_value(Self::sort_key_property())
    }
    pub fn set_sort_key(&self, value: CategorySortKey) {
        self.set_value(Self::sort_key_property(), value);
    }
    pub fn sort_order(&self) -> SortOrder {
        self.get_value(Self::sort_order_property())
    }
    pub fn set_sort_order(&self, value: SortOrder) {
        self.set_value(Self::sort_order_property(), value);
    }

    /// Source indices in display order. Equal values retain source order.
    /// The comparator allows typed numeric/date values to remain typed.
    pub fn sorted_indices_by(
        &self,
        count: usize,
        mut compare: impl FnMut(usize, usize) -> Ordering,
    ) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..count).collect();
        let descending = self.sort_order() == SortOrder::Descending;
        if self.sort_key() == CategorySortKey::Value {
            indices.sort_by(|&left, &right| {
                let order = compare(left, right);
                if descending {
                    order.reverse()
                } else {
                    order
                }
            });
        } else if descending {
            indices.reverse();
        }
        indices
    }
}
