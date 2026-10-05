//! The control theme of the pips pager of the test theme: the theme of the
//! reference simple theme built in code, with the same template, part names,
//! bindings and nested styles.
//!
//! Left out: the setters whose values are theme brushes (the foreground of
//! the pager, the fills of the pips in their states) and the brush
//! transitions of the buttons and of the pips.

use crate::primitives::{ScrollBarVisibility, SelectingItemsControl, TemplatedControl};
use crate::shapes::Ellipse;
use crate::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, ITemplateOf};
use crate::{Button, Control, ItemsControl, ListBox, ListBoxItem, Panel, PathIcon, PipsPager, ScrollViewer, StackPanel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::{BindingMode, ReflectionBinding, RelativeSource, RelativeSourceMode, TemplateBinding};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::media::{Brushes, Geometry, IBrush};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style, Styles};
use ferroui_base::{AnyValue, FerroObject, FerroProperty, Ref, StaticType, Thickness, Visual};
use std::rc::Rc;

const PREVIOUS_GLYPH: &str = "M 8.12,2.29 L 3.41,7.00 C 3.14,7.27 3.14,7.71 3.41,7.98 L 8.12,12.69 C 8.57,13.14 9.33,12.82 9.33,12.19 L 9.33,2.79 C 9.33,2.16 8.57,1.84 8.12,2.29 Z";
const NEXT_GLYPH: &str = "M 3.88,2.29 L 8.59,7.00 C 8.86,7.27 8.86,7.71 8.59,7.98 L 3.88,12.69 C 3.43,13.14 2.67,12.82 2.67,12.19 L 2.67,2.79 C 2.67,2.16 3.43,1.84 3.88,2.29 Z";
const PREVIOUS_GLYPH_VERTICAL: &str = "M 2.29,9.33 L 7.00,4.62 C 7.27,4.35 7.71,4.35 7.98,4.62 L 12.69,9.33 C 13.14,9.78 12.82,10.54 12.19,10.54 L 2.79,10.54 C 2.16,10.54 1.84,9.78 2.29,9.33 Z";
const NEXT_GLYPH_VERTICAL: &str = "M 2.29,4.46 L 7.00,9.17 C 7.27,9.44 7.71,9.44 7.98,9.17 L 12.69,4.46 C 13.14,4.01 12.82,3.25 12.19,3.25 L 2.79,3.25 C 2.16,3.25 1.84,4.01 2.29,4.46 Z";

fn bind_template(target: &FerroObject, target_property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(target_property, &TemplateBinding::new(source));
}

fn transparent() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::transparent())
}

fn glyph(data: &str) -> Option<Ref<Geometry>> {
    Some(Geometry::parse(data).expect("the glyph of the reference theme is a valid path"))
}

/// One of the two navigation buttons of the template.
fn navigation_button(
    name: &str,
    theme: &'static FerroProperty,
    is_visible: &'static FerroProperty,
    data: &str,
) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.to_string()));
    bind_template(&button, ferroui_base::StyledElement::theme_property().as_property(), theme);
    button.set_width(24.0);
    button.set_height(24.0);
    button.set_padding(Thickness::uniform(0.0));
    button.set_horizontal_content_alignment(HorizontalAlignment::Center);
    button.set_vertical_content_alignment(VerticalAlignment::Center);
    bind_template(&button, Visual::is_visible_property().as_property(), is_visible);
    button.set_vertical_alignment(VerticalAlignment::Center);
    button.set_horizontal_alignment(HorizontalAlignment::Center);
    button.set_margin(Thickness::uniform(4.0));

    let icon = PathIcon::new();
    icon.set_width(12.0);
    icon.set_height(12.0);
    icon.set_data(glyph(data));
    button.set_content(Some(Control::boxed(icon)));
    button
}

/// The control template of the items of the pips list: a panel with the pip.
fn pip_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, _| {
        let panel = Panel::new();
        panel.set_background(transparent());

        let pip = Ellipse::new();
        pip.set_name(Some("Pip".to_string()));
        pip.set_width(12.0);
        pip.set_height(12.0);
        panel.children().add(pip);
        panel.upcast()
    })
}

/// The pips list of the template, with its items panel and its styles.
fn pips_list() -> Ref<ListBox> {
    let list = ListBox::new();
    list.set_name(Some("PART_PipsPagerList".to_string()));
    list.set_background(transparent());
    list.set_border_thickness(Thickness::uniform(0.0));
    list.set_padding(Thickness::uniform(0.0));
    list.set_horizontal_alignment(HorizontalAlignment::Center);
    list.set_vertical_alignment(VerticalAlignment::Center);
    list.set_value(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Hidden);
    list.set_value(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Hidden);

    let items_source = ReflectionBinding::new("TemplateSettings.Pips");
    items_source.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    list.bind_binding(ItemsControl::items_source_property().as_property(), &items_source);

    let selected_index = ReflectionBinding::new("SelectedPageIndex").with_mode(BindingMode::TwoWay);
    selected_index.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    list.bind_binding(SelectingItemsControl::selected_index_property().as_property(), &selected_index);

    list.set_auto_scroll_to_selected_item(false);
    list.set_clip_to_bounds(false);

    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
        let panel = StackPanel::new();
        let orientation = ReflectionBinding::new("Orientation");
        orientation.set_relative_source(Some(
            RelativeSource::new(RelativeSourceMode::FindAncestor).with_ancestor_type(Some(PipsPager::TYPE)),
        ));
        panel.bind_binding(StackPanel::orientation_property().as_property(), &orientation);
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(4.0);
        Some(panel.upcast::<Panel>())
    });
    list.set_items_panel(items_panel);

    let styles = list.styles();
    styles.add(Style::with_setters(
        Selectors::of_type::<ScrollViewer>(),
        [Setter::new(Visual::clip_to_bounds_property(), false)],
    ));
    styles.add(Style::with_setters(
        Selectors::of_type::<ListBoxItem>(),
        [
            Setter::new(Layoutable::width_property(), 12.0),
            Setter::new(Layoutable::height_property(), 24.0),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(0.0)),
            Setter::new(Layoutable::margin_property(), Thickness::uniform(0.0)),
            Setter::new(Layoutable::min_height_property(), 0.0),
            Setter::new(Layoutable::min_width_property(), 0.0),
            Setter::new(Visual::clip_to_bounds_property(), false),
            Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
            Setter::new(TemplatedControl::template_property(), Some(pip_template())),
        ],
    ));
    styles.add(Style::with_setters(
        Selectors::of_type::<PipsPager>().class(":vertical").descendant().of_type::<ListBoxItem>(),
        [Setter::new(Layoutable::width_property(), 24.0), Setter::new(Layoutable::height_property(), 12.0)],
    ));

    list
}

/// The control template of the pips pager.
pub fn pips_pager_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let root = StackPanel::new();
        root.set_name(Some("PART_RootPanel".to_string()));
        bind_template(
            &root,
            StackPanel::orientation_property().as_property(),
            PipsPager::orientation_property().as_property(),
        );
        root.set_horizontal_alignment(HorizontalAlignment::Center);
        root.set_vertical_alignment(VerticalAlignment::Center);
        root.set_clip_to_bounds(false);

        let previous = navigation_button(
            "PART_PreviousButton",
            PipsPager::previous_button_theme_property().as_property(),
            PipsPager::is_previous_button_visible_property().as_property(),
            PREVIOUS_GLYPH,
        );
        root.children().add(previous.register_in_name_scope(&**ns));

        root.children().add(pips_list().register_in_name_scope(&**ns));

        let next = navigation_button(
            "PART_NextButton",
            PipsPager::next_button_theme_property().as_property(),
            PipsPager::is_next_button_visible_property().as_property(),
            NEXT_GLYPH,
        );
        root.children().add(next.register_in_name_scope(&**ns));

        root.register_in_name_scope(&**ns).upcast()
    })
}

/// The control theme of the pips pager. `button_theme` is the control theme
/// of the button the reference theme gives to the two navigation buttons
/// (its static resource reference).
pub fn pips_pager_theme(button_theme: Option<Ref<ControlTheme>>) -> Ref<ControlTheme> {
    let theme = ControlTheme::with_setters(
        PipsPager::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::border_brush_property(), transparent()),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0)),
            Setter::new(InputElement::is_tab_stop_property(), false),
            Setter::new(PipsPager::previous_button_theme_property(), button_theme.clone()),
            Setter::new(PipsPager::next_button_theme_property(), button_theme),
            Setter::new(TemplatedControl::template_property(), Some(pips_pager_template())),
        ],
    );

    let button = |classes: &[&str], name: &str| {
        let mut selector = Selectors::nesting(None);
        for class in classes {
            selector = selector.class(class);
        }
        selector.template().of_type::<Button>().name(name)
    };

    theme.add_style(Style::with_setters(
        button(&[":vertical"], "PART_PreviousButton").descendant().of_type::<PathIcon>(),
        [Setter::new(PathIcon::data_property(), glyph(PREVIOUS_GLYPH_VERTICAL))],
    ));
    theme.add_style(Style::with_setters(
        button(&[":vertical"], "PART_NextButton").descendant().of_type::<PathIcon>(),
        [Setter::new(PathIcon::data_property(), glyph(NEXT_GLYPH_VERTICAL))],
    ));
    theme.add_style(Style::with_setters(
        button(&[":first-page"], "PART_PreviousButton"),
        [Setter::new(Visual::opacity_property(), 0.0), Setter::new(InputElement::is_hit_test_visible_property(), false)],
    ));
    theme.add_style(Style::with_setters(
        button(&[":last-page"], "PART_NextButton"),
        [Setter::new(Visual::opacity_property(), 0.0), Setter::new(InputElement::is_hit_test_visible_property(), false)],
    ));

    theme
}

/// Adds the control theme of the pips pager. The theme of the navigation
/// buttons is the button theme `styles` holds at this point, if any.
pub fn add_pips_pager_themes(styles: &Ref<Styles>) {
    let button_theme = styles
        .resources()
        .try_get_resource(&ResourceKey::Type(<Button as StaticType>::TYPE), None)
        .flatten()
        .and_then(|value| {
            let value: &dyn AnyValue = &*value;
            value.downcast_ref::<Ref<ControlTheme>>().cloned()
        });
    styles.resources().add_value(ResourceKey::Type(PipsPager::TYPE), pips_pager_theme(button_theme));
}
