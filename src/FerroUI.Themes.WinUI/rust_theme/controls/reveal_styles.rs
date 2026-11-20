//! `Controls/RevealStyles.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/Materials/Reveal/RevealBrush_themeresources.xaml (the styles ButtonRevealStyle,
//!   RepeatButtonRevealStyle and ToggleButtonRevealStyle; their resources are
//!   Resources/RevealBrush_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4; docs/porting/winui-materials.md):
//! - The visual states are pseudo-class styles: PointerOver is :pointerover, Pressed is :pressed,
//!   Disabled is :disabled, and the checked and indeterminate states of the toggle button combine
//!   :checked and :indeterminate with them, in the order of the source.
//! - The setters of RevealBrush.State (revealBrushPresent:Setter Target="RootGrid.(media:RevealBrush.State)")
//!   set the attached property RevealBrush.State of the border RootGrid (the Grid RootGrid of the
//!   source, which has a background and a corner radius), which gives it the Reveal hover and press
//!   lights; its background is the Reveal background brush, the border of the content presenter
//!   the Reveal border brush.
//! - The PointerUpThemeAnimation and PointerDownThemeAnimation of the states, ContentTransitions
//!   and UseSystemFocusVisuals have no counterpart; the focus visual with FocusVisualMargin="-3" is
//!   the FocusAdorner, as for Button.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FontWeight;
use ferroui_base::Thickness;
use ferroui_controls::media::RevealBrush;
use ferroui_controls::media::RevealBrushState;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::RepeatButton;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("ButtonRevealStyle", button_reveal_style);
    resources.control_theme("RepeatButtonRevealStyle", repeat_button_reveal_style);
    resources.control_theme("ToggleButtonRevealStyle", toggle_button_reveal_style);
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

/// The focus adorner of `button_reveal_style`.
fn button_reveal_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `Button` of `button_reveal_style`.
fn button_reveal_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                .set(ContentPresenter::recognizes_access_key_property(), true),
        )
        .control())
}

/// `ButtonRevealStyle`: a control theme of `Button`.
fn button_reveal_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "ButtonRevealBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ButtonRevealBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ButtonRevealBorderThemeThickness",
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
        Some(focus_adorner(theme.scope(), button_reveal_style_focus_adorner)),
    );
    theme.template(button_reveal_style_template);
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::PointerOver);
            style.dynamic(Border::background_property(), "ButtonRevealBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ButtonRevealBorderBrushPointerOver",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::Pressed);
            style.dynamic(Border::background_property(), "ButtonRevealBackgroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ButtonRevealBorderBrushPressed",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "ButtonRevealBackgroundDisabled");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ButtonRevealBorderBrushDisabled",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ButtonForegroundDisabled");
            Ok(())
        },
    )?;
    Ok(())
}

/// The focus adorner of `repeat_button_reveal_style`.
fn repeat_button_reveal_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `RepeatButton` of `repeat_button_reveal_style`.
fn repeat_button_reveal_style_template(t: &Template<RepeatButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                .set(ContentPresenter::recognizes_access_key_property(), true),
        )
        .control())
}

/// `RepeatButtonRevealStyle`: a control theme of `RepeatButton`.
fn repeat_button_reveal_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "RepeatButtonRevealBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "RepeatButtonForeground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "RepeatButtonRevealBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "RepeatButtonRevealBorderThemeThickness",
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
        Some(focus_adorner(theme.scope(), repeat_button_reveal_style_focus_adorner)),
    );
    theme.template(repeat_button_reveal_style_template);
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::PointerOver);
            style.dynamic(Border::background_property(), "RepeatButtonRevealBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "RepeatButtonRevealBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RepeatButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::Pressed);
            style.dynamic(Border::background_property(), "RepeatButtonRevealBackgroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "RepeatButtonRevealBorderBrushPressed",
            );
            style.dynamic(ContentPresenter::foreground_property(), "RepeatButtonForegroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "RepeatButtonRevealBackgroundDisabled");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "RepeatButtonRevealBorderBrushDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RepeatButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The focus adorner of `toggle_button_reveal_style`.
fn toggle_button_reveal_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `ToggleButton` of `toggle_button_reveal_style`.
fn toggle_button_reveal_style_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
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
                .set(ContentPresenter::recognizes_access_key_property(), true),
        )
        .control())
}

/// `ToggleButtonRevealStyle`: a control theme of `ToggleButton`.
fn toggle_button_reveal_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::background_property(), "ToggleButtonRevealBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ToggleButtonForeground");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "ToggleButtonRevealBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ToggleButtonRevealBorderThemeThickness",
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
        Some(focus_adorner(theme.scope(), toggle_button_reveal_style_focus_adorner)),
    );
    theme.template(toggle_button_reveal_style_template);
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::PointerOver);
            style.dynamic(Border::background_property(), "ToggleButtonRevealBackgroundPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::Pressed);
            style.dynamic(Border::background_property(), "ToggleButtonRevealBackgroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushPressed",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ToggleButtonForegroundPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":disabled").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "ToggleButtonRevealBackgroundDisabled");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":checked").template().of::<Border>().name("RootGrid"),
        |style| {
            style.dynamic(Border::background_property(), "ToggleButtonRevealBackgroundChecked");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushChecked",
            );
            style.dynamic(ContentPresenter::foreground_property(), "ToggleButtonForegroundChecked");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::PointerOver);
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushCheckedPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundCheckedPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::Pressed);
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushCheckedPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundCheckedPressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundCheckedDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":checked")
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushCheckedDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundCheckedDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundIndeterminate",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushIndeterminate",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundIndeterminate",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::PointerOver);
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundIndeterminatePointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushIndeterminatePointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundIndeterminatePointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.set(RevealBrush::state_property(), RevealBrushState::Pressed);
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundIndeterminatePressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushIndeterminatePressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundIndeterminatePressed",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("RootGrid"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ToggleButtonRevealBackgroundIndeterminateDisabled",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "ToggleButtonRevealBorderBrushIndeterminateDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "ToggleButtonForegroundIndeterminateDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}
