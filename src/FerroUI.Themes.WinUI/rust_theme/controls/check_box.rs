//! `Controls/CheckBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CheckBox_themeresources.xaml (the styles; its resources are
//!   Resources/CheckBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid RootGrid has a background, a border and a corner radius, which a FerroUI panel does
//!   not have: it is a Border of the same name around the Grid.
//! - The check glyph is an AnimatedIcon (AnimatedAcceptVisualSource, a Lottie animation) with a
//!   FontIconSource fallback of the symbol font; FerroUI has neither, so it is a Path of the glyph
//!   geometries GlyphGeometryE73E (CheckBoxCheckedGlyph) and GlyphGeometryE9AE
//!   (CheckBoxIndeterminateGlyph) of Resources/Glyphs.xaml. The states of the animated icon
//!   (NormalOff, PointerOverOn, ...) become its opacity: hidden when unchecked, shown otherwise.
//! - The visual states of CombinedStates are nested styles on the pseudo-classes of the check
//!   state and the pointer state, in the order of the source.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FlowDirection;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::CheckBox;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultCheckBoxStyle", default_check_box_style);
    resources.control_theme(type_key::<CheckBox>(), check_box_theme);
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

/// The focus adorner of `default_check_box_style`.
fn default_check_box_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(-7.0, -3.0, -7.0, -3.0))
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

/// The template of `CheckBox` of `default_check_box_style`.
fn default_check_box_style_template(t: &Template<CheckBox>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
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
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*"))?);
                    Ok(())
                })?
                .children([
                    t.make(Grid::new())
                        .set(Grid::column_property(), 0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .static_resource(Layoutable::height_property(), "CheckBoxSize")?
                        .static_resource(Layoutable::width_property(), "CheckBoxSize")?
                        // The glyph is drawn at the size of its ink in the symbol font at CheckBoxGlyphSize.
                        .children([
                            t.make(Rectangle::new())
                                .name("NormalRectangle")?
                                .dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillUnchecked")
                                .dynamic(Shape::stroke_property(), "CheckBoxCheckBackgroundStrokeUnchecked")
                                .static_resource(Shape::stroke_thickness_property(), "CheckBoxBorderThickness")?
                                .set(Layoutable::use_layout_rounding_property(), false)
                                .static_resource(Layoutable::height_property(), "CheckBoxSize")?
                                .static_resource(Layoutable::width_property(), "CheckBoxSize")?
                                .template_bind_with(
                                    Rectangle::radius_x_property(),
                                    Some(TemplatedControl::corner_radius_property().property()),
                                    Some(t.scope().converter("TopLeftCornerRadiusDoubleValueConverter")?),
                                    None,
                                    None,
                                )
                                .template_bind_with(
                                    Rectangle::radius_y_property(),
                                    Some(TemplatedControl::corner_radius_property().property()),
                                    Some(t.scope().converter("BottomRightCornerRadiusDoubleValueConverter")?),
                                    None,
                                    None,
                                )
                                .control(),
                            t.make(Path::new())
                                .name("CheckGlyph")?
                                .dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundUnchecked")
                                .static_resource(Path::data_property(), "GlyphGeometryE73E")?
                                .set(Shape::stretch_property(), Stretch::Uniform)
                                .set(Layoutable::width_property(), 10.0)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Visual::flow_direction_property(), FlowDirection::LeftToRight)
                                .set(Layoutable::margin_property(), Thickness::new(0.0, 1.0, 0.0, -1.0))
                                .set(Visual::opacity_property(), 0.0)
                                .control(),
                        ])
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        )
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                ]),
        )
        .control())
}

/// `DefaultCheckBoxStyle`: a control theme of `CheckBox`.
fn default_check_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<CheckBox>();
    theme.dynamic(TemplatedControl::background_property(), "CheckBoxBackgroundUnchecked");
    theme.dynamic(TemplatedControl::foreground_property(), "CheckBoxForegroundUnchecked");
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "CheckBoxBorderBrushUnchecked",
    );
    theme.static_resource(TemplatedControl::padding_property(), "CheckBoxPadding")?;
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.static_resource(Layoutable::min_width_property(), "CheckBoxMinWidth")?;
    theme.static_resource(Layoutable::min_height_property(), "CheckBoxHeight")?;
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // UseSystemFocusVisuals with FocusVisualMargin="{StaticResource CheckBoxFocusVisualMargin}" (-7,-3,-7,-3).
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_check_box_style_focus_adorner)),
    );
    theme.template(default_check_box_style_template);
    // The glyph of the check states (AnimatedIcon.State and DownLevelCheckGlyph.Glyph of the source).
    theme.style(
        nesting().class(":checked").template().of::<Path>().name("CheckGlyph"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":indeterminate")
            .template()
            .of::<Path>()
            .name("CheckGlyph"),
        |style| {
            style.static_resource(Path::data_property(), "GlyphGeometryE9AE")?;
            style.set(Layoutable::width_property(), 8.0);
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    // UncheckedPointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundUncheckedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundUncheckedPointerOver");
            style.dynamic(
                Border::border_brush_property(),
                "CheckBoxBorderBrushUncheckedPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeUncheckedPointerOver",
                );
                style.dynamic(
                    Shape::fill_property(),
                    "CheckBoxCheckBackgroundFillUncheckedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CheckBoxCheckGlyphForegroundUncheckedPointerOver",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // UncheckedPressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundUncheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundUncheckedPressed");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushUncheckedPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeUncheckedPressed",
                );
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillUncheckedPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundUncheckedPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // UncheckedDisabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundUncheckedDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundUncheckedDisabled");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushUncheckedDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeUncheckedDisabled",
                );
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillUncheckedDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundUncheckedDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    // CheckedNormal
    theme.style(nesting().class(":checked"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "CheckBoxForegroundChecked");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundChecked");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushChecked");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CheckBoxCheckBackgroundStrokeChecked");
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillChecked");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundChecked");
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundCheckedPointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundCheckedPointerOver");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushCheckedPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeCheckedPointerOver",
                );
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillCheckedPointerOver");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundCheckedPointerOver");
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundCheckedPressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundCheckedPressed");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushCheckedPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CheckBoxCheckBackgroundStrokeCheckedPressed");
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillCheckedPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundCheckedPressed");
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundCheckedDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundCheckedDisabled");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushCheckedDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CheckBoxCheckBackgroundStrokeCheckedDisabled");
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillCheckedDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundCheckedDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    // IndeterminateNormal
    theme.style(nesting().class(":indeterminate"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundIndeterminate",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundIndeterminate");
            style.dynamic(Border::border_brush_property(), "CheckBoxBorderBrushIndeterminate");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CheckBoxCheckBackgroundStrokeIndeterminate");
                style.dynamic(Shape::fill_property(), "CheckBoxCheckBackgroundFillIndeterminate");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "CheckBoxCheckGlyphForegroundIndeterminate");
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundIndeterminatePointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "CheckBoxBackgroundIndeterminatePointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "CheckBoxBorderBrushIndeterminatePointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeIndeterminatePointerOver",
                );
                style.dynamic(
                    Shape::fill_property(),
                    "CheckBoxCheckBackgroundFillIndeterminatePointerOver",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CheckBoxCheckGlyphForegroundIndeterminatePointerOver",
            );
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundIndeterminatePressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundIndeterminatePressed");
            style.dynamic(
                Border::border_brush_property(),
                "CheckBoxBorderBrushIndeterminatePressed",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeIndeterminatePressed",
                );
                style.dynamic(
                    Shape::fill_property(),
                    "CheckBoxCheckBackgroundFillIndeterminatePressed",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CheckBoxCheckGlyphForegroundIndeterminatePressed",
            );
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            Ok(())
        })?;
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
                    ContentPresenter::foreground_property(),
                    "CheckBoxForegroundIndeterminateDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "CheckBoxBackgroundIndeterminateDisabled");
            style.dynamic(
                Border::border_brush_property(),
                "CheckBoxBorderBrushIndeterminateDisabled",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("NormalRectangle"),
            |style| {
                style.dynamic(
                    Shape::stroke_property(),
                    "CheckBoxCheckBackgroundStrokeIndeterminateDisabled",
                );
                style.dynamic(
                    Shape::fill_property(),
                    "CheckBoxCheckBackgroundFillIndeterminateDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("CheckGlyph"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "CheckBoxCheckGlyphForegroundIndeterminateDisabled",
            );
            style.set(Layoutable::margin_property(), Thickness::uniform(0.0));
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `CheckBox`.
fn check_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CheckBox>();
    theme.based_on("DefaultCheckBoxStyle")?;
    Ok(())
}
