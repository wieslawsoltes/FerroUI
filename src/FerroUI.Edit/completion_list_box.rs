use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    TypeInfo, Vector, VisualImpl,
};
use ferroui_controls::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, ItemsControlImpl, ListBox};

/// Completion navigation over the native virtualized list and scroll viewer.
#[repr(C)]
pub struct CompletionListBox {
    base: ListBox,
}
ferro_class!(CompletionListBox: ListBox);
ferroui_base::ferro_class_info!(CompletionListBox {
    new: CompletionListBox::new
});
ferro_impl_classes!(CompletionListBox: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ItemsControlImpl, SelectingItemsControlImpl);
impl FerroObjectImpl for CompletionListBox {}
impl StyledElementImpl for CompletionListBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ListBox::TYPE
    }
}
impl CompletionListBox {
    pub fn construct() -> Self {
        Self {
            base: ListBox::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn first_visible_item(&self) -> usize {
        let count = self.item_count().max(0) as usize;
        let Some(scroll) = self.scroll() else {
            return 0;
        };
        let Some(scroll) = scroll.as_scrollable() else {
            return 0;
        };
        let extent = scroll.extent().height;
        if count == 0 || !extent.is_finite() || extent <= 0.0 {
            return 0;
        }
        ((count as f64 * scroll.offset().y / extent).floor().max(0.0) as usize).min(count - 1)
    }
    pub fn visible_item_count(&self) -> usize {
        let count = self.item_count().max(0) as usize;
        let Some(scroll) = self.scroll() else {
            return 10;
        };
        let Some(scroll) = scroll.as_scrollable() else {
            return 10;
        };
        let extent = scroll.extent().height;
        if count == 0 || !extent.is_finite() || extent <= 0.0 {
            return 10;
        }
        (count as f64 * scroll.viewport().height / extent)
            .ceil()
            .max(3.0) as usize
    }
    pub fn set_first_visible_item(&self, index: usize) {
        let count = self.item_count().max(0) as usize;
        let Some(scroll) = self.scroll() else {
            return;
        };
        let Some(scroll) = scroll.as_scrollable() else {
            return;
        };
        if count == 0 {
            return;
        }
        let index = index.min(count.saturating_sub(self.visible_item_count()));
        scroll.set_offset(Vector::new(
            scroll.offset().x,
            index as f64 * scroll.extent().height / count as f64,
        ));
    }
    pub fn clear_selection(&self) {
        self.set_selected_index(-1);
    }
    pub fn select_index(&self, index: usize) {
        let count = self.item_count().max(0) as usize;
        if count == 0 {
            self.clear_selection();
            return;
        }
        let index = index.min(count - 1) as i32;
        self.set_selected_index(index);
        self.scroll_into_view(index);
    }
    pub fn center_view_on(&self, index: usize) {
        self.set_first_visible_item(index.saturating_sub(self.visible_item_count() / 2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    #[test]
    fn selection_clamps_and_empty_lists_have_no_phantom_item() {
        let _scope = crate::tests::test_scope();
        let list = CompletionListBox::new();
        list.select_index(usize::MAX);
        assert_eq!(list.selected_index(), -1);
        for item in ["a", "b", "c"] {
            list.items().add(Some(Rc::new(item.to_string())));
        }
        list.select_index(usize::MAX);
        assert_eq!(list.selected_index(), 2);
        assert_eq!(list.visible_item_count(), 10);
        list.clear_selection();
        assert_eq!(list.selected_index(), -1);
    }
}
