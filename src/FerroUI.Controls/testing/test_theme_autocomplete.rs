//! Control themes for the controls an auto-complete box is made of: the
//! text box, the list box and its items, the scroll viewer and the data
//! validation errors control. The templates are those of the reference
//! simple theme built in code, with the same element nesting, part names and
//! template bindings.
//!
//! They are not part of the test theme (a themed text box or scroll viewer
//! would change every test that shows one): a test adds them to the styles
//! of its application with [`add_autocomplete_themes`].
//!
//! Left out: the setters and nested styles of the themes whose values are
//! theme brushes and resources, the context flyout of the text box, the
//! visibility bindings of the placeholders (multi-bindings with converters;
//! the floating placeholder is hidden), the scroll bars and the scroll
//! gesture recognizer of the scroll viewer, and the error display of the
//! data validation errors control.

use super::test_theme::add_template_theme;
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter, TextPresenter};
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{
    Border, ColumnDefinition, ColumnDefinitions, ContentControl, Control, DataValidationErrors, Dock, DockPanel,
    Grid, GridLength, GridUnitType, ItemsControl, ListBox, ListBoxItem, Panel, RowDefinition, RowDefinitions,
    ScrollViewer, TextBlock, TextBox,
};
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::Styles;
use ferroui_base::{FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// Binds `target` of an element of a template to `source` of the templated
/// parent.
fn template_bind(element: &FerroObject, target: &'static FerroProperty, source: &'static FerroProperty) {
    element.bind_binding(target, &TemplateBinding::new(source));
}

/// The control template of the text box of the reference simple theme.
pub fn text_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TextBox>(|_, scope| {
        let floating_placeholder = TextBlock::new();
        floating_placeholder.set_name(Some("floatingPlaceholder".to_string()));
        DockPanel::set_dock(&floating_placeholder, Dock::Top);
        template_bind(
            &floating_placeholder,
            TextBlock::foreground_property().as_property(),
            TextBox::placeholder_foreground_property().as_property(),
        );
        template_bind(
            &floating_placeholder,
            TextBlock::text_property().as_property(),
            TextBox::placeholder_text_property().as_property(),
        );
        floating_placeholder.set_is_visible(false);

        let inner_left = ContentPresenter::new();
        Grid::set_column(&inner_left, 0);
        Grid::set_column_span(&inner_left, 1);
        template_bind(
            &inner_left,
            ContentPresenter::content_property().as_property(),
            TextBox::inner_left_content_property().as_property(),
        );

        let placeholder = TextBlock::new();
        placeholder.set_name(Some("placeholder".to_string()));
        template_bind(
            &placeholder,
            TextBlock::foreground_property().as_property(),
            TextBox::placeholder_foreground_property().as_property(),
        );
        template_bind(
            &placeholder,
            Layoutable::horizontal_alignment_property().as_property(),
            TextBox::horizontal_content_alignment_property().as_property(),
        );
        template_bind(
            &placeholder,
            Layoutable::vertical_alignment_property().as_property(),
            TextBox::vertical_content_alignment_property().as_property(),
        );
        template_bind(
            &placeholder,
            TextBlock::text_property().as_property(),
            TextBox::placeholder_text_property().as_property(),
        );
        template_bind(
            &placeholder,
            TextBlock::text_alignment_property().as_property(),
            TextBox::text_alignment_property().as_property(),
        );
        template_bind(
            &placeholder,
            TextBlock::text_wrapping_property().as_property(),
            TextBox::text_wrapping_property().as_property(),
        );

        let presenter = TextPresenter::new();
        presenter.set_name(Some("PART_TextPresenter".to_string()));
        let one_way = [
            (
                TextPresenter::caret_blink_interval_property().as_property(),
                TextBox::caret_blink_interval_property().as_property(),
            ),
            (TextPresenter::caret_brush_property().as_property(), TextBox::caret_brush_property().as_property()),
            (TextPresenter::caret_index_property().as_property(), TextBox::caret_index_property().as_property()),
            (TextPresenter::line_height_property().as_property(), TextBox::line_height_property().as_property()),
            (TextPresenter::letter_spacing_property().as_property(), TextBlock::letter_spacing_property().as_property()),
            (TextPresenter::password_char_property().as_property(), TextBox::password_char_property().as_property()),
            (
                TextPresenter::reveal_password_property().as_property(),
                TextBox::reveal_password_property().as_property(),
            ),
            (
                TextPresenter::selection_brush_property().as_property(),
                TextBox::selection_brush_property().as_property(),
            ),
            (TextPresenter::selection_end_property().as_property(), TextBox::selection_end_property().as_property()),
            (
                TextPresenter::selection_foreground_brush_property().as_property(),
                TextBox::selection_foreground_brush_property().as_property(),
            ),
            (
                TextPresenter::selection_start_property().as_property(),
                TextBox::selection_start_property().as_property(),
            ),
            (
                TextPresenter::text_alignment_property().as_property(),
                TextBox::text_alignment_property().as_property(),
            ),
            (TextPresenter::text_wrapping_property().as_property(), TextBox::text_wrapping_property().as_property()),
        ];
        for (target, source) in one_way {
            template_bind(&presenter, target, source);
        }
        presenter.bind_binding(
            TextPresenter::text_property().as_property(),
            &TemplateBinding::new(TextBox::text_property().as_property()).with_mode(BindingMode::TwoWay),
        );

        let panel = Panel::new();
        panel.children().add(placeholder);
        panel.children().add(presenter.register_in_name_scope(&**scope));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        Grid::set_column(&scroll_viewer, 1);
        Grid::set_column_span(&scroll_viewer, 1);
        let attached = [
            ScrollViewer::allow_auto_hide_property().as_property(),
            ScrollViewer::bring_into_view_on_focus_change_property().as_property(),
            ScrollViewer::horizontal_scroll_bar_visibility_property().as_property(),
            ScrollViewer::is_scroll_chaining_enabled_property().as_property(),
            ScrollViewer::vertical_scroll_bar_visibility_property().as_property(),
        ];
        for property in attached {
            template_bind(&scroll_viewer, property, property);
        }
        scroll_viewer.set_content(Some(Control::boxed(panel)));

        let inner_right = ContentPresenter::new();
        Grid::set_column(&inner_right, 2);
        Grid::set_column_span(&inner_right, 1);
        template_bind(
            &inner_right,
            ContentPresenter::content_property().as_property(),
            TextBox::inner_right_content_property().as_property(),
        );

        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::from_items([
            ColumnDefinition::with_width(GridLength::AUTO),
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
            ColumnDefinition::with_width(GridLength::AUTO),
        ]));
        grid.children().add(inner_left);
        grid.children().add(scroll_viewer.register_in_name_scope(&**scope));
        grid.children().add(inner_right);

        let errors = DataValidationErrors::new();
        errors.set_content(Some(Control::boxed(grid)));

        let dock_panel = DockPanel::new();
        template_bind(
            &dock_panel,
            Layoutable::margin_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );
        template_bind(
            &dock_panel,
            Layoutable::horizontal_alignment_property().as_property(),
            TextBox::horizontal_content_alignment_property().as_property(),
        );
        template_bind(
            &dock_panel,
            Layoutable::vertical_alignment_property().as_property(),
            TextBox::vertical_content_alignment_property().as_property(),
        );
        dock_panel.children().add(floating_placeholder);
        dock_panel.children().add(errors);

        let border = Border::new();
        border.set_name(Some("border".to_string()));
        bind_border(&border);
        border.set_child(dock_panel);
        border.upcast()
    })
}

/// Binds the background and the border of a border of a template to those
/// of the templated parent.
fn bind_border(border: &Border) {
    let bindings = [
        (Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property()),
        (
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        ),
        (Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property()),
    ];
    for (target, source) in bindings {
        template_bind(border, target, source);
    }
}

/// The control template of the list box of the reference simple theme.
pub fn list_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBox>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        template_bind(
            &presenter,
            Layoutable::margin_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );
        template_bind(
            &presenter,
            ItemsPresenter::items_panel_property().as_property(),
            ItemsControl::items_panel_property().as_property(),
        );

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        let attached = [
            ScrollViewer::allow_auto_hide_property().as_property(),
            ScrollViewer::bring_into_view_on_focus_change_property().as_property(),
            ScrollViewer::horizontal_scroll_bar_visibility_property().as_property(),
            ScrollViewer::is_scroll_chaining_enabled_property().as_property(),
            ScrollViewer::is_deferred_scrolling_enabled_property().as_property(),
            ScrollViewer::vertical_scroll_bar_visibility_property().as_property(),
            ScrollViewer::vertical_snap_points_type_property().as_property(),
            ScrollViewer::horizontal_snap_points_type_property().as_property(),
        ];
        for property in attached {
            template_bind(&scroll_viewer, property, property);
        }
        template_bind(
            &scroll_viewer,
            TemplatedControl::background_property().as_property(),
            TemplatedControl::background_property().as_property(),
        );
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));

        let border = Border::new();
        border.set_name(Some("border".to_string()));
        bind_border(&border);
        border.set_child(scroll_viewer.register_in_name_scope(&**scope));
        border.upcast()
    })
}

/// A content presenter named `PART_ContentPresenter` showing the content of
/// the templated content control.
fn content_presenter(scope: &ferroui_base::controls::NameScopeRef) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    let bindings = [
        (ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property()),
        (
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ),
    ];
    for (target, source) in bindings {
        template_bind(&presenter, target, source);
    }
    presenter.register_in_name_scope(&**scope)
}

/// The control template of the list box item of the reference simple theme.
pub fn list_box_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBoxItem>(|_, scope| {
        let presenter = content_presenter(scope);
        let bindings = [
            (
                ContentPresenter::background_property().as_property(),
                TemplatedControl::background_property().as_property(),
            ),
            (
                ContentPresenter::border_brush_property().as_property(),
                TemplatedControl::border_brush_property().as_property(),
            ),
            (
                ContentPresenter::border_thickness_property().as_property(),
                TemplatedControl::border_thickness_property().as_property(),
            ),
            (
                ContentPresenter::corner_radius_property().as_property(),
                TemplatedControl::corner_radius_property().as_property(),
            ),
            (ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property()),
            (
                ContentPresenter::horizontal_content_alignment_property().as_property(),
                ContentControl::horizontal_content_alignment_property().as_property(),
            ),
            (
                ContentPresenter::vertical_content_alignment_property().as_property(),
                ContentControl::vertical_content_alignment_property().as_property(),
            ),
        ];
        for (target, source) in bindings {
            template_bind(&presenter, target, source);
        }
        presenter.upcast()
    })
}

/// The control template of the scroll viewer of the reference simple theme,
/// without its scroll bars.
pub fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        template_bind(
            &presenter,
            ContentPresenter::padding_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );

        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::from_items([
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
            ColumnDefinition::with_width(GridLength::AUTO),
        ]));
        grid.set_row_definitions(RowDefinitions::from_items([
            RowDefinition::with_value(1.0, GridUnitType::Star),
            RowDefinition::with_height(GridLength::AUTO),
        ]));
        grid.children().add(presenter.register_in_name_scope(&**scope));
        grid.upcast()
    })
}

/// The control template of the data validation errors control of the
/// reference simple theme, without the display of the errors.
pub fn data_validation_errors_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<DataValidationErrors>(|_, scope| {
        let dock_panel = DockPanel::new();
        dock_panel.set_last_child_fill(true);
        dock_panel.children().add(content_presenter(scope));
        dock_panel.upcast()
    })
}

/// Adds the control themes of the parts of an auto-complete box to
/// `styles`.
pub fn add_autocomplete_themes(styles: &Ref<Styles>) {
    add_template_theme::<TextBox>(styles, text_box_template());
    add_template_theme::<ListBox>(styles, list_box_template());
    add_template_theme::<ListBoxItem>(styles, list_box_item_template());
    add_template_theme::<ScrollViewer>(styles, scroll_viewer_template());
    add_template_theme::<DataValidationErrors>(styles, data_validation_errors_template());
}
