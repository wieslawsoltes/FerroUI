//! Port of `Pages/BackdropEffectPage.xaml.cs`: the class of the document
//! `Pages/BackdropEffectPage.xaml`, a frosted panel over live content whose backdrop cache
//! mode the combo box selects.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::BackdropEffectCache;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Border, ComboBox, ContentPage};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct BackdropEffectPage {
    base: ContentPage,
    // The field of the named element: assigned once the document is loaded, so that the
    // handler that runs while it loads sees none.
    frosted: RefCell<Option<Ref<Border>>>,
}

content_page_class!(BackdropEffectPage);
ferro_class_info!(BackdropEffectPage {
    new: BackdropEffectPage::new,
    markup: {
        methods: [
            fn CacheMode_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<BackdropEffectPage>, sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.cache_mode_selection_changed(&sender)
                },
        ],
    },
});
xaml_class!(BackdropEffectPage, "/Pages/BackdropEffectPage.xaml");

/// The cache mode of the item at `index` of the combo box.
fn cache_mode_of(index: i32) -> Option<BackdropEffectCache> {
    match index {
        0 => Some(BackdropEffectCache::Default),
        1 => Some(BackdropEffectCache::Retained),
        2 => Some(BackdropEffectCache::Volatile),
        _ => None,
    }
}

impl BackdropEffectPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            frosted: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        *this.frosted.borrow_mut() = this.find_control::<Border>("Frosted");
        this
    }

    /// Raised while the document is still loading too, before the named controls are
    /// assigned.
    pub fn cache_mode_selection_changed(&self, sender: &Option<BoxedValue>) {
        let Some(frosted) = self.frosted.borrow().clone() else {
            return;
        };
        let combo_box = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<ComboBox>());
        if let Some(mode) =
            combo_box.and_then(|combo_box| cache_mode_of(combo_box.selected_index()))
        {
            frosted.set_backdrop_effect_cache(mode);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_mode_of_each_item() {
        assert_eq!(cache_mode_of(0), Some(BackdropEffectCache::Default));
        assert_eq!(cache_mode_of(1), Some(BackdropEffectCache::Retained));
        assert_eq!(cache_mode_of(2), Some(BackdropEffectCache::Volatile));
        assert_eq!(cache_mode_of(3), None);
    }
}
