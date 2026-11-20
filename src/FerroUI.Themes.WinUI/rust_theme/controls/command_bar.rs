//! `Controls/CommandBar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CommandBar_themeresources.xaml (DefaultCommandBarStyle,
//!   DefaultCommandBarOverflowPresenterStyle, EllipsisButton and the implicit styles of
//!   CommandBar and CommandBarOverflowPresenter; the resources are
//!   Resources/CommandBar_themeresources.xaml)
//!   controls/dev/CommonStyles/AppBarButton_themeresources.xaml (DefaultAppBarButtonStyle,
//!   AppBarButtonOverflowStyle and the implicit style of AppBarButton; the resources are
//!   Resources/AppBarButton_themeresources.xaml)
//!   controls/dev/CommonStyles/AppBarToggleButton_themeresources.xaml
//!   (DefaultAppBarToggleButtonStyle, AppBarToggleButtonOverflowStyle and the implicit style of
//!   AppBarToggleButton; the resources are Resources/AppBarToggleButton_themeresources.xaml)
//!   controls/dev/CommonStyles/AppBarSeparator_themeresources.xaml (DefaultAppBarSeparatorStyle and
//!   the implicit style of AppBarSeparator; the resources are
//!   Resources/AppBarSeparator_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's CommandBarButton, CommandBarToggleButton and CommandBarSeparator are WinUI's
//! AppBarButton, AppBarToggleButton and AppBarSeparator.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The visual states the buttons choose in code (AppBarButtonHelpers::ChangeCommonVisualStates:
//!   Overflow before LabelOnRight before LabelCollapsed before Compact before FullSize) are the
//!   properties FerroUI's CommandBar sets on its elements: ^[IsCompact=True] (Compact),
//!   ^[LabelPosition=Collapsed] (LabelCollapsed), ^[LabelPosition=Right] (LabelOnRight, with the
//!   Width NaN of the code's storyboard) and ^[IsInOverflow=True] (Overflow, with the setters of
//!   AppBarButtonOverflowStyle and AppBarToggleButtonOverflowStyle that the overflow presenter
//!   applies), written in that order so that the state of WinUI wins; a later state sets back
//!   what an earlier one changed and it does not. FerroUI's CommandBar has no closed display
//!   mode: its buttons are in the FullSize state (labels under the icons) where WinUI's closed
//!   compact CommandBar shows them Compact (labels hidden until it opens).
//! - The overflow variants OverflowWithToggleButtons, OverflowWithMenuIcons and
//!   OverflowWithToggleButtonsAndMenuIcons, which WinUI's CommandBar chooses from all its
//!   secondary commands, are left out: the overflow items are in the Overflow state (a toggle
//!   button in the overflow has its check glyph, as in WinUI). OverflowNormal, OverflowPointerOver
//!   and OverflowPressed of AppBarButton set what PointerOver and Pressed set; the Overflow*
//!   states of AppBarToggleButton are the nested styles of ^[IsInOverflow=True] after the
//!   others.
//! - The icon is FerroUI's Icon in the Viewbox ContentViewbox (AppBarButtonContentHeight), a path
//!   icon there unscaled as WinUI's (PathIconUnscaledTemplate, DEVIATIONS.md PI-3). The
//!   KeyboardAcceleratorTextLabel, the flyout chevrons (SubItemChevronPanel, hidden in a command
//!   bar by AppBarButtonHasFlyoutChevronVisibility), InputModeStates and
//!   KeyboardAcceleratorTextVisibility are left out (FerroUI's command bar buttons have no
//!   accelerator text, flyout chevron or input mode). AllowFocusOnInteraction="False" is left
//!   out: FerroUI's CommandBar moves the keyboard focus between its buttons.
//! - Border.BackgroundTransition is a SolidColorBrushTransition (WinUI's BrushTransition, BTN-3) of
//!   the background of the inner border.
//! - The check glyph of the overflow toggle button (E73E in a TextBlock of 16 px, 14 x 14) is a
//!   path of GlyphGeometryE73E; the ellipsis of the More button (FontIcon E712 of 20 px whose
//!   height is AppBarExpandButtonCircleDiameter) is a path of GlyphGeometryE712, 15 x 3.
//! - CommandBar: the ContentControl ContentControl is the PART_ContentPresenter, the ItemsControl
//!   PrimaryItemsControl shows VisiblePrimaryCommands, the Button MoreButton is the
//!   PART_OverflowButton (shown by IsOverflowButtonVisible), the Popup OverflowPopup the
//!   PART_OverflowPopup (open while IsOpen, under the bar and aligned with its right edge) and the
//!   CommandBarOverflowPresenter SecondaryItemsControl the ItemsControl PART_OverflowPresenter with
//!   the control theme DefaultCommandBarOverflowPresenterStyle, showing OverflowItems. The states
//!   of the open bar (CompactOpenUp, CompactOpenDown) are ^[IsOpen=True]; the clip and the
//!   translations of the bar and of the overflow content, their transitions and the Minimal and
//!   Hidden display modes are left out (FerroUI's CommandBar has no closed display modes). The
//!   drop shadow of the overflow content (ApplyElevationEffect on SecondaryItemsControlShadowWrapper,
//!   CommandBar_Partial.cpp: a ThemeShadow at a Translation.Z of 32) is a ThemeShadow (the shared
//!   WinUIThemeShadow, Translation.Z as in WinUI) on SecondaryItemsControlShadowWrapper, with the margin
//!   ThemeShadowFlyoutMargin as its room in the popup (Resources/ThemeShadows.xaml; DEVIATIONS.md,
//!   G-24). AvailableCommandsStates and DynamicOverflowStates are left
//!   out: FerroUI's CommandBar computes its visible and overflow items in code.
//! - FerroUI's CommandBar has no ContentTemplate, HorizontalContentAlignment and
//!   VerticalContentAlignment: the setters of the content alignments are left out and the content
//!   presenter is aligned left and top, their values. ClosedDisplayMode,
//!   ExitDisplayModeOnAccessKeyInvoked and CommandBarOverflowPresenterStyle (the overflow presenter
//!   has its theme in the template) have no counterpart. ItemWidthBottom and ItemWidthCollapsed,
//!   the widths FerroUI's CommandBar assumes for its items when it computes the dynamic overflow,
//!   are the Width of the buttons (68).
//! - The Grid.Resources of the template (the opening and closing storyboards of the overlay, the
//!   styles of SplitButton in a command bar and AppBarButtonHasFlyoutChevronVisibility) are left
//!   out with what uses them.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SolidColorBrushTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextAlignment;
use ferroui_base::media::TextLineBounds;
use ferroui_base::media::TextWrapping;
use ferroui_base::styling::Selectors;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::CommandBar;
use ferroui_controls::CommandBarButton;
use ferroui_controls::CommandBarDefaultLabelPosition;
use ferroui_controls::CommandBarSeparator;
use ferroui_controls::CommandBarToggleButton;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::PathIcon;
use ferroui_controls::PlacementMode;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // AppBarButton
    resources.control_theme("DefaultAppBarButtonStyle", default_app_bar_button_style);
    resources.control_theme("AppBarButtonOverflowStyle", app_bar_button_overflow_style);
    resources.control_theme(type_key::<CommandBarButton>(), command_bar_button_theme);
    // AppBarToggleButton
    resources.control_theme("DefaultAppBarToggleButtonStyle", default_app_bar_toggle_button_style);
    resources.control_theme("AppBarToggleButtonOverflowStyle", app_bar_toggle_button_overflow_style);
    resources.control_theme(type_key::<CommandBarToggleButton>(), command_bar_toggle_button_theme);
    // AppBarSeparator
    resources.control_theme("DefaultAppBarSeparatorStyle", default_app_bar_separator_style);
    resources.control_theme(type_key::<CommandBarSeparator>(), command_bar_separator_theme);
    // The More button of the command bar.
    resources.control_theme("EllipsisButton", ellipsis_button);
    // CommandBarOverflowPresenter: the ItemsControl PART_OverflowPresenter of the command bar.
    resources.control_theme(
        "DefaultCommandBarOverflowPresenterStyle",
        default_command_bar_overflow_presenter_style,
    );
    resources.control_theme("DefaultCommandBarStyle", default_command_bar_style);
    resources.control_theme(type_key::<CommandBar>(), command_bar_theme);
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

/// The focus adorner of `default_app_bar_button_style`.
fn default_app_bar_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `CommandBarButton` of `default_app_bar_button_style`.
fn default_app_bar_button_style_template(t: &Template<CommandBarButton>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("Root")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([
            t.make(Border::new())
                .name("AppBarButtonInnerBorder")?
                .dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    Border::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .set(
                    Animatable::transitions_property(),
                    Some({
                        let object = Transitions::new();
                        object.add(
                            t.make(SolidColorBrushTransition::new())
                                .set(
                                    TransitionBase::property_property(),
                                    Some(Border::background_property().property()),
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
                .control(),
            t.make(Grid::new())
                .name("ContentRoot")?
                .dynamic(Layoutable::min_height_property(), "AppBarThemeMinHeight")
                .with(|e| {
                    e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                .with(|e| {
                    e.set_row_definitions(literal(RowDefinitions::parse("Auto,Auto"))?);
                    Ok(())
                })?
                .children([
                    t.make(Viewbox::new())
                        .name("ContentViewbox")?
                        .dynamic(Layoutable::height_property(), "AppBarButtonContentHeight")
                        .dynamic(
                            Layoutable::margin_property(),
                            "AppBarButtonContentViewboxCollapsedMargin",
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .child(
                            Viewbox::child_property(),
                            t.make(ContentPresenter::new())
                                .name("Content")?
                                .template_bind(ContentPresenter::content_property(), CommandBarButton::icon_property())
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                // A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
                                .style(of::<PathIcon>(), |style| {
                                    style.set(Layoutable::width_property(), f64::NAN);
                                    style.set(Layoutable::height_property(), f64::NAN);
                                    style.static_resource(
                                        TemplatedControl::template_property(),
                                        "PathIconUnscaledTemplate",
                                    )?;
                                    Ok(())
                                })?,
                        )
                        .control(),
                    t.make(TextBlock::new())
                        .name("TextLabel")?
                        .set(Grid::row_property(), 1)
                        .template_bind(TextBlock::text_property(), CommandBarButton::label_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .dynamic(TextBlock::font_size_property(), "AppBarButtonLabelFontSize")
                        .template_bind(
                            TextBlock::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                        .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                        .dynamic(Layoutable::margin_property(), "AppBarButtonTextLabelMargin")
                        .control(),
                    t.make(TextBlock::new())
                        .name("OverflowTextLabel")?
                        .template_bind(TextBlock::text_property(), CommandBarButton::label_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .dynamic(TextBlock::font_size_property(), "ControlContentThemeFontSize")
                        .template_bind(
                            TextBlock::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .set(TextBlock::text_alignment_property(), TextAlignment::Left)
                        .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 12.0, 0.0))
                        .dynamic(TextBlock::padding_property(), "AppBarButtonOverflowTextLabelPadding")
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// `DefaultAppBarButtonStyle`: a control theme of `CommandBarButton`.
fn default_app_bar_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarButton>();
    theme.dynamic(TemplatedControl::background_property(), "AppBarButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "AppBarButtonBorderBrush");
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(Layoutable::width_property(), 68.0);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    // UseSystemFocusVisuals with FocusVisualMargin="-3".
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_app_bar_button_style_focus_adorner)),
    );
    theme.template(default_app_bar_button_style_template);
    // ApplicationViewStates: Compact
    theme.style(
        nesting().property_is(CommandBarButton::is_compact_property(), true),
        |style| {
            style.style(
                nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderCompactMargin");
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // LabelCollapsed
    theme.style(
        nesting().property_is(
            CommandBarButton::label_position_property(),
            CommandBarDefaultLabelPosition::Collapsed,
        ),
        |style| {
            style.style(
                nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin");
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight");
                Ok(())
            })?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // LabelOnRight (and the Width NaN of AppBarButton::CreateStoryboardForWidthAdjustmentsForLabelOnRightStyle)
    theme.style(
        nesting().property_is(
            CommandBarButton::label_position_property(),
            CommandBarDefaultLabelPosition::Right,
        ),
        |style| {
            style.set(Layoutable::width_property(), f64::NAN);
            style.style(
                nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin");
                    Ok(())
                },
            )?;
            // The icon and the label centred in the height of the button: the content grid, in the
            // middle of the root, which keeps the compact height, and the label bounded by its
            // capitals and its baseline (TextLineBounds Tight), so that its capitals are in the middle
            // whatever its font (WinUI places both 16 below the top, which lines them up with the
            // metrics of Segoe UI only). Deviation (winui-theme/DEVIATIONS.md, G-29).
            style.style(nesting().template().of::<Viewbox>().name("ContentViewbox"), |style| {
                style.set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 0.0, 0.0));
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("Root"), |style| {
                style.dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight");
                Ok(())
            })?;
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.set(Layoutable::min_height_property(), 0.0);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                Ok(())
            })?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), true);
                style.set(Grid::row_property(), 0);
                style.set(Grid::column_property(), 1);
                style.set(TextBlock::text_alignment_property(), TextAlignment::Left);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                style.set(TextBlock::text_line_bounds_property(), TextLineBounds::Tight);
                style.set(Layoutable::margin_property(), Thickness::new(8.0, 0.0, 12.0, 0.0));
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // Overflow (with the setters of AppBarButtonOverflowStyle)
    theme.style(
        nesting().property_is(CommandBarButton::is_in_overflow_property(), true),
        |style| {
            style.set(
                Layoutable::horizontal_alignment_property(),
                HorizontalAlignment::Stretch,
            );
            style.set(Layoutable::width_property(), f64::NAN);
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.set(Layoutable::min_height_property(), 0.0);
                Ok(())
            })?;
            style.style(
                nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderOverflowMargin");
                    Ok(())
                },
            )?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Viewbox>().name("ContentViewbox"),
                    nesting().template().of::<TextBlock>().name("TextLabel"),
                ]),
                |style| {
                    style.set(Visual::is_visible_property(), false);
                    Ok(())
                },
            )?;
            style.style(
                nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                |style| {
                    style.set(Visual::is_visible_property(), true);
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // CommonStates: PointerOver (also OverflowPointerOver)
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarButtonBackgroundPointerOver");
                style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushPointerOver");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarButtonForegroundPointerOver",
            );
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<TextBlock>().name("TextLabel"),
                nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            ]),
            |style| {
                style.dynamic(TextBlock::foreground_property(), "AppBarButtonForegroundPointerOver");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed (also OverflowPressed)
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarButtonBackgroundPressed");
                style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(ContentPresenter::foreground_property(), "AppBarButtonForegroundPressed");
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<TextBlock>().name("TextLabel"),
                nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            ]),
            |style| {
                style.dynamic(TextBlock::foreground_property(), "AppBarButtonForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Border>().name("AppBarButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarButtonBackgroundDisabled");
                style.dynamic(Border::border_brush_property(), "AppBarButtonBorderBrushDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarButtonForegroundDisabled",
            );
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<TextBlock>().name("TextLabel"),
                nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            ]),
            |style| {
                style.dynamic(TextBlock::foreground_property(), "AppBarButtonForegroundDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// `AppBarButtonOverflowStyle`: a control theme of `CommandBarButton`.
fn app_bar_button_overflow_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarButton>();
    theme.based_on("DefaultAppBarButtonStyle")?;
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::width_property(), f64::NAN);
    Ok(())
}

/// The default control theme of `CommandBarButton`.
fn command_bar_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarButton>();
    theme.based_on("DefaultAppBarButtonStyle")?;
    Ok(())
}

/// The focus adorner of `default_app_bar_toggle_button_style`.
fn default_app_bar_toggle_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `CommandBarToggleButton` of `default_app_bar_toggle_button_style`.
fn default_app_bar_toggle_button_style_template(t: &Template<CommandBarToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("Root")?
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([
            t.make(Border::new())
                .name("AppBarToggleButtonInnerBorder")?
                .dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    Border::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                )
                .set(
                    Animatable::transitions_property(),
                    Some({
                        let object = Transitions::new();
                        object.add(
                            t.make(SolidColorBrushTransition::new())
                                .set(
                                    TransitionBase::property_property(),
                                    Some(Border::background_property().property()),
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
                .control(),
            t.make(Grid::new())
                .name("ContentRoot")?
                .dynamic(Layoutable::min_height_property(), "AppBarThemeMinHeight")
                .with(|e| {
                    e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto"))?);
                    Ok(())
                })?
                .with(|e| {
                    e.set_row_definitions(literal(RowDefinitions::parse("Auto,Auto"))?);
                    Ok(())
                })?
                // E73E CheckMark of 16 px in a box of 14 x 14.
                .children([
                    t.make(Path::new())
                        .name("OverflowCheckGlyph")?
                        .static_resource(Path::data_property(), "GlyphGeometryE73E")?
                        .dynamic(Shape::fill_property(), "AppBarToggleButtonCheckGlyphForeground")
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 13.0)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .dynamic(Layoutable::margin_property(), "AppBarToggleButtonOverflowCheckMargin")
                        .set(Visual::opacity_property(), 0.0)
                        .set(Visual::is_visible_property(), false)
                        .control(),
                    t.make(Viewbox::new())
                        .name("ContentViewbox")?
                        .dynamic(Layoutable::height_property(), "AppBarButtonContentHeight")
                        .dynamic(
                            Layoutable::margin_property(),
                            "AppBarButtonContentViewboxCollapsedMargin",
                        )
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .child(
                            Viewbox::child_property(),
                            t.make(ContentPresenter::new())
                                .name("Content")?
                                .template_bind(
                                    ContentPresenter::content_property(),
                                    CommandBarToggleButton::icon_property(),
                                )
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                // A path icon as WinUI's, unscaled, so that the Viewbox scales its geometry as WinUI's does (PI-3).
                                .style(of::<PathIcon>(), |style| {
                                    style.set(Layoutable::width_property(), f64::NAN);
                                    style.set(Layoutable::height_property(), f64::NAN);
                                    style.static_resource(
                                        TemplatedControl::template_property(),
                                        "PathIconUnscaledTemplate",
                                    )?;
                                    Ok(())
                                })?,
                        )
                        .control(),
                    t.make(TextBlock::new())
                        .name("TextLabel")?
                        .set(Grid::row_property(), 1)
                        .template_bind(TextBlock::text_property(), CommandBarToggleButton::label_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .set(TextBlock::font_size_property(), 12.0)
                        .template_bind(
                            TextBlock::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                        .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                        .dynamic(Layoutable::margin_property(), "AppBarToggleButtonTextLabelMargin")
                        .control(),
                    t.make(TextBlock::new())
                        .name("OverflowTextLabel")?
                        .template_bind(TextBlock::text_property(), CommandBarToggleButton::label_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .dynamic(TextBlock::font_size_property(), "ControlContentThemeFontSize")
                        .template_bind(
                            TextBlock::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .set(TextBlock::text_alignment_property(), TextAlignment::Left)
                        .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::margin_property(), Thickness::new(38.0, 0.0, 12.0, 0.0))
                        .dynamic(
                            TextBlock::padding_property(),
                            "AppBarToggleButtonOverflowTextLabelPadding",
                        )
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// `DefaultAppBarToggleButtonStyle`: a control theme of `CommandBarToggleButton`.
fn default_app_bar_toggle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarToggleButton>();
    theme.dynamic(TemplatedControl::background_property(), "AppBarToggleButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarToggleButtonForeground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "AppBarToggleButtonBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "AppBarToggleButtonBorderThemeThickness",
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(Layoutable::width_property(), 68.0);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    // UseSystemFocusVisuals with FocusVisualMargin="-3".
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            default_app_bar_toggle_button_style_focus_adorner,
        )),
    );
    theme.template(default_app_bar_toggle_button_style_template);
    // ApplicationViewStates: Compact
    theme.style(
        nesting().property_is(CommandBarToggleButton::is_compact_property(), true),
        |style| {
            style.style(
                nesting()
                    .template()
                    .of::<Border>()
                    .name("AppBarToggleButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderCompactMargin");
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // LabelCollapsed
    theme.style(
        nesting().property_is(
            CommandBarToggleButton::label_position_property(),
            CommandBarDefaultLabelPosition::Collapsed,
        ),
        |style| {
            style.style(
                nesting()
                    .template()
                    .of::<Border>()
                    .name("AppBarToggleButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin");
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight");
                Ok(())
            })?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // LabelOnRight (and the Width NaN of the code's storyboard)
    theme.style(
        nesting().property_is(
            CommandBarToggleButton::label_position_property(),
            CommandBarDefaultLabelPosition::Right,
        ),
        |style| {
            style.set(Layoutable::width_property(), f64::NAN);
            style.style(
                nesting()
                    .template()
                    .of::<Border>()
                    .name("AppBarToggleButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderMargin");
                    Ok(())
                },
            )?;
            // The icon and the label centred in the height of the button (G-29, as the button).
            style.style(nesting().template().of::<Viewbox>().name("ContentViewbox"), |style| {
                style.set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 0.0, 0.0));
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                Ok(())
            })?;
            style.style(nesting().template().of::<Panel>().name("Root"), |style| {
                style.dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight");
                Ok(())
            })?;
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.set(Layoutable::min_height_property(), 0.0);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                Ok(())
            })?;
            style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                style.set(Visual::is_visible_property(), true);
                style.set(Grid::row_property(), 0);
                style.set(Grid::column_property(), 1);
                style.set(TextBlock::text_alignment_property(), TextAlignment::Left);
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
                style.set(TextBlock::text_line_bounds_property(), TextLineBounds::Tight);
                style.set(Layoutable::margin_property(), Thickness::new(8.0, 0.0, 12.0, 0.0));
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // Overflow (with the setters of AppBarToggleButtonOverflowStyle)
    theme.style(
        nesting().property_is(CommandBarToggleButton::is_in_overflow_property(), true),
        |style| {
            style.set(
                Layoutable::horizontal_alignment_property(),
                HorizontalAlignment::Stretch,
            );
            style.set(Layoutable::width_property(), f64::NAN);
            style.style(nesting().template().of::<Grid>().name("ContentRoot"), |style| {
                style.set(Layoutable::min_height_property(), 0.0);
                Ok(())
            })?;
            style.style(
                nesting()
                    .template()
                    .of::<Border>()
                    .name("AppBarToggleButtonInnerBorder"),
                |style| {
                    style.dynamic(Layoutable::margin_property(), "AppBarButtonInnerBorderOverflowMargin");
                    Ok(())
                },
            )?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Viewbox>().name("ContentViewbox"),
                    nesting().template().of::<TextBlock>().name("TextLabel"),
                ]),
                |style| {
                    style.set(Visual::is_visible_property(), false);
                    Ok(())
                },
            )?;
            style.style(
                Selectors::or([
                    nesting().template().of::<Path>().name("OverflowCheckGlyph"),
                    nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                ]),
                |style| {
                    style.set(Visual::is_visible_property(), true);
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // CommonStates: PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarToggleButtonBackgroundPointerOver");
                style.dynamic(
                    Border::border_brush_property(),
                    "AppBarToggleButtonBorderBrushPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "AppBarToggleButtonForegroundPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarToggleButtonBackgroundPressed");
                style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(TextBlock::foreground_property(), "AppBarToggleButtonForegroundPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundPressed",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarToggleButtonBackgroundDisabled");
                style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(TextBlock::foreground_property(), "AppBarToggleButtonForegroundDisabled");
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundDisabled",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Checked
    theme.style(nesting().class(":checked"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(Border::background_property(), "AppBarToggleButtonBackgroundChecked");
                style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushChecked");
                style.set(Border::background_sizing_property(), BackgroundSizing::OuterBorderEdge);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundChecked",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(TextBlock::foreground_property(), "AppBarToggleButtonForegroundChecked");
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // CheckedPointerOver
    theme.style(nesting().class(":checked").class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPointerOver",
                );
                style.dynamic(
                    Border::border_brush_property(),
                    "AppBarToggleButtonBorderBrushCheckedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundCheckedPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundCheckedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // CheckedPressed
    theme.style(nesting().class(":checked").class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "AppBarToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    Border::border_brush_property(),
                    "AppBarToggleButtonBorderBrushCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "AppBarToggleButtonForegroundCheckedPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundCheckedPressed",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // CheckedDisabled
    theme.style(nesting().class(":checked").class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("AppBarToggleButtonInnerBorder"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "AppBarToggleButtonBackgroundCheckedDisabled",
                );
                style.dynamic(
                    Border::border_brush_property(),
                    "AppBarToggleButtonBorderBrushCheckedDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarToggleButtonForegroundCheckedDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
            style.dynamic(
                TextBlock::foreground_property(),
                "AppBarToggleButtonForegroundCheckedDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "AppBarToggleButtonOverflowLabelForegroundCheckedDisabled",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
            |style| {
                style.dynamic(
                    TextBlock::foreground_property(),
                    "AppBarToggleButtonOverflowLabelForegroundCheckedDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The Overflow* states of the CommonStates group, which AppBarToggleButton::ChangeVisualState
    // chooses after the ones above for an enabled button in the overflow.
    theme.style(
        nesting().property_is(CommandBarToggleButton::is_in_overflow_property(), true),
        |style| {
            // OverflowPointerOver
            style.style(nesting().class(":pointerover"), |style| {
                style.style(
                    nesting()
                        .template()
                        .of::<Border>()
                        .name("AppBarToggleButtonInnerBorder"),
                    |style| {
                        style.dynamic(
                            Border::background_property(),
                            "AppBarToggleButtonBackgroundHighLightOverlayPointerOver",
                        );
                        style.dynamic(
                            Border::border_brush_property(),
                            "AppBarToggleButtonBorderBrushPointerOver",
                        );
                        Ok(())
                    },
                )?;
                style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "AppBarToggleButtonForegroundPointerOver",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                    style.dynamic(
                        TextBlock::foreground_property(),
                        "AppBarToggleButtonForegroundPointerOver",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
                    style.dynamic(
                        Shape::fill_property(),
                        "AppBarToggleButtonCheckGlyphForegroundPointerOver",
                    );
                    Ok(())
                })?;
                style.style(
                    nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                    |style| {
                        style.dynamic(
                            TextBlock::foreground_property(),
                            "AppBarToggleButtonOverflowLabelForegroundPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // OverflowPressed
            style.style(nesting().class(":pressed"), |style| {
                style.style(
                    nesting()
                        .template()
                        .of::<Border>()
                        .name("AppBarToggleButtonInnerBorder"),
                    |style| {
                        style.dynamic(
                            Border::background_property(),
                            "AppBarToggleButtonBackgroundHighLightOverlayPressed",
                        );
                        style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushPressed");
                        Ok(())
                    },
                )?;
                style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "AppBarToggleButtonForegroundPressed",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                    style.dynamic(TextBlock::foreground_property(), "AppBarToggleButtonForegroundPressed");
                    Ok(())
                })?;
                style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
                    style.dynamic(Shape::fill_property(), "AppBarToggleButtonCheckGlyphForegroundPressed");
                    Ok(())
                })?;
                style.style(
                    nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                    |style| {
                        style.dynamic(
                            TextBlock::foreground_property(),
                            "AppBarToggleButtonOverflowLabelForegroundPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // OverflowChecked: the background of the button, the foreground of an unchecked one
            style.style(
                nesting()
                    .class(":checked")
                    .not(of::<CommandBarToggleButton>().class(":disabled")),
                |style| {
                    style.style(
                        nesting()
                            .template()
                            .of::<Border>()
                            .name("AppBarToggleButtonInnerBorder"),
                        |style| {
                            style.template_bind(Border::background_property(), TemplatedControl::background_property());
                            style.template_bind(
                                Border::background_sizing_property(),
                                TemplatedControl::background_sizing_property(),
                            );
                            style.dynamic(Border::border_brush_property(), "AppBarToggleButtonBorderBrushChecked");
                            Ok(())
                        },
                    )?;
                    style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
                        style.dynamic(ContentPresenter::foreground_property(), "AppBarToggleButtonForeground");
                        Ok(())
                    })?;
                    style.style(
                        Selectors::or([
                            nesting().template().of::<TextBlock>().name("TextLabel"),
                            nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                        ]),
                        |style| {
                            style.dynamic(TextBlock::foreground_property(), "AppBarToggleButtonForeground");
                            Ok(())
                        },
                    )?;
                    style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
                        style.dynamic(Shape::fill_property(), "AppBarToggleButtonForeground");
                        Ok(())
                    })?;
                    Ok(())
                },
            )?;
            // OverflowCheckedPointerOver
            style.style(nesting().class(":checked").class(":pointerover"), |style| {
                style.style(
                    nesting()
                        .template()
                        .of::<Border>()
                        .name("AppBarToggleButtonInnerBorder"),
                    |style| {
                        style.dynamic(
                            Border::background_property(),
                            "AppBarToggleButtonBackgroundHighLightOverlayCheckedPointerOver",
                        );
                        Ok(())
                    },
                )?;
                style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "AppBarToggleButtonOverflowLabelForegroundCheckedPointerOver",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                    style.dynamic(
                        TextBlock::foreground_property(),
                        "AppBarToggleButtonForegroundCheckedPointerOver",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
                    style.dynamic(
                        Shape::fill_property(),
                        "AppBarToggleButtonOverflowLabelForegroundCheckedPointerOver",
                    );
                    Ok(())
                })?;
                style.style(
                    nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                    |style| {
                        style.dynamic(
                            TextBlock::foreground_property(),
                            "AppBarToggleButtonOverflowLabelForegroundCheckedPointerOver",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            // OverflowCheckedPressed
            style.style(nesting().class(":checked").class(":pressed"), |style| {
                style.style(
                    nesting()
                        .template()
                        .of::<Border>()
                        .name("AppBarToggleButtonInnerBorder"),
                    |style| {
                        style.dynamic(
                            Border::background_property(),
                            "AppBarToggleButtonBackgroundHighLightOverlayCheckedPressed",
                        );
                        style.dynamic(
                            Border::border_brush_property(),
                            "AppBarToggleButtonBorderBrushCheckedPressed",
                        );
                        Ok(())
                    },
                )?;
                style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "AppBarToggleButtonOverflowLabelForegroundCheckedPressed",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<TextBlock>().name("TextLabel"), |style| {
                    style.dynamic(
                        TextBlock::foreground_property(),
                        "AppBarToggleButtonForegroundCheckedPressed",
                    );
                    Ok(())
                })?;
                style.style(nesting().template().of::<Path>().name("OverflowCheckGlyph"), |style| {
                    style.dynamic(
                        Shape::fill_property(),
                        "AppBarToggleButtonOverflowLabelForegroundCheckedPressed",
                    );
                    Ok(())
                })?;
                style.style(
                    nesting().template().of::<TextBlock>().name("OverflowTextLabel"),
                    |style| {
                        style.dynamic(
                            TextBlock::foreground_property(),
                            "AppBarToggleButtonOverflowLabelForegroundCheckedPressed",
                        );
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
            Ok(())
        },
    )?;
    Ok(())
}

/// `AppBarToggleButtonOverflowStyle`: a control theme of `CommandBarToggleButton`.
fn app_bar_toggle_button_overflow_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarToggleButton>();
    theme.based_on("DefaultAppBarToggleButtonStyle")?;
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::width_property(), f64::NAN);
    Ok(())
}

/// The default control theme of `CommandBarToggleButton`.
fn command_bar_toggle_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarToggleButton>();
    theme.based_on("DefaultAppBarToggleButtonStyle")?;
    Ok(())
}

/// The template of `CommandBarSeparator` of `default_app_bar_separator_style`.
fn default_app_bar_separator_style_template(t: &Template<CommandBarSeparator>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("RootGrid")?
        .children([t
            .make(Rectangle::new())
            .name("SeparatorRectangle")?
            .static_resource(Rectangle::radius_x_property(), "AppBarSeparatorCornerRadius")?
            .static_resource(Rectangle::radius_y_property(), "AppBarSeparatorCornerRadius")?
            .static_resource(Layoutable::width_property(), "AppBarSeparatorWidth")?
            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
            .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
            .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
            .control()])
        .control())
}

/// `DefaultAppBarSeparatorStyle`: a control theme of `CommandBarSeparator`.
fn default_app_bar_separator_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarSeparator>();
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarSeparatorForeground");
    theme.dynamic(TemplatedControl::padding_property(), "AppBarSeparatorMargin");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.template(default_app_bar_separator_style_template);
    // ApplicationViewStates: Compact
    theme.style(
        nesting()
            .property_is(CommandBarSeparator::is_compact_property(), true)
            .template()
            .of::<Panel>()
            .name("RootGrid"),
        |style| {
            style.dynamic(Layoutable::height_property(), "AppBarThemeCompactHeight");
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
            Ok(())
        },
    )?;
    // Overflow
    theme.style(
        nesting()
            .property_is(CommandBarSeparator::is_in_overflow_property(), true)
            .template()
            .of::<Panel>()
            .name("RootGrid"),
        |style| {
            style.set(Layoutable::height_property(), f64::NAN);
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .property_is(CommandBarSeparator::is_in_overflow_property(), true)
            .template()
            .of::<Rectangle>()
            .name("SeparatorRectangle"),
        |style| {
            style.set(Layoutable::width_property(), f64::NAN);
            style.set(
                Layoutable::horizontal_alignment_property(),
                HorizontalAlignment::Stretch,
            );
            style.static_resource(Layoutable::height_property(), "AppBarOverflowSeparatorHeight")?;
            style.static_resource(Layoutable::margin_property(), "AppBarOverflowSeparatorMargin")?;
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `CommandBarSeparator`.
fn command_bar_separator_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarSeparator>();
    theme.based_on("DefaultAppBarSeparatorStyle")?;
    Ok(())
}

/// The template of `Button` of `ellipsis_button`.
fn ellipsis_button_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(
                ContentPresenter::background_property(),
                TemplatedControl::background_property(),
            )
            .template_bind(
                ContentPresenter::border_brush_property(),
                TemplatedControl::border_brush_property(),
            )
            .set(
                ContentPresenter::background_sizing_property(),
                BackgroundSizing::InnerBorderEdge,
            )
            .template_bind(
                ContentPresenter::border_thickness_property(),
                TemplatedControl::border_thickness_property(),
            )
            .template_bind(
                ContentPresenter::corner_radius_property(),
                TemplatedControl::corner_radius_property(),
            )
            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
            .template_bind(
                ContentPresenter::content_template_property(),
                ContentControl::content_template_property(),
            )
            .template_bind(
                ContentPresenter::foreground_property(),
                TemplatedControl::foreground_property(),
            )
            .static_resource(Layoutable::margin_property(), "AppBarEllipsisButtonInnerBorderMargin")?
            .set(ContentPresenter::padding_property(), Thickness::uniform(0.0))
            .template_bind(
                ContentPresenter::horizontal_content_alignment_property(),
                ContentControl::horizontal_content_alignment_property(),
            )
            .template_bind(
                ContentPresenter::vertical_content_alignment_property(),
                ContentControl::vertical_content_alignment_property(),
            )
            .control()])
        .control())
}

/// `EllipsisButton`: a control theme of `Button`.
fn ellipsis_button(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "AppBarButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "AppBarButtonBorderBrush");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::SemiBold);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(Layoutable::width_property(), "AppBarExpandButtonThemeWidth");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // FocusVisualMargin="-3": the focus adorner of DefaultButtonStyle.
    theme.template(ellipsis_button_template);
    // The foreground of the states is also the foreground of the button, which the ellipsis
    // glyph of the command bar follows.
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "AppBarButtonForegroundPointerOver",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "AppBarButtonForegroundPressed");
        Ok(())
    })?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "AppBarButtonForegroundDisabled",
        );
        Ok(())
    })?;
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "AppBarButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AppBarButtonBorderBrushPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(ContentPresenter::background_property(), "AppBarButtonBackgroundPressed");
            style.dynamic(ContentPresenter::foreground_property(), "AppBarButtonForegroundPressed");
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AppBarButtonBorderBrushPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "AppBarButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AppBarButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AppBarButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ItemsControl` of `default_command_bar_overflow_presenter_style`.
fn default_command_bar_overflow_presenter_style_template(t: &Template<ItemsControl>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .dynamic(
            Border::border_thickness_property(),
            "CommandBarOverflowPresenterBorderThickness",
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ScrollViewer::new())
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        )
                        .dynamic(Layoutable::margin_property(), "CommandBarOverflowPresenterMargin"),
                )?,
        )
        .control())
}

/// `DefaultCommandBarOverflowPresenterStyle`: a control theme of `ItemsControl`.
fn default_command_bar_overflow_presenter_style(theme: &StyleBuilder) -> Result {
    theme.target::<ItemsControl>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "CommandBarOverflowPresenterBackground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "CommandBarOverflowPresenterBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::padding_property(),
        "CommandBarOverflowPresenterBorderPadding",
    );
    theme.dynamic(Layoutable::max_width_property(), "CommandBarOverflowMaxWidth");
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.template(default_command_bar_overflow_presenter_style_template);
    Ok(())
}

/// The items panel of `template`.
fn template_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Horizontal)
        .end()
        .upcast())
}

/// The items panel of `template`.
fn template_items_panel_2(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new()).end().upcast())
}

/// The template of `CommandBar` of `default_command_bar_style`.
fn default_command_bar_style_template(t: &Template<CommandBar>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("LayoutRoot")?
        .children([
            t.make(Border::new())
                .name("ContentRoot")?
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            let items = Grid::column_definitions(e);
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), literal(GridLength::parse("*"))?)
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .static_resource(
                                        ColumnDefinition::min_width_property(),
                                        "AppBarMoreButtonColumnMinWidth",
                                    )?
                                    .end()
                                    .into(),
                            );
                            Ok(())
                        })?
                        .children([
                            t.make(Grid::new())
                                .with(|e| {
                                    e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto"))?);
                                    Ok(())
                                })?
                                .children([
                                    t.make(ContentPresenter::new())
                                        .name("PART_ContentPresenter")?
                                        .template_bind(
                                            Layoutable::margin_property(),
                                            TemplatedControl::padding_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            CommandBar::content_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::foreground_property(),
                                            TemplatedControl::foreground_property(),
                                        )
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .set(
                                            ContentPresenter::horizontal_content_alignment_property(),
                                            HorizontalAlignment::Left,
                                        )
                                        .set(
                                            ContentPresenter::vertical_content_alignment_property(),
                                            VerticalAlignment::Top,
                                        )
                                        .control(),
                                    t.make(ItemsControl::new())
                                        .name("PrimaryItemsControl")?
                                        .set(Grid::column_property(), 1)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                        .dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight")
                                        .set(InputElement::is_tab_stop_property(), false)
                                        .bind(
                                            ItemsControl::items_source_property(),
                                            path()
                                                .templated_parent()
                                                .class_member::<CommandBar>("VisiblePrimaryCommands")
                                                .binding(),
                                        )
                                        .set(
                                            ItemsControl::items_panel_property(),
                                            items_panel(t.scope(), template_items_panel),
                                        )
                                        .control(),
                                ])
                                .control(),
                            t.make(Button::new())
                                .name("PART_OverflowButton")?
                                .set(Grid::column_property(), 1)
                                .static_resource(StyledElement::theme_property(), "EllipsisButton")?
                                .template_bind(
                                    TemplatedControl::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .dynamic(Layoutable::min_height_property(), "AppBarThemeCompactHeight")
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                .template_bind(
                                    Visual::is_visible_property(),
                                    CommandBar::is_overflow_button_visible_property(),
                                )
                                // E712 More of 20 px, as high as AppBarExpandButtonCircleDiameter.
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .name("EllipsisIcon")?
                                        .static_resource(Path::data_property(), "GlyphGeometryE712")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 15.0)
                                        .dynamic(Layoutable::height_property(), "AppBarExpandButtonCircleDiameter")
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
                                )?
                                .control(),
                            t.make(Rectangle::new())
                                .name("HighContrastBorder")?
                                .set(Grid::column_span_property(), 2)
                                .set(Visual::is_visible_property(), false)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .dynamic(Shape::stroke_property(), "CommandBarHighContrastBorder")
                                .set(Shape::stroke_thickness_property(), 1.0)
                                .control(),
                            t.make(Border::new())
                                .name("OpenBorder")?
                                .set(Grid::column_span_property(), 2)
                                .set(Visual::is_visible_property(), false)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .dynamic(Border::border_thickness_property(), "CommandBarBorderThicknessOpen")
                                .dynamic(Border::border_brush_property(), "CommandBarBorderBrushOpen")
                                .template_bind(
                                    Border::corner_radius_property(),
                                    TemplatedControl::corner_radius_property(),
                                )
                                .control(),
                        ]),
                )
                .control(),
            t.make(Popup::new())
                .name("PART_OverflowPopup")?
                .template_bind_with(
                    Popup::is_open_property(),
                    Some(CommandBar::is_open_property().property()),
                    None,
                    Some(BindingMode::TwoWay),
                    None,
                )
                .bind(
                    Popup::placement_target_property(),
                    path().named(t.names(), "ContentRoot").binding(),
                )
                .set(Popup::placement_property(), PlacementMode::BottomEdgeAlignedRight)
                .set(Popup::is_light_dismiss_enabled_property(), true)
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .name("SecondaryItemsControlShadowWrapper")?
                        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                        .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?
                        .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                        .child(
                            Decorator::child_property(),
                            t.make(ItemsControl::new())
                                .name("PART_OverflowPresenter")?
                                .static_resource(
                                    StyledElement::theme_property(),
                                    "DefaultCommandBarOverflowPresenterStyle",
                                )?
                                .set(InputElement::is_tab_stop_property(), false)
                                .bind(
                                    ItemsControl::items_source_property(),
                                    path()
                                        .templated_parent()
                                        .class_member::<CommandBar>("OverflowItems")
                                        .binding(),
                                )
                                .set(
                                    ItemsControl::items_panel_property(),
                                    items_panel(t.scope(), template_items_panel_2),
                                ),
                        ),
                )
                .control(),
        ])
        .control())
}

/// `DefaultCommandBarStyle`: a control theme of `CommandBar`.
fn default_command_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBar>();
    theme.dynamic(TemplatedControl::background_property(), "CommandBarBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "CommandBarForeground");
    theme.set(TemplatedControl::padding_property(), Thickness::new(4.0, 0.0, 0.0, 0.0));
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // The widths FerroUI's CommandBar assumes for an item when it computes the dynamic overflow:
    // the Width 68 of DefaultAppBarButtonStyle with the labels at the bottom or collapsed.
    theme.set(CommandBar::item_width_bottom_property(), 68.0);
    theme.set(CommandBar::item_width_collapsed_property(), 68.0);
    theme.template(default_command_bar_style_template);
    // CommonStates: Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Path>()
            .name("EllipsisIcon"),
        |style| {
            style.dynamic(Shape::fill_property(), "CommandBarEllipsisIconForegroundDisabled");
            Ok(())
        },
    )?;
    // DisplayModeStates: CompactOpenDown and CompactOpenUp (the overflow is open)
    theme.style(nesting().property_is(CommandBar::is_open_property(), true), |style| {
        style.style(nesting().template().of::<Border>().name("ContentRoot"), |style| {
            style.dynamic(Border::background_property(), "CommandBarBackgroundOpen");
            style.template_bind(
                Border::corner_radius_property(),
                TemplatedControl::corner_radius_property(),
            );
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<Border>().name("OpenBorder"),
                nesting().template().of::<Rectangle>().name("HighContrastBorder"),
            ]),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Button>().name("PART_OverflowButton"),
            |style| {
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `CommandBar`.
fn command_bar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBar>();
    theme.based_on("DefaultCommandBarStyle")?;
    Ok(())
}
