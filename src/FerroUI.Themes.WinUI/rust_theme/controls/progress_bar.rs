//! `Controls/ProgressBar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/ProgressBar/ProgressBar.xaml (the implicit style; the resources of
//!   controls/dev/ProgressBar/ProgressBar_themeresources.xaml are
//!   Resources/ProgressBar_themeresources.xaml)
//!   controls/dev/CommonStyles/Deprecated_themeresources.xaml (DefaultProgressBarStyle, a keyed
//!   style without setters, which WinUI applies over the default style)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The implicit style of the source is the keyed theme DefaultProgressBarStyle, which the
//!   default theme of ProgressBar is based on: an application that sets the empty keyed style of
//!   WinUI gets the default style, as with this theme.
//! - Template parts: DeterminateProgressBarIndicator is FerroUI's PART_Indicator, which must be a
//!   Border; the rectangles of the source (ProgressBarTrack and the indicators) are borders, their
//!   RadiusX/RadiusY (the corner radius of the control through TopLeft/BottomRight converters,
//!   ProgressBarTrackCornerRadius for the track) are corner radii. The control sets the width of
//!   PART_Indicator from the value as WinUI's control does; the indeterminate indicators take
//!   their widths from TemplateSettings (ContainerWidth, Container2Width), which WinUI's control
//!   sets in code.
//! - The inner Border with Clip="{Binding TemplateSettings.ClipRect}" (the area inside the
//!   padding) clips to its bounds (ClipToBounds), the same area.
//! - CommonStates: Determinate is the base. Indeterminate shows the indeterminate indicators and
//!   hides the track (its setters) and runs the storyboard of the source as keyframe animations
//!   of the translation of each indicator (2 s, forever, spline 0.4,0,0.6,1, the key times of the
//!   source) in the nested style of :indeterminate; PART_Indicator is hidden, which WinUI does by
//!   setting its width to 0 in code. FerroUI's ProgressBar has no ShowError and no ShowPaused:
//!   Error, Paused, UpdatingError, IndeterminateError and IndeterminatePaused have no
//!   counterpart. Updating and the visual transitions (RepositionThemeAnimation of the indicator
//!   when the value changes, FadeInThemeAnimation when an indeterminate bar becomes determinate,
//!   the colour animations from Error and Paused) are not ported.
//! - FerroUI's ProgressBar has an Orientation, WinUI's has none: the vertical bar is the
//!   horizontal one turned, with the track and the indicators along the height, PART_Indicator at
//!   the bottom, and the minimum width of ProgressBarMinHeight.
//! - FerroUI's ShowProgressText has no counterpart in WinUI: the text (ProgressTextFormat with the
//!   value, the percentage, the minimum and the maximum) is shown after the bar, under a vertical
//!   bar, in TextFillColorPrimaryBrush.
//! - ProgressBarTrack binds its Width to the Width of the control; the port lets it stretch, which
//!   is the same width inside the bar. IsTabStop is kept.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animation;
use ferroui_base::animation::IterationCount;
use ferroui_base::animation::KeyFrame;
use ferroui_base::animation::KeySpline;
use ferroui_base::animation::SignedDuration;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::TranslateTransform;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::converters::StringFormatConverter;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::LayoutTransformControl;
use ferroui_controls::ProgressBar;
use ferroui_controls::ProgressBarTemplateSettings;
use ferroui_controls::RowDefinitions;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::bound_setter;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The text of ShowProgressText (FerroUI): ProgressTextFormat with its arguments.
    resources.deferred("ProgressBarTextFormatConverter", |_| {
        Ok(value(::std::rc::Rc::new(StringFormatConverter::new())))
    });
    resources.control_theme("DefaultProgressBarStyle", default_progress_bar_style);
    resources.control_theme(type_key::<ProgressBar>(), progress_bar_theme);
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

/// The template of `ProgressBar` of `default_progress_bar_style`.
fn default_progress_bar_style_template(t: &Template<ProgressBar>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("LayoutRoot")?
        .with(|e| {
            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,Auto"))?);
            Ok(())
        })?
        .with(|e| {
            Grid::set_row_definitions(e, literal(RowDefinitions::parse("*,Auto"))?);
            Ok(())
        })?
        .children([
            t.make(Border::new())
                .name("ProgressBarRoot")?
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Border::new())
                        .name("ProgressBarClip")?
                        .set(Visual::clip_to_bounds_property(), true)
                        .child(
                            Decorator::child_property(),
                            t.make(Grid::new()).name("ProgressBarGrid")?.children([
                                t.make(Border::new())
                                    .name("ProgressBarTrack")?
                                    .template_bind(
                                        Border::background_property(),
                                        TemplatedControl::background_property(),
                                    )
                                    .static_resource(Border::corner_radius_property(), "ProgressBarTrackCornerRadius")?
                                    .control(),
                                t.make(Border::new())
                                    .name("PART_Indicator")?
                                    .template_bind(
                                        Border::background_property(),
                                        TemplatedControl::foreground_property(),
                                    )
                                    .template_bind(
                                        Border::corner_radius_property(),
                                        TemplatedControl::corner_radius_property(),
                                    )
                                    .control(),
                                t.make(Border::new())
                                    .name("IndeterminateProgressBarIndicator")?
                                    .template_bind(
                                        Border::background_property(),
                                        TemplatedControl::foreground_property(),
                                    )
                                    .template_bind(
                                        Border::corner_radius_property(),
                                        TemplatedControl::corner_radius_property(),
                                    )
                                    .set(Visual::opacity_property(), 0.0)
                                    .control(),
                                t.make(Border::new())
                                    .name("IndeterminateProgressBarIndicator2")?
                                    .template_bind(
                                        Border::background_property(),
                                        TemplatedControl::foreground_property(),
                                    )
                                    .template_bind(
                                        Border::corner_radius_property(),
                                        TemplatedControl::corner_radius_property(),
                                    )
                                    .set(Visual::opacity_property(), 0.0)
                                    .control(),
                            ]),
                        ),
                )
                .control(),
            t.make(LayoutTransformControl::new())
                .name("ProgressTextRoot")?
                .set(Grid::column_property(), 1)
                .set(Layoutable::margin_property(), Thickness::new(8.0, 0.0, 0.0, 0.0))
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .template_bind(
                    Visual::is_visible_property(),
                    ProgressBar::show_progress_text_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(TextBlock::new())
                        .name("ProgressText")?
                        .dynamic(TextBlock::foreground_property(), "TextFillColorPrimaryBrush")
                        .bind(TextBlock::text_property(), {
                            let binding = MultiBinding::new().with_converter_value(Some(
                                t.scope().multi_converter("ProgressBarTextFormatConverter")?,
                            ));
                            let bindings = binding.bindings();
                            bindings.add(template_binding(ProgressBar::progress_text_format_property()));
                            bindings.add(path().templated_parent().prop(RangeBase::value_property()).binding());
                            bindings.add(template_binding(ProgressBar::percentage_property()));
                            bindings.add(template_binding(RangeBase::minimum_property()));
                            bindings.add(template_binding(RangeBase::maximum_property()));
                            binding
                        }),
                )
                .control(),
        ])
        .control())
}

/// `DefaultProgressBarStyle`: a control theme of `ProgressBar`.
fn default_progress_bar_style(theme: &StyleBuilder) -> Result {
    theme.target::<ProgressBar>();
    theme.dynamic(TemplatedControl::foreground_property(), "ProgressBarForeground");
    theme.dynamic(TemplatedControl::background_property(), "ProgressBarBackground");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ProgressBarBorderThemeThickness",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ProgressBarBorderBrush");
    theme.static_resource(Layoutable::min_height_property(), "ProgressBarMinHeight")?;
    theme.set(RangeBase::maximum_property(), 100.0);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.static_resource(TemplatedControl::corner_radius_property(), "ProgressBarCornerRadius")?;
    theme.template(default_progress_bar_style_template);
    // The horizontal bar of the source: Grid Height="{TemplateBinding MinHeight}", the track
    // ProgressBarTrackHeight high and centred, the indicators from the left.
    theme.style(nesting().class(":horizontal"), |style| {
        style.style(nesting().template().of::<Grid>().name("ProgressBarGrid"), |style| {
            style.template_bind(Layoutable::height_property(), Layoutable::min_height_property());
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("ProgressBarTrack"), |style| {
            style.static_resource(Layoutable::height_property(), "ProgressBarTrackHeight")?;
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("PART_Indicator"), |style| {
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator"),
            |style| {
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
                style.bind(
                    Layoutable::width_property(),
                    path()
                        .ancestor_of::<ProgressBar>(0)
                        .class_member::<ProgressBar>("TemplateSettings")
                        .prop(ProgressBarTemplateSettings::container_width_property())
                        .binding(),
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator2"),
            |style| {
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
                style.bind(
                    Layoutable::width_property(),
                    path()
                        .ancestor_of::<ProgressBar>(0)
                        .class_member::<ProgressBar>("TemplateSettings")
                        .prop(ProgressBarTemplateSettings::container2_width_property())
                        .binding(),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The vertical bar (FerroUI): the horizontal one turned.
    theme.style(nesting().class(":vertical"), |style| {
        style.static_resource(Layoutable::min_width_property(), "ProgressBarMinHeight")?;
        style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
        style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
        style.style(nesting().template().of::<Grid>().name("ProgressBarGrid"), |style| {
            style.template_bind(Layoutable::width_property(), Layoutable::min_width_property());
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("ProgressBarTrack"), |style| {
            style.static_resource(Layoutable::width_property(), "ProgressBarTrackHeight")?;
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("PART_Indicator"), |style| {
            style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator"),
            |style| {
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
                style.bind(
                    Layoutable::height_property(),
                    path()
                        .ancestor_of::<ProgressBar>(0)
                        .class_member::<ProgressBar>("TemplateSettings")
                        .prop(ProgressBarTemplateSettings::container_width_property())
                        .binding(),
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator2"),
            |style| {
                style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
                style.bind(
                    Layoutable::height_property(),
                    path()
                        .ancestor_of::<ProgressBar>(0)
                        .class_member::<ProgressBar>("TemplateSettings")
                        .prop(ProgressBarTemplateSettings::container2_width_property())
                        .binding(),
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<LayoutTransformControl>()
                .name("ProgressTextRoot"),
            |style| {
                style.set(Grid::column_property(), 0);
                style.set(Grid::row_property(), 1);
                style.set(Layoutable::margin_property(), Thickness::new(0.0, 8.0, 0.0, 0.0));
                style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
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
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<Border>()
                .name("IndeterminateProgressBarIndicator2"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("ProgressBarTrack"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("PART_Indicator"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // The storyboard of Indeterminate (RepeatBehavior="Forever"): the translation of
    // IndeterminateProgressBarIndicator from ContainerAnimationStartPosition to
    // ContainerAnimationEndPosition in 1.5 s, held to 2 s, and of
    // IndeterminateProgressBarIndicator2 held at Container2AnimationStartPosition to 0.75 s,
    // then to Container2AnimationEndPosition at 2 s.
    theme.style(nesting().class(":horizontal").class(":indeterminate").template().of::<Border>().name("IndeterminateProgressBarIndicator"), |style| {
style.animation(style.scope().make(Animation::new()).set(Animation::iteration_count_property(), literal(IterationCount::parse("Infinite"))?).set(Animation::duration_property(), SignedDuration::from_milliseconds(2000.0)).with(|e| { let items = Animation::children(e); items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(0.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_spline(e, Some(literal(KeySpline::parse("0.4,0,0.6,1"))?)); Ok(()) })?.with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(1500.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_end_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(2000.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_end_position_property()).binding())); Ok(()) })?.end().into()); Ok(()) })?.end());
Ok(())
})?;
    theme.style(nesting().class(":horizontal").class(":indeterminate").template().of::<Border>().name("IndeterminateProgressBarIndicator2"), |style| {
style.animation(style.scope().make(Animation::new()).set(Animation::iteration_count_property(), literal(IterationCount::parse("Infinite"))?).set(Animation::duration_property(), SignedDuration::from_milliseconds(2000.0)).with(|e| { let items = Animation::children(e); items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(0.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(750.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_spline(e, Some(literal(KeySpline::parse("0.4,0,0.6,1"))?)); Ok(()) })?.with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(2000.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::x_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_end_position_property()).binding())); Ok(()) })?.end().into()); Ok(()) })?.end());
Ok(())
})?;
    theme.style(nesting().class(":vertical").class(":indeterminate").template().of::<Border>().name("IndeterminateProgressBarIndicator"), |style| {
style.animation(style.scope().make(Animation::new()).set(Animation::iteration_count_property(), literal(IterationCount::parse("Infinite"))?).set(Animation::duration_property(), SignedDuration::from_milliseconds(2000.0)).with(|e| { let items = Animation::children(e); items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(0.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_spline(e, Some(literal(KeySpline::parse("0.4,0,0.6,1"))?)); Ok(()) })?.with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(1500.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_end_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(2000.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container_animation_end_position_property()).binding())); Ok(()) })?.end().into()); Ok(()) })?.end());
Ok(())
})?;
    theme.style(nesting().class(":vertical").class(":indeterminate").template().of::<Border>().name("IndeterminateProgressBarIndicator2"), |style| {
style.animation(style.scope().make(Animation::new()).set(Animation::iteration_count_property(), literal(IterationCount::parse("Infinite"))?).set(Animation::duration_property(), SignedDuration::from_milliseconds(2000.0)).with(|e| { let items = Animation::children(e); items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(0.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(750.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_start_position_property()).binding())); Ok(()) })?.end().into());items.add(style.scope().make(KeyFrame::new()).with(|e| { KeyFrame::set_key_spline(e, Some(literal(KeySpline::parse("0.4,0,0.6,1"))?)); Ok(()) })?.with(|e| { KeyFrame::set_key_time(e, SignedDuration::from_milliseconds(2000.0)); Ok(()) })?.with(|e| { let items = KeyFrame::setters(e); items.add(bound_setter(TranslateTransform::y_property(), path().ancestor_of::<ProgressBar>(0).class_member::<ProgressBar>("TemplateSettings").prop(ProgressBarTemplateSettings::container2_animation_end_position_property()).binding())); Ok(()) })?.end().into()); Ok(()) })?.end());
Ok(())
})?;
    Ok(())
}

/// The default control theme of `ProgressBar`.
fn progress_bar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ProgressBar>();
    theme.based_on("DefaultProgressBarStyle")?;
    Ok(())
}
