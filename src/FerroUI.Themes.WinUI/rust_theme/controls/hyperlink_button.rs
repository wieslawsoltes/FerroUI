//! `Controls/HyperlinkButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/HyperlinkButton_themeresources.xaml (the styles; its resources are
//!   Resources/HyperlinkButton_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes as Button.xaml. WinUI draws a visited hyperlink button as any other, so the
//! :visited pseudo-class of FerroUI's HyperlinkButton has no style here.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SolidColorBrushTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Thickness;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::HyperlinkButton;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultHyperlinkButtonStyle", default_hyperlink_button_style);
    resources.control_theme(type_key::<HyperlinkButton>(), hyperlink_button_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `default_hyperlink_button_style`.
fn default_hyperlink_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `HyperlinkButton` of `default_hyperlink_button_style`.
fn default_hyperlink_button_style_template(t: &Template<HyperlinkButton>) -> Result<Ref<Control>> {
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
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
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

/// `DefaultHyperlinkButtonStyle`: a control theme of `HyperlinkButton`.
fn default_hyperlink_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<HyperlinkButton>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    // WinUI sets the hand cursor in the code of the control (HyperlinkButton_Partial.cpp).
    theme.set(InputElement::cursor_property(), Some(literal(Cursor::parse("Hand"))?));
    theme.dynamic(TemplatedControl::background_property(), "HyperlinkButtonBackground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::OuterBorderEdge,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "HyperlinkButtonForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "HyperlinkButtonBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "HyperlinkButtonBorderThemeThickness",
    );
    theme.dynamic(TemplatedControl::padding_property(), "ButtonPadding");
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            default_hyperlink_button_style_focus_adorner,
        )),
    );
    theme.template(default_hyperlink_button_style_template);
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

/// The default control theme of `HyperlinkButton`.
fn hyperlink_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<HyperlinkButton>();
    theme.based_on("DefaultHyperlinkButtonStyle")?;
    Ok(())
}
