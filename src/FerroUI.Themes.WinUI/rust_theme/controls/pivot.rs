//! `Controls/Pivot.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/Pivot_themeresources.xaml (DefaultPivotStyle with its NextTemplate,
//!   PreviousTemplate and TitleContentControlStyle, DefaultPivotItemStyle,
//!   DefaultPivotHeaderItemStyle and the implicit styles; the resources are
//!   Resources/Pivot_themeresources.xaml) and PivotTitleContentControlStyle
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The templates of WinUI's Pivot, PivotItem and PivotHeaderItem, which FerroUI ports as controls
//! (ferroui-controls, docs/porting/winui-calculator-gaps.md, PV-1 to PV-3); the Fluent theme has no
//! document for them. Translation notes (docs/porting/winui-theme.md, section 4):
//! - The parts keep their names: RootElement, TitleContentControl, ScrollViewer, Panel,
//!   PivotLayoutElement, LeftHeaderPresenter, HeaderClipper, StaticHeader, Header, FocusFollower,
//!   PreviousButton, NextButton, RightHeaderPresenter and PivotItemPresenter; the parts of the
//!   header item are Grid, ContentPresenter and SelectedPipe.
//! - The transforms of the template (PivotLayoutElementTranslateTransform,
//!   HeaderOffsetTranslateTransform, StaticHeaderTranslateTransform, HeaderTranslateTransform,
//!   ItemsPresenterTranslateTransform, ItemsPresenterCompositeTransform) are not parts: the control
//!   drives the composition Translation of the same elements (the layout element, the parent of the
//!   header panels, the header panels, the items presenter) with expression and key frame
//!   animations, on the compositor.
//! - The visual states are pseudo-classes, as in the other WinUI controls of the theme (decision
//!   D2): NavigationButtonsVisibility is :previous-button-visible and :next-button-visible of the
//!   pivot; HeaderStates (HeaderStatic) is set by the control on the header panels and shown as
//!   :header-static; the Orientation states have the same padding (PivotPortraitThemePadding and
//!   PivotLandscapeThemePadding are equal), so the title keeps the portrait margin. The selection
//!   states of the header item are :selected, :pressed, :pointerover, :disabled and :locked
//!   (UnselectedLocked); the VisualTransitions between Unselected and UnselectedLocked (0.33 s) are
//!   the transitions of Opacity of the content presenter.
//! - The clip of HeaderClipper (HeaderClipperGeometry, the size of the clipper) is ClipToBounds.
//! - Visibility="Collapsed" is IsVisible="False"; CharacterSpacing (PivotHeaderItemCharacterSpacing)
//!   and OpticalMarginAlignment are not FerroUI properties and are left out.
//! - The items of the pivot lie on each other in a Grid (WinUI's ItemsPanel).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TranslateTransform;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::SnapPointsAlignment;
use ferroui_controls::primitives::SnapPointsType;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::FontIcon;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::Pivot;
use ferroui_controls::PivotHeaderItem;
use ferroui_controls::PivotHeaderPanel;
use ferroui_controls::PivotItem;
use ferroui_controls::PivotPanel;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::ZoomMode;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::control_template;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.deferred("NextTemplate", |scope| {
        Ok(value(control_template(scope, next_template_template)))
    });
    resources.deferred("PreviousTemplate", |scope| {
        Ok(value(control_template(scope, previous_template_template)))
    });
    // The button of NextTemplate with the CommonStates of the template (a style of the
    // pivot cannot reach into the template of its button: one template selector per style).
    resources.control_theme("PivotNextButtonStyle", pivot_next_button_style);
    // The button of PreviousTemplate with the CommonStates of the template (a style of the
    // pivot cannot reach into the template of its button: one template selector per style).
    resources.control_theme("PivotPreviousButtonStyle", pivot_previous_button_style);
    resources.control_theme("TitleContentControlStyle", title_content_control_style);
    // WinUI's top-level PivotTitleContentControlStyle. Its font family, PhoneFontFamilyNormal, is not
    // a resource of the WinUI 3 theme (a reference WinUI leaves unresolved), so it is left out.
    resources.control_theme("PivotTitleContentControlStyle", pivot_title_content_control_style);
    resources.control_theme("DefaultPivotStyle", default_pivot_style);
    resources.control_theme(type_key::<Pivot>(), pivot_theme);
    resources.control_theme("DefaultPivotItemStyle", default_pivot_item_style);
    resources.control_theme(type_key::<PivotItem>(), pivot_item_theme);
    resources.control_theme("DefaultPivotHeaderItemStyle", default_pivot_header_item_style);
    resources.control_theme(type_key::<PivotHeaderItem>(), pivot_header_item_theme);
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

/// The template of `Button` of `NextTemplate`.
fn next_template_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "PivotNextButtonBackground")
        .dynamic(
            Border::border_thickness_property(),
            "PivotNavButtonBorderThemeThickness",
        )
        .dynamic(Border::border_brush_property(), "PivotNextButtonBorderBrush")
        .child(
            Decorator::child_property(),
            t.make(FontIcon::new())
                .name("Arrow")?
                .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                .set(TemplatedControl::font_size_property(), 12.0)
                .dynamic(TemplatedControl::foreground_property(), "PivotNextButtonForeground")
                .set(FontIcon::glyph_property(), String::from(""))
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(FontIcon::mirrored_when_right_to_left_property(), true)
                .set(Layoutable::use_layout_rounding_property(), false),
        )
        .control())
}

/// The template of `Button` of `PreviousTemplate`.
fn previous_template_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .dynamic(Border::background_property(), "PivotPreviousButtonBackground")
        .dynamic(
            Border::border_thickness_property(),
            "PivotNavButtonBorderThemeThickness",
        )
        .dynamic(Border::border_brush_property(), "PivotPreviousButtonBorderBrush")
        .child(
            Decorator::child_property(),
            t.make(FontIcon::new())
                .name("Arrow")?
                .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                .set(TemplatedControl::font_size_property(), 12.0)
                .dynamic(TemplatedControl::foreground_property(), "PivotPreviousButtonForeground")
                .set(FontIcon::glyph_property(), String::from(""))
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(FontIcon::mirrored_when_right_to_left_property(), true)
                .set(Layoutable::use_layout_rounding_property(), false),
        )
        .control())
}

/// `PivotNextButtonStyle`: a control theme of `Button`.
fn pivot_next_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.static_resource(TemplatedControl::template_property(), "NextTemplate")?;
    theme.style(
        nesting().class(":pointerover").template().of::<Border>().name("Root"),
        |style| {
            style.dynamic(Border::background_property(), "PivotNextButtonBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "PivotNextButtonBorderBrushPointerOver");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<FontIcon>()
            .name("Arrow"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "PivotNextButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("Root"),
        |style| {
            style.dynamic(Border::background_property(), "PivotNextButtonBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "PivotNextButtonBorderBrushPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<FontIcon>().name("Arrow"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "PivotNextButtonForegroundPressed",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// `PivotPreviousButtonStyle`: a control theme of `Button`.
fn pivot_previous_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.static_resource(TemplatedControl::template_property(), "PreviousTemplate")?;
    theme.style(
        nesting().class(":pointerover").template().of::<Border>().name("Root"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "PivotPreviousButtonBackgroundPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PivotPreviousButtonBorderBrushPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<FontIcon>()
            .name("Arrow"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "PivotPreviousButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("Root"),
        |style| {
            style.dynamic(Border::background_property(), "PivotPreviousButtonBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "PivotPreviousButtonBorderBrushPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting().class(":pressed").template().of::<FontIcon>().name("Arrow"),
        |style| {
            style.dynamic(
                TemplatedControl::foreground_property(),
                "PivotPreviousButtonForegroundPressed",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `ContentControl` of `title_content_control_style`.
fn title_content_control_style_template(t: &Template<ContentControl>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
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
        .control())
}

/// `TitleContentControlStyle`: a control theme of `ContentControl`.
fn title_content_control_style(theme: &StyleBuilder) -> Result {
    theme.target::<ContentControl>();
    theme.dynamic(TemplatedControl::font_family_property(), "PivotTitleFontFamily");
    theme.dynamic(TemplatedControl::font_weight_property(), "PivotTitleThemeFontWeight");
    theme.dynamic(TemplatedControl::font_size_property(), "PivotTitleFontSize");
    theme.template(title_content_control_style_template);
    Ok(())
}

/// `PivotTitleContentControlStyle`: a control theme of `ContentControl`.
fn pivot_title_content_control_style(theme: &StyleBuilder) -> Result {
    theme.target::<ContentControl>();
    theme.set(TemplatedControl::font_weight_property(), FontWeight::SemiBold);
    theme.dynamic(TemplatedControl::font_size_property(), "TextStyleLargeFontSize");
    theme.dynamic(Layoutable::margin_property(), "PivotPortraitThemePadding");
    Ok(())
}

/// The items panel of `default_pivot_style`.
fn default_pivot_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(Grid::new()).end().upcast())
}

/// The template of `Pivot` of `default_pivot_style`.
fn default_pivot_style_template(t: &Template<Pivot>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootElement")?
        .template_bind(
            Layoutable::horizontal_alignment_property(),
            Layoutable::horizontal_alignment_property(),
        )
        .template_bind(
            Layoutable::vertical_alignment_property(),
            Layoutable::vertical_alignment_property(),
        )
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            e.set_row_definitions(literal(RowDefinitions::parse("Auto,*"))?);
            Ok(())
        })?
        .children([
            t.make(ContentControl::new())
                .name("TitleContentControl")?
                .set(InputElement::is_tab_stop_property(), false)
                .static_resource(Layoutable::margin_property(), "PivotPortraitThemePadding")?
                .static_resource(StyledElement::theme_property(), "TitleContentControlStyle")?
                .set(Visual::is_visible_property(), false)
                .template_bind(ContentControl::content_property(), Pivot::title_property())
                .template_bind(
                    ContentControl::content_template_property(),
                    Pivot::title_template_property(),
                )
                .control(),
            t.make(Grid::new())
                .set(Grid::row_property(), 1)
                .children([t
                    .make(ScrollViewer::new())
                    .name("ScrollViewer")?
                    .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                    .set(
                        ScrollViewer::horizontal_snap_points_type_property(),
                        SnapPointsType::MandatorySingle,
                    )
                    .set(
                        ScrollViewer::horizontal_snap_points_alignment_property(),
                        SnapPointsAlignment::Center,
                    )
                    .set(
                        ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Hidden,
                    )
                    .set(
                        ScrollViewer::vertical_scroll_bar_visibility_property(),
                        ScrollBarVisibility::Disabled,
                    )
                    .set(ScrollViewer::vertical_snap_points_type_property(), SnapPointsType::None)
                    .set(
                        ContentControl::vertical_content_alignment_property(),
                        VerticalAlignment::Stretch,
                    )
                    .set(ScrollViewer::zoom_mode_property(), ZoomMode::Disabled)
                    .static_resource(
                        TemplatedControl::template_property(),
                        "ScrollViewerScrollBarlessTemplate",
                    )?
                    .set(ScrollViewer::bring_into_view_on_focus_change_property(), false)
                    .content(
                        ContentControl::content_property(),
                        t.make(PivotPanel::new())
                            .name("Panel")?
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                            .children([t
                                .make(Grid::new())
                                .name("PivotLayoutElement")?
                                .with(|e| {
                                    e.set_row_definitions(literal(RowDefinitions::parse("Auto,*"))?);
                                    Ok(())
                                })?
                                .with(|e| {
                                    e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                                    Ok(())
                                })?
                                .children([
                                    t.make(ContentPresenter::new())
                                        .name("LeftHeaderPresenter")?
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            Pivot::left_header_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_template_property(),
                                            Pivot::left_header_template_property(),
                                        )
                                        .set(
                                            Layoutable::horizontal_alignment_property(),
                                            HorizontalAlignment::Stretch,
                                        )
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                        .control(),
                                    t.make(ContentControl::new())
                                        .name("HeaderClipper")?
                                        .set(Grid::column_property(), 1)
                                        .set(InputElement::focusable_property(), true)
                                        .set(Visual::clip_to_bounds_property(), true)
                                        .set(
                                            ContentControl::horizontal_content_alignment_property(),
                                            HorizontalAlignment::Stretch,
                                        )
                                        .content(
                                            ContentControl::content_property(),
                                            t.make(Grid::new())
                                                .dynamic(Panel::background_property(), "PivotHeaderBackground")
                                                .children([
                                                    t.make(PivotHeaderPanel::new())
                                                        .name("StaticHeader")?
                                                        .set(Visual::is_visible_property(), false)
                                                        .control(),
                                                    t.make(PivotHeaderPanel::new()).name("Header")?.control(),
                                                    t.make(Rectangle::new())
                                                        .name("FocusFollower")?
                                                        .set(InputElement::is_hit_test_visible_property(), false)
                                                        .set(
                                                            Shape::fill_property(),
                                                            Some(literal(Brush::parse("Transparent"))?),
                                                        )
                                                        .set(
                                                            Layoutable::horizontal_alignment_property(),
                                                            HorizontalAlignment::Stretch,
                                                        )
                                                        .set(
                                                            Layoutable::vertical_alignment_property(),
                                                            VerticalAlignment::Stretch,
                                                        )
                                                        .control(),
                                                ]),
                                        )?
                                        .control(),
                                    t.make(Button::new())
                                        .name("PreviousButton")?
                                        .set(Grid::column_property(), 1)
                                        .static_resource(StyledElement::theme_property(), "PivotPreviousButtonStyle")?
                                        .set(Layoutable::width_property(), 20.0)
                                        .set(Layoutable::height_property(), 36.0)
                                        .dynamic(Layoutable::margin_property(), "PivotNavButtonMargin")
                                        .set(InputElement::is_tab_stop_property(), false)
                                        .set(InputElement::is_enabled_property(), false)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .set(Visual::opacity_property(), 0.0)
                                        .set(
                                            TemplatedControl::background_property(),
                                            Some(literal(Brush::parse("Transparent"))?),
                                        )
                                        .control(),
                                    t.make(Button::new())
                                        .name("NextButton")?
                                        .set(Grid::column_property(), 1)
                                        .static_resource(StyledElement::theme_property(), "PivotNextButtonStyle")?
                                        .set(Layoutable::width_property(), 20.0)
                                        .set(Layoutable::height_property(), 36.0)
                                        .dynamic(Layoutable::margin_property(), "PivotNavButtonMargin")
                                        .set(InputElement::is_tab_stop_property(), false)
                                        .set(InputElement::is_enabled_property(), false)
                                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                                        .set(Visual::opacity_property(), 0.0)
                                        .set(
                                            TemplatedControl::background_property(),
                                            Some(literal(Brush::parse("Transparent"))?),
                                        )
                                        .control(),
                                    t.make(ContentPresenter::new())
                                        .name("RightHeaderPresenter")?
                                        .set(Grid::column_property(), 2)
                                        .template_bind(
                                            ContentPresenter::content_property(),
                                            Pivot::right_header_property(),
                                        )
                                        .template_bind(
                                            ContentPresenter::content_template_property(),
                                            Pivot::right_header_template_property(),
                                        )
                                        .set(
                                            Layoutable::horizontal_alignment_property(),
                                            HorizontalAlignment::Stretch,
                                        )
                                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                        .control(),
                                    t.make(ItemsPresenter::new())
                                        .name("PivotItemPresenter")?
                                        .set(Grid::row_property(), 1)
                                        .set(Grid::column_span_property(), 3)
                                        .template_bind(
                                            ItemsPresenter::items_panel_property(),
                                            ItemsControl::items_panel_property(),
                                        )
                                        .control(),
                                ])
                                .control()]),
                    )?
                    .control()])
                .control(),
        ])
        .control())
}

/// `DefaultPivotStyle`: a control theme of `Pivot`.
fn default_pivot_style(theme: &StyleBuilder) -> Result {
    theme.target::<Pivot>();
    theme.set(Layoutable::margin_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(TemplatedControl::background_property(), "PivotBackground");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_pivot_style_items_panel),
    );
    theme.template(default_pivot_style_template);
    // NavigationButtonsVisibility: NavigationButtonsVisible, PreviousButtonVisible,
    // NextButtonVisible.
    theme.style(
        nesting()
            .class(":previous-button-visible")
            .template()
            .of::<Button>()
            .name("PreviousButton"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            style.set(InputElement::is_enabled_property(), true);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":next-button-visible")
            .template()
            .of::<Button>()
            .name("NextButton"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            style.set(InputElement::is_enabled_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `Pivot`.
fn pivot_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Pivot>();
    theme.based_on("DefaultPivotStyle")?;
    Ok(())
}

/// The template of `PivotItem` of `default_pivot_item_style`.
fn default_pivot_item_style_template(t: &Template<PivotItem>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .template_bind(
            Layoutable::horizontal_alignment_property(),
            Layoutable::horizontal_alignment_property(),
        )
        .template_bind(
            Layoutable::vertical_alignment_property(),
            Layoutable::vertical_alignment_property(),
        )
        .children([t
            .make(ContentPresenter::new())
            .template_bind(
                Layoutable::horizontal_alignment_property(),
                ContentControl::horizontal_content_alignment_property(),
            )
            .template_bind(
                Layoutable::vertical_alignment_property(),
                ContentControl::vertical_content_alignment_property(),
            )
            .template_bind(
                ContentPresenter::content_template_property(),
                ContentControl::content_template_property(),
            )
            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
            .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
            .control()])
        .control())
}

/// `DefaultPivotItemStyle`: a control theme of `PivotItem`.
fn default_pivot_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<PivotItem>();
    theme.dynamic(TemplatedControl::background_property(), "PivotItemBackground");
    theme.dynamic(Layoutable::margin_property(), "PivotItemMargin");
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.template(default_pivot_item_style_template);
    Ok(())
}

/// The default control theme of `PivotItem`.
fn pivot_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PivotItem>();
    theme.based_on("DefaultPivotItemStyle")?;
    Ok(())
}

/// The template of `PivotHeaderItem` of `default_pivot_header_item_style`.
fn default_pivot_header_item_style_template(t: &Template<PivotHeaderItem>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("Grid")?
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([
            t.make(ContentPresenter::new())
                .name("ContentPresenter")?
                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::font_size_property(),
                    TemplatedControl::font_size_property(),
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
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .set(
                    Animatable::transitions_property(),
                    Some({
                        let object = Transitions::new();
                        object.add(
                            t.make(DoubleTransition::new())
                                .set(
                                    TransitionBase::property_property(),
                                    Some(Visual::opacity_property().property()),
                                )
                                .set(
                                    TransitionBase::duration_property(),
                                    SignedDuration::from_milliseconds(330.0),
                                )
                                .end()
                                .into(),
                        );
                        object
                    }),
                )
                .control(),
            t.make(Rectangle::new())
                .name("SelectedPipe")?
                .dynamic(Shape::fill_property(), "PivotHeaderItemSelectedPipeFill")
                .set(Layoutable::height_property(), 3.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                .set(
                    Layoutable::horizontal_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 2.0))
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
                .set(Visual::is_visible_property(), false)
                .control(),
        ])
        .control())
}

/// `DefaultPivotHeaderItemStyle`: a control theme of `PivotHeaderItem`.
fn default_pivot_header_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<PivotHeaderItem>();
    theme.dynamic(TemplatedControl::font_size_property(), "PivotHeaderItemFontSize");
    theme.dynamic(TemplatedControl::font_family_property(), "PivotHeaderItemFontFamily");
    theme.dynamic(
        TemplatedControl::font_weight_property(),
        "PivotHeaderItemThemeFontWeight",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "PivotHeaderItemBackgroundUnselected",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PivotHeaderItemForegroundUnselected",
    );
    theme.dynamic(TemplatedControl::padding_property(), "PivotHeaderItemMargin");
    theme.set(Layoutable::height_property(), 48.0);
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(
        TemplatedControl::corner_radius_property(),
        "PivotHeaderItemSelectedPipeCornerRadius",
    );
    theme.template(default_pivot_header_item_style_template);
    // SelectionStates. Unselected, UnselectedPointerOver, UnselectedPressed and Disabled
    // collapse the pipe (the default here); Selected and its pointer states show it.
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundUnselectedPointerOver",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundUnselectedPointerOver",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundUnselectedPressed",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundUnselectedPressed",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":selected"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundSelected",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundSelected",
        );
        style.style(nesting().template().of::<Rectangle>().name("SelectedPipe"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundSelectedPointerOver",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundSelectedPointerOver",
        );
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundSelectedPressed",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundSelectedPressed",
        );
        Ok(())
    })?;
    // UnselectedLocked: the content moves by PivotHeaderItemLockedTranslation (40, the same in
    // every theme dictionary; a transform takes no dynamic resource) and fades out.
    theme.style(
        nesting()
            .class(":locked")
            .template()
            .of::<ContentPresenter>()
            .name("ContentPresenter"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set_value(
                Visual::render_transform_property(),
                value(
                    style
                        .scope()
                        .make(TranslateTransform::new())
                        .set(TranslateTransform::x_property(), 40.0)
                        .end(),
                ),
            );
            Ok(())
        },
    )?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "PivotHeaderItemBackgroundDisabled",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PivotHeaderItemForegroundDisabled",
        );
        style.style(nesting().template().of::<Rectangle>().name("SelectedPipe"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `PivotHeaderItem`.
fn pivot_header_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PivotHeaderItem>();
    theme.based_on("DefaultPivotHeaderItemStyle")?;
    Ok(())
}
