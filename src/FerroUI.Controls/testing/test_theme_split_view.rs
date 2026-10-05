//! The control theme of the split view of the test theme: the theme of the
//! reference simple theme built in code, with the same templates, part names,
//! bindings, nested styles and transitions.
//!
//! Left out: the pane background setter and the fill of the light dismiss
//! layer of the `:lightDismiss` state, whose values are theme brushes.

use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::shapes::{Rectangle, Shape};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{
    ColumnDefinition, ContentControl, Control, Grid, GridLength, Panel, RowDefinition, SplitView,
    SplitViewPanePlacement,
};
use ferroui_base::animation::easings::{Easing, SplineEasing};
use ferroui_base::animation::{Animatable, DoubleTransition, ITransition, TimeSpan, Transitions};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::{BindingBase, ReflectionBinding, RelativeSource, RelativeSourceMode, TemplateBinding};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::styling::{ControlTheme, Selector, Selectors, Setter, Style, Styles};
use ferroui_base::{FerroObject, FerroProperty, Ref, Visual};
use std::rc::Rc;

/// The theme lengths of the reference simple theme.
const SPLIT_VIEW_OPEN_PANE_THEME_LENGTH: f64 = 320.0;
const SPLIT_VIEW_COMPACT_PANE_THEME_LENGTH: f64 = 48.0;

/// A binding to `path` of the templated parent.
fn templated_parent_binding(path: &str) -> Rc<ReflectionBinding> {
    let binding = ReflectionBinding::new(path);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    binding
}

/// The control template of the split view for one pane placement.
fn split_view_template(placement: SplitViewPanePlacement) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, ns| {
        let horizontal = matches!(placement, SplitViewPanePlacement::Left | SplitViewPanePlacement::Right);
        let pane_first = matches!(placement, SplitViewPanePlacement::Left | SplitViewPanePlacement::Top);

        let container = Grid::new();
        container.set_name(Some("Container".to_string()));
        container.bind_binding(
            Panel::background_property().as_property(),
            &TemplateBinding::new(TemplatedControl::background_property().as_property()),
        );

        // The definitions are not styled elements: their bindings find the
        // templated parent through the grid.
        let anchor = container.clone().upcast::<FerroObject>().downgrade();
        if horizontal {
            let pane_column = ColumnDefinition::new();
            let binding = templated_parent_binding("TemplateSettings.PaneColumnGridLength");
            binding.set_default_anchor(Some(anchor));
            pane_column.bind_binding(ColumnDefinition::width_property().as_property(), &binding);
            let content_column = ColumnDefinition::with_width(GridLength::STAR);
            if pane_first {
                container.column_definitions().add(pane_column);
                container.column_definitions().add(content_column);
            } else {
                container.column_definitions().add(content_column);
                container.column_definitions().add(pane_column);
            }
        } else {
            let pane_row = RowDefinition::new();
            let binding = templated_parent_binding("TemplateSettings.PaneRowGridLength");
            binding.set_default_anchor(Some(anchor));
            pane_row.bind_binding(RowDefinition::height_property().as_property(), &binding);
            let content_row = RowDefinition::with_height(GridLength::STAR);
            if pane_first {
                container.row_definitions().add(pane_row);
                container.row_definitions().add(content_row);
            } else {
                container.row_definitions().add(content_row);
                container.row_definitions().add(pane_row);
            }
        }

        let pane_root = Panel::new();
        pane_root.set_name(Some("PART_PaneRoot".to_string()));
        pane_root.bind_binding(
            Panel::background_property().as_property(),
            &TemplateBinding::new(SplitView::pane_background_property().as_property()),
        );
        pane_root.set_clip_to_bounds(true);
        match placement {
            SplitViewPanePlacement::Left => pane_root.set_horizontal_alignment(HorizontalAlignment::Left),
            SplitViewPanePlacement::Right => pane_root.set_horizontal_alignment(HorizontalAlignment::Right),
            SplitViewPanePlacement::Top => pane_root.set_vertical_alignment(VerticalAlignment::Top),
            SplitViewPanePlacement::Bottom => pane_root.set_vertical_alignment(VerticalAlignment::Bottom),
        }
        pane_root.set_z_index(100);

        let pane_presenter = ContentPresenter::new();
        pane_presenter.set_name(Some("PART_PanePresenter".to_string()));
        pane_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(SplitView::pane_property().as_property()),
        );
        pane_presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(SplitView::pane_template_property().as_property()),
        );
        pane_root.children().add(pane_presenter.register_in_name_scope(&**ns));

        let pane_border = Rectangle::new();
        pane_border.set_name(Some("HCPaneBorder".to_string()));
        pane_border.set_fill(Some(Brushes::transparent()));
        match placement {
            SplitViewPanePlacement::Left => {
                pane_border.set_width(1.0);
                pane_border.set_horizontal_alignment(HorizontalAlignment::Right);
            }
            SplitViewPanePlacement::Right => {
                pane_border.set_width(1.0);
                pane_border.set_horizontal_alignment(HorizontalAlignment::Left);
            }
            SplitViewPanePlacement::Top => {
                pane_border.set_height(1.0);
                pane_border.set_vertical_alignment(VerticalAlignment::Bottom);
            }
            SplitViewPanePlacement::Bottom => {
                pane_border.set_height(1.0);
                pane_border.set_vertical_alignment(VerticalAlignment::Top);
            }
        }
        pane_root.children().add(pane_border.register_in_name_scope(&**ns));
        container.children().add(pane_root.register_in_name_scope(&**ns));

        let content_root = Panel::new();
        content_root.set_name(Some("ContentRoot".to_string()));
        let content_index = if pane_first { 1 } else { 0 };
        if horizontal {
            Grid::set_column(&content_root, content_index);
        } else {
            Grid::set_row(&content_root, content_index);
        }

        let content_presenter = ContentPresenter::new();
        content_presenter.set_name(Some("PART_ContentPresenter".to_string()));
        content_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        content_presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(ContentControl::content_template_property().as_property()),
        );
        content_root.children().add(content_presenter.register_in_name_scope(&**ns));

        let light_dismiss_layer = Rectangle::new();
        light_dismiss_layer.set_name(Some("LightDismissLayer".to_string()));
        light_dismiss_layer.set_is_visible(false);
        content_root.children().add(light_dismiss_layer.register_in_name_scope(&**ns));
        container.children().add(content_root.register_in_name_scope(&**ns));

        container.register_in_name_scope(&**ns).upcast::<Control>()
    })
}

/// `^<classes> /template/ Panel#<name>`.
fn panel_selector(classes: &[&str], name: &str) -> Selector {
    let mut selector = Selectors::nesting(None);
    for class in classes {
        selector = selector.class(class);
    }
    selector.template().of_type::<Panel>().name(name)
}

/// `^<classes> /template/ Rectangle#LightDismissLayer`.
fn light_dismiss_layer_selector(classes: &[&str]) -> Selector {
    let mut selector = Selectors::nesting(None);
    for class in classes {
        selector = selector.class(class);
    }
    selector.template().of_type::<Rectangle>().name("LightDismissLayer")
}

/// A setter of a grid position property.
fn grid_setter(property: &'static FerroProperty, value: i32) -> Rc<Setter> {
    Setter::new_untyped(property, Rc::new(value))
}

/// A setter that binds `property` to `path` of the templated parent.
fn templated_parent_setter(property: &'static FerroProperty, path: &str) -> Rc<Setter> {
    let binding: Rc<dyn BindingBase> = templated_parent_binding(path);
    Setter::new_binding_base(property, binding)
}

/// The setter of the transitions of the pane root: one double transition of
/// `property` with the pane animation easing of the reference theme.
fn transitions_setter(property: &'static FerroProperty, duration: TimeSpan) -> Rc<Setter> {
    let transition = DoubleTransition::new();
    transition.set_property(Some(property));
    transition.set_duration(duration);
    transition.set_easing(Easing::new(SplineEasing::with_points(0.1, 0.9, 0.2, 1.0)));
    let transition: Rc<dyn ITransition> = transition.into();
    Setter::new(Animatable::transitions_property(), Some(Transitions::from_items([transition])))
}

/// Adds the styles of one pane placement: the template and the grid
/// positions and the closed size of the pane root per display mode.
fn add_placement_styles(theme: &ControlTheme, placement: SplitViewPanePlacement) {
    let horizontal = matches!(placement, SplitViewPanePlacement::Left | SplitViewPanePlacement::Right);
    let pane_first = matches!(placement, SplitViewPanePlacement::Left | SplitViewPanePlacement::Top);
    let placement_class = match placement {
        SplitViewPanePlacement::Left => ":left",
        SplitViewPanePlacement::Right => ":right",
        SplitViewPanePlacement::Top => ":top",
        SplitViewPanePlacement::Bottom => ":bottom",
    };

    let (index_property, span_property, size_property, closed_size_path) = if horizontal {
        (
            Grid::column_property().as_property(),
            Grid::column_span_property().as_property(),
            Layoutable::width_property().as_property(),
            "TemplateSettings.ClosedPaneWidth",
        )
    } else {
        (
            Grid::row_property().as_property(),
            Grid::row_span_property().as_property(),
            Layoutable::height_property().as_property(),
            "TemplateSettings.ClosedPaneHeight",
        )
    };
    let pane_index = if pane_first { 0 } else { 1 };
    let content_index = if pane_first { 1 } else { 0 };

    theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(placement_class),
        [Setter::new(TemplatedControl::template_property(), Some(split_view_template(placement)))],
    ));

    // Overlay. The reference theme gives the pane root a span of one for the
    // top placement and of two for the others.
    let overlay_pane_span = if placement == SplitViewPanePlacement::Top { 1 } else { 2 };
    theme.add_style(Style::with_setters(
        panel_selector(&[":overlay", placement_class], "PART_PaneRoot"),
        [
            templated_parent_setter(size_property, closed_size_path),
            grid_setter(span_property, overlay_pane_span),
            grid_setter(index_property, pane_index),
        ],
    ));
    theme.add_style(Style::with_setters(
        panel_selector(&[":overlay", placement_class], "ContentRoot"),
        [grid_setter(index_property, content_index), grid_setter(span_property, 2)],
    ));

    // CompactInline
    theme.add_style(Style::with_setters(
        panel_selector(&[":compactinline", placement_class], "PART_PaneRoot"),
        [
            grid_setter(span_property, 1),
            grid_setter(index_property, pane_index),
            templated_parent_setter(size_property, closed_size_path),
        ],
    ));
    theme.add_style(Style::with_setters(
        panel_selector(&[":compactinline", placement_class], "ContentRoot"),
        [grid_setter(index_property, content_index), grid_setter(span_property, 1)],
    ));

    // CompactOverlay. As for the overlay mode, the span of the pane root is
    // one for the top placement and two for the others.
    theme.add_style(Style::with_setters(
        panel_selector(&[":compactoverlay", placement_class], "PART_PaneRoot"),
        [
            grid_setter(span_property, overlay_pane_span),
            grid_setter(index_property, pane_index),
            templated_parent_setter(size_property, closed_size_path),
        ],
    ));
    theme.add_style(Style::with_setters(
        panel_selector(&[":compactoverlay", placement_class], "ContentRoot"),
        [grid_setter(index_property, content_index), grid_setter(span_property, 1)],
    ));

    // Inline
    theme.add_style(Style::with_setters(
        panel_selector(&[":inline", placement_class], "PART_PaneRoot"),
        [
            grid_setter(span_property, 1),
            grid_setter(index_property, pane_index),
            templated_parent_setter(size_property, closed_size_path),
        ],
    ));
    theme.add_style(Style::with_setters(
        panel_selector(&[":inline", placement_class], "ContentRoot"),
        [grid_setter(index_property, content_index), grid_setter(span_property, 1)],
    ));
}

/// The control theme of the split view.
pub fn split_view_theme() -> Ref<ControlTheme> {
    let theme = ControlTheme::with_setters(
        SplitView::TYPE,
        [
            Setter::new(SplitView::open_pane_length_property(), SPLIT_VIEW_OPEN_PANE_THEME_LENGTH),
            Setter::new(SplitView::compact_pane_length_property(), SPLIT_VIEW_COMPACT_PANE_THEME_LENGTH),
        ],
    );

    add_placement_styles(&theme, SplitViewPanePlacement::Left);
    add_placement_styles(&theme, SplitViewPanePlacement::Right);
    add_placement_styles(&theme, SplitViewPanePlacement::Top);
    add_placement_styles(&theme, SplitViewPanePlacement::Bottom);

    // Open/close pane animation: width for the horizontal placements, height
    // for the vertical ones.
    let open_duration = TimeSpan::from_seconds(0.2);
    let close_duration = TimeSpan::from_seconds(0.1);
    let width = Layoutable::width_property().as_property();
    let height = Layoutable::height_property().as_property();
    for (placement_class, property) in [(":left", width), (":right", width)] {
        theme.add_style(Style::with_setters(
            panel_selector(&[placement_class, ":open"], "PART_PaneRoot"),
            [transitions_setter(property, open_duration), templated_parent_setter(property, "OpenPaneLength")],
        ));
    }
    for placement_class in [":left", ":right"] {
        theme.add_style(Style::with_setters(
            panel_selector(&[placement_class, ":closed"], "PART_PaneRoot"),
            [
                transitions_setter(width, close_duration),
                templated_parent_setter(width, "TemplateSettings.ClosedPaneWidth"),
            ],
        ));
    }
    for (placement_class, property) in [(":top", height), (":bottom", height)] {
        theme.add_style(Style::with_setters(
            panel_selector(&[placement_class, ":open"], "PART_PaneRoot"),
            [transitions_setter(property, open_duration), templated_parent_setter(property, "OpenPaneLength")],
        ));
    }
    for placement_class in [":top", ":bottom"] {
        theme.add_style(Style::with_setters(
            panel_selector(&[placement_class, ":closed"], "PART_PaneRoot"),
            [
                transitions_setter(height, close_duration),
                templated_parent_setter(height, "TemplateSettings.ClosedPaneHeight"),
            ],
        ));
    }

    let transparent: Option<Rc<dyn IBrush>> = Some(Brushes::transparent());
    theme.add_style(Style::with_setters(
        light_dismiss_layer_selector(&[]),
        [Setter::new(Visual::is_visible_property(), false), Setter::new(Shape::fill_property(), transparent)],
    ));
    theme.add_style(Style::with_setters(
        light_dismiss_layer_selector(&[":overlay", ":open"]),
        [Setter::new(Visual::is_visible_property(), true)],
    ));
    theme.add_style(Style::with_setters(
        light_dismiss_layer_selector(&[":compactoverlay", ":open"]),
        [Setter::new(Visual::is_visible_property(), true)],
    ));

    theme
}

/// Adds the control theme of the split view.
pub fn add_split_view_themes(styles: &Ref<Styles>) {
    styles.resources().add_value(ResourceKey::Type(SplitView::TYPE), split_view_theme());
}

// ---------------------------------------------------------------------------
// The themes of the controls that the list box tests with a split view take
// from the theme: list box, list box item, scroll viewer, scroll bar and
// repeat button, as in the reference simple theme. They are not part of the
// test theme: a test adds them with `create_split_view_list_box_theme`.
//
// Left out: the setters whose values are theme brushes (backgrounds, border
// brushes, foregrounds, path fills) and the styles that only set them, and
// the context flyouts of the scroll bar. The theme lengths are the values of
// the reference theme.
// ---------------------------------------------------------------------------

const THEME_BORDER_THICKNESS: f64 = 1.0;
const THEME_DISABLED_OPACITY: f64 = 0.5;
const SCROLL_BAR_THICKNESS: f64 = 18.0;
const SCROLL_BAR_THUMB_THICKNESS: f64 = 8.0;

fn bind_template(target: &FerroObject, target_property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(target_property, &TemplateBinding::new(source));
}

/// The control template of the list box.
fn list_box_template() -> Rc<dyn IControlTemplate> {
    use crate::presenters::ItemsPresenter;
    use crate::{Border, ItemsControl, ScrollViewer};

    FuncControlTemplate::new(|_, ns| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        bind_template(
            &items_presenter,
            Layoutable::margin_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );
        bind_template(
            &items_presenter,
            ItemsPresenter::items_panel_property().as_property(),
            ItemsControl::items_panel_property().as_property(),
        );

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        for property in [
            ScrollViewer::allow_auto_hide_property().as_property(),
            ScrollViewer::bring_into_view_on_focus_change_property().as_property(),
            TemplatedControl::background_property().as_property(),
            ScrollViewer::horizontal_scroll_bar_visibility_property().as_property(),
            ScrollViewer::is_scroll_chaining_enabled_property().as_property(),
            ScrollViewer::is_deferred_scrolling_enabled_property().as_property(),
            ScrollViewer::vertical_scroll_bar_visibility_property().as_property(),
            ScrollViewer::vertical_snap_points_type_property().as_property(),
            ScrollViewer::horizontal_snap_points_type_property().as_property(),
        ] {
            bind_template(&scroll_viewer, property, property);
        }
        scroll_viewer.set_content(Some(Control::boxed(items_presenter.register_in_name_scope(&**ns))));

        let border = Border::new();
        border.set_name(Some("border".to_string()));
        bind_template(
            &border,
            Border::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        bind_template(
            &border,
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind_template(
            &border,
            Border::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        border.set_child(scroll_viewer.register_in_name_scope(&**ns));
        border.register_in_name_scope(&**ns).upcast()
    })
}

/// The content presenter of the templates of the list box item and the
/// repeat button.
fn content_presenter_part(ns: &ferroui_base::controls::NameScopeRef) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
        bind_template(&presenter, target, source);
    };
    bind(ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property());
    bind(
        ContentPresenter::horizontal_content_alignment_property().as_property(),
        ContentControl::horizontal_content_alignment_property().as_property(),
    );
    bind(
        ContentPresenter::vertical_content_alignment_property().as_property(),
        ContentControl::vertical_content_alignment_property().as_property(),
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
    presenter.register_in_name_scope(&**ns)
}

/// The control template of the scroll viewer.
fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    use crate::presenters::ScrollContentPresenter;
    use crate::primitives::ScrollBar;
    use crate::{ColumnDefinitions, RowDefinitions, ScrollViewer};
    use ferroui_base::data::{BindingMode, IndexerBinding};
    use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
    use ferroui_base::layout::Orientation;

    FuncControlTemplate::new(|_, ns| {
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("valid column definitions"));
        grid.set_row_definitions(RowDefinitions::parse("*,Auto").expect("valid row definitions"));

        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        bind_template(
            &presenter,
            ContentPresenter::padding_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        );
        for (target, source) in [
            (
                ScrollContentPresenter::horizontal_snap_points_type_property().as_property(),
                ScrollViewer::horizontal_snap_points_type_property().as_property(),
            ),
            (
                ScrollContentPresenter::vertical_snap_points_type_property().as_property(),
                ScrollViewer::vertical_snap_points_type_property().as_property(),
            ),
            (
                ScrollContentPresenter::horizontal_snap_points_alignment_property().as_property(),
                ScrollViewer::horizontal_snap_points_alignment_property().as_property(),
            ),
            (
                ScrollContentPresenter::vertical_snap_points_alignment_property().as_property(),
                ScrollViewer::vertical_snap_points_alignment_property().as_property(),
            ),
            (
                ContentPresenter::background_property().as_property(),
                TemplatedControl::background_property().as_property(),
            ),
            (
                ScrollViewer::is_scroll_inertia_enabled_property().as_property(),
                ScrollViewer::is_scroll_inertia_enabled_property().as_property(),
            ),
        ] {
            bind_template(&presenter, target, source);
        }
        let presenter = presenter.register_in_name_scope(&**ns);

        // The gesture recognizer is bound to the presenter by element name.
        let recognizer = ScrollGestureRecognizer::new();
        let name_scope = Rc::downgrade(&ns.0);
        for (property, path) in [
            (ScrollGestureRecognizer::can_horizontally_scroll_property().as_property(), "CanHorizontallyScroll"),
            (ScrollGestureRecognizer::can_vertically_scroll_property().as_property(), "CanVerticallyScroll"),
            (ScrollGestureRecognizer::offset_property().as_property(), "Offset"),
            (ScrollGestureRecognizer::viewport_property().as_property(), "Viewport"),
            (ScrollGestureRecognizer::extent_property().as_property(), "Extent"),
        ] {
            let binding = ReflectionBinding::new(path);
            binding.set_element_name(Some("PART_ContentPresenter".to_string()));
            binding.set_name_scope(Some(name_scope.clone()));
            recognizer.bind_binding(property, &binding);
        }
        // The path of this binding names an attached property; it is bound
        // through the property itself.
        recognizer.bind_binding(
            ScrollGestureRecognizer::is_scroll_inertia_enabled_property().as_property(),
            &IndexerBinding::new(
                presenter.clone().upcast(),
                ScrollViewer::is_scroll_inertia_enabled_property().as_property(),
                BindingMode::OneWay,
            ),
        );
        presenter.gesture_recognizers().add(recognizer);
        grid.children().add(presenter);

        let horizontal_scroll_bar = ScrollBar::new();
        horizontal_scroll_bar.set_name(Some("PART_HorizontalScrollBar".to_string()));
        Grid::set_row(&horizontal_scroll_bar, 1);
        horizontal_scroll_bar.set_orientation(Orientation::Horizontal);
        grid.children().add(horizontal_scroll_bar.register_in_name_scope(&**ns));

        let vertical_scroll_bar = ScrollBar::new();
        vertical_scroll_bar.set_name(Some("PART_VerticalScrollBar".to_string()));
        Grid::set_column(&vertical_scroll_bar, 1);
        vertical_scroll_bar.set_orientation(Orientation::Vertical);
        grid.children().add(vertical_scroll_bar.register_in_name_scope(&**ns));

        let corner = Panel::new();
        Grid::set_row(&corner, 1);
        Grid::set_column(&corner, 1);
        grid.children().add(corner);

        grid.upcast()
    })
}

/// The control template of the scroll bar for one orientation.
fn scroll_bar_template(horizontal: bool) -> Rc<dyn IControlTemplate> {
    use crate::automation::AutomationProperties;
    use crate::primitives::{RangeBase, ScrollBar, Thumb, Track};
    use crate::shapes::Path;
    use crate::{Border, ColumnDefinitions, RepeatButton, RowDefinitions, ScrollViewer};
    use ferroui_base::data::BindingMode;
    use ferroui_base::media::PathGeometry;

    FuncControlTemplate::new(move |_, ns| {
        let grid = Grid::new();
        if horizontal {
            grid.set_column_definitions(ColumnDefinitions::parse("Auto,*,Auto").expect("valid column definitions"));
        } else {
            grid.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").expect("valid row definitions"));
        }

        let line_button = |name: &str, path_data: &str, automation_name: &str| {
            let button = RepeatButton::new();
            button.set_name(Some(name.to_string()));
            if horizontal {
                button.set_min_width(SCROLL_BAR_THICKNESS);
                button.set_vertical_alignment(VerticalAlignment::Center);
            } else {
                button.set_min_height(SCROLL_BAR_THICKNESS);
                button.set_horizontal_alignment(HorizontalAlignment::Center);
            }
            button.classes().add("repeat");
            button.set_focusable(false);
            AutomationProperties::set_name(&button, Some(automation_name));
            let path = Path::new();
            path.set_data(PathGeometry::parse(path_data).expect("valid path data").upcast::<ferroui_base::media::Geometry>());
            button.set_content(Some(Control::boxed(path)));
            button
        };

        let line_up_button = line_button(
            "PART_LineUpButton",
            if horizontal { "M 4 0 L 4 8 L 0 4 Z" } else { "M 0 4 L 8 4 L 4 0 Z" },
            if horizontal { "Column left" } else { "Line up" },
        );
        Grid::set_row(&line_up_button, 0);
        if horizontal {
            Grid::set_column(&line_up_button, 0);
        }
        grid.children().add(line_up_button.register_in_name_scope(&**ns));

        let track = Track::new();
        Grid::set_row(&track, 1);
        Grid::set_column(&track, 1);
        if !horizontal {
            track.set_is_direction_reversed(true);
        }
        bind_template(&track, Track::maximum_property().as_property(), RangeBase::maximum_property().as_property());
        bind_template(&track, Track::minimum_property().as_property(), RangeBase::minimum_property().as_property());
        bind_template(
            &track,
            Track::orientation_property().as_property(),
            ScrollBar::orientation_property().as_property(),
        );
        bind_template(
            &track,
            Track::viewport_size_property().as_property(),
            ScrollBar::viewport_size_property().as_property(),
        );
        bind_template(
            &track,
            Track::defer_thumb_drag_property().as_property(),
            ScrollViewer::is_deferred_scrolling_enabled_property().as_property(),
        );
        track.bind_binding(
            Track::value_property().as_property(),
            &TemplateBinding::new(RangeBase::value_property().as_property()).with_mode(BindingMode::TwoWay),
        );

        let page_button = |name: &str, automation_name: &str| {
            let button = RepeatButton::new();
            button.set_name(Some(name.to_string()));
            button.classes().add("repeattrack");
            button.set_focusable(false);
            AutomationProperties::set_name(&button, Some(automation_name));
            button
        };
        track.set_decrease_button(
            page_button("PART_PageUpButton", if horizontal { "Page left" } else { "Page up" })
                .register_in_name_scope(&**ns)
                .upcast::<crate::Button>(),
        );
        track.set_increase_button(
            page_button("PART_PageDownButton", if horizontal { "Page right" } else { "Page down" })
                .register_in_name_scope(&**ns)
                .upcast::<crate::Button>(),
        );
        let thumb = Thumb::new();
        thumb.set_name(Some("thumb".to_string()));
        AutomationProperties::set_name(&thumb, Some("Position"));
        track.set_thumb(thumb.register_in_name_scope(&**ns));
        grid.children().add(track);

        let line_down_button = line_button(
            "PART_LineDownButton",
            if horizontal { "M 0 0 L 4 4 L 0 8 Z" } else { "M 0 0 L 4 4 L 8 0 Z" },
            if horizontal { "Column right" } else { "Line down" },
        );
        Grid::set_row(&line_down_button, 2);
        Grid::set_column(&line_down_button, 2);
        grid.children().add(line_down_button.register_in_name_scope(&**ns));

        let border = Border::new();
        border.set_use_layout_rounding(false);
        border.set_child(grid);
        border.upcast()
    })
}

/// A template of a border whose background is the background of the
/// templated parent.
fn background_border_template() -> Rc<dyn IControlTemplate> {
    use crate::Border;

    FuncControlTemplate::new(|_, _| {
        let border = Border::new();
        bind_template(
            &border,
            Border::background_property().as_property(),
            TemplatedControl::background_property().as_property(),
        );
        border.upcast()
    })
}

/// Adds the control themes of the list box, the list box item, the scroll
/// viewer, the scroll bar and the repeat button.
pub fn add_split_view_list_box_themes(styles: &Ref<Styles>) {
    use crate::primitives::{ScrollBar, ScrollBarVisibility, Thumb};
    use crate::{ListBox, ListBoxItem, RepeatButton, ScrollViewer};
    use ferroui_base::input::{Cursor, InputElement, StandardCursorType};
    use ferroui_base::Thickness;

    let transparent = || -> Option<Rc<dyn IBrush>> { Some(Brushes::transparent()) };

    // List box.
    let list_box_theme = ControlTheme::with_setters(
        ListBox::TYPE,
        [
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(THEME_BORDER_THICKNESS)),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(4.0)),
            Setter::new(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Auto),
            Setter::new(ScrollViewer::vertical_scroll_bar_visibility_property(), ScrollBarVisibility::Auto),
            Setter::new(ScrollViewer::is_scroll_chaining_enabled_property(), true),
            Setter::new(TemplatedControl::template_property(), Some(list_box_template())),
        ],
    );
    list_box_theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(":disabled").template().of_type::<crate::Border>().name("border"),
        [Setter::new(Visual::opacity_property(), THEME_DISABLED_OPACITY)],
    ));
    styles.resources().add_value(ResourceKey::Type(ListBox::TYPE), list_box_theme);

    // List box item.
    let list_box_item_template: Rc<dyn IControlTemplate> =
        FuncControlTemplate::new(|_, ns| content_presenter_part(ns).upcast());
    let list_box_item_theme = ControlTheme::with_setters(
        ListBoxItem::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::border_brush_property(), transparent()),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0)),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(2.0, 1.0, 2.0, 1.0)),
            Setter::new(TemplatedControl::template_property(), Some(list_box_item_template)),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(ListBoxItem::TYPE), list_box_item_theme);

    // Scroll viewer.
    let scroll_viewer_theme = ControlTheme::with_setters(
        ScrollViewer::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::template_property(), Some(scroll_viewer_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(ScrollViewer::TYPE), scroll_viewer_theme);

    // Scroll bar.
    let scroll_bar_theme = ControlTheme::with_setters(
        ScrollBar::TYPE,
        [Setter::new(InputElement::cursor_property(), Some(Cursor::new(StandardCursorType::Arrow)))],
    );
    scroll_bar_theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(":horizontal"),
        [
            Setter::new(Layoutable::height_property(), SCROLL_BAR_THICKNESS),
            Setter::new(TemplatedControl::template_property(), Some(scroll_bar_template(true))),
        ],
    ));
    scroll_bar_theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(":vertical"),
        [
            Setter::new(Layoutable::width_property(), SCROLL_BAR_THICKNESS),
            Setter::new(TemplatedControl::template_property(), Some(scroll_bar_template(false))),
        ],
    ));
    let thumb = |classes: &[&str]| {
        let mut selector = Selectors::nesting(None);
        for class in classes {
            selector = selector.class(class);
        }
        selector.template().of_type::<Thumb>().name("thumb")
    };
    scroll_bar_theme.add_style(Style::with_setters(
        thumb(&[]),
        [Setter::new(TemplatedControl::template_property(), Some(background_border_template()))],
    ));
    scroll_bar_theme.add_style(Style::with_setters(
        thumb(&[":horizontal"]),
        [
            Setter::new(Layoutable::min_width_property(), SCROLL_BAR_THICKNESS),
            Setter::new(Layoutable::height_property(), SCROLL_BAR_THUMB_THICKNESS),
        ],
    ));
    scroll_bar_theme.add_style(Style::with_setters(
        thumb(&[":vertical"]),
        [
            Setter::new(Layoutable::min_height_property(), SCROLL_BAR_THICKNESS),
            Setter::new(Layoutable::width_property(), SCROLL_BAR_THUMB_THICKNESS),
        ],
    ));
    scroll_bar_theme.add_style(Style::with_setters(
        Selectors::nesting(None).template().of_type::<RepeatButton>().class("repeat"),
        [
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(2.0)),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0)),
        ],
    ));
    scroll_bar_theme.add_style(Style::with_setters(
        Selectors::nesting(None).template().of_type::<RepeatButton>().class("repeattrack"),
        [Setter::new(TemplatedControl::template_property(), Some(background_border_template()))],
    ));
    styles.resources().add_value(ResourceKey::Type(ScrollBar::TYPE), scroll_bar_theme);

    // Repeat button.
    let repeat_button_template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
        let presenter = content_presenter_part(ns);
        bind_template(
            &presenter,
            crate::documents::TextElement::foreground_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
        );
        presenter.upcast()
    });
    let repeat_button_theme = ControlTheme::with_setters(
        RepeatButton::TYPE,
        [
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(THEME_BORDER_THICKNESS)),
            Setter::new(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Center),
            Setter::new(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Center),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(4.0)),
            Setter::new(TemplatedControl::template_property(), Some(repeat_button_template)),
        ],
    );
    repeat_button_theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(":disabled"),
        [Setter::new(Visual::opacity_property(), THEME_DISABLED_OPACITY)],
    ));
    styles.resources().add_value(ResourceKey::Type(RepeatButton::TYPE), repeat_button_theme);
}

/// Creates the test theme together with the themes of
/// [`add_split_view_list_box_themes`].
pub fn create_split_view_list_box_theme() -> Rc<dyn ferroui_base::styling::IStyle> {
    let styles = Styles::new();
    styles.add(super::create_test_theme());
    add_split_view_list_box_themes(&styles);
    ferroui_base::styling::styles_as_style(&styles)
}
