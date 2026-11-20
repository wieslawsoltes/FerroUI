//! `Controls/ListBoxItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ListBox_themeresources.xaml (the style DefaultListBoxItemStyle; the
//!   resources of the document are Resources/ListBox_themeresources.xaml)
//!   controls/dev/CommonStyles/ListViewItem_themeresources.xaml (the keyed styles
//!   DefaultListViewItemStyle and ListViewItemExpanded; the resources of the document are
//!   Resources/ListViewItem_themeresources.xaml)
//!   dxaml/xcp/core/core/elements/ListViewBaseItemChrome.cpp (the chrome that
//!   ListViewItemPresenter draws for DefaultListViewItemStyle)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's ListBoxItem is WinUI's ListBoxItem: the default theme is DefaultListBoxItemStyle.
//!   The keyed styles of ListViewItem are keyed themes of ListBoxItem, for lists that look as a
//!   WinUI ListView (the suggestions of AutoCompleteBox use DefaultListViewItemStyle).
//! - DefaultListBoxItemStyle: Grid LayoutRoot is a Border (it has a border thickness), which also
//!   draws the border brush and corner radius of the item (FerroUI's ListBoxItem honours them in
//!   its other themes; WinUI's template has no brush and no radius, so a WinUI item has none); the
//!   ContentPresenter of the source is PART_ContentPresenter, with the setters of the
//!   template-local styles BaseContentPresenterStyle and BodyContentPresenterStyle (the body font,
//!   normal weight), TextLineBounds as FerroUI's attached TextBlock.TextLineBounds; LineStackingStrategy
//!   and OpticalMarginAlignment of those styles have no counterpart. The states of the single group CommonStates are the pseudo-classes in the
//!   order of the source; Disabled is written last, as ListBoxItem::ChangeVisualState chooses it
//!   for a selected item too (there is no SelectedDisabled state in the template), and resets the
//!   fill of PressedBackground. Selected and SelectedUnfocused have the same values: one style.
//! - DefaultListViewItemStyle: the template is a ListViewItemPresenter, chrome that WinUI draws in
//!   native code. The port draws it with elements, with the values of ListViewBaseItemChrome.cpp: a
//!   backplate with the margin 4,2,4,2 (s_backplateMargin) and the corner radius
//!   ListViewItemCornerRadius in the background of the state; the selection indicator, a 3 by 16 px
//!   bar (s_selectionIndicatorSize) at 4 px from the left edge (s_selectionIndicatorMargin), in
//!   ListViewItemSelectionIndicatorCornerRadius, shown when the item is selected and shortened by
//!   6 px (s_selectionIndicatorHeightShrinkage) while it is pressed; the content moved by the width
//!   of the indicator and its margins (7 px, the inline indicator mode); the disabled opacity
//!   ListViewItemDisabledThemeOpacity. The multi-select check box of the presenter (CheckMode),
//!   drag and reorder visuals and the placeholder of data virtualization have no counterpart:
//!   FerroUI's ListBox has none of these modes.
//! - ListViewItemExpanded: Grid ContentBorder is a Border; the states of CommonStates and
//!   DisabledStates are ported. MultiSelectStates (the check box of a list in multiple selection
//!   mode, which the list view sets), DataVirtualizationStates, ReorderHintStates and DragStates,
//!   with the elements only they show (MultiSelectSquare, MultiSelectCheck, PlaceholderTextBlock,
//!   PlaceholderRect, MultiArrangeOverlayTextBorder), have no counterpart, as for
//!   DefaultListViewItemStyle.
//! - UseSystemFocusVisuals is the default focus adorner; DefaultListViewItemStyle has its own focus
//!   brushes, thicknesses and margin (FocusVisualPrimaryBrush, FocusVisualSecondaryBrush,
//!   FocusVisualMargin="1"), which are its FocusAdorner. IsHoldingEnabled, AllowDrop and
//!   ContentTransitions have no counterpart.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransformOperationsTransition;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextLineBounds;
use ferroui_base::media::TextWrapping;
use ferroui_base::RelativePoint;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultListBoxItemStyle", default_list_box_item_style);
    resources.control_theme(type_key::<ListBoxItem>(), list_box_item_theme);
    // DefaultListViewItemStyle: the chrome of ListViewItemPresenter drawn with elements. WinUI
    //       applies the default style of a control beneath an explicit style, so the keyed themes are
    //       based on the default theme (docs/porting/winui-theme.md, section 4.2).
    resources.control_theme("DefaultListViewItemStyle", default_list_view_item_style);
    resources.control_theme("ListViewItemExpanded", list_view_item_expanded);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ListBoxItem` of `default_list_box_item_style`.
fn default_list_box_item_style_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
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
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Panel::new())
                // BodyContentPresenterStyle (based on BaseContentPresenterStyle) with
                //                 TextWrapping="NoWrap".
                .children([
                    t.make(Rectangle::new())
                        .name("PressedBackground")?
                        .set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?))
                        .set(TemplatedControl::is_template_focus_target_property(), true)
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .dynamic(
                            ContentPresenter::font_family_property(),
                            "ContentControlThemeFontFamily",
                        )
                        .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                        .dynamic(ContentPresenter::font_size_property(), "ControlContentThemeFontSize")
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::NoWrap)
                        .set(TextBlock::text_line_bounds_property(), TextLineBounds::Full)
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

/// `DefaultListBoxItemStyle`: a control theme of `ListBoxItem`.
fn default_list_box_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    //         (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.static_resource(TemplatedControl::padding_property(), "ListBoxItemPadding")?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.template(default_list_box_item_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListBoxItemBackgroundPointerOver");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListBoxItemForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListBoxItemBackgroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListBoxItemForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected, SelectedUnfocused
    theme.style(nesting().class(":selected"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListBoxItemBackgroundSelected");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListBoxItemForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPointerOver
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListBoxItemBackgroundSelectedPointerOver");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPressed
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListBoxItemBackgroundSelectedPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("PressedBackground"),
            |style| {
                style.set(Shape::fill_property(), Some(literal(Brush::parse("Transparent"))?));
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListBoxItemForegroundDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ListBoxItem`.
fn list_box_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.based_on("DefaultListBoxItemStyle")?;
    Ok(())
}

/// The focus adorner of `default_list_view_item_style`.
fn default_list_view_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(1.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ListViewItemCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualSecondaryBrush")
                .dynamic(Border::corner_radius_property(), "ListViewItemCornerRadius"),
        )
        .control())
}

/// The template of `ListBoxItem` of `default_list_view_item_style`.
fn default_list_view_item_style_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("Root")?
        // The backplate of the rounded chrome (s_backplateMargin).
        // The selection indicator (s_selectionIndicatorSize, s_selectionIndicatorMargin).
        // The content, moved by the inline selection indicator (4 + 3 px).
        .children([
            t.make(Border::new())
                .name("Backplate")?
                .set(Layoutable::margin_property(), Thickness::new(4.0, 2.0, 4.0, 2.0))
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
                .dynamic(Border::corner_radius_property(), "ListViewItemCornerRadius")
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(TemplatedControl::is_template_focus_target_property(), true)
                .control(),
            t.make(Border::new())
                .name("SelectionIndicator")?
                .set(Layoutable::margin_property(), Thickness::new(4.0, 0.0, 0.0, 0.0))
                .set(Layoutable::width_property(), 3.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .dynamic(
                    Border::corner_radius_property(),
                    "ListViewItemSelectionIndicatorCornerRadius",
                )
                .dynamic(Border::background_property(), "ListViewItemSelectionIndicatorBrush")
                .set(Visual::opacity_property(), 0.0)
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
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
                        object
                    }),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .set(Layoutable::margin_property(), Thickness::new(7.0, 0.0, 0.0, 0.0))
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
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
                .control(),
        ])
        .control())
}

/// `DefaultListViewItemStyle`: a control theme of `ListBoxItem`.
fn default_list_view_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.based_on("DefaultListBoxItemStyle")?;
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::background_property(), "ListViewItemBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ListViewItemForeground");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(16.0, 0.0, 12.0, 0.0),
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(Layoutable::min_width_property(), "ListViewItemMinWidth");
    theme.dynamic(Layoutable::min_height_property(), "ListViewItemMinHeight");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_list_view_item_style_focus_adorner)),
    );
    theme.template(default_list_view_item_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundPointerOver");
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
                    "ListViewItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListViewItemForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelected");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Border>().name("SelectionIndicator"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
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
                    "ListViewItemForegroundSelected",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPointerOver
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(
                Border::background_property(),
                "ListViewItemBackgroundSelectedPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Border>().name("SelectionIndicator"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "ListViewItemSelectionIndicatorPointerOverBrush",
                );
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
                    "ListViewItemForegroundSelectedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // SelectedPressed: the indicator 6 px shorter (10 of 16 px).
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelectedPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Border>().name("SelectionIndicator"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "ListViewItemSelectionIndicatorPressedBrush",
                );
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("scaleY(0.625)"))?),
                );
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
                    "ListViewItemForegroundSelectedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(
        nesting().class(":disabled").template().of::<Panel>().name("Root"),
        |style| {
            style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
            Ok(())
        },
    )?;
    // SelectedDisabled
    theme.style(nesting().class(":selected").class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("Backplate"), |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelectedDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Border>().name("SelectionIndicator"),
            |style| {
                style.dynamic(
                    Border::background_property(),
                    "ListViewItemSelectionIndicatorDisabledBrush",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `ListBoxItem` of `list_view_item_expanded`.
fn list_view_item_expanded_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ContentBorder")?
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
        .set(
            Visual::render_transform_origin_property(),
            literal(RelativePoint::parse("50%,50%"))?,
        )
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).children([
                t.make(Rectangle::new())
                    .name("BorderBackground")?
                    .set(InputElement::is_hit_test_visible_property(), false)
                    .dynamic(Shape::fill_property(), "ListViewItemBorderBackground")
                    .set(Visual::opacity_property(), 0.0)
                    .set(TemplatedControl::is_template_focus_target_property(), true)
                    .control(),
                t.make(Panel::new())
                    .name("ContentPresenterGrid")?
                    .set(
                        Panel::background_property(),
                        Some(literal(Brush::parse("Transparent"))?),
                    )
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
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .control()])
                    .control(),
            ]),
        )
        .control())
}

/// `ListViewItemExpanded`: a control theme of `ListBoxItem`.
fn list_view_item_expanded(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.based_on("DefaultListBoxItemStyle")?;
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set_null(TemplatedControl::border_brush_property());
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(TemplatedControl::foreground_property(), "ListViewItemForeground");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(12.0, 0.0, 12.0, 0.0),
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(Layoutable::min_width_property(), "ListViewItemMinWidth");
    theme.dynamic(Layoutable::min_height_property(), "ListViewItemMinHeight");
    theme.template(list_view_item_expanded_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("BorderBackground"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                style.dynamic(Shape::fill_property(), "ListViewItemBackgroundPointerOver");
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
                    "ListViewItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("BorderBackground"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                style.dynamic(Shape::fill_property(), "ListViewItemBackgroundPressed");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "ListViewItemForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("BorderBackground"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                style.dynamic(Shape::fill_property(), "ListViewItemBackgroundSelected");
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
                    "ListViewItemForegroundSelected",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PointerOverSelected
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("BorderBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListViewItemBackgroundSelectedPointerOver");
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
                    "ListViewItemForegroundSelectedPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PressedSelected
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("BorderBackground"),
            |style| {
                style.dynamic(Shape::fill_property(), "ListViewItemBackgroundSelectedPressed");
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
                    "ListViewItemForegroundSelectedPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled of DisabledStates
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("ContentBorder"),
        |style| {
            style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
            Ok(())
        },
    )?;
    Ok(())
}
