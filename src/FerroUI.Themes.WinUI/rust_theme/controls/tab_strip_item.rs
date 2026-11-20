//! `Controls/TabStripItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/SelectorBar/SelectorBar.xaml (DefaultSelectorBarItemStyle and the implicit style
//!   of SelectorBarItem)
//!   controls/dev/SelectorBar/SelectorBar_themeresources.xaml (SelectorBarItemPill, a style of the
//!   selection rectangle, based on ComboBoxItemPill of
//!   controls/dev/ComboBox/ComboBox_themeresources.xaml; the resources are
//!   Resources/SelectorBar_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's TabStripItem is SelectorBarItem: an item of the strip WinUI calls SelectorBar.
//!
//! WinUI: SelectorBarItemPill
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid PART_ContainerRoot has a background and a corner radius: it is a Border of the same
//!   name around the Grid.
//! - SelectorBarItem shows its Icon in PART_IconVisual and its Text in the TextBlock
//!   PART_TextVisual; FerroUI's TabStripItem is a content control without an icon, so the text
//!   block is the content presenter PART_ContentPresenter (with the margin and the wrapping of the
//!   text block) and the icon presenter is left out. The foreground the states set on the text and
//!   the icon is the foreground of the content presenter.
//! - The style SelectorBarItemPill of the Rectangle PART_SelectionVisual (and the
//!   ComboBoxItemPill it is based on) is written on the rectangle: a FerroUI shape has no styles
//!   of its own. Its values are those of the styles where the rectangle does not set them itself
//!   (Width 4, Height 3, the fill, RenderTransformOrigin 0.5,0.5; RadiusX, RadiusY and
//!   HorizontalAlignment are the rectangle's).
//! - The selected states animate the ScaleX of the CompositeTransform PillTransform to 4 and the
//!   opacity of the pill to 1 over ComboBoxItemScaleAnimationDuration (167 ms, key spline 0,0,0,1):
//!   a RenderTransform setter scaleX(4) and an Opacity setter with transitions of that duration and
//!   easing on the rectangle.
//! - The Rectangle PART_CommonVisual (fill, stroke and stroke thickness of the theme resources,
//!   radii of the corner radius) is a Border with the background, border brush, border thickness
//!   and corner radius of the same resources.
//! - PointerUpThemeAnimation of the pointer-over and selected states has no counterpart (FerroUI
//!   has no theme animations, and no pointer-down animation that it would undo).
//! - SelectedPointerOver and SelectedPressed do not set the background of PART_ContainerRoot,
//!   which the pointer-over, pressed and selected states set: ^:selected:pointerover and
//!   ^:selected:pressed set it back to the background of the control.
//! - UseSystemFocusVisuals with FocusVisualMargin="{ThemeResource SelectorBarItemFocusVisualMargin}"
//!   (-2) is the FocusAdorner.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransformOperationsTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextWrapping;
use ferroui_base::RelativePoint;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TabStripItem;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::RowDefinitions;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultSelectorBarItemStyle", default_selector_bar_item_style);
    resources.control_theme(type_key::<TabStripItem>(), tab_strip_item_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `default_selector_bar_item_style`.
fn default_selector_bar_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(-2.0))
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

/// The template of `TabStripItem` of `default_selector_bar_item_style`.
fn default_selector_bar_item_style_template(t: &Template<TabStripItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("PART_ContainerRoot")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,Auto"))?);
                    Ok(())
                })?
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,Auto"))?);
                    Ok(())
                })?
                .children([
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::column_property(), 1)
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
                            ContentPresenter::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .template_bind(
                            ContentPresenter::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .template_bind(
                            ContentPresenter::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .dynamic(ContentPresenter::padding_property(), "SelectorBarItemTextVisualMargin")
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
                    t.make(Rectangle::new())
                        .name("PART_SelectionVisual")?
                        .set(Grid::row_property(), 1)
                        .set(Grid::column_span_property(), 2)
                        .dynamic(Shape::fill_property(), "SelectorBarItemPillFill")
                        .static_resource(Layoutable::height_property(), "SelectorBarItemPillHeight")?
                        .static_resource(Layoutable::width_property(), "SelectorBarItemPillWidth")?
                        .set(Rectangle::radius_x_property(), 0.5)
                        .set(Rectangle::radius_y_property(), 1.0)
                        .dynamic(Layoutable::margin_property(), "SelectorBarItemSelectionVisualMargin")
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .set(Visual::opacity_property(), 0.0)
                        .set(
                            Visual::render_transform_origin_property(),
                            literal(RelativePoint::parse("50%,50%"))?,
                        )
                        .set(
                            Visual::render_transform_property(),
                            Some(literal(TransformOperations::parse("scaleX(1)"))?),
                        )
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
                                object.add(
                                    t.make(DoubleTransition::new())
                                        .set(
                                            TransitionBase::property_property(),
                                            Some(Visual::opacity_property().property()),
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
                    t.make(Border::new())
                        .name("PART_CommonVisual")?
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .set(Grid::row_span_property(), 2)
                        .set(Grid::column_span_property(), 2)
                        .dynamic(Border::background_property(), "SelectorBarItemBackground")
                        .dynamic(Border::border_brush_property(), "SelectorBarItemBorderBrush")
                        .dynamic(Border::border_thickness_property(), "SelectorBarSelectedInnerThickness")
                        .set(
                            Layoutable::horizontal_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .template_bind(
                            Border::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `DefaultSelectorBarItemStyle`: a control theme of `TabStripItem`.
fn default_selector_bar_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<TabStripItem>();
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
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::OuterBorderEdge,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "SelectorBarItemForeground");
    theme.dynamic(TemplatedControl::border_brush_property(), "SelectorBarItemBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "SelectorBarItemBorderThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "SelectorBarItemPadding")?;
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            default_selector_bar_item_style_focus_adorner,
        )),
    );
    theme.template(default_selector_bar_item_style_template);
    // CombinedStates: UnselectedPointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.dynamic(Border::background_property(), "SelectorBarItemBackgroundPointerOver");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // UnselectedPressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.dynamic(Border::background_property(), "SelectorBarItemBackgroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedNormal
    theme.style(nesting().class(":selected"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.dynamic(Border::background_property(), "SelectorBarItemBackgroundSelected");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundSelected",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("PART_SelectionVisual"),
            |style| {
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("scaleX(4)"))?),
                );
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPointerOver: the background of the container is not set by the state.
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.template_bind(Border::background_property(), TemplatedControl::background_property());
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPressed (the foreground of the pointer-over state, as in the source)
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.template_bind(Border::background_property(), TemplatedControl::background_property());
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // DisabledStates: Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Border>().name("PART_ContainerRoot"),
            |style| {
                style.dynamic(Border::background_property(), "SelectorBarItemBackgroundDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "SelectorBarItemForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Rectangle>().name("PART_SelectionVisual"),
            |style| {
                style.dynamic(Shape::fill_property(), "SelectorBarItemDisabledPillFill");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `TabStripItem`.
fn tab_strip_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TabStripItem>();
    theme.based_on("DefaultSelectorBarItemStyle")?;
    Ok(())
}
