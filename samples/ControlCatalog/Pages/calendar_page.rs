//! Port of `Pages/CalendarPage.xaml.cs`: the class of the document
//! `Pages/CalendarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Calendar, CalendarDateRange, ContentPage, Control, ItemsSource, Panel, WrapPanel};
use ferroui_controls_repeater::{ItemsRepeater, UniformGridLayout};

#[repr(C)]
pub struct CalendarPage {
    base: ContentPage,
}

content_page_class!(CalendarPage);
ferro_class_info!(CalendarPage { new: CalendarPage::new });
xaml_class!(CalendarPage, "/Pages/CalendarPage.xaml");

impl CalendarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let today = DateTime::today();
        this.display_dates_calendar().set_display_date_start(Some(today.add_days(-25.0)));
        this.display_dates_calendar().set_display_date_end(Some(today.add_days(25.0)));

        this.blackout_dates_calendar().blackout_dates().add_dates_in_past();
        this.blackout_dates_calendar().blackout_dates().add(CalendarDateRange::new(today.add_days(6.0)));
        this.virtualize_examples();
        this
    }

    /// Keep the markup's controls and state, but realize their templates only
    /// near the scroll viewport. A plain WrapPanel measures all nine calendars.
    fn virtualize_examples(&self) {
        let examples = self
            .display_dates_calendar()
            .parent()
            .and_then(|group| group.parent())
            .and_then(|panel| panel.cast::<WrapPanel>())
            .expect("the calendar examples are in a wrap panel");
        let parent = examples
            .parent()
            .and_then(|panel| panel.cast::<Panel>())
            .expect("the wrap panel has a panel parent");
        let index = parent
            .children()
            .index_of(&examples.clone().upcast())
            .expect("the examples are a child");
        let items = examples.children().snapshot();
        examples.children().clear();

        let repeater = ItemsRepeater::new();
        repeater.set_name(Some("CalendarExamplesRepeater".to_string()));
        repeater.set_max_width(examples.max_width());
        repeater.set_horizontal_alignment(examples.horizontal_alignment());
        repeater.set_margin(examples.margin());
        KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Continue);
        repeater.set_horizontal_cache_length(0.0);
        repeater.set_vertical_cache_length(0.0);
        // The untouched uniform grid measures the first item for its cell size,
        // so calendar sizing follows the active theme rather than a fixed height.
        repeater.set_layout(Some(UniformGridLayout::new().upcast()));
        let styles = examples.styles().snapshot();
        examples.styles().clear();
        repeater.styles().add_range(styles.iter().cloned());
        // Direct control items retain their selection/configuration when recycled.
        repeater.set_items_source(Some(ItemsSource::from_items(
            items.iter().map(|item| Some(Control::boxed(item))),
        )));
        parent.children().set(index, repeater.upcast::<Control>());
    }

    fn display_dates_calendar(&self) -> Ref<Calendar> {
        self.get_control::<Calendar>("DisplayDatesCalendar")
    }

    fn blackout_dates_calendar(&self) -> Ref<Calendar> {
        self.get_control::<Calendar>("BlackoutDatesCalendar")
    }
}
