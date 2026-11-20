//! `Controls/ComboBoxItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/ComboBox/ComboBox_themeresources.xaml (the styles DefaultComboBoxItemStyle and
//!   ComboBoxItemPill; the resources of the document are Resources/ComboBox_themeresources.xaml)
//!   dxaml/xcp/dxaml/lib/ComboBoxItem_Partial.cpp (the choice of the visual states)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: the ContentPresenter of the source is FerroUI's PART_ContentPresenter. Grid
//!   LayoutRoot (background, border, corner radius) is a Border.
//! - ComboBoxItemPill, a style of Rectangle, is the keyed theme ComboBoxItemPill of a Border: a
//!   border with the background and the corner radius ComboBoxItemPillCornerRadius draws the
//!   rounded bar the rectangle draws with the radii converted from that corner radius.
//! - The states of CommonStates follow ComboBoxItem::ChangeVisualState: a disabled item is in
//!   Disabled whether it is selected or not (written last, it hides the pill); a selected item is in
//!   the Selected states and the others in PointerOver and Pressed. Selected and SelectedUnfocused
//!   (the selection of the drop-down is not active) are not told apart: the items show while the
//!   drop-down is open, where the selection is active. SelectedPressed scales the pill to
//!   ComboBoxItemPillMinScale (0.625) over ComboBoxItemScaleAnimationDuration (167 ms) with the
//!   spline 0,0,0,1, a transition of its transform.
//! - InputModeStates (TouchInputMode and GameControllerInputMode, the paddings
//!   ComboBoxItemThemeTouchPadding and ComboBoxItemThemeGameControllerPadding) have no counterpart:
//!   FerroUI does not report the input device that opened the drop-down.
//! - FocusVisualMargin="-3" is the FocusAdorner with that margin.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransformOperationsTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ComboBoxItem;
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
    resources.control_theme("ComboBoxItemPill", combo_box_item_pill);
    resources.control_theme("DefaultComboBoxItemStyle", default_combo_box_item_style);
    resources.control_theme(type_key::<ComboBoxItem>(), combo_box_item_theme);
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

/// `ComboBoxItemPill`: a control theme of `Border`.
fn combo_box_item_pill(theme: &StyleBuilder) -> Result {
    theme.target::<Border>();
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.static_resource(Layoutable::height_property(), "ComboBoxItemPillHeight")?;
    theme.static_resource(Layoutable::width_property(), "ComboBoxItemPillWidth")?;
    theme.dynamic(Border::background_property(), "ComboBoxItemPillFillBrush");
    theme.static_resource(Border::corner_radius_property(), "ComboBoxItemPillCornerRadius")?;
    theme.set(
        Visual::render_transform_origin_property(),
        literal(RelativePoint::parse("50%,50%"))?,
    );
    Ok(())
}

/// The focus adorner of `default_combo_box_item_style`.
fn default_combo_box_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
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

/// The template of `ComboBoxItem` of `default_combo_box_item_style`.
fn default_combo_box_item_style_template(t: &Template<ComboBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .set(Layoutable::margin_property(), Thickness::new(5.0, 2.0, 5.0, 2.0))
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .set(TemplatedControl::is_template_focus_target_property(), true)
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).children([
                t.make(Border::new())
                    .name("Pill")?
                    .static_resource(StyledElement::theme_property(), "ComboBoxItemPill")?
                    .set(Visual::opacity_property(), 0.0)
                    .set(
                        Animatable::transitions_property(),
                        Some({
                            let object = Transitions::new();
                            object.add(
                                t.make(TransformOperationsTransition::new())
                                    .set(
                                        TransitionBase::property_property(),
                                        Some(Visual::render_transform_property().property()),
                                    )
                                    .set(
                                        TransitionBase::duration_property(),
                                        SignedDuration::from_milliseconds(167.0),
                                    )
                                    .set(TransitionBase::easing_property(), literal(Easing::parse("0,0,0,1"))?)
                                    .end()
                                    .into(),
                            );
                            object
                        }),
                    )
                    .control(),
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
                    .template_bind(
                        Layoutable::horizontal_alignment_property(),
                        ContentControl::horizontal_content_alignment_property(),
                    )
                    .template_bind(
                        Layoutable::vertical_alignment_property(),
                        ContentControl::vertical_content_alignment_property(),
                    )
                    .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                    .control(),
            ]),
        )
        .control())
}

/// `DefaultComboBoxItemStyle`: a control theme of `ComboBoxItem`.
fn default_combo_box_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ComboBoxItem>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "ComboBoxItemForeground");
    theme.dynamic(TemplatedControl::background_property(), "ComboBoxItemBackground");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.dynamic(TemplatedControl::padding_property(), "ComboBoxItemThemePadding");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.static_resource(TemplatedControl::corner_radius_property(), "ComboBoxItemCornerRadius")?;
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_combo_box_item_style_focus_adorner)),
    );
    theme.template(default_combo_box_item_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxItemBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "ComboBoxItemBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ComboBoxItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxItemBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "ComboBoxItemBorderBrushPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ComboBoxItemForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("Pill"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxItemBackgroundSelected");
            style.dynamic(Border::border_brush_property(), "ComboBoxItemBorderBrushSelected");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ComboBoxItemForegroundSelected",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPointerOver
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(
                Border::background_property(),
                "ComboBoxItemBackgroundSelectedPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "ComboBoxItemBorderBrushSelectedPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ComboBoxItemForegroundSelectedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPressed
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("Pill"), |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scaleY(0.625)"))?),
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxItemBackgroundSelectedPressed");
            style.dynamic(
                Border::border_brush_property(),
                "ComboBoxItemBorderBrushSelectedPressed",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ComboBoxItemForegroundSelectedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("Pill"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("LayoutRoot"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxItemBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "ComboBoxItemBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ComboBoxItemForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ComboBoxItem`.
fn combo_box_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ComboBoxItem>();
    theme.based_on("DefaultComboBoxItemStyle")?;
    Ok(())
}
