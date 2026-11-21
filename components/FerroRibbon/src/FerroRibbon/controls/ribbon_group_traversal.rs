//! Source Flatten/GetAllItems and expanded-item collection/layout paths.
use super::*;
use ferroribbon_core::model::RibbonItemsSource;
impl RibbonGroup {
    pub(crate) fn flatten(element: Option<Ref<Control>>) -> RibbonItemsSource<Ref<Control>> {
        RibbonItemsSource::deferred(move || {
            if let Some(container) = element
                .as_ref()
                .and_then(|e| e.cast::<super::super::RibbonItemsContainer>())
            {
                return Box::new(
                    container
                        .items()
                        .iter()
                        .filter_map(|child| {
                            let child = child.expect("NullReference: container child.Visibility");
                            child.is_visible().then_some(child)
                        })
                        .flat_map(|child| Self::flatten(Some(child)).iter()),
                );
            }
            let item = element.clone().filter(|item| {
                !item.is::<super::super::RibbonSeparator>()
                    && !item.is::<super::super::RibbonLabel>()
                    && (Self::as_item(item).is_some()
                        || item.is::<TemplatedControl>()
                            && !item.is::<ferroui_controls::IconElement>())
            });
            Box::new(item.into_iter())
        })
    }
    pub fn get_all_items(&self) -> RibbonItemsSource<Ref<Control>> {
        let items = self.items();
        let slide = self.slide_out_items();
        RibbonItemsSource::deferred(move || {
            let slide = slide.clone();
            Box::new(
                items
                    .iter()
                    .chain(std::iter::once_with(move || slide.iter()).flatten())
                    .flat_map(|item| Self::flatten(item).iter()),
            )
        })
    }
    /// Source compiler-generated yield traversal. Every phase reads live state at
    /// MoveNext, while nested Flatten/collection iterators retain their own state.
    #[doc(hidden)]
    pub fn get_key_tip_targets(&self) -> RibbonItemsSource<Ref<Control>> {
        let owner = self.to_ref();
        RibbonItemsSource::deferred(move || {
            let group = owner.clone();
            let mut phase = 0;
            let mut index = 0;
            let mut nested: Option<ferroribbon_core::model::RibbonIterator<Ref<Control>>> = None;
            Box::new(std::iter::from_fn(move || {
                loop {
                    if let Some(iter) = nested.as_mut() {
                        if let Some(target) = iter.next() {
                            return Some(target);
                        }
                        nested = None;
                    }
                    match phase {
                        0 => {
                            if group.state() == RibbonGroupState::Collapsed
                                && !group.is_simplified()
                                && !group.is_popup_open()
                            {
                                phase = 5;
                                return group.collapsed_button().map(|b| b.upcast());
                            }
                            phase = 1;
                        }
                        1 => {
                            if index >= group.items_panel.children().count() {
                                phase = 2;
                                continue;
                            }
                            let current = index;
                            let child = group.items_panel.children().get(index);
                            index += 1;
                            if !child.is_visible()
                                || group.is_simplified()
                                    && !group.items_panel.is_in_line(current as i32)
                            {
                                continue;
                            }
                            nested = Some(Self::flatten(Some(child)).iter());
                        }
                        2 => {
                            phase = 4;
                            if group.is_popup_open() {
                                nested = Some(group.get_slide_out_key_tip_targets().iter());
                            } else if !group.is_simplified()
                                && let Some(button) =
                                    group.slide_out_button().filter(|b| b.is_visible())
                            {
                                return Some(button.upcast());
                            }
                        }
                        4 => {
                            phase = 5;
                            if group.is_popup_open()
                                && let Some(button) =
                                    group.popup_launcher().filter(|b| b.is_visible())
                            {
                                return Some(button.upcast());
                            }
                            if !group.is_simplified() {
                                return group
                                    .dialog_launcher()
                                    .filter(|b| b.is_visible())
                                    .map(|b| b.upcast());
                            }
                        }
                        _ => return None,
                    }
                }
            }))
        })
    }
    #[doc(hidden)]
    pub fn get_slide_out_key_tip_targets(&self) -> RibbonItemsSource<Ref<Control>> {
        let items = self.slide_out_items();
        RibbonItemsSource::deferred(move || {
            Box::new(
                items
                    .iter()
                    .filter_map(|item| {
                        let item = item.expect("NullReference: slide-out item.Visibility");
                        item.is_visible().then_some(item)
                    })
                    .flat_map(|item| Self::flatten(Some(item)).iter()),
            )
        })
    }
    pub(super) fn on_slide_out_items_changed(&self) {
        self.slide_out_panel.children().clear();
        for item in self.slide_out_items.iter() {
            let item = item.expect("ArgumentNull: slide-out item");
            if let Some(parent) = item.get_logical_parent().and_then(|p| p.cast::<Panel>())
                && !parent.ptr_eq(&self.slide_out_panel)
            {
                parent.children().remove(item.clone());
            }
            if let Some(ribbon) = self.ribbon() {
                super::super::RibbonItemHelper::set_owner(&item, Some(ribbon.item_owner()))
                    .expect("slide-out item owner");
            }
            self.slide_out_panel.children().add(item);
        }
        self.apply_slide_out_layouts();
        self.update_slide_out_button();
        if let Some(ribbon) = self.ribbon() {
            ribbon.invalidate_shortcuts();
        }
        self.invalidate_items_layout();
    }
    pub(super) fn apply_slide_out_layouts(&self) {
        for item in self
            .slide_out_items
            .iter()
            .flatten()
            .filter_map(|c| Self::as_item(&c))
        {
            let preferred = item
                .effective_size_definition()
                .get_size(RibbonGroupState::Large);
            let size = if preferred == RibbonItemSize::Large {
                RibbonItemSize::Medium
            } else {
                preferred
            };
            item.apply_layout(RibbonItemLayout::new(size, Some(self.metrics())));
        }
    }
}
