//! `Controls/NavigationPage.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/NavigationView/NavigationBackButton.xaml (NavigationBackButtonNormalStyle and
//!   NavigationBackButtonSmallStyle; the resources are
//!   Resources/NavigationBackButton_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! WinUI has no navigation page with a stack of pages and a navigation bar: an application shows
//! its pages in a Frame and its back button with NavigationBackButtonNormalStyle (the back button
//! of NavigationView). The template is the one FerroUI's NavigationPage requires
//! (src/FerroUI.Themes.Fluent/Controls/NavigationPage.xaml: the navigation bar with the back
//! button, the header and the top command bar, the presenters of the pages and of the modal
//! pages, the bottom command bar and the shadow of the bar), drawn with the resources of WinUI:
//! - the back button PART_BackButton has the control theme NavigationBackButtonNormalStyle, and
//!   its default icon PART_BackButtonDefaultIcon is the Back glyph (E72B) of that style, 16 x 16,
//!   in the foreground of the button, which the states of the style set;
//! - the navigation bar has the background of the top pane of NavigationView
//!   (NavigationViewTopPaneBackground) and the header the text of SubtitleTextBlockStyle
//!   (SubtitleTextBlockFontSize, SemiBold) in TextFillColorPrimaryBrush, 12 px from the back
//!   button or the edge;
//! - the shadow under the bar (shown by the page's HasShadow) is the 1 px top border of the
//!   content grid of NavigationView with its pane at the top (NavigationViewContentGridBorderBrush,
//!   TopNavigationViewContentGridBorderThickness);
//! - a CommandBar in the bar has a transparent background and its labels collapsed (the 48 px of
//!   a closed compact WinUI command bar, AppBarThemeCompactHeight), unless the application sets
//!   them;
//! - the page transitions are FerroUI's (the PageSlide of the Fluent theme): WinUI's Frame plays
//!   its NavigationThemeTransition, which is not part of the theme.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4) of the back button styles:
//! - The Grid RootGrid (background, corner radius) is a Border of the same name around a Panel.
//! - The states of DefaultButtonStyle, which the keyed theme is based on (G-15), would draw their
//!   backgrounds and borders on the content presenter: the states set them transparent, as the
//!   template of the source has no such presenter.
//! - The AnimatedIcon Content with the AnimatedBackVisualSource and the fallback glyph of the
//!   button's Content (E72B by the style) is a path of GlyphGeometryE72B, 16 x 16, shown while
//!   the button has no content; the Content setter of the glyph is left out, and content the
//!   application (or NavigationPage) gives the button is shown by the content presenter
//!   PART_ContentPresenter. The AnimatedIcon.State setters are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::SplineEasing;
use ferroui_base::animation::FillMode;
use ferroui_base::animation::PageSlide;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::SlideAxis;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::Stretch;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::CommandBar;
use ferroui_controls::CommandBarDefaultLabelPosition;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::NavigationPage;
use ferroui_controls::Page;
use ferroui_controls::Panel;
use ferroui_controls::ToolTip;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("NavigationBackButtonNormalStyle", navigation_back_button_normal_style);
    resources.control_theme("NavigationBackButtonSmallStyle", navigation_back_button_small_style);
    resources.control_theme(type_key::<NavigationPage>(), navigation_page_theme);
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

/// The focus adorner of `navigation_back_button_normal_style`.
fn navigation_back_button_normal_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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

/// The template of `Button` of `navigation_back_button_normal_style`.
fn navigation_back_button_normal_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).children([
                t.make(Path::new())
                    .name("Content")?
                    .static_resource(Path::data_property(), "GlyphGeometryE72B")?
                    .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                    .set(Shape::stretch_property(), Stretch::Uniform)
                    .set(Layoutable::height_property(), 16.0)
                    .set(Layoutable::width_property(), 16.0)
                    .template_bind(
                        Layoutable::horizontal_alignment_property(),
                        ContentControl::horizontal_content_alignment_property(),
                    )
                    .template_bind(
                        Layoutable::vertical_alignment_property(),
                        ContentControl::vertical_content_alignment_property(),
                    )
                    .template_bind_with(
                        Visual::is_visible_property(),
                        Some(ContentControl::content_property().property()),
                        Some(ObjectConverters::is_null()),
                        None,
                        None,
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
                        ContentPresenter::horizontal_content_alignment_property(),
                        ContentControl::horizontal_content_alignment_property(),
                    )
                    .template_bind(
                        ContentPresenter::vertical_content_alignment_property(),
                        ContentControl::vertical_content_alignment_property(),
                    )
                    .set(ContentPresenter::recognizes_access_key_property(), true)
                    .control(),
            ]),
        )
        .control())
}

/// `NavigationBackButtonNormalStyle`: a control theme of `Button`.
fn navigation_back_button_normal_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "NavigationViewBackButtonBackground",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "NavigationViewItemForeground");
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.set(TemplatedControl::font_size_property(), 16.0);
    theme.dynamic(Layoutable::height_property(), "NavigationBackButtonHeight");
    theme.dynamic(Layoutable::width_property(), "NavigationBackButtonWidth");
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(Layoutable::margin_property(), Thickness::symmetric(4.0, 2.0));
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    // UseSystemFocusVisuals: the default focus adorner (no FocusVisualMargin).
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            navigation_back_button_normal_style_focus_adorner,
        )),
    );
    theme.template(navigation_back_button_normal_style_template);
    // PointerOver (the foreground of the state is also the foreground of the button, which
    // content such as the default icon of NavigationPage follows)
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "NavigationViewButtonForegroundPointerOver",
        );
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "NavigationViewButtonBackgroundPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "NavigationViewButtonForegroundPointerOver");
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
                    "NavigationViewButtonForegroundPointerOver",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "NavigationViewButtonForegroundPressed",
        );
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(Border::background_property(), "NavigationViewButtonBackgroundPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "NavigationViewButtonForegroundPressed");
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
                    "NavigationViewButtonForegroundPressed",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled (the background of the control stays)
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "NavigationViewButtonForegroundDisabled",
        );
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.template_bind(Border::background_property(), TemplatedControl::background_property());
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "NavigationViewButtonForegroundDisabled");
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
                    "NavigationViewButtonForegroundDisabled",
                );
                style.set(
                    ContentPresenter::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                style.set(
                    ContentPresenter::border_brush_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// `NavigationBackButtonSmallStyle`: a control theme of `Button`.
fn navigation_back_button_small_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("NavigationBackButtonNormalStyle")?;
    theme.set(Layoutable::margin_property(), Thickness::new(4.0, 2.0, 0.0, 2.0));
    Ok(())
}

/// The template of `NavigationPage` of `navigation_page_theme`.
fn navigation_page_theme_template(t: &Template<NavigationPage>) -> Result<Ref<Control>> {
    // ClipToBounds keeps the slides of the pages inside the control.
    Ok(t.make(Panel::new())
        .set(Visual::clip_to_bounds_property(), true)
        // The navigation bar floats above the content.
        // The line under the bar; its top margin is set in code to the height of the bar.
        // The modal pages: ZIndex 99 holds the static layer during transitions, 100 the
        // animating one.
        .children([
            t.make(DockPanel::new())
                // Reserves the height of the bar in the inset mode (FerroUI's :nav-bar-inset).
                // The back and front presenters of the slide transitions.
                .children([
                    t.make(Border::new())
                        .name("PART_NavBarSpacer")?
                        .template_bind(
                            Layoutable::height_property(),
                            NavigationPage::effective_bar_height_property(),
                        )
                        .set(Visual::is_visible_property(), false)
                        .set(DockPanel::dock_property(), Dock::Top)
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_BottomCommandBar")?
                        .set(DockPanel::dock_property(), Dock::Bottom)
                        .set(
                            ContentPresenter::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .template_bind(
                            ContentPresenter::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .bind(
                            ContentPresenter::content_property(),
                            path()
                                .templated_parent()
                                .prop_or_null(Page::current_page_property())
                                .prop(NavigationPage::bottom_command_bar_property())
                                .binding()
                                .with_fallback_value(None),
                        )
                        .control(),
                    t.make(Grid::new())
                        .name("PART_ContentHost")?
                        .template_bind(Panel::background_property(), TemplatedControl::background_property())
                        .set(Visual::clip_to_bounds_property(), true)
                        .children([
                            t.make(ContentPresenter::new())
                                .name("PART_PageBackPresenter")?
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_PagePresenter")?
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                                .control(),
                        ])
                        .control(),
                ])
                .control(),
            t.make(Border::new())
                .name("PART_NavigationBar")?
                .set(Visual::z_index_property(), 1)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .set(Visual::clip_to_bounds_property(), true)
                .template_bind(
                    Layoutable::height_property(),
                    NavigationPage::effective_bar_height_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .children([
                            t.make(Button::new())
                                .name("PART_BackButton")?
                                .set(Grid::column_property(), 0)
                                .static_resource(StyledElement::theme_property(), "NavigationBackButtonNormalStyle")?
                                .template_bind(
                                    Visual::is_visible_property(),
                                    NavigationPage::is_back_button_effectively_visible_property(),
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set_value(ToolTip::tip_property(), None)?
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Panel::new()).children([
                                        t.make(Path::new())
                                            .name("PART_BackButtonDefaultIcon")?
                                            .static_resource(Path::data_property(), "GlyphGeometryE72B")?
                                            .bind(
                                                Shape::fill_property(),
                                                path()
                                                    .ancestor_of::<Button>(0)
                                                    .prop(TemplatedControl::foreground_property())
                                                    .binding(),
                                            )
                                            .set(Layoutable::width_property(), 16.0)
                                            .set(Layoutable::height_property(), 16.0)
                                            .set(Shape::stretch_property(), Stretch::Uniform)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .control(),
                                        t.make(ContentPresenter::new())
                                            .name("PART_BackButtonContentPresenter")?
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .control(),
                                    ]),
                                )?
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_Header")?
                                .set(Grid::column_property(), 1)
                                .static_resource(ContentPresenter::font_size_property(), "SubtitleTextBlockFontSize")?
                                .set(ContentPresenter::font_weight_property(), FontWeight::SemiBold)
                                .dynamic(ContentPresenter::foreground_property(), "TextFillColorPrimaryBrush")
                                .bind(
                                    ContentPresenter::content_property(),
                                    path()
                                        .templated_parent()
                                        .prop_or_null(Page::current_page_property())
                                        .prop(Page::header_property())
                                        .binding()
                                        .with_fallback_value(None),
                                )
                                .bind(
                                    ContentPresenter::content_template_property(),
                                    path()
                                        .templated_parent()
                                        .prop_or_null(Page::current_page_property())
                                        .prop(Page::header_template_property())
                                        .binding()
                                        .with_fallback_value(None),
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(
                                    ContentPresenter::horizontal_content_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 12.0, 0.0))
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_TopCommandBar")?
                                .set(Grid::column_property(), 2)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                .set(
                                    ContentPresenter::vertical_content_alignment_property(),
                                    VerticalAlignment::Stretch,
                                )
                                .bind(
                                    ContentPresenter::content_property(),
                                    path()
                                        .templated_parent()
                                        .prop_or_null(Page::current_page_property())
                                        .prop(NavigationPage::top_command_bar_property())
                                        .binding()
                                        .with_fallback_value(None),
                                )
                                .style(of::<CommandBar>(), |style| {
                                    style.set(
                                        TemplatedControl::background_property(),
                                        Some(literal(Brush::parse("Transparent"))?),
                                    );
                                    style.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
                                    style.set(
                                        CommandBar::default_label_position_property(),
                                        CommandBarDefaultLabelPosition::Collapsed,
                                    );
                                    Ok(())
                                })?
                                .control(),
                        ]),
                )
                .control(),
            t.make(Border::new())
                .name("PART_NavBarShadow")?
                .set(Visual::z_index_property(), 1)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .set(Visual::is_visible_property(), false)
                .set(Layoutable::height_property(), 1.0)
                .set(InputElement::is_hit_test_visible_property(), false)
                .dynamic(Border::background_property(), "NavigationViewContentGridBorderBrush")
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ModalBackPresenter")?
                .set(Visual::z_index_property(), 99)
                .set(Visual::is_visible_property(), false)
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ModalPresenter")?
                .set(Visual::z_index_property(), 100)
                .set(Visual::is_visible_property(), false)
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `NavigationPage`.
fn navigation_page_theme(theme: &StyleBuilder) -> Result {
    theme.target::<NavigationPage>();
    // Transparent keeps the hit-test surface of the back-swipe gesture (FerroUI's contract).
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set_value(
        NavigationPage::page_transition_property(),
        value({
            let object = ::std::rc::Rc::new(PageSlide::new());
            object.set_duration(SignedDuration::from_milliseconds(300.0));
            object.set_orientation(SlideAxis::Horizontal);
            object.set_fill_mode(FillMode::Forward);
            object.set_slide_in_easing({
                let object = SplineEasing::new();
                object.set_x1(0.16);
                object.set_y1(1.0);
                object.set_x2(0.3);
                object.set_y2(1.0);
                object
            });
            object.set_slide_out_easing({
                let object = SplineEasing::new();
                object.set_x1(0.2);
                object.set_y1(0.85);
                object.set_x2(0.3);
                object.set_y2(1.0);
                object
            });
            object
        }),
    );
    theme.set_value(
        NavigationPage::modal_transition_property(),
        value({
            let object = ::std::rc::Rc::new(PageSlide::new());
            object.set_duration(SignedDuration::from_milliseconds(250.0));
            object.set_orientation(SlideAxis::Vertical);
            object.set_fill_mode(FillMode::Forward);
            object.set_slide_in_easing({
                let object = SplineEasing::new();
                object.set_x1(0.16);
                object.set_y1(1.0);
                object.set_x2(0.3);
                object.set_y2(1.0);
                object
            });
            object.set_slide_out_easing({
                let object = SplineEasing::new();
                object.set_x1(0.2);
                object.set_y1(0.85);
                object.set_x2(0.3);
                object.set_y2(1.0);
                object
            });
            object
        }),
    );
    theme.template(navigation_page_theme_template);
    // The background of the bar, which an application overrides with a more specific style.
    theme.style(
        nesting().template().of::<Border>().name("PART_NavigationBar"),
        |style| {
            style.dynamic(Border::background_property(), "NavigationViewTopPaneBackground");
            Ok(())
        },
    )?;
    // :nav-bar-inset: the spacer pushes the content under the bar.
    theme.style(
        nesting()
            .class(":nav-bar-inset")
            .template()
            .of::<Border>()
            .name("PART_NavBarSpacer"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // :nav-bar-compact: a bar lower than 40 px collapses the labels of its command bar.
    theme.style(
        nesting()
            .class(":nav-bar-compact")
            .template()
            .of::<ContentPresenter>()
            .name("PART_TopCommandBar")
            .descendant()
            .of::<CommandBar>(),
        |style| {
            style.set(
                CommandBar::default_label_position_property(),
                CommandBarDefaultLabelPosition::Collapsed,
            );
            Ok(())
        },
    )?;
    Ok(())
}
