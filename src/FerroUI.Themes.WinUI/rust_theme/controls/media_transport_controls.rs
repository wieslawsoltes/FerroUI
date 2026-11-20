//! `Controls/MediaTransportControls.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/MediaTransportControls_themeresources.xaml
//!   (DefaultMediaTransportControlsStyle and the implicit style of MediaTransportControls; the
//!   resources are Resources/MediaTransportControls_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The parts and their names, the grid of ControlPanelGrid, the sizes and the margins are the
//!   source's. The visual states are pseudo-classes the control sets
//!   (MediaTransportControls::PC_*): :controlpanelfadeout (ControlPanelFadeOut), :buffering,
//!   :loading and :error (MediaStates), :audioselectionavailable, :ccselectionavailable,
//!   :compactmode (CompactMode), :pausestate (PauseState), :mutestate (MuteState),
//!   :fullwindowstate (FullWindowState, commented out in the source, where full window is not
//!   supported) and :repeatonestate (RepeatOneState). FocusStates is the focus adorner of the parts.
//! - The commands are AppBarButtons (CommandBarButton, CommandBarToggleButton) of the style of
//!   the source's AppBarButtonStyle (MediaTransportControlsAppBarButtonStyle below) in three groups
//!   in place of the CommandBar MediaControlsCommandBar: the commands left of LeftSeparator, those
//!   between the separators and those right of RightSeparator. The control shows the commands
//!   their properties and the media allow, and leaves out those of the lowest
//!   MediaTransportControlsHelper.DropoutOrder while the row is too narrow (the orders of the
//!   source, in code). The visibility of the buttons and of PlayPauseButton in CompactMode is set
//!   by the control.
//! - The icons are path icons of the glyphs (GlyphGeometry<code point>, Resources/Glyphs.xaml),
//!   where the source has FontIcon and SymbolIcon of Segoe Fluent Icons.
//! - The volume flyout VolumeFlyout of VolumeMuteButton (AudioMuteButton, VolumeSlider and
//!   VolumeValue) is made by the control, with the values of the source; its presenter has
//!   MediaTransportControlsFlyoutStyle (the source's FlyoutStyle).
//! - ControlPanelFadeIn and ControlPanelFadeOut fade the opacity in 0.3 seconds (the source fades
//!   in in 0.3 seconds and out in 0.7, and moves the panel by 50); the faded panel is not hit
//!   test visible, as the source's.
//! - The background, the border and the corner radius of ControlPanelGrid are on the border
//!   ControlPanelGridBorder around it (FerroUI's Grid has no border); its BorderThickness is 1
//!   (MediaTransportControlsBorderThickness, a double in the source).
//!   MediaTransportControlsTitleSafeBounds (the television layout) is waived.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FlowDirection;
use ferroui_base::media::FontFeatureCollection;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::CommandBarButton;
use ferroui_controls::CommandBarDefaultLabelPosition;
use ferroui_controls::CommandBarToggleButton;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::FlyoutPresenter;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::PathIcon;
use ferroui_controls::ProgressBar;
use ferroui_controls::RowDefinitions;
use ferroui_controls::Slider;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;
use ferroui_media_video_controls::MediaTransportControls;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // AppBarButtonStyle and AppBarToggleButtonStyle of the template: 40 by 40, with the
    // CommandBar.Resources of MediaControlsCommandBar (AppBarButtonInnerBorderMargin 5,
    // AppBarButtonContentViewboxCollapsedMargin 0,12, AppBarThemeCompactHeight 40) and its
    // DefaultLabelPosition Collapsed.
    resources.control_theme(
        "MediaTransportControlsAppBarButtonStyle",
        media_transport_controls_app_bar_button_style,
    );
    resources.control_theme(
        "MediaTransportControlsAppBarToggleButtonStyle",
        media_transport_controls_app_bar_toggle_button_style,
    );
    // FlyoutStyle of the template: the presenter of VolumeFlyout.
    resources.control_theme(
        "MediaTransportControlsFlyoutStyle",
        media_transport_controls_flyout_style,
    );
    resources.control_theme(
        "DefaultMediaTransportControlsStyle",
        default_media_transport_controls_style,
    );
    resources.control_theme(type_key::<MediaTransportControls>(), media_transport_controls_theme);
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

/// `MediaTransportControlsAppBarButtonStyle`: a control theme of `CommandBarButton`.
fn media_transport_controls_app_bar_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarButton>();
    theme.based_on("DefaultAppBarButtonStyle")?;
    theme.dynamic(
        Layoutable::height_property(),
        "MediaTransportControlsAppBarButtonHeight",
    );
    theme.dynamic(Layoutable::width_property(), "MediaTransportControlsAppBarButtonWidth");
    theme.set(
        CommandBarButton::label_position_property(),
        CommandBarDefaultLabelPosition::Collapsed,
    );
    theme.style(
        nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
        |style| {
            style.set(Layoutable::margin_property(), Thickness::uniform(5.0));
            Ok(())
        },
    )?;
    theme.style(nesting().template().of::<Viewbox>().name("ContentViewbox"), |style| {
        style.set(Layoutable::margin_property(), Thickness::symmetric(0.0, 12.0));
        Ok(())
    })?;
    theme.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
        style.set(Layoutable::min_height_property(), 40.0);
        Ok(())
    })?;
    Ok(())
}

/// `MediaTransportControlsAppBarToggleButtonStyle`: a control theme of `CommandBarToggleButton`.
fn media_transport_controls_app_bar_toggle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarToggleButton>();
    theme.based_on("DefaultAppBarToggleButtonStyle")?;
    theme.dynamic(
        Layoutable::height_property(),
        "MediaTransportControlsAppBarButtonHeight",
    );
    theme.dynamic(Layoutable::width_property(), "MediaTransportControlsAppBarButtonWidth");
    theme.set(
        CommandBarToggleButton::label_position_property(),
        CommandBarDefaultLabelPosition::Collapsed,
    );
    Ok(())
}

/// `MediaTransportControlsFlyoutStyle`: a control theme of `FlyoutPresenter`.
fn media_transport_controls_flyout_style(theme: &StyleBuilder) -> Result {
    theme.target::<FlyoutPresenter>();
    theme.based_on("DefaultFlyoutPresenterStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "MediaTransportControlsFlyoutBackground",
    );
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    Ok(())
}

/// The template of `MediaTransportControls` of `default_media_transport_controls_style`.
fn default_media_transport_controls_style_template(t: &Template<MediaTransportControls>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(Border::new())
            .name("ControlPanel_ControlPanelVisibilityStates_Border")?
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
                                SignedDuration::from_milliseconds(300.0),
                            )
                            .end()
                            .into(),
                    );
                    object
                }),
            )
            .child(
                Decorator::child_property(),
                t.make(Border::new())
                    .name("ControlPanelGridBorder")?
                    .dynamic(Border::background_property(), "MediaTransportControlsPanelBackground")
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                    .dynamic(Border::border_brush_property(), "MediaTransportControlsBorderBrush")
                    .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                    .template_bind(
                        Border::corner_radius_property(),
                        TemplatedControl::corner_radius_property(),
                    )
                    .child(
                        Decorator::child_property(),
                        t.make(Grid::new())
                            .name("ControlPanelGrid")?
                            .with(|e| {
                                e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                e.set_row_definitions(literal(RowDefinitions::parse("Auto,*,Auto"))?);
                                Ok(())
                            })?
                            .children([
                                t.make(Border::new())
                                    .name("ErrorBorder")?
                                    .set(Layoutable::width_property(), 320.0)
                                    .set(Layoutable::height_property(), 96.0)
                                    .set(Grid::column_span_property(), 3)
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                    .dynamic(Border::background_property(), "MediaTransportControlsPanelBackground")
                                    .set(Visual::is_visible_property(), false)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(TextBlock::new())
                                            .name("ErrorTextBlock")?
                                            .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                                            .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                                            .set(Layoutable::margin_property(), Thickness::uniform(12.0)),
                                    )
                                    .control(),
                                t.make(Border::new())
                                    .name("MediaTransportControls_Timeline_Border")?
                                    .set(Grid::column_property(), 1)
                                    .set(Grid::row_property(), 1)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(Grid::new())
                                            .name("MediaTransportControls_Timeline_Grid")?
                                            .with(|e| {
                                                e.set_row_definitions(literal(RowDefinitions::parse("*,Auto"))?);
                                                Ok(())
                                            })?
                                            .children([
                                                t.make(Slider::new())
                                                    .name("ProgressSlider")?
                                                    .dynamic(
                                                        Layoutable::margin_property(),
                                                        "MediaTransportControlsProgressSliderMargin",
                                                    )
                                                    .set(Layoutable::min_width_property(), 80.0)
                                                    .set(RangeBase::minimum_property(), 0.0)
                                                    .set(RangeBase::maximum_property(), 1.0)
                                                    .dynamic(
                                                        Layoutable::height_property(),
                                                        "MediaTransportControlsSliderHeight",
                                                    )
                                                    .set(
                                                        Layoutable::vertical_alignment_property(),
                                                        VerticalAlignment::Center,
                                                    )
                                                    .set(
                                                        AutomationProperties::name_property(),
                                                        Some(String::from("Playback position")),
                                                    )
                                                    .control(),
                                                t.make(ProgressBar::new())
                                                    .name("BufferingProgressBar")?
                                                    .set(Layoutable::height_property(), 4.0)
                                                    .set(ProgressBar::is_indeterminate_property(), true)
                                                    .set(InputElement::is_hit_test_visible_property(), false)
                                                    .set(
                                                        Layoutable::vertical_alignment_property(),
                                                        VerticalAlignment::Top,
                                                    )
                                                    .set(
                                                        Layoutable::margin_property(),
                                                        Thickness::new(0.0, 2.0, 0.0, 0.0),
                                                    )
                                                    .set(Visual::is_visible_property(), false)
                                                    .control(),
                                                t.make(Grid::new())
                                                    .name("TimeTextGrid")?
                                                    .set(
                                                        Layoutable::margin_property(),
                                                        Thickness::new(7.0, 0.0, 7.0, 2.0),
                                                    )
                                                    .set(Grid::row_property(), 1)
                                                    .set(Layoutable::height_property(), 16.0)
                                                    .children([
                                                        t.make(TextBlock::new())
                                                            .name("TimeElapsedElement")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "CaptionTextBlockStyle",
                                                            )?
                                                            .set(Layoutable::margin_property(), Thickness::uniform(0.0))
                                                            .set(
                                                                TextBlock::text_property(),
                                                                Some(String::from("00:00")),
                                                            )
                                                            .set(
                                                                Layoutable::horizontal_alignment_property(),
                                                                HorizontalAlignment::Left,
                                                            )
                                                            .set(
                                                                Layoutable::vertical_alignment_property(),
                                                                VerticalAlignment::Bottom,
                                                            )
                                                            .set(
                                                                TextBlock::font_features_property(),
                                                                Some(literal(FontFeatureCollection::parse("tnum"))?),
                                                            )
                                                            .set(
                                                                AutomationProperties::name_property(),
                                                                Some(String::from("Time elapsed")),
                                                            )
                                                            .control(),
                                                        t.make(TextBlock::new())
                                                            .name("TimeRemainingElement")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "CaptionTextBlockStyle",
                                                            )?
                                                            .set(
                                                                TextBlock::text_property(),
                                                                Some(String::from("00:00")),
                                                            )
                                                            .set(
                                                                Layoutable::horizontal_alignment_property(),
                                                                HorizontalAlignment::Right,
                                                            )
                                                            .set(
                                                                Layoutable::vertical_alignment_property(),
                                                                VerticalAlignment::Bottom,
                                                            )
                                                            .set(
                                                                TextBlock::font_features_property(),
                                                                Some(literal(FontFeatureCollection::parse("tnum"))?),
                                                            )
                                                            .set(
                                                                AutomationProperties::name_property(),
                                                                Some(String::from("Time remaining")),
                                                            )
                                                            .control(),
                                                    ])
                                                    .control(),
                                            ]),
                                    )
                                    .control(),
                                t.make(Border::new())
                                    .name("LeftSidePlayBorder")?
                                    .set(Grid::column_property(), 0)
                                    .set(Grid::row_property(), 1)
                                    .set(Visual::is_visible_property(), false)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(CommandBarButton::new())
                                            .name("PlayPauseButtonOnLeft")?
                                            .set(Layoutable::margin_property(), Thickness::uniform(0.0))
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "MediaTransportControlsAppBarButtonStyle",
                                            )?
                                            .content(
                                                CommandBarButton::icon_property(),
                                                t.make(PathIcon::new())
                                                    .name("PlayPauseSymbolLeft")?
                                                    .static_resource(PathIcon::data_property(), "GlyphGeometryF5B0")?,
                                            )?,
                                    )
                                    .control(),
                                t.make(Border::new())
                                    .name("MediaTransportControls_Command_Border")?
                                    .set(Grid::column_property(), 1)
                                    .set(Grid::row_property(), 2)
                                    .child(
                                        Decorator::child_property(),
                                        t.make(Grid::new())
                                            .name("MediaControlsCommandBar")?
                                            .set(Layoutable::margin_property(), Thickness::symmetric(0.0, 3.0))
                                            .with(|e| {
                                                e.set_column_definitions(literal(ColumnDefinitions::parse(
                                                    "Auto,*,Auto",
                                                ))?);
                                                Ok(())
                                            })?
                                            .children([
                                                t.make(StackPanel::new())
                                                    .name("LeftCommands")?
                                                    .set(StackPanel::orientation_property(), Orientation::Horizontal)
                                                    .children([
                                                        t.make(CommandBarButton::new())
                                                            .name("VolumeMuteButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new())
                                                                    .name("VolumeMuteSymbol")?
                                                                    .static_resource(
                                                                        PathIcon::data_property(),
                                                                        "GlyphGeometryE15D",
                                                                    )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("CCSelectionButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryED1E",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("AudioTracksSelectionButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryED1F",
                                                                )?,
                                                            )?
                                                            .control(),
                                                    ])
                                                    .control(),
                                                t.make(StackPanel::new())
                                                    .name("CenterCommands")?
                                                    .set(Grid::column_property(), 1)
                                                    .set(StackPanel::orientation_property(), Orientation::Horizontal)
                                                    .set(
                                                        Layoutable::horizontal_alignment_property(),
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .children([
                                                        t.make(CommandBarButton::new())
                                                            .name("StopButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryE15B",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("SkipBackwardButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryED3C",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("PreviousTrackButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryF8AC",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("RewindButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryE627",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("PlayPauseButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new())
                                                                    .name("PlayPauseSymbol")?
                                                                    .static_resource(
                                                                        PathIcon::data_property(),
                                                                        "GlyphGeometryF5B0",
                                                                    )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("FastForwardButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryE628",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("NextTrackButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryF8AD",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("SkipForwardButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryED3D",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("PlaybackRateButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryEC57",
                                                                )?,
                                                            )?
                                                            .control(),
                                                    ])
                                                    .control(),
                                                t.make(StackPanel::new())
                                                    .name("RightCommands")?
                                                    .set(Grid::column_property(), 2)
                                                    .set(StackPanel::orientation_property(), Orientation::Horizontal)
                                                    .children([
                                                        t.make(CommandBarToggleButton::new())
                                                            .name("RepeatButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarToggleButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarToggleButton::icon_property(),
                                                                t.make(PathIcon::new())
                                                                    .name("RepeatSymbol")?
                                                                    .static_resource(
                                                                        PathIcon::data_property(),
                                                                        "GlyphGeometryE1CD",
                                                                    )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("ZoomButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryE799",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("CastButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new()).static_resource(
                                                                    PathIcon::data_property(),
                                                                    "GlyphGeometryEC15",
                                                                )?,
                                                            )?
                                                            .control(),
                                                        t.make(CommandBarButton::new())
                                                            .name("FullWindowButton")?
                                                            .static_resource(
                                                                StyledElement::theme_property(),
                                                                "MediaTransportControlsAppBarButtonStyle",
                                                            )?
                                                            .content(
                                                                CommandBarButton::icon_property(),
                                                                t.make(PathIcon::new())
                                                                    .name("FullWindowSymbol")?
                                                                    .static_resource(
                                                                        PathIcon::data_property(),
                                                                        "GlyphGeometryE1D9",
                                                                    )?,
                                                            )?
                                                            .control(),
                                                    ])
                                                    .control(),
                                            ]),
                                    )
                                    .control(),
                            ]),
                    ),
            )
            .control()])
        .control())
}

/// `DefaultMediaTransportControlsStyle`: a control theme of `MediaTransportControls`.
fn default_media_transport_controls_style(theme: &StyleBuilder) -> Result {
    theme.target::<MediaTransportControls>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(Visual::flow_direction_property(), FlowDirection::LeftToRight);
    theme.dynamic(Layoutable::margin_property(), "MediaTransportControlsMargin");
    theme.dynamic(Layoutable::max_width_property(), "MediaTransportControlsMaxWidth");
    theme.dynamic(Layoutable::min_width_property(), "MediaTransportControlsMinWidth");
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.template(default_media_transport_controls_style_template);
    // ControlPanelVisibilityStates: ControlPanelFadeOut.
    theme.style(
        nesting()
            .class(":controlpanelfadeout")
            .template()
            .of::<Border>()
            .name("ControlPanel_ControlPanelVisibilityStates_Border"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set(InputElement::is_hit_test_visible_property(), false);
            Ok(())
        },
    )?;
    // MediaStates: Buffering, Loading and Error.
    theme.style(
        nesting()
            .class(":buffering")
            .template()
            .of::<ProgressBar>()
            .name("BufferingProgressBar"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(nesting().class(":loading"), |style| {
        style.style(
            nesting().template().of::<ProgressBar>().name("BufferingProgressBar"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Slider>().name("ProgressSlider"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Grid>().name("MediaControlsCommandBar"),
            |style| {
                style.set(Visual::opacity_property(), 0.0);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(
        nesting().class(":error").template().of::<Border>().name("ErrorBorder"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // MediaTransportControlMode: CompactMode.
    theme.style(nesting().class(":compactmode"), |style| {
        style.style(
            nesting().template().of::<Border>().name("LeftSidePlayBorder"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("TimeTextGrid"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("MediaTransportControls_Command_Border"),
            |style| {
                style.set(Grid::column_property(), 2);
                style.set(Grid::row_property(), 1);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Grid>().name("ControlPanelGrid"), |style| {
            style.set(Layoutable::height_property(), 48.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Grid>().name("MediaControlsCommandBar"),
            |style| {
                style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Slider>().name("ProgressSlider"), |style| {
            style.dynamic(
                Layoutable::margin_property(),
                "MediaTransportControlsProgressSliderCompactMargin",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // PlayPauseStates: PauseState.
    theme.style(nesting().class(":pausestate"), |style| {
        style.style(nesting().template().of::<PathIcon>().name("PlayPauseSymbol"), |style| {
            style.static_resource(PathIcon::data_property(), "GlyphGeometryF8AE")?;
            Ok(())
        })?;
        style.style(
            nesting().template().of::<PathIcon>().name("PlayPauseSymbolLeft"),
            |style| {
                style.static_resource(PathIcon::data_property(), "GlyphGeometryF8AE")?;
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // VolumeMuteStates: MuteState.
    theme.style(
        nesting()
            .class(":mutestate")
            .template()
            .of::<PathIcon>()
            .name("VolumeMuteSymbol"),
        |style| {
            style.static_resource(PathIcon::data_property(), "GlyphGeometryE198")?;
            Ok(())
        },
    )?;
    // FullWindowStates: FullWindowState.
    theme.style(
        nesting()
            .class(":fullwindowstate")
            .template()
            .of::<PathIcon>()
            .name("FullWindowSymbol"),
        |style| {
            style.static_resource(PathIcon::data_property(), "GlyphGeometryE1D8")?;
            Ok(())
        },
    )?;
    // RepeatStates: RepeatOneState (IsChecked of RepeatButton is set by the control).
    theme.style(
        nesting()
            .class(":repeatonestate")
            .template()
            .of::<PathIcon>()
            .name("RepeatSymbol"),
        |style| {
            style.static_resource(PathIcon::data_property(), "GlyphGeometryE1CC")?;
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `MediaTransportControls`.
fn media_transport_controls_theme(theme: &StyleBuilder) -> Result {
    theme.target::<MediaTransportControls>();
    theme.based_on("DefaultMediaTransportControlsStyle")?;
    Ok(())
}
