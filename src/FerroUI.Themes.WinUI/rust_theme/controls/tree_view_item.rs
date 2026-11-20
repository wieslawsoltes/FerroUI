//! `Controls/TreeViewItem.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TreeView/TreeViewItem.xaml (the style MUX_TreeViewItemStyle, the implicit style of
//!   TreeViewItem of controls/dev/TreeView/TreeView.xaml; the resources are
//!   Resources/TreeView_themeresources.xaml)
//!   controls/dev/CommonStyles/ListViewItem_themeresources.xaml (DefaultListViewItemStyle, the base
//!   of MUX_TreeViewItemStyle)
//!   controls/dev/TreeView/TreeView.idl, TreeViewItem.cpp (the default glyphs, their size and the
//!   indentation of a level)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - WinUI's tree item is a row of a flat list; FerroUI's TreeViewItem holds its header and its
//!   child items. The template is the row of the source (Grid ContentPresenterGrid, a Border named
//!   PART_Header, the part FerroUI's item brings into view) above the items presenter
//!   PART_ItemsPresenter, shown while the item is expanded. The ContentPresenter of the header is
//!   PART_HeaderPresenter (the part that expands the item on a double tap).
//! - MUX_TreeViewItemStyle is based on DefaultListViewItemStyle, a style of ListViewItem; FerroUI's
//!   TreeViewItem is not a list box item, so the setters it takes from that style (font, foreground,
//!   navigation, alignments, minimum width, focus brushes) are written in the theme.
//! - The pointer states are those of the row: PointerOver is the pointer over PART_Header, Pressed
//!   the pressed item with the pointer over its row, so that the states of a child do not show on
//!   its parents.
//! - The indentation (TreeViewItemTemplateSettings.Indentation, 16 px per level in
//!   TreeViewItem::UpdateIndentation) is the padding of MultiSelectGrid, from the Level of the item
//!   with a MarginMultiplierConverter of an indent of 16.
//! - ExpandCollapseChevron (a Grid that expands and collapses the item when tapped) is a ToggleButton
//!   bound to IsExpanded with the keyed theme TreeViewItemExpandCollapseChevron; its glyphs
//!   CollapsedGlyph (E76C) and ExpandedGlyph (E70D), TextBlocks of 12 by 12 px with a padding of
//!   2 px in GlyphSize 8, are paths of the glyph geometries at the ink size of the 8 px glyphs, and
//!   show as the toggle is checked (CollapsedGlyphVisibility, ExpandedGlyphVisibility). GlyphOpacity
//!   (0 when the item has no children) is the opacity 0 of the chevron of an item without items
//!   (:empty); GlyphBrush is the foreground of the chevron.
//! - The selection indicator (a Rectangle with the radii 2) is a Border with a corner radius of 2.
//! - TreeViewMultiSelectStates (the check box of a tree in multiple selection mode), DragStates and
//!   ReorderedPlaceholder, with MultiSelectCheckBox and MultiArrangeOverlayTextBorder, have no
//!   counterpart: FerroUI's tree view has no check mode and no drag reordering.
//! - FocusVisualMargin="0,-1,0,-1" with the focus brushes of DefaultListViewItemStyle is the
//!   FocusAdorner of the row (the template focus target).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::CornerRadius;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::converters::MarginMultiplierConverter;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::HeaderedItemsControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::StackPanel;
use ferroui_controls::TreeViewItem;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // TreeViewItemTemplateSettings.Indentation: 16 px per level.
    resources.deferred("TreeViewItemIndentationConverter", |_| {
        Ok(value({
            let object = ::std::rc::Rc::new(MarginMultiplierConverter::new());
            object.set_left(true);
            object.set_indent(16.0);
            object
        }))
    });
    // ExpandCollapseChevron of the template of MUX_TreeViewItemStyle.
    resources.control_theme(
        "TreeViewItemExpandCollapseChevron",
        tree_view_item_expand_collapse_chevron,
    );
    resources.control_theme("MUX_TreeViewItemStyle", mux_tree_view_item_style);
    resources.control_theme(type_key::<TreeViewItem>(), tree_view_item_theme);
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

/// The template of `ToggleButton` of `tree_view_item_expand_collapse_chevron`.
fn tree_view_item_expand_collapse_chevron_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        // The 12 by 12 px glyph TextBlocks with a padding of 2 px.
        .child(
            Decorator::child_property(),
            t.make(Panel::new())
                .set(Layoutable::width_property(), 12.0)
                .set(Layoutable::height_property(), 12.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                // E76C at GlyphSize 8.
                // E70D at GlyphSize 8.
                .children([
                    t.make(Path::new())
                        .name("CollapsedGlyph")?
                        .static_resource(Path::data_property(), "GlyphGeometryE76C")?
                        .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 4.3)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .control(),
                    t.make(Path::new())
                        .name("ExpandedGlyph")?
                        .static_resource(Path::data_property(), "GlyphGeometryE70D")?
                        .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 7.9)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Visual::is_visible_property(), false)
                        .control(),
                ]),
        )
        .control())
}

/// `TreeViewItemExpandCollapseChevron`: a control theme of `ToggleButton`.
fn tree_view_item_expand_collapse_chevron(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.set(TemplatedControl::padding_property(), Thickness::symmetric(14.0, 0.0));
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.dynamic(TemplatedControl::foreground_property(), "TreeViewItemForeground");
    theme.set(InputElement::focusable_property(), false);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.template(tree_view_item_expand_collapse_chevron_template);
    // ExpandedGlyphVisibility, CollapsedGlyphVisibility
    theme.style(nesting().class(":checked"), |style| {
        style.style(nesting().template().of::<Path>().name("CollapsedGlyph"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("ExpandedGlyph"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The focus adorner of `mux_tree_view_item_style`.
fn mux_tree_view_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(0.0, -1.0, 0.0, -1.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualSecondaryBrush")
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `TreeViewItem` of `mux_tree_view_item_style`.
fn mux_tree_view_item_style_template(t: &Template<TreeViewItem>) -> Result<Ref<Control>> {
    Ok(t.make(StackPanel::new())
        // ContentPresenterGrid
        .children([
            t.make(Border::new())
                .name("PART_Header")?
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
                .dynamic(Layoutable::margin_property(), "TreeViewItemPresenterMargin")
                .dynamic(Decorator::padding_property(), "TreeViewItemPresenterPadding")
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
                .set(TemplatedControl::is_template_focus_target_property(), true)
                .child(
                    Decorator::child_property(),
                    t.make(Panel::new()).children([
                        t.make(Border::new())
                            .name("SelectionIndicator")?
                            .set(Layoutable::width_property(), 3.0)
                            .set(Layoutable::height_property(), 16.0)
                            .dynamic(
                                Border::background_property(),
                                "TreeViewItemSelectionIndicatorForeground",
                            )
                            .set(Visual::opacity_property(), 0.0)
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .set(Border::corner_radius_property(), CornerRadius::uniform(2.0))
                            .control(),
                        t.make(Border::new())
                            .name("MultiSelectGrid")?
                            .dynamic(
                                Layoutable::margin_property(),
                                "TreeViewItemMultiSelectSelectedItemBorderMargin",
                            )
                            .template_bind_with(
                                Decorator::padding_property(),
                                Some(TreeViewItem::level_property().property()),
                                Some(t.scope().converter("TreeViewItemIndentationConverter")?),
                                None,
                                None,
                            )
                            .set(
                                Border::border_brush_property(),
                                Some(literal(Brush::parse("Transparent"))?),
                            )
                            .dynamic(Border::border_thickness_property(), "TreeViewItemBorderThemeThickness")
                            .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
                            .child(
                                Decorator::child_property(),
                                t.make(Grid::new())
                                    .with(|e| {
                                        e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*"))?);
                                        Ok(())
                                    })?
                                    .children([
                                        t.make(ToggleButton::new())
                                            .name("ExpandCollapseChevron")?
                                            .set(Grid::column_property(), 0)
                                            .static_resource(
                                                StyledElement::theme_property(),
                                                "TreeViewItemExpandCollapseChevron",
                                            )?
                                            .template_bind_with(
                                                ToggleButton::is_checked_property(),
                                                Some(TreeViewItem::is_expanded_property().property()),
                                                None,
                                                Some(BindingMode::TwoWay),
                                                None,
                                            )
                                            .control(),
                                        t.make(ContentPresenter::new())
                                            .name("PART_HeaderPresenter")?
                                            .set(Grid::column_property(), 1)
                                            .template_bind(
                                                ContentPresenter::content_property(),
                                                HeaderedItemsControl::header_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::content_template_property(),
                                                HeaderedItemsControl::header_template_property(),
                                            )
                                            .template_bind(
                                                ContentPresenter::foreground_property(),
                                                TemplatedControl::foreground_property(),
                                            )
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Stretch,
                                            )
                                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                            .dynamic(Layoutable::min_height_property(), "TreeViewItemContentHeight")
                                            .template_bind(
                                                Layoutable::margin_property(),
                                                TemplatedControl::padding_property(),
                                            )
                                            .set(InputElement::focusable_property(), false)
                                            .control(),
                                    ]),
                            )
                            .control(),
                    ]),
                )
                .control(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(Visual::is_visible_property(), TreeViewItem::is_expanded_property())
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                )
                .control(),
        ])
        .control())
}

/// `MUX_TreeViewItemStyle`: a control theme of `TreeViewItem`.
fn mux_tree_view_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<TreeViewItem>();
    // The setters of DefaultListViewItemStyle.
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::foreground_property(), "ListViewItemForeground");
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    theme.dynamic(Layoutable::min_width_property(), "ListViewItemMinWidth");
    // The setters of MUX_TreeViewItemStyle.
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(TemplatedControl::background_property(), "TreeViewItemBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TreeViewItemBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TreeViewItemBorderThemeThickness",
    );
    theme.dynamic(Layoutable::min_height_property(), "TreeViewItemMinHeight");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), mux_tree_view_item_style_focus_adorner)),
    );
    theme.template(mux_tree_view_item_style_template);
    // PointerOver
    theme.style(
        nesting()
            .template()
            .of::<Border>()
            .name("PART_Header")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "TreeViewItemBorderBrushPointerOver");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundPointerOver",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundPointerOver",
                    );
                    style.set(Visual::opacity_property(), 0.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundPointerOver",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("PART_Header")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "TreeViewItemBorderBrushPressed");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(ContentPresenter::foreground_property(), "TreeViewItemForegroundPressed");
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundPressed",
                    );
                    style.set(Visual::opacity_property(), 0.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(TemplatedControl::foreground_property(), "TreeViewItemForegroundPressed");
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // Selected
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Border>()
            .name("PART_Header"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundSelected");
            style.dynamic(Border::border_brush_property(), "TreeViewItemBorderBrushSelected");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundSelected",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForeground",
                    );
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundSelected",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("PART_Header"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "TreeViewItemBorderBrushDisabled");
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundDisabled",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // PointerOverSelected
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Border>()
            .name("PART_Header")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TreeViewItemBackgroundSelectedPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "TreeViewItemBorderBrushSelectedPointerOver",
            );
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundSelectedPointerOver",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundPointerOver",
                    );
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundSelectedPointerOver",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // PressedSelected
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("PART_Header")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundSelectedPressed");
            style.dynamic(
                Border::border_brush_property(),
                "TreeViewItemBorderBrushSelectedPressed",
            );
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundSelectedPressed",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundPressed",
                    );
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundSelectedPressed",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // SelectedDisabled
    theme.style(
        nesting()
            .class(":selected")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("PART_Header"),
        |style| {
            style.dynamic(Border::background_property(), "TreeViewItemBackgroundSelectedDisabled");
            style.dynamic(
                Border::border_brush_property(),
                "TreeViewItemBorderBrushSelectedDisabled",
            );
            style.style(
                nesting()
                    .descendant()
                    .of::<ContentPresenter>()
                    .name("PART_HeaderPresenter"),
                |style| {
                    style.dynamic(
                        ContentPresenter::foreground_property(),
                        "TreeViewItemForegroundSelectedDisabled",
                    );
                    Ok(())
                },
            )?;
            style.style(
                nesting().descendant().of::<Border>().name("SelectionIndicator"),
                |style| {
                    style.dynamic(
                        Border::background_property(),
                        "TreeViewItemSelectionIndicatorForegroundDisabled",
                    );
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(
                nesting()
                    .descendant()
                    .of::<ToggleButton>()
                    .name("ExpandCollapseChevron"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "TreeViewItemForegroundSelectedDisabled",
                    );
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    // GlyphOpacity: 0 for an item without children.
    theme.style(
        nesting()
            .class(":empty")
            .template()
            .of::<ToggleButton>()
            .name("ExpandCollapseChevron"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `TreeViewItem`.
fn tree_view_item_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TreeViewItem>();
    theme.based_on("MUX_TreeViewItemStyle")?;
    Ok(())
}
