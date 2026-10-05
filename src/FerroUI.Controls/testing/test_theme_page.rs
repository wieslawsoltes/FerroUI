//! The control themes of the pages of the test theme: the themes of the
//! reference simple theme built in code, with the same templates, part names
//! and bindings.
//!
//! Covered: the content page, the carousel page and the carousel the
//! carousel page hosts. Left out: the background and foreground setters,
//! whose values are theme brushes.

use crate::page::{CarouselPage, ContentPage, MultiPage};
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{ScrollBarVisibility, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{Carousel, ContentControl, Control, Dock, DockPanel, ItemsControl, ScrollViewer};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::styling::{ControlTheme, IStyle, Setter, Styles};
use ferroui_base::{FerroObject, FerroProperty, Ref, StaticType};
use std::rc::Rc;

fn bind_template(target: &FerroObject, target_property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(target_property, &TemplateBinding::new(source));
}

/// The control template of the content page.
pub fn content_page_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let panel = DockPanel::new();

        // Visibility managed in code-behind.
        let top_command_bar = ContentPresenter::new();
        top_command_bar.set_name(Some("PART_TopCommandBar".to_string()));
        DockPanel::set_dock(&top_command_bar, Dock::Top);
        top_command_bar.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        top_command_bar.set_is_visible(false);
        panel.children().add(top_command_bar.register_in_name_scope(&**ns));

        // Visibility managed in code-behind.
        let bottom_command_bar = ContentPresenter::new();
        bottom_command_bar.set_name(Some("PART_BottomCommandBar".to_string()));
        DockPanel::set_dock(&bottom_command_bar, Dock::Bottom);
        bottom_command_bar.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        bottom_command_bar.set_is_visible(false);
        panel.children().add(bottom_command_bar.register_in_name_scope(&**ns));

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            bind_template(&presenter, target, source);
        };
        bind(
            ContentPresenter::horizontal_content_alignment_property().as_property(),
            ContentPage::horizontal_content_alignment_property().as_property(),
        );
        bind(
            ContentPresenter::vertical_content_alignment_property().as_property(),
            ContentPage::vertical_content_alignment_property().as_property(),
        );
        bind(ContentPresenter::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(
            ContentPresenter::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        bind(
            ContentPresenter::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        bind(
            ContentPresenter::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        panel.children().add(presenter.register_in_name_scope(&**ns));

        panel.upcast()
    })
}

/// The control template of the carousel.
pub fn carousel_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        bind_template(
            &items_presenter,
            ItemsPresenter::items_panel_property().as_property(),
            ItemsControl::items_panel_property().as_property(),
        );
        bind_template(
            &items_presenter,
            Layoutable::margin_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        for property in [
            TemplatedControl::background_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        ] {
            bind_template(&scroll_viewer, property, property);
        }
        scroll_viewer.set_bring_into_view_on_focus_change(false);
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_content(Some(Control::boxed(items_presenter.register_in_name_scope(&**ns))));
        scroll_viewer.register_in_name_scope(&**ns).upcast()
    })
}

/// The control template of the carousel page.
pub fn carousel_page_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let carousel = Carousel::new();
        carousel.set_name(Some("PART_Carousel".to_string()));
        bind_template(
            &carousel,
            TemplatedControl::background_property().as_property(),
            TemplatedControl::background_property().as_property(),
        );
        bind_template(
            &carousel,
            ItemsControl::item_template_property().as_property(),
            MultiPage::page_template_property().as_property(),
        );
        bind_template(
            &carousel,
            Carousel::page_transition_property().as_property(),
            CarouselPage::page_transition_property().as_property(),
        );
        bind_template(
            &carousel,
            ItemsControl::items_panel_property().as_property(),
            CarouselPage::items_panel_property().as_property(),
        );
        carousel.set_horizontal_alignment(HorizontalAlignment::Stretch);
        carousel.set_vertical_alignment(VerticalAlignment::Stretch);
        carousel.register_in_name_scope(&**ns).upcast()
    })
}

fn template_theme<T: StaticType>(template: Rc<dyn IControlTemplate>) -> Ref<ControlTheme> {
    ControlTheme::with_setters(T::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

/// Adds the control themes of the content page, the carousel page and the
/// carousel.
pub fn add_page_themes(styles: &Ref<Styles>) {
    styles.resources().add_value(ResourceKey::Type(ContentPage::TYPE), template_theme::<ContentPage>(content_page_template()));
    styles.resources().add_value(ResourceKey::Type(Carousel::TYPE), template_theme::<Carousel>(carousel_template()));
    styles
        .resources()
        .add_value(ResourceKey::Type(CarouselPage::TYPE), template_theme::<CarouselPage>(carousel_page_template()));
}

/// Creates the test theme together with the themes of [`add_page_themes`]
/// and the scroll viewer themes the carousel needs.
pub fn create_page_test_theme() -> Rc<dyn IStyle> {
    let styles = Styles::new();
    styles.add(super::create_test_theme());
    super::add_split_view_list_box_themes(&styles);
    add_page_themes(&styles);
    ferroui_base::styling::styles_as_style(&styles)
}
