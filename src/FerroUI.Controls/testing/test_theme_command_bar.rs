//! The control themes the test theme gives to the command bar and its
//! elements: the themes of the reference simple theme, built in code.
//!
//! Left out of every theme: the setters and styles whose values are theme
//! resources (the brushes of the backgrounds and borders, the border
//! thickness) and with them the pointer-over, pressed and checked styles,
//! which set nothing else.

use crate::command_bar::{
    CommandBar, CommandBarButton, CommandBarDefaultLabelPosition, CommandBarElementCollection, CommandBarSeparator,
    CommandBarToggleButton,
};
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{Popup, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, ITemplateOf};
use crate::{
    Border, Button, ContentControl, Control, Decorator, Dock, DockPanel, ItemsControl, Panel, PathIcon, PlacementMode,
    StackPanel, TextBlock,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder, TemplateBinding};
use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Geometry, IBrush, TextTrimming};
use ferroui_base::styling::{ControlTheme, Selector, Selectors, Setter, Style, Styles};
use ferroui_base::{BoxedValue, FerroProperty, ObjectType, Ref, StyledProperty, Thickness, Visual};
use std::rc::Rc;

/// The glyph of the overflow button of the reference simple theme.
const OVERFLOW_GLYPH: &str = "M6,10A2,2 0 0,0 4,12A2,2 0 0,0 6,14A2,2 0 0,0 8,12A2,2 0 0,0 6,10M12,10A2,2 0 0,0 10,12A2,2 0 0,0 12,14A2,2 0 0,0 14,12A2,2 0 0,0 12,10M18,10A2,2 0 0,0 16,12A2,2 0 0,0 18,14A2,2 0 0,0 20,12A2,2 0 0,0 18,10Z";

/// The properties of a command bar button class that its theme uses: the
/// two button classes declare the same properties, each its own.
struct ButtonProperties {
    label: &'static StyledProperty<Option<String>>,
    icon: &'static StyledProperty<Option<BoxedValue>>,
    is_compact: &'static StyledProperty<bool>,
    label_position: &'static StyledProperty<CommandBarDefaultLabelPosition>,
    is_in_overflow: &'static StyledProperty<bool>,
}

fn button_properties() -> ButtonProperties {
    ButtonProperties {
        label: CommandBarButton::label_property(),
        icon: CommandBarButton::icon_property(),
        is_compact: CommandBarButton::is_compact_property(),
        label_position: CommandBarButton::label_position_property(),
        is_in_overflow: CommandBarButton::is_in_overflow_property(),
    }
}

fn toggle_button_properties() -> ButtonProperties {
    ButtonProperties {
        label: CommandBarToggleButton::label_property(),
        icon: CommandBarToggleButton::icon_property(),
        is_compact: CommandBarToggleButton::is_compact_property(),
        label_position: CommandBarToggleButton::label_position_property(),
        is_in_overflow: CommandBarToggleButton::is_in_overflow_property(),
    }
}

/// The control template the reference simple theme gives to both command
/// bar button classes: a border around a stack panel with the icon
/// presenter and the label.
fn button_template(
    label: &'static StyledProperty<Option<String>>,
    icon: &'static StyledProperty<Option<BoxedValue>>,
) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(move |_, ns| {
        let icon_presenter = ContentPresenter::new();
        icon_presenter.set_name(Some("PART_IconPresenter".to_string()));
        icon_presenter
            .bind_binding(ContentPresenter::content_property().as_property(), &TemplateBinding::new(icon.as_property()));
        icon_presenter.bind_binding(
            ContentPresenter::foreground_property().as_property(),
            &TemplateBinding::new(TemplatedControl::foreground_property().as_property()),
        );
        icon_presenter.set_horizontal_alignment(HorizontalAlignment::Center);

        let label_text = TextBlock::new();
        label_text.set_name(Some("PART_Label".to_string()));
        label_text.bind_binding(TextBlock::text_property().as_property(), &TemplateBinding::new(label.as_property()));
        label_text.bind_binding(
            TextBlock::foreground_property().as_property(),
            &TemplateBinding::new(TemplatedControl::foreground_property().as_property()),
        );
        label_text.set_font_size(11.0);
        label_text.set_horizontal_alignment(HorizontalAlignment::Center);
        label_text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());

        let layout_root = StackPanel::new();
        layout_root.set_name(Some("PART_LayoutRoot".to_string()));
        layout_root.set_spacing(4.0);
        layout_root.set_horizontal_alignment(HorizontalAlignment::Center);
        layout_root.set_vertical_alignment(VerticalAlignment::Center);
        layout_root.children().add(icon_presenter.register_in_name_scope(&**ns));
        layout_root.children().add(label_text.register_in_name_scope(&**ns));

        let border = Border::new();
        border.set_name(Some("PART_Border".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            border.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        bind(Decorator::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        border.set_child(layout_root.register_in_name_scope(&**ns));
        border.register_in_name_scope(&**ns).upcast()
    })
}

/// The control theme of a command bar button class (the two themes of the
/// reference simple theme differ in the checked style only, which sets
/// resource brushes).
fn button_theme<T: ObjectType>(properties: ButtonProperties) -> Ref<ControlTheme> {
    let transparent = || -> Option<Rc<dyn IBrush>> { Some(Brushes::transparent()) };

    let theme = ControlTheme::with_setters(
        T::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::border_brush_property(), transparent()),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0)),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(8.0, 4.0, 8.0, 4.0)),
            Setter::new(Layoutable::width_property(), 68.0),
            Setter::new(Layoutable::min_height_property(), 40.0),
            Setter::new(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Center),
            Setter::new(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Center),
            Setter::new(
                TemplatedControl::template_property(),
                Some(button_template(properties.label, properties.icon)),
            ),
        ],
    );

    let add = |selector: Selector, setters: Vec<Rc<Setter>>| {
        theme.add_style(Style::with_setters(selector, setters));
    };
    let compact = || Selectors::nesting(None).property_equals(properties.is_compact, true);
    let right =
        || Selectors::nesting(None).property_equals(properties.label_position, CommandBarDefaultLabelPosition::Right);
    let in_overflow = || Selectors::nesting(None).property_equals(properties.is_in_overflow, true);
    let layout_root = |selector: Selector| selector.template().of_type::<StackPanel>().name("PART_LayoutRoot");
    let icon_presenter =
        |selector: Selector| selector.template().of_type::<ContentPresenter>().name("PART_IconPresenter");
    let label = |selector: Selector| selector.template().of_type::<TextBlock>().name("PART_Label");

    // Compact: hide label, reduce width
    add(compact(), vec![Setter::new(Layoutable::width_property(), 40.0)]);
    add(label(compact()), vec![Setter::new(Visual::is_visible_property(), false)]);

    // LabelPosition=Right: horizontal layout, auto-width
    add(
        right(),
        vec![Setter::new(Layoutable::width_property(), f64::NAN), Setter::new(Layoutable::min_width_property(), 40.0)],
    );
    add(layout_root(right()), vec![Setter::new(StackPanel::orientation_property(), Orientation::Horizontal)]);
    add(
        icon_presenter(right()),
        vec![Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)],
    );
    add(label(right()), vec![Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)]);

    // IsInOverflow: full-width horizontal layout
    add(
        in_overflow(),
        vec![
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch),
            Setter::new(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Left),
            Setter::new(Layoutable::width_property(), f64::NAN),
            Setter::new(Layoutable::min_height_property(), 36.0),
        ],
    );
    add(
        layout_root(in_overflow()),
        vec![
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left),
            Setter::new(StackPanel::orientation_property(), Orientation::Horizontal),
            Setter::new(StackPanel::spacing_property(), 0.0),
        ],
    );
    add(
        icon_presenter(in_overflow()),
        vec![
            Setter::new(Layoutable::width_property(), 16.0),
            Setter::new(Layoutable::height_property(), 16.0),
            Setter::new(Layoutable::margin_property(), Thickness::new(4.0, 0.0, 10.0, 0.0)),
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left),
            Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        ],
    );
    add(
        label(in_overflow()),
        vec![
            Setter::new(Visual::is_visible_property(), true),
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left),
            Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        ],
    );

    theme
}

/// The control theme the test theme gives to command bar buttons.
pub fn command_bar_button_theme() -> Ref<ControlTheme> {
    button_theme::<CommandBarButton>(button_properties())
}

/// The control theme the test theme gives to command bar toggle buttons.
pub fn command_bar_toggle_button_theme() -> Ref<ControlTheme> {
    button_theme::<CommandBarToggleButton>(toggle_button_properties())
}

/// The control theme the test theme gives to command bar separators: a
/// border with the size of the separator (its background is a resource
/// brush in the reference).
pub fn command_bar_separator_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, _| {
        let border = Border::new();
        let bind = |property: &'static FerroProperty| {
            border.bind_binding(property, &TemplateBinding::new(property));
        };
        bind(Layoutable::width_property().as_property());
        bind(Layoutable::height_property().as_property());
        bind(Layoutable::min_height_property().as_property());
        border.upcast()
    });

    let theme = ControlTheme::with_setters(
        CommandBarSeparator::TYPE,
        [
            Setter::new(Layoutable::width_property(), 1.0),
            Setter::new(Layoutable::min_height_property(), 24.0),
            Setter::new(Layoutable::margin_property(), Thickness::new(8.0, 4.0, 8.0, 4.0)),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    );

    // IsInOverflow: horizontal separator
    theme.add_style(Style::with_setters(
        Selectors::nesting(None).property_equals(CommandBarSeparator::is_in_overflow_property(), true),
        [
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch),
            Setter::new(Layoutable::width_property(), f64::NAN),
            Setter::new(Layoutable::height_property(), 1.0),
            Setter::new(Layoutable::min_height_property(), 0.0),
            Setter::new(Layoutable::margin_property(), Thickness::new(4.0, 4.0, 4.0, 4.0)),
        ],
    ));

    theme
}

/// The control theme of the items controls inside the command bar template
/// (the items control theme of the reference simple theme, which the test
/// theme does not have for every items control): a border around the items
/// presenter.
fn items_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        items_presenter.bind_binding(
            ItemsPresenter::items_panel_property().as_property(),
            &TemplateBinding::new(ItemsControl::items_panel_property().as_property()),
        );

        let border = Border::new();
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            border.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Decorator::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        border.set_child(items_presenter.register_in_name_scope(&**ns));
        border.upcast()
    });

    ControlTheme::with_setters(ItemsControl::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

/// The control theme of the overflow button inside the command bar template
/// (the button theme of the reference simple theme, which the test theme
/// does not have for every button): the content presenter.
fn overflow_button_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
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
        bind(ContentPresenter::foreground_property().as_property(), TemplatedControl::foreground_property().as_property());
        presenter.register_in_name_scope(&**ns).upcast()
    });

    ControlTheme::with_setters(
        Button::TYPE,
        [
            Setter::new(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Center),
            Setter::new(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Center),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(4.0)),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    )
}

/// The binding of the items source of an items control of the command bar
/// template to one of the read-only collections of the bar: a binding to a
/// property of the templated parent (the form in which markup compiles the
/// binding of the reference).
fn collection_binding(
    name: &'static str,
    get: fn(&CommandBar) -> CommandBarElementCollection,
) -> Rc<CompiledBinding> {
    let path = CompiledBindingPathBuilder::new()
        .templated_parent()
        .property_untyped(
            name,
            ValueType::of::<CommandBarElementCollection>(),
            Rc::new(move |owner: &BoxedValue| {
                Ok(ValueTypes::as_object(&**owner).and_then(|object| {
                    object.downcast_ref::<CommandBar>().map(|command_bar| -> BoxedValue { Rc::new(get(command_bar)) })
                }))
            }),
            None,
            None,
        )
        .build();
    CompiledBinding::new(path)
}

/// The control template the test theme gives to command bars (the template
/// of the reference simple theme). The placement target of the overflow
/// popup is set to the overflow button directly, where the reference binds
/// it to the button by name.
pub fn command_bar_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        // Overflow button (docked right)
        let overflow_glyph = PathIcon::new();
        overflow_glyph.set_width(16.0);
        overflow_glyph.set_height(16.0);
        overflow_glyph.bind_binding(
            TemplatedControl::foreground_property().as_property(),
            &TemplateBinding::new(TemplatedControl::foreground_property().as_property()),
        );
        overflow_glyph.set_data(Geometry::parse(OVERFLOW_GLYPH).ok());

        let overflow_button = Button::new();
        overflow_button.set_name(Some("PART_OverflowButton".to_string()));
        overflow_button.set_theme(overflow_button_theme());
        DockPanel::set_dock(&overflow_button, Dock::Right);
        let transparent: Option<Rc<dyn IBrush>> = Some(Brushes::transparent());
        overflow_button.set_background(transparent);
        overflow_button.set_border_thickness(Thickness::uniform(0.0));
        overflow_button.set_padding(Thickness::new(12.0, 0.0, 12.0, 0.0));
        overflow_button.set_min_width(40.0);
        overflow_button.set_vertical_alignment(VerticalAlignment::Stretch);
        overflow_button.bind_binding(
            Visual::is_visible_property().as_property(),
            &TemplateBinding::new(CommandBar::is_overflow_button_visible_property().as_property()),
        );
        overflow_button.set_content(Some(Control::boxed(overflow_glyph)));
        let overflow_button = overflow_button.register_in_name_scope(&**ns);

        // Custom content area (docked left)
        let content_presenter = ContentPresenter::new();
        content_presenter.set_name(Some("PART_ContentPresenter".to_string()));
        content_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(CommandBar::content_property().as_property()),
        );
        DockPanel::set_dock(&content_presenter, Dock::Left);
        content_presenter.set_vertical_alignment(VerticalAlignment::Center);

        // Primary commands (fills remaining space, right-aligned)
        let primary_commands = ItemsControl::new();
        primary_commands.set_name(Some("PART_PrimaryCommands".to_string()));
        primary_commands.set_theme(items_control_theme());
        primary_commands.bind_binding(
            ItemsControl::items_source_property().as_property(),
            &collection_binding("VisiblePrimaryCommands", CommandBar::visible_primary_commands),
        );
        primary_commands.set_horizontal_alignment(HorizontalAlignment::Right);
        primary_commands.set_vertical_alignment(VerticalAlignment::Stretch);
        primary_commands.set_clip_to_bounds(true);
        let primary_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
            let panel = StackPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            panel.set_spacing(2.0);
            Some(panel.upcast::<Panel>())
        });
        primary_commands.set_items_panel(primary_panel);

        let dock_panel = DockPanel::new();
        KeyboardNavigation::set_tab_navigation(&dock_panel, KeyboardNavigationMode::Cycle);
        dock_panel.children().add(overflow_button.clone());
        dock_panel.children().add(content_presenter.register_in_name_scope(&**ns));
        dock_panel.children().add(primary_commands.register_in_name_scope(&**ns));

        // Overflow popup
        let overflow_presenter = ItemsControl::new();
        overflow_presenter.set_name(Some("PART_OverflowPresenter".to_string()));
        overflow_presenter.set_theme(items_control_theme());
        overflow_presenter.bind_binding(
            ItemsControl::items_source_property().as_property(),
            &collection_binding("OverflowItems", CommandBar::overflow_items),
        );
        let overflow_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
            let panel = StackPanel::new();
            panel.set_spacing(2.0);
            Some(panel.upcast::<Panel>())
        });
        overflow_presenter.set_items_panel(overflow_panel);

        let popup_border = Border::new();
        popup_border.set_padding(Thickness::uniform(4.0));
        popup_border.set_min_width(160.0);
        popup_border.set_child(overflow_presenter.register_in_name_scope(&**ns));

        let overflow_popup = Popup::new();
        overflow_popup.set_name(Some("PART_OverflowPopup".to_string()));
        overflow_popup.bind_binding(
            Popup::is_open_property().as_property(),
            &TemplateBinding::new(CommandBar::is_open_property().as_property()).with_mode(BindingMode::TwoWay),
        );
        overflow_popup.set_placement_target(&overflow_button);
        overflow_popup.set_placement(PlacementMode::Bottom);
        overflow_popup.set_is_light_dismiss_enabled(true);
        overflow_popup.set_window_manager_add_shadow_hint(true);
        overflow_popup.set_child(popup_border);

        let panel = Panel::new();
        panel.children().add(dock_panel);
        panel.children().add(overflow_popup.register_in_name_scope(&**ns));

        let border = Border::new();
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            border.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        bind(Decorator::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(Layoutable::min_height_property().as_property(), Layoutable::min_height_property().as_property());
        border.set_child(panel);
        border.upcast()
    })
}

/// The control theme the test theme gives to command bars.
pub fn command_bar_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        CommandBar::TYPE,
        [
            Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch),
            Setter::new(Layoutable::min_height_property(), 48.0),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(4.0, 0.0, 4.0, 0.0)),
            Setter::new(CommandBar::item_width_bottom_property(), 70.0),
            Setter::new(CommandBar::item_width_right_property(), 102.0),
            Setter::new(CommandBar::item_width_collapsed_property(), 42.0),
            Setter::new(TemplatedControl::template_property(), Some(command_bar_template())),
        ],
    )
}

/// Adds the control themes of the command bar, of its buttons and of its
/// separator.
pub fn add_command_bar_themes(styles: &Ref<Styles>) {
    let resources = styles.resources();
    resources.add_value(ResourceKey::Type(CommandBarButton::TYPE), command_bar_button_theme());
    resources.add_value(ResourceKey::Type(CommandBarToggleButton::TYPE), command_bar_toggle_button_theme());
    resources.add_value(ResourceKey::Type(CommandBarSeparator::TYPE), command_bar_separator_theme());
    resources.add_value(ResourceKey::Type(CommandBar::TYPE), command_bar_theme());
}
