//! Port of `Pages/NavigationDemoHelper.cs`.

use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{
    Brushes, Color, FontStyle, FontWeight, Geometry, IBrush, SolidColorBrush, TextAlignment, TextWrapping,
};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{
    Button, ColumnDefinitions, ContentPage, Control, Grid, NavigationPage, PathIcon, ScrollViewer, Separator,
    StackPanel, TextBlock, UserControl, WrapPanel,
};
use std::rc::Rc;

/// An entry of a registry of demos: its group, title, description and the
/// factory of its control.
pub(crate) type Demo = (&'static str, &'static str, &'static str, fn() -> Ref<UserControl>);

/// The colors of the pastel background brushes cycled by page index.
const PAGE_COLORS: [&str; 6] = ["#BBDEFB", "#C8E6C9", "#FFE0B2", "#E1BEE7", "#FFCDD2", "#B2EBF2"];

const CLOSE_ICON: &str = "M4.397 4.397a1 1 0 0 1 1.414 0L12 10.585l6.19-6.188a1 1 0 0 1 1.414 1.414L13.413 12l6.19 6.189a1 1 0 0 1-1.414 1.414L12 13.413l-6.189 6.19a1 1 0 0 1-1.414-1.414L10.585 12 4.397 5.811a1 1 0 0 1 0-1.414z";

/// `Color.Parse(text)`.
///
/// # Panics
/// Panics if the text is not a color (the format exception of the original).
pub(crate) fn parse_color(text: &str) -> Color {
    match Color::parse(text) {
        Ok(color) => color,
        Err(error) => panic!("{error}"),
    }
}

/// `Geometry.Parse(text)`.
///
/// # Panics
/// Panics if the text is not path data (the format exception of the original).
pub(crate) fn parse_geometry(text: &str) -> Ref<Geometry> {
    match Geometry::parse(text) {
        Ok(geometry) => geometry,
        Err(error) => panic!("{error}"),
    }
}

/// A string as an untyped value (`object`).
pub(crate) fn boxed_text(text: &str) -> BoxedValue {
    Rc::new(text.to_string())
}

/// `value?.ToString()` of an untyped value that is a string or a control
/// (the headers of the demo pages): the text, or the name of the class.
pub(crate) fn value_text(value: &Option<BoxedValue>) -> Option<String> {
    let value = value.as_ref()?;
    if let Some(text) = value.downcast_ref::<String>() {
        return Some(text.clone());
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return Some((*text).to_string());
    }
    Some(match Control::from_boxed(value) {
        Some(control) => control.get_type().full_name(),
        None => String::new(),
    })
}

/// Shared helpers for the demo pages of the catalog.
pub(crate) struct NavigationDemoHelper;

impl NavigationDemoHelper {
    thread_local! {
        /// Pastel background brushes cycled by page index.
        static PAGE_BRUSHES: Vec<Rc<dyn IBrush>> = PAGE_COLORS
            .iter()
            .map(|color| SolidColorBrush::with_color(parse_color(color)).into())
            .collect();
    }

    pub(crate) fn get_page_brush(index: i32) -> Rc<dyn IBrush> {
        Self::PAGE_BRUSHES.with(|brushes| {
            let length = brushes.len() as i32;
            brushes[(((index % length) + length) % length) as usize].clone()
        })
    }

    /// Creates a simple demo page with a centered title and subtitle.
    pub(crate) fn make_page(header: &str, body: &str, color_index: i32) -> Ref<ContentPage> {
        let header_text = TextBlock::new();
        header_text.set_text(Some(header));
        header_text.set_font_size(20.0);
        header_text.set_font_weight(FontWeight::SemiBold);
        header_text.set_horizontal_alignment(HorizontalAlignment::Center);

        let body_text = TextBlock::new();
        body_text.set_text(Some(body));
        body_text.set_font_size(13.0);
        body_text.set_opacity(0.7);
        body_text.set_text_wrapping(TextWrapping::Wrap);
        body_text.set_text_alignment(TextAlignment::Center);
        body_text.set_max_width(260.0);

        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(8.0);
        panel.children().add(header_text);
        panel.children().add(body_text);

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(header)));
        page.set_background(Some(Self::get_page_brush(color_index)));
        page.set_content(Some(Control::boxed(panel)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }

    /// Creates a demo page with an icon, title, body, and hint text (used
    /// by the detail pages of the drawer page demos).
    #[allow(dead_code)] // Used by the demos of the drawer page, which are not ported yet.
    pub(crate) fn make_section_page(
        header: &str,
        icon_data: &str,
        title: &str,
        body: &str,
        color_index: i32,
        hint: Option<&str>,
    ) -> Ref<ContentPage> {
        let panel = StackPanel::new();
        panel.set_margin(Thickness::symmetric(24.0, 20.0));
        panel.set_spacing(12.0);

        let icon = PathIcon::new();
        icon.set_width(48.0);
        icon.set_height(48.0);
        icon.set_data(parse_geometry(icon_data));
        icon.set_foreground(Some(SolidColorBrush::with_color(parse_color("#0078D4")).into()));
        panel.children().add(icon);

        let title_text = TextBlock::new();
        title_text.set_text(Some(title));
        title_text.set_font_size(26.0);
        title_text.set_font_weight(FontWeight::Bold);
        panel.children().add(title_text);

        let body_text = TextBlock::new();
        body_text.set_text(Some(body));
        body_text.set_font_size(14.0);
        body_text.set_opacity(0.8);
        body_text.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(body_text);

        let separator = Separator::new();
        separator.set_margin(Thickness::symmetric(0.0, 8.0));
        panel.children().add(separator);

        if let Some(hint) = hint {
            let hint_text = TextBlock::new();
            hint_text.set_text(Some(hint));
            hint_text.set_font_size(12.0);
            hint_text.set_opacity(0.45);
            hint_text.set_font_style(FontStyle::Italic);
            hint_text.set_text_wrapping(TextWrapping::Wrap);
            panel.children().add(hint_text);
        }

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(panel)));

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(header)));
        page.set_background(Some(Self::get_page_brush(color_index)));
        page.set_content(Some(Control::boxed(scroll_viewer)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }

    /// Builds the demo gallery home page for the registries of demos of the
    /// navigation, tabbed and drawer pages.
    pub(crate) fn create_gallery_home_page(nav: &Ref<NavigationPage>, demos: &[Demo]) -> Ref<ContentPage> {
        let stack = StackPanel::new();
        stack.set_margin(Thickness::uniform(12.0));
        stack.set_spacing(16.0);

        // The groups in the order of their first demo.
        let mut groups: Vec<(&'static str, Ref<WrapPanel>)> = Vec::new();

        for &(group, title, description, factory) in demos {
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

            // The card ends up in a page of `nav`: its handler holds `nav` weakly.
            let weak_nav = nav.downgrade();
            card.click(move |_, _| {
                let Some(nav) = weak_nav.upgrade() else {
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
                close_icon.set_data(parse_geometry(CLOSE_ICON));
                let close_btn = Button::new();
                close_btn.set_content(Some(Control::boxed(close_icon)));
                let transparent: Rc<dyn IBrush> = Brushes::transparent();
                close_btn.set_background(Some(transparent));
                close_btn.set_border_thickness(Thickness::uniform(0.0));
                close_btn.set_padding(Thickness::symmetric(8.0, 4.0));
                close_btn.set_vertical_alignment(VerticalAlignment::Center);
                Grid::set_column(&close_btn, 1);
                header_grid.children().add(close_btn.clone());
                let weak_nav = nav.downgrade();
                close_btn.click(move |_, _| {
                    if let Some(nav) = weak_nav.upgrade() {
                        drop(nav.pop_async_with_transition(None));
                    }
                });

                let page = ContentPage::new();
                page.set_header(Some(Control::boxed(header_grid)));
                page.set_content(Some(Control::boxed(demo_factory())));
                page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
                page.set_vertical_content_alignment(VerticalAlignment::Stretch);
                NavigationPage::set_has_back_button(&page, false);
                drop(nav.push_async_with_transition(page, None));
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

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};

    #[test]
    fn page_brushes_cycle_in_both_directions() {
        let first = NavigationDemoHelper::get_page_brush(0);
        assert!(*first == *NavigationDemoHelper::get_page_brush(6));
        assert!(*first == *NavigationDemoHelper::get_page_brush(-6));
        assert!(*NavigationDemoHelper::get_page_brush(5) == *NavigationDemoHelper::get_page_brush(-1));
        assert!(*first != *NavigationDemoHelper::get_page_brush(1));
    }

    #[test]
    fn make_page_sets_the_header_and_the_background() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let page = NavigationDemoHelper::make_page("Home", "body", 2);
        assert_eq!(Some(String::from("Home")), value_text(&page.header()));
        assert!(page.background().is_some_and(|brush| *brush == *NavigationDemoHelper::get_page_brush(2)));
    }
}
