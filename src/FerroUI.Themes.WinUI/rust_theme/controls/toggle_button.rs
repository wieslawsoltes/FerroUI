//! `Controls/ToggleButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ToggleButton_themeresources.xaml (the styles; its resources are
//!   Resources/ToggleButton_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes as Button.xaml. The visual states of the one group CommonStates combine the
//! pointer state and the check state (CheckedPointerOver); each is a nested style on the
//! pseudo-classes of both (^:checked:pointerover), in the order of the source, so that the
//! combined state is the last to match.

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
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultToggleButtonStyle", default_toggle_button_style);
    resources.control_theme(type_key::<ToggleButton>(), toggle_button_theme);
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

/// The focus adorner of `default_toggle_button_style`.
fn default_toggle_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `ToggleButton` of `default_toggle_button_style`.
fn default_toggle_button_style_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
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

/// `DefaultToggleButtonStyle`: a control theme of `ToggleButton`.
fn default_toggle_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
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
    theme.dynamic(TemplatedControl::background_property(), "ToggleButtonBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "ToggleButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ToggleButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ToggleButtonBorderThemeThickness",
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
        Some(focus_adorner(theme.scope(), default_toggle_button_style_focus_adorner)),
    );
    theme.template(default_toggle_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundPointerOver",
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
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::background_property(), "ToggleButtonBackgroundPressed");
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushPressed",
                );
                style.dynamic(ContentPresenter::foreground_property(), "ToggleButtonForegroundPressed");
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
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundDisabled",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundDisabled",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushDisabled",
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
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::background_property(), "ToggleButtonBackgroundChecked");
                style.dynamic(ContentPresenter::foreground_property(), "ToggleButtonForegroundChecked");
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushChecked",
                );
                style.dynamic(
                    ContentPresenter::background_sizing_property(),
                    "ToggleButtonCheckedStateBackgroundSizing",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // CheckedPointerOver
    theme.style(nesting().class(":checked").class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundCheckedPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushCheckedPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundCheckedPointerOver",
                );
                style.dynamic(
                    ContentPresenter::background_sizing_property(),
                    "ToggleButtonCheckedStateBackgroundSizing",
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
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundCheckedPressed",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundCheckedPressed",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushCheckedPressed",
                );
                style.dynamic(
                    ContentPresenter::background_sizing_property(),
                    "ToggleButtonCheckedStateBackgroundSizing",
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
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundCheckedDisabled",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundCheckedDisabled",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushCheckedDisabled",
                );
                // CheckedDisabled does not set BackgroundSizing: the value of the control, not the one of Checked.
                style.template_bind(
                    ContentPresenter::background_sizing_property(),
                    TemplatedControl::background_sizing_property(),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Indeterminate
    theme.style(nesting().class(":indeterminate"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundIndeterminate",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundIndeterminate",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushIndeterminate",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // IndeterminatePointerOver
    theme.style(nesting().class(":indeterminate").class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundIndeterminatePointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushIndeterminatePointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundIndeterminatePointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // IndeterminatePressed
    theme.style(nesting().class(":indeterminate").class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundIndeterminatePressed",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushIndeterminatePressed",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundIndeterminatePressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // IndeterminateDisabled
    theme.style(nesting().class(":indeterminate").class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "ToggleButtonBackgroundIndeterminateDisabled",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleButtonForegroundIndeterminateDisabled",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "ToggleButtonBorderBrushIndeterminateDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ToggleButton`.
fn toggle_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.based_on("DefaultToggleButtonStyle")?;
    Ok(())
}
