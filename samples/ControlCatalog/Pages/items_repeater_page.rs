//! Exercises the WinUI Repeater port using compiled XAML and the native
//! collection, layout, scrolling, template, and control implementations.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage, ItemsSource, TextBlock};
use ferroui_controls_repeater::{ItemsRepeater, Layout, LinedFlowLayout};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct ItemsRepeaterPage {
    base: ContentPage,
    items: Rc<FerroList<String>>,
    next_item: Cell<usize>,
    prepared_count: Cell<usize>,
    cleared_count: Cell<usize>,
}

content_page_class!(ItemsRepeaterPage);
ferro_class_info!(ItemsRepeaterPage {
    new: ItemsRepeaterPage::new
});
xaml_class!(ItemsRepeaterPage, "/Pages/ItemsRepeaterPage.xaml");

impl ItemsRepeaterPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            items: Rc::new(FerroList::new()),
            next_item: Cell::new(10_000),
            prepared_count: Cell::new(0),
            cleared_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.reset_items();
        let repeater = this.get_control::<ItemsRepeater>("Repeater");
        repeater.set_items_source(Some(ItemsSource::from(this.items.clone())));

        for name in [
            "StackLayoutButton",
            "UniformGridLayoutButton",
            "FlowLayoutButton",
            "LinedFlowLayoutButton",
        ] {
            let button = this.get_control::<Button>(name);
            let layout =
                from_markup_value::<Ref<Layout>>(&button.tag()).expect("the layout of the button");
            if let Some(lined) = layout.cast::<LinedFlowLayout>() {
                let owner = this.downgrade();
                lined.items_info_requested(move |args| {
                    if let Some(owner) = owner.upgrade() {
                        // These text cards have known minimum widths and a fixed 64-DIP height.
                        // Supplying the complete source lets the metadata path wrap at those
                        // minimums. The estimated regular path intentionally retains WinUI's
                        // overflowing rows when a hard MinWidth prevents its items from shrinking.
                        let minimums: Vec<_> = owner
                            .items
                            .snapshot()
                            .iter()
                            .map(|item| {
                                if item.starts_with("Inserted item ") {
                                    192.0
                                } else {
                                    128.0
                                }
                            })
                            .collect();
                        let ratios: Vec<_> = minimums.iter().map(|width| width / 64.0).collect();
                        args.set_items_range_start_index(0)
                            .expect("the source starts at zero");
                        args.set_desired_aspect_ratios(&ratios)
                            .expect("the complete source covers the requested range");
                        args.set_min_widths(&minimums)
                            .expect("each aspect ratio has a matching minimum width");
                    }
                });
            }
            let owner = this.downgrade();
            button.click(move |_, _| {
                if let Some(owner) = owner.upgrade() {
                    owner
                        .get_control::<ItemsRepeater>("Repeater")
                        .set_layout(Some(layout.clone()));
                }
            });
        }
        let owner = this.downgrade();
        this.get_control::<Button>("InsertItemButton")
            .click(move |_, _| {
                if let Some(owner) = owner.upgrade() {
                    let index = owner.next_item.get();
                    owner.next_item.set(index + 1);
                    owner.items.insert(0, format!("Inserted item {index:05}"));
                    owner.refresh_status();
                }
            });
        let owner = this.downgrade();
        this.get_control::<Button>("RemoveItemButton")
            .click(move |_, _| {
                if let Some(owner) = owner.upgrade() {
                    if owner.items.count() != 0 {
                        owner.items.remove_at(0);
                    }
                    owner.refresh_status();
                }
            });
        let owner = this.downgrade();
        this.get_control::<Button>("MoveItemButton")
            .click(move |_, _| {
                if let Some(owner) = owner.upgrade() {
                    let count = owner.items.count();
                    if count > 1 {
                        owner.items.move_item(0, count - 1);
                    }
                }
            });
        let owner = this.downgrade();
        this.get_control::<Button>("ResetItemsButton")
            .click(move |_, _| {
                if let Some(owner) = owner.upgrade() {
                    owner.reset_items();
                    owner.refresh_status();
                }
            });
        let owner = this.downgrade();
        repeater.element_prepared(move |_| {
            if let Some(owner) = owner.upgrade() {
                owner.prepared_count.set(owner.prepared_count.get() + 1);
            }
        });
        let owner = this.downgrade();
        repeater.element_clearing(move |_| {
            if let Some(owner) = owner.upgrade() {
                owner.cleared_count.set(owner.cleared_count.get() + 1);
            }
        });
        let owner = this.downgrade();
        repeater.layout_updated(move || {
            if let Some(owner) = owner.upgrade() {
                owner.refresh_status();
            }
        });
        this.refresh_status();
        this
    }

    fn reset_items(&self) {
        self.items.clear();
        self.items
            .add_range((0..10_000).map(|index| format!("Item {index:05}")));
        self.next_item.set(10_000);
    }

    fn refresh_status(&self) {
        let repeater = self.get_control::<ItemsRepeater>("Repeater");
        let status = format!(
            "{} items · {} realized · {} prepared · {} cleared",
            self.items.count(),
            repeater.realized_elements().len(),
            self.prepared_count.get(),
            self.cleared_count.get(),
        );
        self.get_control::<TextBlock>("RealizationStatus")
            .set_text(Some(&status));
    }
}
