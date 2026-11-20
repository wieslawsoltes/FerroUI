//! `Controls/NotificationCard.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/InfoBar/InfoBar.xaml (the implicit InfoBar style)
//!   controls/dev/InfoBar/InfoBar_themeresources.xaml (InfoBarCloseButtonStyle; its resources are
//!   Resources/InfoBar_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! NotificationCard is FerroUI's notification, the counterpart of WinUI's InfoBar: the template is
//! the one of InfoBar, open, with the standard icon of its severity and its close button.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - SeverityLevels (Informational, Error, Warning, Success) are the pseudo-classes of the type of
//!   the notification (the base, :error, :warning, :success); :information is the base.
//! - The icon of the severity is two glyphs of the symbol font on top of each other
//!   (InfoBarIconBackgroundGlyph F136 in the icon background of the severity, the symbol of the
//!   severity F13F, F13D, F13C or F13E in its icon foreground) at InfoBarIconFontSize: two paths of
//!   the geometries GlyphGeometry<code point> centred in a box of 16 x 16. The user icon of InfoBar
//!   (IconSource, UserIconBox) and IsIconVisible have no counterpart: the standard icon is always
//!   shown (StandardIconVisible).
//! - The title, the message and the action button of InfoBar (the InfoBarPanel) are the content of
//!   the card, presented where InfoBar has its InfoBarPanel (InfoBarPanelMargin). A notification
//!   shown by WindowNotificationManager is presented with the title and the message of InfoBar in
//!   the InfoBarPanel of the theme (Primitives/info_bar_panel.rs), on one line when they fit
//!   (WindowNotificationManager.xaml). ContentArea, the second
//!   row for the Content of InfoBar, has no counterpart (NoBannerContent).
//! - The close button is the CloseButton of InfoBar with InfoBarCloseButtonStyle; it closes the card
//!   with NotificationCard.CloseOnClick (CloseButtonCommand). IsClosable has no counterpart: the
//!   button is always shown (CloseButtonVisible). The SymbolIcon of InfoBarCloseButtonSymbol (Cancel)
//!   in a Viewbox of InfoBarCloseButtonGlyphSize is the path of GlyphGeometryE711 at the size of the
//!   ink WinUI draws: the SymbolIcon is 20 x 20 (font size 20), the Viewbox of 16 scales it by 0.8,
//!   and the ink of the Cancel glyph, 12.5 wide at 20, is 10 wide (measured against native WinUI 3,
//!   which also gives its strokes the width of one pixel). The AppBarButton
//!   brushes the template gives the close button through Button.Resources are the brushes of
//!   InfoBarCloseButtonStyle.
//! - InfoBarVisibility (IsOpen) is the closing of the card: a closing card (IsClosing) fades out over
//!   ControlFasterAnimationDuration (83 ms) and is then closed (IsClosed), which removes it from
//!   WindowNotificationManager; InfoBar hides at once. ForegroundSet is the inherited foreground of
//!   the content. AutomationProperties (LandmarkType, IsDialog) are left out.
//! - WinUI's InfoBar itself is the InfoBar control of ferroui-controls, styled by Controls/InfoBar.xaml,
//!   which uses InfoBarCloseButtonStyle of this document.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::Animation;
use ferroui_base::animation::Cue;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::FillMode;
use ferroui_base::animation::KeyFrame;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::NotificationCard;
use ferroui_controls::Panel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::setter;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // InfoBarCloseButtonStyle (InfoBar_themeresources.xaml), with the AppBarButton brushes the
    // InfoBar template gives its close button.
    resources.control_theme("InfoBarCloseButtonStyle", info_bar_close_button_style);
    resources.control_theme(type_key::<NotificationCard>(), notification_card_theme);
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

/// `InfoBarCloseButtonStyle`: a control theme of `Button`.
fn info_bar_close_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.static_resource(Layoutable::width_property(), "InfoBarCloseButtonSize")?;
    theme.static_resource(Layoutable::height_property(), "InfoBarCloseButtonSize")?;
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(Layoutable::margin_property(), Thickness::uniform(5.0));
    theme.dynamic(TemplatedControl::background_property(), "AppBarButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "AppBarButtonBorderBrush");
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "AppBarButtonForegroundPointerOver",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "AppBarButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AppBarButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "AppBarButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AppBarButtonBorderBrushPressed",
                );
                style.dynamic(ContentPresenter::foreground_property(), "AppBarButtonForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "AppBarButtonForegroundDisabled",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "AppBarButtonBackgroundDisabled",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AppBarButtonBorderBrushDisabled",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "AppBarButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `NotificationCard` of `notification_card_theme`.
fn notification_card_theme_template(t: &Template<NotificationCard>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ContentRoot")?
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
        .dynamic(
            Border::background_property(),
            "InfoBarInformationalSeverityBackgroundBrush",
        )
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(DoubleTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(Visual::opacity_property().property()),
                        )
                        .set(
                            TransitionBase::duration_property(),
                            SignedDuration::from_milliseconds(83.0),
                        )
                        .end()
                        .into(),
                );
                object
            }),
        )
        // The background of the card over the background of the severity.
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .dynamic(Layoutable::min_height_property(), "InfoBarMinHeight")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .static_resource(Decorator::padding_property(), "InfoBarContentRootPadding")?
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Panel::new())
                                .name("StandardIconArea")?
                                .static_resource(Layoutable::width_property(), "InfoBarIconFontSize")?
                                .static_resource(Layoutable::height_property(), "InfoBarIconFontSize")?
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .static_resource(Layoutable::margin_property(), "InfoBarIconMargin")?
                                .children([
                                    t.make(Path::new())
                                        .name("IconBackground")?
                                        .static_resource(Path::data_property(), "GlyphGeometryF136")?
                                        .dynamic(Shape::fill_property(), "InfoBarInformationalSeverityIconBackground")
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 16.0)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                        .control(),
                                    t.make(Path::new())
                                        .name("StandardIcon")?
                                        .static_resource(Path::data_property(), "GlyphGeometryF13F")?
                                        .dynamic(Shape::fill_property(), "InfoBarInformationalSeverityIconForeground")
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::height_property(), 8.0)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                        .control(),
                                ])
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_ContentPresenter")?
                                .set(Grid::column_property(), 1)
                                .static_resource(Layoutable::margin_property(), "InfoBarPanelMargin")?
                                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ContentControl::content_template_property(),
                                )
                                .template_bind(
                                    ContentPresenter::horizontal_content_alignment_property(),
                                    ContentControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    ContentPresenter::vertical_content_alignment_property(),
                                    ContentControl::vertical_content_alignment_property(),
                                )
                                .control(),
                            t.make(Button::new())
                                .name("CloseButton")?
                                .set(Grid::column_property(), 2)
                                .static_resource(StyledElement::theme_property(), "InfoBarCloseButtonStyle")?
                                .set(NotificationCard::close_on_click_property(), true)
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryE711")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 10.0)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                                )?
                                .control(),
                        ]),
                ),
        )
        .control())
}

/// The default control theme of `NotificationCard`.
fn notification_card_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NotificationCard>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "InfoBarBorderBrush");
    theme.dynamic(TemplatedControl::border_thickness_property(), "InfoBarBorderThickness");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // The text of the content: the message of InfoBar (InfoBarMessageForeground).
    theme.dynamic(TemplatedControl::foreground_property(), "InfoBarMessageForeground");
    theme.static_resource(TemplatedControl::font_size_property(), "InfoBarMessageFontSize")?;
    theme.template(notification_card_theme_template);
    // Error
    theme.style(
        nesting().class(":error").template().of::<Border>().name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarErrorSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":error").template().of::<Path>().name("IconBackground"),
        |style| {
            style.dynamic(Shape::fill_property(), "InfoBarErrorSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":error").template().of::<Path>().name("StandardIcon"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryF13D")?;
            style.set(Layoutable::height_property(), 6.0);
            style.dynamic(Shape::fill_property(), "InfoBarErrorSeverityIconForeground");
            Ok(())
        },
    )?;
    // Warning
    theme.style(
        nesting()
            .class(":warning")
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarWarningSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":warning")
            .template()
            .of::<Path>()
            .name("IconBackground"),
        |style| {
            style.dynamic(Shape::fill_property(), "InfoBarWarningSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":warning").template().of::<Path>().name("StandardIcon"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryF13C")?;
            style.dynamic(Shape::fill_property(), "InfoBarWarningSeverityIconForeground");
            Ok(())
        },
    )?;
    // Success
    theme.style(
        nesting()
            .class(":success")
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.dynamic(Border::background_property(), "InfoBarSuccessSeverityBackgroundBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":success")
            .template()
            .of::<Path>()
            .name("IconBackground"),
        |style| {
            style.dynamic(Shape::fill_property(), "InfoBarSuccessSeverityIconBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":success").template().of::<Path>().name("StandardIcon"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryF13E")?;
            style.set(Layoutable::height_property(), 6.5);
            style.dynamic(Shape::fill_property(), "InfoBarSuccessSeverityIconForeground");
            Ok(())
        },
    )?;
    // InfoBarCollapsed: the closing card fades out and is then closed.
    theme.style(
        nesting()
            .property_is(NotificationCard::is_closing_property(), true)
            .template()
            .of::<Border>()
            .name("ContentRoot"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    theme.style(
        nesting().property_is(NotificationCard::is_closing_property(), true),
        |style| {
            style.animation(
                style
                    .scope()
                    .make(Animation::new())
                    .set(Animation::duration_property(), SignedDuration::from_milliseconds(83.0))
                    .set(Animation::fill_mode_property(), FillMode::Forward)
                    .with(|e| {
                        let items = Animation::children(e);
                        items.add(
                            style
                                .scope()
                                .make(KeyFrame::new())
                                .with(|e| {
                                    KeyFrame::set_cue(e, literal(Cue::parse("100%"))?);
                                    Ok(())
                                })?
                                .with(|e| {
                                    let items = KeyFrame::setters(e);
                                    items.add(setter(NotificationCard::is_closed_property(), true));
                                    Ok(())
                                })?
                                .end()
                                .into(),
                        );
                        Ok(())
                    })?
                    .end(),
            );
            Ok(())
        },
    )?;
    Ok(())
}
