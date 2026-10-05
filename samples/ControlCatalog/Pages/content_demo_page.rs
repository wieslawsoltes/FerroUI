//! Port of `Pages/ContentDemoPage.xaml.cs`: the class of the document
//! `Pages/ContentDemoPage.xaml`.

use super::{
    ContentPageCommandBarPage, ContentPageCustomizationPage, ContentPageEventsPage, ContentPageFirstLookPage,
    ContentPageSafeAreaPage,
};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, FontWeight, Geometry, IBrush, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, Ref, Thickness};
use ferroui_controls::{
    Button, ColumnDefinitions, ContentPage, Control, Grid, NavigationPage, PathIcon, ScrollViewer, StackPanel,
    TextBlock, UserControl, WrapPanel,
};
use std::rc::Rc;

/// An entry of the registry of the samples: its group, title, description
/// and the factory of its control.
///
/// The entry "Performance Monitor" (group "Performance") of the original is
/// not listed: its page, `ContentPagePerformancePage`, is not ported (see
/// `excluded.txt`).
type Demo = (&'static str, &'static str, &'static str, fn() -> Ref<UserControl>);

const DEMOS: &[Demo] = &[
    // Overview
    (
        "Overview",
        "First Look",
        "Basic ContentPage with header, content, and background inside a NavigationPage.",
        || ContentPageFirstLookPage::new().upcast(),
    ),
    // Appearance
    (
        "Appearance",
        "Customization",
        "Adjust content alignment, background color, and padding to understand how ContentPage adapts its layout.",
        || ContentPageCustomizationPage::new().upcast(),
    ),
    // Features
    (
        "Features",
        "CommandBar",
        "Attach a CommandBar to the top or bottom of a ContentPage. Add and remove items at runtime.",
        || ContentPageCommandBarPage::new().upcast(),
    ),
    (
        "Features",
        "Safe Area",
        "Understand how AutomaticallyApplySafeAreaPadding absorbs platform insets.",
        || ContentPageSafeAreaPage::new().upcast(),
    ),
    (
        "Features",
        "Events",
        "Observe page lifecycle events: NavigatedTo, NavigatedFrom, and Navigating.",
        || ContentPageEventsPage::new().upcast(),
    ),
];

#[repr(C)]
pub struct ContentDemoPage {
    base: ContentPage,
}

content_page_class!(ContentDemoPage);
ferro_class_info!(ContentDemoPage { new: ContentDemoPage::new });
xaml_class!(ContentDemoPage, "/Pages/ContentDemoPage.xaml");

impl ContentDemoPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn sample_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("SampleNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let _ = self.sample_nav().push_async_with_transition(self.create_home_page(), None);
    }

    fn create_home_page(&self) -> Ref<ContentPage> {
        let stack = StackPanel::new();
        stack.set_margin(Thickness::uniform(12.0));
        stack.set_spacing(16.0);

        // The groups in the order of their first sample.
        let mut groups: Vec<(&'static str, Ref<WrapPanel>)> = Vec::new();

        for &(group, title, description, factory) in DEMOS {
            if !groups.iter().any(|(name, _)| *name == group) {
                let panel = WrapPanel::new();
                panel.set_orientation(Orientation::Horizontal);
                panel.set_horizontal_alignment(HorizontalAlignment::Left);
                groups.push((group, panel));
            }

            let demo_factory = factory;
            let demo_title = title;

            let title_text = TextBlock::new();
            title_text.set_text(Some(title));
            title_text.set_font_size(13.0);
            title_text.set_font_weight(FontWeight::SemiBold);
            title_text.set_text_wrapping(TextWrapping::Wrap);

            let description_text = TextBlock::new();
            description_text.set_text(Some(description));
            description_text.set_font_size(11.0);
            description_text.set_opacity(0.6);
            description_text.set_text_wrapping(TextWrapping::Wrap);

            let card_content = StackPanel::new();
            card_content.set_spacing(4.0);
            card_content.children().add(title_text);
            card_content.children().add(description_text);

            let card = Button::new();
            card.set_width(170.0);
            card.set_min_height(80.0);
            card.set_margin(Thickness::new(0.0, 0.0, 8.0, 8.0));
            card.set_vertical_alignment(VerticalAlignment::Top);
            card.set_horizontal_content_alignment(HorizontalAlignment::Left);
            card.set_vertical_content_alignment(VerticalAlignment::Top);
            card.set_padding(Thickness::symmetric(12.0, 8.0));
            card.set_content(Some(Control::boxed(card_content)));

            // The card is a descendant of the page: its handler holds the page weakly.
            let weak = self.to_ref().downgrade();
            card.click(move |_s, _e| {
                let Some(this) = weak.upgrade() else {
                    return;
                };

                let header_grid = Grid::new();
                header_grid.set_column_definitions(match ColumnDefinitions::parse("*, Auto") {
                    Ok(definitions) => definitions,
                    Err(error) => panic!("{error}"),
                });
                let header_text = TextBlock::new();
                header_text.set_text(Some(demo_title));
                header_text.set_vertical_alignment(VerticalAlignment::Center);
                header_grid.children().add(header_text);

                let close_icon = PathIcon::new();
                match Geometry::parse(CLOSE_ICON) {
                    Ok(data) => close_icon.set_data(data),
                    Err(error) => panic!("{error}"),
                }
                let close_btn = Button::new();
                close_btn.set_content(Some(Control::boxed(close_icon)));
                let transparent: Rc<dyn IBrush> = Brushes::transparent();
                close_btn.set_background(Some(transparent));
                close_btn.set_border_thickness(Thickness::uniform(0.0));
                close_btn.set_padding(Thickness::symmetric(8.0, 4.0));
                close_btn.set_vertical_alignment(VerticalAlignment::Center);
                Grid::set_column(&close_btn, 1);
                header_grid.children().add(close_btn.clone());
                let weak_page = this.downgrade();
                close_btn.click(move |_, _| {
                    if let Some(this) = weak_page.upgrade() {
                        let _ = this.sample_nav().pop_async_with_transition(None);
                    }
                });

                let page = ContentPage::new();
                page.set_header(Some(Control::boxed(header_grid)));
                page.set_content(Some(Control::boxed(demo_factory())));
                page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
                page.set_vertical_content_alignment(VerticalAlignment::Stretch);
                NavigationPage::set_has_back_button(&page, false);
                let _ = this.sample_nav().push_async_with_transition(page, None);
            });

            if let Some((_, panel)) = groups.iter().find(|(name, _)| *name == group) {
                panel.children().add(card);
            }
        }

        for (group_name, panel) in groups {
            let group_text = TextBlock::new();
            group_text.set_text(Some(group_name));
            group_text.set_font_size(13.0);
            group_text.set_font_weight(FontWeight::SemiBold);
            group_text.set_margin(Thickness::new(0.0, 0.0, 0.0, 4.0));
            group_text.set_opacity(0.6);
            stack.children().add(group_text);
            stack.children().add(panel);
        }

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(stack)));

        let home_page = ContentPage::new();
        home_page.set_content(Some(Control::boxed(scroll_viewer)));
        home_page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        home_page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        NavigationPage::set_has_navigation_bar(&home_page, false);
        home_page
    }
}

const CLOSE_ICON: &str = "M4.397 4.397a1 1 0 0 1 1.414 0L12 10.585l6.19-6.188a1 1 0 0 1 1.414 1.414L13.413 12l6.19 6.189a1 1 0 0 1-1.414 1.414L12 13.413l-6.189 6.19a1 1 0 0 1-1.414-1.414L10.585 12 4.397 5.811a1 1 0 0 1 0-1.414z";
