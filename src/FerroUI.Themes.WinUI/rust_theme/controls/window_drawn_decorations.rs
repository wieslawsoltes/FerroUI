//! `Controls/WindowDrawnDecorations.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TitleBar/TitleBar.xaml (DefaultTitleBarStyle and the implicit TitleBar style)
//!   controls/dev/TitleBar/TitleBar_themeresources.xaml (TitleBarBackButtonStyle,
//!   TitleBarPaneToggleButtonStyle; its resources are Resources/TitleBar_themeresources.xaml)
//!   dxaml/xcp/dxaml/themes/generic.xaml (WindowCaptionButton, WindowChromeStyle; the resources
//!   of the caption buttons are in Resources/Generic.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WindowDrawnDecorations is the frame FerroUI draws for a window whose decorations the platform
//! does not draw: the frame, the title bar and the caption buttons. WinUI 3 draws the title bar
//! with its TitleBar control and leaves the frame and the caption buttons to the system
//! (AppWindowTitleBar); the caption buttons of the XAML of WinUI are the WindowCaptionButton style
//! of generic.xaml. The template is the one FerroUI's control requires (the parts, the layers and
//! the pseudo-classes of src/FerroUI.Themes.Fluent/Controls/WindowDrawnDecorations.xaml) with the
//! look of those sources:
//! - The title bar is DefaultTitleBarStyle at TitleBarCompactHeight (32): the title in
//!   CaptionTextBlockStyle and TitleBarForegroundBrush, after TitleBarLeftPaddingWidth (2) and
//!   TitleBarLeftHeaderPaddingWidth (14), with TitleBarTitleMargin (0,0,8,0). Its background is the
//!   background of the window (SolidBackgroundFillColorBaseBrush, the Mica fallback of Window.xaml),
//!   which WinUI's transparent title bar shows. The back button, the pane toggle button, the
//!   headers, the icon, the subtitle and the content of TitleBar have no counterpart in
//!   WindowDrawnDecorations; TitleBarBackButtonStyle and TitleBarPaneToggleButtonStyle are ported as
//!   keyed themes of Button for applications. The Deactivated states (the window is not active) are
//!   left out: the decorations have no pseudo-class of the activation of the window.
//! - The caption buttons are WindowCaptionButton: 40 x 28 at the top of the title bar, the states
//!   Normal, PointerOver and Pressed as pseudo-classes. The close button takes the CloseButton*
//!   resources of generic.xaml on pointer over and press. The glyph of a button is the Path of the
//!   source drawn with the outline of the glyph of the symbol font the system draws on its caption
//!   buttons (GlyphGeometryE921 ChromeMinimize, E922 ChromeMaximize, E923 ChromeRestore, E8BB
//!   ChromeClose, E740 FullScreen, E73F BackToWindow; Resources/Glyphs.xaml) at 10 px, filled in the
//!   foreground of the button. MinMaxStates (WindowStateNormal, WindowStateMaximized) is the
//!   pseudo-class :maximized of the decorations, which shows ChromeRestore.
//! - The frame (1 px, SurfaceStrokeColorDefaultBrush) and the shadow (the shadow of the Fluent theme
//!   of FerroUI, WindowDrawnDecorationsWindowBorderBoxShadow) stand for the frame and the shadow the
//!   system draws around a WinUI window.
//! AnimatedIcon (AnimatedBackVisualSource, AnimatedGlobalNavigationButtonVisualSource) of the title
//! bar buttons is the fallback glyph (GlyphGeometryE72B Back, GlyphGeometryE700
//! GlobalNavigationButton); x:DeferLoadStrategy, AutomationProperties.AccessibilityView,
//! ToolTipService.ToolTip and ContentTransitions are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::WindowDecorationsElementRole;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BoxShadows;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextTrimming;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::chrome::WindowDecorationProperties;
use ferroui_controls::chrome::WindowDrawnDecorations;
use ferroui_controls::chrome::WindowDrawnDecorationsContent;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::decorations_template;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The shadow of a window with drawn decorations: the value of the Fluent theme of FerroUI.
    resources.deferred("WindowDrawnDecorationsWindowBorderBoxShadow", |_| {
        Ok(value(literal(BoxShadows::parse("0 2 10 2 #80000000"))?))
    });
    // WindowCaptionButton (generic.xaml): a caption button. The glyph is the content of the button
    // (a Path filled with the foreground of the button); the states set the background of the
    // template and the foreground of the button, which the source sets on the Path.
    resources.control_theme("WindowCaptionButton", window_caption_button);
    // The close button: CloseButtonPointerOver and CloseButtonPressed of WindowCaptionButton, with
    // the CloseButton* resources of generic.xaml.
    resources.control_theme(
        "WindowDrawnDecorationsCloseButton",
        window_drawn_decorations_close_button,
    );
    // TitleBarBackButtonStyle (TitleBar_themeresources.xaml). The Content of the source (E72B) is the
    // glyph of the template.
    resources.control_theme("TitleBarBackButtonStyle", title_bar_back_button_style);
    // TitleBarPaneToggleButtonStyle (TitleBar_themeresources.xaml). The Content of the source (E700)
    // is the glyph of the template.
    resources.control_theme("TitleBarPaneToggleButtonStyle", title_bar_pane_toggle_button_style);
    // WindowChromeStyle (generic.xaml): the style WinUI's window chrome applies to the content of
    // a window with a custom title bar. ClientAreaPresenter is FerroUI's PART_ContentPresenter.
    resources.control_theme("WindowChromeStyle", window_chrome_style);
    // DefaultTitleBarStyle (TitleBar.xaml), as the drawn decorations of a window.
    resources.control_theme("DefaultTitleBarStyle", default_title_bar_style);
    resources.control_theme(type_key::<WindowDrawnDecorations>(), window_drawn_decorations_theme);
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

/// The template of `Button` of `window_caption_button`.
fn window_caption_button_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .dynamic(Border::background_property(), "WindowCaptionButtonBackground")
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
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
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center),
        )
        .control())
}

/// `WindowCaptionButton`: a control theme of `Button`.
fn window_caption_button(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.dynamic(TemplatedControl::background_property(), "WindowCaptionBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "WindowCaptionForeground");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(Layoutable::width_property(), 40.0);
    theme.set(Layoutable::height_property(), 28.0);
    theme.set(
        WindowDecorationProperties::element_role_property(),
        WindowDecorationsElementRole::DecorationsElement,
    );
    theme.template(window_caption_button_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "WindowCaptionForeground");
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("LayoutRoot"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "WindowCaptionButtonBackgroundPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "WindowCaptionForegroundDisabled",
        );
        Ok(())
    })?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("LayoutRoot"),
        |style| {
            style.dynamic(Border::background_property(), "WindowCaptionButtonBackgroundPressed");
            Ok(())
        },
    )?;
    // Not a state of the source: a caption button of an action the window does not allow.
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "WindowCaptionForegroundDisabled",
        );
        Ok(())
    })?;
    Ok(())
}

/// `WindowDrawnDecorationsCloseButton`: a control theme of `Button`.
fn window_drawn_decorations_close_button(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("WindowCaptionButton")?;
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "CloseButtonStrokePointerOver");
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("LayoutRoot"),
        |style| {
            style.dynamic(Border::background_property(), "CloseButtonBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "CloseButtonStrokePressed");
        Ok(())
    })?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("LayoutRoot"),
        |style| {
            style.dynamic(Border::background_property(), "CloseButtonBackgroundPressed");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `title_bar_back_button_style`.
fn title_bar_back_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // The fallback glyph of the AnimatedIcon (16 x 16) at the size of its ink.
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Content")?
                .static_resource(Path::data_property(), "GlyphGeometryE72B")?
                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 14.0)
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `TitleBarBackButtonStyle`: a control theme of `Button`.
fn title_bar_back_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "TitleBarBackButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "TitleBarBackButtonForeground");
    theme.dynamic(Layoutable::width_property(), "TitleBarBackButtonWidth");
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(Layoutable::margin_property(), Thickness::uniform(2.0));
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(title_bar_back_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "TitleBarBackButtonBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarBackButtonForegroundPointerOver");
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "TitleBarBackButtonBackgroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarBackButtonForegroundPressed");
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting().class(":disabled").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "TitleBarBackButtonBackgroundDisabled");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarBackButtonForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `title_bar_pane_toggle_button_style`.
fn title_bar_pane_toggle_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // The fallback glyph of the AnimatedIcon (16 x 16) at the size of its ink.
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Content")?
                .static_resource(Path::data_property(), "GlyphGeometryE700")?
                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 14.0)
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `TitleBarPaneToggleButtonStyle`: a control theme of `Button`.
fn title_bar_pane_toggle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "TitleBarPaneToggleButtonBackground",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "TitleBarPaneToggleButtonForeground",
    );
    theme.dynamic(Layoutable::width_property(), "TitleBarPaneToggleButtonWidth");
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(Layoutable::margin_property(), Thickness::uniform(2.0));
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(title_bar_pane_toggle_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TitleBarPaneToggleButtonBackgroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pointerover").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarPaneToggleButtonForegroundPointerOver");
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TitleBarPaneToggleButtonBackgroundPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarPaneToggleButtonForegroundPressed");
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting().class(":disabled").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TitleBarPaneToggleButtonBackgroundDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Path>().name("Content"),
        |style| {
            style.dynamic(Shape::fill_property(), "TitleBarPaneToggleButtonForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ContentControl` of `window_chrome_style`.
fn window_chrome_style_template(t: &Template<ContentControl>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .control())
}

/// `WindowChromeStyle`: a control theme of `ContentControl`.
fn window_chrome_style(theme: &StyleBuilder) -> Result {
    theme.target::<ContentControl>();
    theme.template(window_chrome_style_template);
    Ok(())
}

/// The decorations of `default_title_bar_style`.
fn default_title_bar_style_decorations(
    t: &Template<WindowDrawnDecorations>,
) -> Result<Ref<WindowDrawnDecorationsContent>> {
    Ok(t.make(WindowDrawnDecorationsContent::new())
        .with(|e| {
            e.set_underlay(
                t.make(Panel::new())
                    .name("PART_UnderlayWrapper")?
                    // PART_LayoutRoot of TitleBar: the drag area and the background of the title bar.
                    .children([
                        t.make(Border::new())
                            .name("PART_WindowBorder")?
                            .dynamic(Border::background_property(), "SystemControlTransparentBrush")
                            .template_bind(
                                Border::border_thickness_property(),
                                WindowDrawnDecorations::frame_thickness_property(),
                            )
                            .dynamic(Border::border_brush_property(), "SurfaceStrokeColorDefaultBrush")
                            .set(InputElement::is_hit_test_visible_property(), false)
                            .control(),
                        t.make(Panel::new())
                            .name("PART_TitleBar")?
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                            .template_bind(
                                Layoutable::height_property(),
                                WindowDrawnDecorations::title_bar_height_property(),
                            )
                            .dynamic(Panel::background_property(), "SolidBackgroundFillColorBaseBrush")
                            .template_bind(
                                Visual::is_visible_property(),
                                WindowDrawnDecorations::has_title_bar_property(),
                            )
                            .set(
                                WindowDecorationProperties::element_role_property(),
                                WindowDecorationsElementRole::TitleBar,
                            )
                            .control(),
                    ])
                    .end(),
            );
            Ok(())
        })?
        .with(|e| {
            e.set_overlay(
                t.make(Panel::new())
                    .name("PART_OverlayWrapper")?
                    // PART_TitleText of TitleBar, above the content of the window. Its left margin is
                    // TitleBarLeftPaddingWidth + TitleBarLeftHeaderPaddingWidth, its right margin
                    // TitleBarTitleMargin.
                    .children([
                        t.make(Panel::new())
                            .name("PART_TitleTextPanel")?
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                            .template_bind(
                                Layoutable::height_property(),
                                WindowDrawnDecorations::title_bar_height_property(),
                            )
                            .set(InputElement::is_hit_test_visible_property(), false)
                            .template_bind(
                                Visual::is_visible_property(),
                                WindowDrawnDecorations::has_title_bar_property(),
                            )
                            .children([t
                                .make(TextBlock::new())
                                .name("PART_TitleText")?
                                .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                                .template_bind(TextBlock::text_property(), WindowDrawnDecorations::title_property())
                                .dynamic(TextBlock::foreground_property(), "TitleBarForegroundBrush")
                                .set(Layoutable::margin_property(), Thickness::new(16.0, 0.0, 8.0, 0.0))
                                .set(
                                    TextBlock::text_trimming_property(),
                                    <dyn TextTrimming>::character_ellipsis(),
                                )
                                .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .control()])
                            .control(),
                        t.make(StackPanel::new())
                            .name("PART_OverlayPanel")?
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                            .template_bind(
                                Layoutable::height_property(),
                                WindowDrawnDecorations::title_bar_height_property(),
                            )
                            .template_bind(
                                Visual::is_visible_property(),
                                WindowDrawnDecorations::has_title_bar_property(),
                            )
                            .set(StackPanel::orientation_property(), Orientation::Horizontal)
                            .children([
                                t.make(Button::new())
                                    .name("PART_FullScreenButton")?
                                    .static_resource(StyledElement::theme_property(), "WindowCaptionButton")?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::FullScreenButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryE740")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                                t.make(Button::new())
                                    .name("PART_MinimizeButton")?
                                    .static_resource(StyledElement::theme_property(), "WindowCaptionButton")?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::MinimizeButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryE921")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                                t.make(Button::new())
                                    .name("PART_MaximizeButton")?
                                    .static_resource(StyledElement::theme_property(), "WindowCaptionButton")?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::MaximizeButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .name("RestoreButtonPath")?
                                            .static_resource(Path::data_property(), "GlyphGeometryE922")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                                t.make(Button::new())
                                    .name("PART_CloseButton")?
                                    .static_resource(
                                        StyledElement::theme_property(),
                                        "WindowDrawnDecorationsCloseButton",
                                    )?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::CloseButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryE8BB")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                            ])
                            .control(),
                    ])
                    .end(),
            );
            Ok(())
        })?
        .with(|e| {
            e.set_fullscreen_popover(
                t.make(Panel::new())
                    .template_bind(
                        Layoutable::height_property(),
                        WindowDrawnDecorations::default_title_bar_height_property(),
                    )
                    .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                    .dynamic(Panel::background_property(), "SolidBackgroundFillColorBaseBrush")
                    .set(
                        WindowDecorationProperties::element_role_property(),
                        WindowDecorationsElementRole::DecorationsElement,
                    )
                    .children([
                        t.make(TextBlock::new())
                            .name("PART_PopoverTitleText")?
                            .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                            .template_bind(TextBlock::text_property(), WindowDrawnDecorations::title_property())
                            .dynamic(TextBlock::foreground_property(), "TitleBarForegroundBrush")
                            .set(Layoutable::margin_property(), Thickness::new(16.0, 0.0, 8.0, 0.0))
                            .set(
                                TextBlock::text_trimming_property(),
                                <dyn TextTrimming>::character_ellipsis(),
                            )
                            .set(TextBlock::text_wrapping_property(), TextWrapping::NoWrap)
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .control(),
                        t.make(StackPanel::new())
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                            .set(StackPanel::orientation_property(), Orientation::Horizontal)
                            .children([
                                t.make(Button::new())
                                    .name("PART_PopoverFullScreenButton")?
                                    .static_resource(StyledElement::theme_property(), "WindowCaptionButton")?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::FullScreenButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryE73F")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                                t.make(Button::new())
                                    .name("PART_PopoverCloseButton")?
                                    .static_resource(
                                        StyledElement::theme_property(),
                                        "WindowDrawnDecorationsCloseButton",
                                    )?
                                    .set(
                                        WindowDecorationProperties::element_role_property(),
                                        WindowDecorationsElementRole::CloseButton,
                                    )
                                    .content(
                                        ContentControl::content_property(),
                                        t.make(Path::new())
                                            .static_resource(Path::data_property(), "GlyphGeometryE8BB")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(Layoutable::width_property(), 10.0),
                                    )?
                                    .control(),
                            ])
                            .control(),
                    ])
                    .end(),
            );
            Ok(())
        })?
        .end())
}

/// `DefaultTitleBarStyle`: a control theme of `WindowDrawnDecorations`.
fn default_title_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<WindowDrawnDecorations>();
    // TitleBarCompactHeight.
    theme.set(WindowDrawnDecorations::default_title_bar_height_property(), 32.0);
    theme.set(
        WindowDrawnDecorations::default_frame_thickness_property(),
        Thickness::uniform(1.0),
    );
    theme.set(
        WindowDrawnDecorations::default_shadow_thickness_property(),
        Thickness::uniform(8.0),
    );
    theme.set(
        WindowDrawnDecorations::template_property(),
        Some(decorations_template(theme.scope(), default_title_bar_style_decorations)),
    );
    // The shadow: the layers are inset by the shadow and the frame draws the shadow.
    theme.style(
        nesting()
            .class(":has-shadow")
            .template()
            .of::<Panel>()
            .name("PART_UnderlayWrapper"),
        |style| {
            style.template_bind(
                Layoutable::margin_property(),
                WindowDrawnDecorations::shadow_thickness_property(),
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":has-shadow")
            .template()
            .of::<Border>()
            .name("PART_WindowBorder"),
        |style| {
            style.dynamic(
                Border::box_shadow_property(),
                "WindowDrawnDecorationsWindowBorderBoxShadow",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":has-shadow")
            .template()
            .of::<Panel>()
            .name("PART_OverlayWrapper"),
        |style| {
            style.template_bind(
                Layoutable::margin_property(),
                WindowDrawnDecorations::shadow_thickness_property(),
            );
            Ok(())
        },
    )?;
    // The frame: the title bar and the caption buttons inside it.
    theme.style(
        nesting()
            .class(":has-border")
            .template()
            .of::<Panel>()
            .name("PART_TitleTextPanel"),
        |style| {
            style.set(Layoutable::margin_property(), Thickness::new(1.0, 1.0, 1.0, 0.0));
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":has-border")
            .template()
            .of::<Panel>()
            .name("PART_TitleBar"),
        |style| {
            style.set(Layoutable::margin_property(), Thickness::new(1.0, 1.0, 1.0, 0.0));
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":has-border")
            .template()
            .of::<StackPanel>()
            .name("PART_OverlayPanel"),
        |style| {
            style.set(Layoutable::margin_property(), Thickness::new(0.0, 1.0, 1.0, 0.0));
            Ok(())
        },
    )?;
    // WindowStateMaximized of MinMaxStates: the restore glyph.
    theme.style(
        nesting()
            .class(":maximized")
            .template()
            .of::<Path>()
            .name("RestoreButtonPath"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryE923")?;
            Ok(())
        },
    )?;
    // The elements of the title bar the platform does not support or the hints of the
    // decorations (TitleBarDecorationHints) do not ask for.
    theme.style(
        nesting()
            .not(class(":has-minimize"))
            .template()
            .of::<Button>()
            .name("PART_MinimizeButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-maximize"))
            .template()
            .of::<Button>()
            .name("PART_MaximizeButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-fullscreen"))
            .template()
            .of::<Button>()
            .name("PART_FullScreenButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-fullscreen"))
            .template()
            .of::<Button>()
            .name("PART_PopoverFullScreenButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-close"))
            .template()
            .of::<Button>()
            .name("PART_CloseButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-close"))
            .template()
            .of::<Button>()
            .name("PART_PopoverCloseButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-title"))
            .template()
            .of::<Panel>()
            .name("PART_TitleTextPanel"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .not(class(":has-title"))
            .template()
            .of::<TextBlock>()
            .name("PART_PopoverTitleText"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // Full screen: the popover replaces the title bar.
    theme.style(
        nesting()
            .class(":fullscreen")
            .template()
            .of::<Panel>()
            .name("PART_TitleTextPanel"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":fullscreen")
            .template()
            .of::<StackPanel>()
            .name("PART_OverlayPanel"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":fullscreen")
            .template()
            .of::<Panel>()
            .name("PART_TitleBar"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `WindowDrawnDecorations`.
fn window_drawn_decorations_theme(theme: &StyleBuilder) -> Result {
    theme.target::<WindowDrawnDecorations>();
    theme.based_on("DefaultTitleBarStyle")?;
    Ok(())
}
