//! Port of `Pages/PipsPager/PipsPagerEventsPage.xaml.cs`: the class of the
//! document `Pages/PipsPager/PipsPagerEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_class_info, instantiate, FerroPropertyChangedEventArgs, Ref};
use ferroui_controls::{ItemsControl, ItemsSource, PipsPager, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct PipsPagerEventsPage {
    base: UserControl,
    events: FerroList<String>,
}

user_control_class!(PipsPagerEventsPage);
ferro_class_info!(PipsPagerEventsPage { new: PipsPagerEventsPage::new });
xaml_class!(PipsPagerEventsPage, "/Pages/PipsPager/PipsPagerEventsPage.xaml");

impl PipsPagerEventsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), events: FerroList::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.event_log().set_items_source(Some(ItemsSource::new(Rc::new(this.events.clone()))));

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = this.downgrade();
        this.event_pager().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_event_pager_property_changed(e);
            }
        });
        this
    }

    fn event_log(&self) -> Ref<ItemsControl> {
        self.get_control::<ItemsControl>("EventLog")
    }

    fn event_pager(&self) -> Ref<PipsPager> {
        self.get_control::<PipsPager>("EventPager")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn on_event_pager_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() != PipsPager::selected_page_index_property().as_property() {
            return;
        }

        let new_index = e.get_new_value::<i32>();
        self.status_text().set_text(Some(&format!("Selected: {new_index}")));
        self.events.insert(0, format!("SelectedPageIndex changed to {new_index}"));

        if self.events.count() > 20 {
            self.events.remove_at(self.events.count() - 1);
        }
    }
}
