//! Port of `Pages/NavigationPage/NavigationPageMvvmPageFactory.cs`: the page factory of the
//! MVVM sample of the navigation page.

use super::navigation_page_mvvm_navigation::{ISamplePageFactory, SampleViewModel};
use super::navigation_page_mvvm_view_models::{
    ProjectActivityViewModel, ProjectCardViewModel, ProjectDetailViewModel, WorkspaceViewModel,
};
use crate::pages::navigation_demo_helper::boxed_text;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush, TextWrapping};
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, ContentPage, Control, Dock, DockPanel, ItemsControl, ItemsSource, NavigationPage, ScrollViewer,
    Separator, StackPanel, TextBlock,
};
use std::rc::Rc;

fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

/// `new SolidColorBrush(Color.FromArgb(alpha, color.R, color.G, color.B))` as the value of a
/// brush property.
fn tint(alpha: u8, color: Color) -> Option<Rc<dyn IBrush>> {
    solid(Color::from_argb(alpha, color.r, color.g, color.b))
}

pub(crate) struct SamplePageFactory;

impl ISamplePageFactory for SamplePageFactory {
    /// # Panics
    /// Panics if the view model is of a class the factory has no page for (the invalid
    /// operation exception of the original, whose message also names the class).
    fn create_page(&self, view_model: SampleViewModel) -> Ref<ContentPage> {
        if let Some(workspace) = view_model.downcast_ref::<WorkspaceViewModel>() {
            Self::create_workspace_page(workspace)
        } else if let Some(detail) = view_model.downcast_ref::<ProjectDetailViewModel>() {
            Self::create_project_detail_page(detail)
        } else if let Some(activity) = view_model.downcast_ref::<ProjectActivityViewModel>() {
            Self::create_project_activity_page(activity)
        } else {
            panic!("Unsupported view model")
        }
    }
}

impl SamplePageFactory {
    fn create_workspace_page(view_model: &WorkspaceViewModel) -> Ref<ContentPage> {
        let stack = StackPanel::new();
        stack.set_margin(Thickness::uniform(20.0));
        stack.set_spacing(14.0);

        let title = TextBlock::new();
        title.set_text(Some(view_model.title()));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);
        stack.children().add(title);

        let description = TextBlock::new();
        description.set_text(Some(view_model.description()));
        description.set_font_size(13.0);
        description.set_opacity(0.75);
        description.set_text_wrapping(TextWrapping::Wrap);
        stack.children().add(description);

        let projects = ItemsControl::new();
        projects.set_items_source(Some(ItemsSource::from_values(view_model.projects().iter().cloned())));
        // Deviation (DEVIATIONS.md, Templates): the template of the original also matches a
        // null item, for which it builds an empty text block; here null is not a project.
        projects.set_item_template(Some(FuncDataTemplate::for_type::<Rc<ProjectCardViewModel>>(
            |item, _| Some(Self::create_project_card(item).upcast()),
            false,
        )));
        stack.children().add(projects);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(stack)));

        let page = ContentPage::new();
        page.set_header(Some(boxed_text("Workspace")));
        page.set_content(Some(Control::boxed(scroll_viewer)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        NavigationPage::set_has_back_button(&page, false);
        page
    }

    /// The card of a project: the body of the item template of the workspace page.
    fn create_project_card(item: &ProjectCardViewModel) -> Ref<Border> {
        let accent_brush = solid(item.accent_color());

        let status = TextBlock::new();
        status.set_text(Some(item.status()));
        status.set_foreground(white());
        status.set_font_size(11.0);
        status.set_font_weight(FontWeight::SemiBold);

        let status_badge = Border::new();
        status_badge.set_background(accent_brush.clone());
        status_badge.set_corner_radius(CornerRadius::uniform(999.0));
        status_badge.set_padding(Thickness::symmetric(10.0, 4.0));
        status_badge.set_child(status);
        DockPanel::set_dock(&status_badge, Dock::Right);

        let name = TextBlock::new();
        name.set_text(Some(item.name()));
        name.set_font_size(17.0);
        name.set_font_weight(FontWeight::SemiBold);

        let header = DockPanel::new();
        header.children().add(status_badge);
        header.children().add(name);

        let summary = TextBlock::new();
        summary.set_text(Some(item.summary()));
        summary.set_font_size(13.0);
        summary.set_opacity(0.72);
        summary.set_text_wrapping(TextWrapping::Wrap);

        let details = TextBlock::new();
        details.set_text(Some(&format!("Owner: {}  \u{2022}  Next: {}", item.owner(), item.next_milestone())));
        details.set_font_size(12.0);
        details.set_opacity(0.6);
        details.set_text_wrapping(TextWrapping::Wrap);

        let open = Button::new();
        open.set_content(Some(boxed_text("Open Project")));
        open.set_horizontal_alignment(HorizontalAlignment::Left);
        open.set_command(Some(item.open_command().as_command()));

        let content = StackPanel::new();
        content.set_spacing(8.0);
        content.children().add(header);
        content.children().add(summary);
        content.children().add(details);
        content.children().add(open);

        let card = Border::new();
        card.set_background(tint(20, item.accent_color()));
        card.set_border_brush(accent_brush);
        card.set_border_thickness(Thickness::uniform(1.0));
        card.set_corner_radius(CornerRadius::uniform(8.0));
        card.set_padding(Thickness::uniform(14.0));
        card.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        card.set_child(content);
        card
    }

    fn create_project_detail_page(view_model: &ProjectDetailViewModel) -> Ref<ContentPage> {
        let accent_brush = solid(view_model.accent_color());
        let panel = StackPanel::new();
        panel.set_margin(Thickness::symmetric(24.0, 20.0));
        panel.set_spacing(12.0);

        let status = TextBlock::new();
        status.set_text(Some(view_model.status()));
        status.set_foreground(white());
        status.set_font_size(11.0);
        status.set_font_weight(FontWeight::SemiBold);

        let status_badge = Border::new();
        status_badge.set_background(accent_brush);
        status_badge.set_corner_radius(CornerRadius::uniform(999.0));
        status_badge.set_padding(Thickness::symmetric(12.0, 5.0));
        status_badge.set_horizontal_alignment(HorizontalAlignment::Left);
        status_badge.set_child(status);
        panel.children().add(status_badge);

        let name = TextBlock::new();
        name.set_text(Some(view_model.name()));
        name.set_font_size(26.0);
        name.set_font_weight(FontWeight::Bold);
        panel.children().add(name);

        let summary = TextBlock::new();
        summary.set_text(Some(view_model.summary()));
        summary.set_font_size(14.0);
        summary.set_opacity(0.78);
        summary.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(summary);

        let owner = TextBlock::new();
        owner.set_text(Some(&format!("Owner: {}", view_model.owner())));
        owner.set_font_size(13.0);
        owner.set_opacity(0.68);
        panel.children().add(owner);

        let next_milestone = TextBlock::new();
        next_milestone.set_text(Some(&format!("Next milestone: {}", view_model.next_milestone())));
        next_milestone.set_font_size(13.0);
        next_milestone.set_opacity(0.68);
        next_milestone.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(next_milestone);

        let separator = Separator::new();
        separator.set_margin(Thickness::symmetric(0.0, 4.0));
        panel.children().add(separator);

        let note = TextBlock::new();
        note.set_text(Some("This page is resolved by SamplePageFactory from a ProjectDetailViewModel. The view model only requests navigation through ISampleNavigationService."));
        note.set_font_size(12.0);
        note.set_opacity(0.7);
        note.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(note);

        let open_activity = Button::new();
        open_activity.set_content(Some(boxed_text("Open Activity")));
        open_activity.set_horizontal_alignment(HorizontalAlignment::Left);
        open_activity.set_command(Some(view_model.open_activity_command().as_command()));
        panel.children().add(open_activity);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(panel)));

        let page = ContentPage::new();
        page.set_header(Some(boxed_text(view_model.name())));
        page.set_background(tint(18, view_model.accent_color()));
        page.set_content(Some(Control::boxed(scroll_viewer)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }

    fn create_project_activity_page(view_model: &ProjectActivityViewModel) -> Ref<ContentPage> {
        let panel = StackPanel::new();
        panel.set_margin(Thickness::symmetric(24.0, 20.0));
        panel.set_spacing(10.0);

        let title = TextBlock::new();
        title.set_text(Some("Activity Timeline"));
        title.set_font_size(24.0);
        title.set_font_weight(FontWeight::Bold);
        panel.children().add(title);

        let description = TextBlock::new();
        description.set_text(Some(&format!(
            "Recent updates for {}. This page was opened from a command on the detail view model.",
            view_model.name()
        )));
        description.set_font_size(13.0);
        description.set_opacity(0.74);
        description.set_text_wrapping(TextWrapping::Wrap);
        panel.children().add(description);

        for item in view_model.items().iter() {
            let text = TextBlock::new();
            text.set_text(Some(item.as_str()));
            text.set_font_size(13.0);
            text.set_text_wrapping(TextWrapping::Wrap);

            let entry = Border::new();
            entry.set_background(tint(14, view_model.accent_color()));
            entry.set_border_brush(tint(80, view_model.accent_color()));
            entry.set_border_thickness(Thickness::uniform(1.0));
            entry.set_corner_radius(CornerRadius::uniform(6.0));
            entry.set_padding(Thickness::symmetric(12.0, 10.0));
            entry.set_child(text);
            panel.children().add(entry);
        }

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(panel)));

        let page = ContentPage::new();
        page.set_header(Some(boxed_text("Activity")));
        page.set_content(Some(Control::boxed(scroll_viewer)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);
        page
    }
}
