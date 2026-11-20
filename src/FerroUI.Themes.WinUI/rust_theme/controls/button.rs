//! `Controls/Button.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/Button_themeresources.xaml (the styles; its resources are
//!   Resources/Button_themeresources.xaml)
//!   dxaml/xcp/dxaml/themes/generic.xaml (TextBlockButtonStyle, of ButtonBase)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4): the AnimatedIcon.State setters of
//! the visual states have no counterpart (FerroUI has no AnimatedIcon); ContentTransitions has none
//! either. The focus visual of UseSystemFocusVisuals with FocusVisualMargin="-3" is the
//! FocusAdorner. RecognizesAccessKey is FerroUI's access key support (WinUI's AccessKey is a
//! property of the control and needs nothing in the template).
//! TextBlockButtonStyle (target type ButtonBase) is a keyed theme of Button, based on
//! DefaultButtonStyle; its ContentPresenter "Text" is PART_ContentPresenter and its Grid, which has
//! only a background and a margin, is a Panel.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SolidColorBrushTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::Thickness;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultButtonStyle", default_button_style);
    // WinUI applies the default style of a control beneath an explicit style, so the keyed
    // themes are based on the default theme (docs/porting/winui-theme.md, section 4.2).
    resources.control_theme("AccentButtonStyle", accent_button_style);
    resources.control_theme("SubtleButtonStyle", subtle_button_style);
    // TextBlockButtonStyle of generic.xaml: a button that looks like a hyperlink without an underline.
    resources.control_theme("TextBlockButtonStyle", text_block_button_style);
    resources.control_theme(type_key::<Button>(), button_theme);
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

/// The focus adorner of `default_button_style`.
fn default_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `Button` of `default_button_style`.
fn default_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .set(ContentPresenter::recognizes_access_key_property(), true)
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(SolidColorBrushTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(ContentPresenter::background_property().property()),
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
        .control())
}

/// `DefaultButtonStyle`: a control theme of `Button`.
fn default_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "ButtonBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "ButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonBorderThemeThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "ButtonPadding")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_button_style_focus_adorner)),
    );
    theme.template(default_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(ContentPresenter::background_property(), "ButtonBackgroundPointerOver");
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ButtonBorderBrushPointerOver",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPointerOver");
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
            style.dynamic(ContentPresenter::background_property(), "ButtonBackgroundPressed");
            style.dynamic(ContentPresenter::border_brush_property(), "ButtonBorderBrushPressed");
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPressed");
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
            style.dynamic(ContentPresenter::background_property(), "ButtonBackgroundDisabled");
            style.dynamic(ContentPresenter::border_brush_property(), "ButtonBorderBrushDisabled");
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundDisabled");
            Ok(())
        },
    )?;
    // The style class `accent` of the framework's controls (a FerroUI addition, DEVIATIONS.md,
    // BTN-6): a button with the class looks as one with AccentButtonStyle.
    theme.style(nesting().class("accent"), |style| {
        style.set(
            TemplatedControl::background_sizing_property(),
            BackgroundSizing::OuterBorderEdge,
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::background_property(), "AccentButtonBackground");
                style.dynamic(ContentPresenter::border_brush_property(), "AccentButtonBorderBrush");
                style.dynamic(ContentPresenter::foreground_property(), "AccentButtonForeground");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .class(":pointerover")
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "AccentButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AccentButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "AccentButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .class(":pressed")
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::background_property(), "AccentButtonBackgroundPressed");
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AccentButtonBorderBrushPressed",
                );
                style.dynamic(ContentPresenter::foreground_property(), "AccentButtonForegroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .class(":disabled")
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "AccentButtonBackgroundDisabled",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "AccentButtonBorderBrushDisabled",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "AccentButtonForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `Button` of `accent_button_style`.
fn accent_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::foreground_property(),
            TemplatedControl::foreground_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .set(ContentPresenter::recognizes_access_key_property(), true)
        .set(
            Animatable::transitions_property(),
            Some({
                let object = Transitions::new();
                object.add(
                    t.make(SolidColorBrushTransition::new())
                        .set(
                            TransitionBase::property_property(),
                            Some(ContentPresenter::background_property().property()),
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
        .control())
}

/// `AccentButtonStyle`: a control theme of `Button`.
fn accent_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::foreground_property(), "AccentButtonForeground");
    theme.dynamic(TemplatedControl::background_property(), "AccentButtonBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::OuterBorderEdge,
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "AccentButtonBorderBrush");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(accent_button_style_template);
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
                "AccentButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AccentButtonBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AccentButtonForegroundPointerOver",
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
            style.dynamic(ContentPresenter::background_property(), "AccentButtonBackgroundPressed");
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AccentButtonBorderBrushPressed",
            );
            style.dynamic(ContentPresenter::foreground_property(), "AccentButtonForegroundPressed");
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
                "AccentButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "AccentButtonBorderBrushDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "AccentButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// `SubtleButtonStyle`: a control theme of `Button`.
fn subtle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "SubtleButtonBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "SubtleButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "SubtleButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonBorderThemeThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "ButtonPadding")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // The template of the source is the one of DefaultButtonStyle, which this theme is based on.
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
                "SubtleButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "SubtleButtonBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "SubtleButtonForegroundPointerOver",
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
            style.dynamic(ContentPresenter::background_property(), "SubtleButtonBackgroundPressed");
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "SubtleButtonBorderBrushPressed",
            );
            style.dynamic(ContentPresenter::foreground_property(), "SubtleButtonForegroundPressed");
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
                "SubtleButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "SubtleButtonBorderBrushDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "SubtleButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `Button` of `text_block_button_style`.
fn text_block_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
            .template_bind(
                ContentPresenter::content_template_property(),
                ContentControl::content_template_property(),
            )
            .template_bind(
                Layoutable::horizontal_alignment_property(),
                ContentControl::horizontal_content_alignment_property(),
            )
            .template_bind(
                Layoutable::vertical_alignment_property(),
                ContentControl::vertical_content_alignment_property(),
            )
            .set(ContentPresenter::recognizes_access_key_property(), true)
            .control()])
        .control())
}

/// `TextBlockButtonStyle`: a control theme of `Button`.
fn text_block_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(TemplatedControl::background_property(), "HyperlinkButtonBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "HyperlinkButtonForeground");
    theme.set(Layoutable::min_width_property(), 0.0);
    theme.set(Layoutable::min_height_property(), 0.0);
    theme.template(text_block_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "HyperlinkButtonForegroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::background_property(),
                "HyperlinkButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "HyperlinkButtonBorderBrushPointerOver",
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
            style.dynamic(
                ContentPresenter::foreground_property(),
                "HyperlinkButtonForegroundPressed",
            );
            style.dynamic(
                ContentPresenter::background_property(),
                "HyperlinkButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "HyperlinkButtonBorderBrushPressed",
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
                ContentPresenter::foreground_property(),
                "HyperlinkButtonForegroundDisabled",
            );
            style.dynamic(
                ContentPresenter::background_property(),
                "HyperlinkButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "HyperlinkButtonBorderBrushDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `Button`.
fn button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    Ok(())
}
