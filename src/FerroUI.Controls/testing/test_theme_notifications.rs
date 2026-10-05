//! The control themes of the notification card and the window notification
//! manager of the test theme: the themes of the reference simple theme built
//! in code, with the same templates, part names, bindings, data templates
//! and nested styles.
//!
//! Left out: the setters whose values are theme resources (the backgrounds,
//! the foreground and the font size of the card) and the animations of the
//! card that move, scale and fade it when it opens and closes. The
//! animation that closes the card (it sets `IsClosed` when the closing
//! animation ends) is part of the theme.

use crate::notifications::{INotification, NotificationCard, ReversibleStackPanel, WindowNotificationManager};
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::{ContentControl, Control, LayoutTransformControl, StackPanel, TextBlock};
use ferroui_base::animation::easings::QuadraticEaseOut;
use ferroui_base::animation::{Animation, Cue, FillMode, KeyFrame, TimeSpan};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::styling::{ControlTheme, Selector, Selectors, Setter, Style, Styles};
use ferroui_base::{
    BoxedValue, FerroProperty, Ref, RelativePoint, RelativeUnit, Thickness, Visual,
};
use std::rc::Rc;

/// The data template of notifications: the title above the message.
fn notification_data_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |data| data.as_ref().is_some_and(|data| <dyn INotification>::from_boxed(data).is_some()),
        |_, _| {
            let panel = StackPanel::new();
            panel.set_margin(Thickness::uniform(12.0));
            panel.set_spacing(8.0);

            let title = TextBlock::new();
            title.set_font_weight(FontWeight::Medium);
            title.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("Title"));
            panel.children().add(title);

            let message = TextBlock::new();
            message.set_max_height(80.0);
            message.set_margin(Thickness::new(0.0, 0.0, 12.0, 0.0));
            message.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("Message"));
            message.set_text_wrapping(TextWrapping::Wrap);
            panel.children().add(message);

            Some(panel.upcast::<Control>())
        },
        false,
    )
}

/// The data template of strings: the text.
fn string_data_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<String>(
        |_, _| {
            let text_block = TextBlock::new();
            text_block.set_margin(Thickness::uniform(12.0));
            text_block.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::default());
            Some(text_block.upcast::<Control>())
        },
        false,
    )
}

/// The control template of the window notification manager.
pub fn window_notification_manager_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let items = ReversibleStackPanel::new();
        items.set_name(Some("PART_Items".to_string()));
        items.data_templates().add(notification_data_template());
        items.data_templates().add(string_data_template());
        items.register_in_name_scope(&**ns).upcast::<Control>()
    })
}

/// The control template of the notification card.
pub fn notification_card_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let layout_transform_control = LayoutTransformControl::new();
        layout_transform_control.set_name(Some("PART_LayoutTransformControl".to_string()));
        layout_transform_control.set_use_render_transform(true);

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.set_min_height(150.0);
        presenter.set_padding(Thickness::new(8.0, 8.0, 0.0, 0.0));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(ContentPresenter::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(
            ContentPresenter::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        bind(
            ContentPresenter::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(
            ContentPresenter::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        layout_transform_control.set_child(presenter.register_in_name_scope(&**ns));

        layout_transform_control.register_in_name_scope(&**ns).upcast::<Control>()
    })
}

/// `^:<position> /template/ ReversibleStackPanel#PART_Items`.
fn items_selector(position_class: &str) -> Selector {
    Selectors::nesting(None).class(position_class).template().of_type::<ReversibleStackPanel>().name("PART_Items")
}

/// The style that places the items panel for one position of the manager.
fn position_style(
    position_class: &str,
    reverse_order: bool,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
) -> Ref<Style> {
    let mut setters = Vec::new();
    if reverse_order {
        setters.push(Setter::new(ReversibleStackPanel::reverse_order_property(), true));
    }
    setters.push(Setter::new(Layoutable::vertical_alignment_property(), vertical_alignment));
    setters.push(Setter::new(Layoutable::horizontal_alignment_property(), horizontal_alignment));
    Style::with_setters(items_selector(position_class), setters)
}

/// The control theme of the window notification manager.
pub fn window_notification_manager_theme() -> Ref<ControlTheme> {
    let theme = ControlTheme::with_setters(
        WindowNotificationManager::TYPE,
        [
            Setter::new(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 8.0, 8.0)),
            Setter::new(TemplatedControl::template_property(), Some(window_notification_manager_template())),
        ],
    );

    use HorizontalAlignment::{Center, Left, Right};
    use VerticalAlignment::{Bottom, Top};
    theme.add_style(position_style(":topleft", false, Top, Left));
    theme.add_style(position_style(":topright", false, Top, Right));
    theme.add_style(position_style(":topcenter", false, Top, Center));
    theme.add_style(position_style(":bottomleft", true, Bottom, Left));
    theme.add_style(position_style(":bottomright", true, Bottom, Right));
    theme.add_style(position_style(":bottomcenter", true, Bottom, Center));
    theme
}

/// The control theme of the notification card.
pub fn notification_card_theme() -> Ref<ControlTheme> {
    let theme = ControlTheme::with_setters(
        NotificationCard::TYPE,
        [
            Setter::new(Layoutable::use_layout_rounding_property(), true),
            Setter::new(Layoutable::width_property(), 350.0),
            Setter::new(Visual::render_transform_origin_property(), RelativePoint::new(0.5, 0.75, RelativeUnit::Relative)),
            Setter::new(TemplatedControl::template_property(), Some(notification_card_template())),
        ],
    );

    // `^[IsClosing=true]`: the card is closed when the animation ends.
    let is_closing: BoxedValue = Rc::new(true);
    let closing = Style::with_setters(
        Selectors::property_equals_untyped(
            Some(Selectors::nesting(None)),
            NotificationCard::is_closing_property().as_property(),
            is_closing,
        ),
        [],
    );
    let animation = Animation::new();
    animation.set_easing(QuadraticEaseOut::new());
    animation.set_fill_mode(FillMode::Forward);
    animation.set_duration(TimeSpan::from_seconds(1.25));
    animation
        .children()
        .add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(NotificationCard::is_closed_property(), true) as _]));
    closing.add_animation(animation);
    theme.add_style(closing);
    theme
}

/// Adds the control themes of the notification controls.
pub fn add_notifications_themes(styles: &Ref<Styles>) {
    styles.resources().add_value(ResourceKey::Type(WindowNotificationManager::TYPE), window_notification_manager_theme());
    styles.resources().add_value(ResourceKey::Type(NotificationCard::TYPE), notification_card_theme());
}
