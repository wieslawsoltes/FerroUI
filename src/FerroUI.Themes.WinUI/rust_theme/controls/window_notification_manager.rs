//! `Controls/WindowNotificationManager.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI 3 has no host of notifications in a window (microsoft-ui-xaml,
//! https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0).
//! The template is the one FerroUI's WindowNotificationManager requires (the panel PART_Items and
//! the pseudo-classes of its position, src/FerroUI.Themes.Fluent/Controls/WindowNotificationManager.xaml);
//! its cards are NotificationCard, the InfoBar of the theme (NotificationCard.xaml). A notification
//! (INotification) is presented as InfoBar presents its title and message, in the InfoBarPanel of
//! the theme (Primitives/info_bar_panel.rs, the port of InfoBarPanel.cpp) with the paddings and the
//! margins of controls/dev/InfoBar/InfoBar.xaml (the resources of
//! Resources/InfoBar_themeresources.xaml): the title and the message on one line when they fit
//! (both 14 down, the message 12 after the title, the panel without padding, so that a card of one
//! line is the 48 of InfoBarMinHeight), else below each other (the message 4 below the title, the
//! panel padded by InfoBarPanelVerticalOrientationPadding); the title in InfoBarTitleFontSize,
//! InfoBarTitleFontWeight and InfoBarTitleForeground, the message in InfoBarMessageFontSize,
//! InfoBarMessageFontWeight and InfoBarMessageForeground, a title or message that is empty left out
//! as InfoBarPanel leaves out an element without size. TextWrapping WrapWholeWords is Wrap. Text
//! content is presented as the message with InfoBarPanelVerticalOrientationPadding (InfoBarPanel
//! lays out one item below the padding of its vertical layout).
//! The width of a card in the host, the spacing between the cards and the margin of the host have
//! no WinUI source (DEVIATIONS.md, WindowNotificationManager).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::Control;
use ferroui_controls::INotification;
use ferroui_controls::NotificationCard;
use ferroui_controls::ReversibleStackPanel;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::WindowNotificationManager;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::data_template_for;
use ferroui_markup_xaml::rust_theme::data_template_of;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(
        type_key::<WindowNotificationManager>(),
        window_notification_manager_theme,
    );
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`: the document included
/// (`ResourceInclude`) rather than merged.
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// A data template of `item`.
fn item_data_template(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(crate::primitives::InfoBarPanel::new())
        .static_resource(
            crate::primitives::InfoBarPanel::horizontal_orientation_padding_property(),
            "InfoBarPanelHorizontalOrientationPadding",
        )?
        .static_resource(
            crate::primitives::InfoBarPanel::vertical_orientation_padding_property(),
            "InfoBarPanelVerticalOrientationPadding",
        )?
        .children([
            t.make(TextBlock::new())
                .bind(
                    TextBlock::text_property(),
                    path().member::<dyn INotification>("Title").binding(),
                )
                .bind(
                    Visual::is_visible_property(),
                    path()
                        .member::<dyn INotification>("Title")
                        .binding()
                        .with_converter_value(Some(StringConverters::is_not_null_or_empty())),
                )
                .static_resource(
                    ::ferroui_controls::primitives::InfoBarPanel::horizontal_orientation_margin_property(),
                    "InfoBarTitleHorizontalOrientationMargin",
                )?
                .static_resource(
                    ::ferroui_controls::primitives::InfoBarPanel::vertical_orientation_margin_property(),
                    "InfoBarTitleVerticalOrientationMargin",
                )?
                .dynamic(TextBlock::foreground_property(), "InfoBarTitleForeground")
                .static_resource(TextBlock::font_size_property(), "InfoBarTitleFontSize")?
                .static_resource(TextBlock::font_weight_property(), "InfoBarTitleFontWeight")?
                .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                .control(),
            t.make(TextBlock::new())
                .bind(
                    TextBlock::text_property(),
                    path().member::<dyn INotification>("Message").binding(),
                )
                .bind(
                    Visual::is_visible_property(),
                    path()
                        .member::<dyn INotification>("Message")
                        .binding()
                        .with_converter_value(Some(StringConverters::is_not_null_or_empty())),
                )
                .static_resource(
                    ::ferroui_controls::primitives::InfoBarPanel::horizontal_orientation_margin_property(),
                    "InfoBarMessageHorizontalOrientationMargin",
                )?
                .static_resource(
                    ::ferroui_controls::primitives::InfoBarPanel::vertical_orientation_margin_property(),
                    "InfoBarMessageVerticalOrientationMargin",
                )?
                .dynamic(TextBlock::foreground_property(), "InfoBarMessageForeground")
                .static_resource(TextBlock::font_size_property(), "InfoBarMessageFontSize")?
                .static_resource(TextBlock::font_weight_property(), "InfoBarMessageFontWeight")?
                .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                .control(),
        ])
        .control())
}

/// A data template of `item` for text.
fn item_data_template_2(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(TextBlock::new())
        .static_resource(Layoutable::margin_property(), "InfoBarPanelVerticalOrientationPadding")?
        .bind(TextBlock::text_property(), path().binding())
        .dynamic(TextBlock::foreground_property(), "InfoBarMessageForeground")
        .static_resource(TextBlock::font_size_property(), "InfoBarMessageFontSize")?
        .static_resource(TextBlock::font_weight_property(), "InfoBarMessageFontWeight")?
        .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
        .control())
}

/// The template of `WindowNotificationManager` of `window_notification_manager_theme`.
fn window_notification_manager_theme_template(t: &Template<WindowNotificationManager>) -> Result<Ref<Control>> {
    Ok(t.make(ReversibleStackPanel::new())
        .name("PART_Items")?
        .set(Layoutable::margin_property(), Thickness::uniform(12.0))
        .set(StackPanel::spacing_property(), 8.0)
        .with(|e| {
            let items = Control::data_templates(e);
            items.add(data_template_of::<dyn INotification>(t.scope(), item_data_template));
            items.add(data_template_for::<String>(t.scope(), item_data_template_2));
            Ok(())
        })?
        .control())
}

/// The default control theme of `WindowNotificationManager`.
fn window_notification_manager_theme(theme: &StyleBuilder) -> Result {
    theme.target::<WindowNotificationManager>();
    theme.template(window_notification_manager_theme_template);
    // The cards of the host: the width of a notification card of FerroUI (NotificationCardWidth of
    // the Fluent theme); an InfoBar takes the width of its container.
    theme.style(
        nesting()
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items")
            .child()
            .of::<NotificationCard>(),
        |style| {
            style.set(Layoutable::width_property(), 350.0);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":topleft")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":topright")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":topcenter")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":bottomleft")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(ReversibleStackPanel::reverse_order_property(), true);
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":bottomright")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(ReversibleStackPanel::reverse_order_property(), true);
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":bottomcenter")
            .template()
            .of::<ReversibleStackPanel>()
            .name("PART_Items"),
        |style| {
            style.set(ReversibleStackPanel::reverse_order_property(), true);
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
            Ok(())
        },
    )?;
    Ok(())
}
