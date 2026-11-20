//! `Controls/TableViewRow.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TableView/TableView.xaml (the implicit TableViewRow style)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The resources are Resources/TableView_themeresources.xaml and
//! Resources/TabularSurfaces_themeresources.xaml (TableView.xaml).
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's TableViewRow is a ListBoxItem. CommonStates (Normal, PointerOver, Pressed, Disabled,
//!   Selected, SelectedPointerOver, SelectedPressed, SelectedDisabled) are the pseudo-classes
//!   :pointerover, :pressed, :disabled and :selected and their combinations, in the order of the
//!   source.
//! - PART_CellsHost (the TableViewCellsPanel the row fills with the cells of the columns) is
//!   FerroUI's PART_CellsPresenter (TableViewCellsPresenter). PART_CellForegroundPresenter, the
//!   ContentPresenter that carries the foreground the states set to the text of the cells, is a
//!   Panel whose inherited text foreground (TextElement.Foreground) the states set.
//! - The opacity of PART_SelectionIndicator, a LinearDoubleKeyFrame to 1 at 83 ms in the Selected
//!   states, is a setter with a transition of 83 ms; the return to 0 animates too.
//! - HighContrastAdjustment, the hierarchy (PART_RowExpanderGutter, PART_RowExpanderIcon,
//!   RowExpansionStates, RowExpandabilityStates, RowHierarchyStates) and the focus states of the
//!   cells have no counterpart in FerroUI's TableViewRow and are left out. UseSystemFocusVisuals is
//!   the default focus adorner.
//! - As the Fluent template of FerroUI, the template honours the corner radius, the padding and the
//!   content alignments of the row (the alignment of PART_RootBorder); the CornerRadius setter of the
//!   source, which its template ignores, is left out so that the row keeps its square corners.

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
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::documents::TextElement;
use ferroui_controls::presenters::TableViewCellsPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_controls::TableViewRow;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<TableViewRow>(), table_view_row_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `TableViewRow` of `table_view_row_theme`.
fn table_view_row_theme_template(t: &Template<TableViewRow>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        // The horizontal grid line of the row.
        .children([
            t.make(Border::new())
                .name("PART_RootBorder")?
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                )
                .child(
                    Decorator::child_property(),
                    t.make(Panel::new())
                        // The foreground of the row, inherited, unless a state sets another.
                        // The selection indicator of Fluent: over the leading edge, shown by the Selected
                        //                   states.
                        .children([
                            t.make(Panel::new())
                                .name("PART_CellForegroundPresenter")?
                                .children([t
                                    .make(TableViewCellsPresenter::new())
                                    .name("PART_CellsPresenter")?
                                    .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                    .control()])
                                .control(),
                            t.make(Rectangle::new())
                                .name("PART_SelectionIndicator")?
                                .set(Layoutable::width_property(), 3.0)
                                .set(Layoutable::height_property(), 16.0)
                                .set(Rectangle::radius_x_property(), 1.5)
                                .set(Rectangle::radius_y_property(), 1.5)
                                .dynamic(Shape::fill_property(), "TabularSurfaceSelectionIndicatorBrush")
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                                .set(Layoutable::margin_property(), Thickness::new(2.0, 0.0, 0.0, 0.0))
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .set(Visual::opacity_property(), 0.0)
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
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                        ]),
                )
                .control(),
            t.make(Border::new())
                .name("PART_GridLineBorder")?
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
        ])
        .control())
}

/// The default control theme of `TableViewRow`.
fn table_view_row_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TableViewRow>();
    // CornerRadius (ControlCornerRadius in the source, which its template does not use) is not
    //         set: the template honours the corner radius of the row.
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.dynamic(Layoutable::min_height_property(), "TableViewRowMinHeight");
    theme.dynamic(
        TemplatedControl::background_property(),
        "TabularSurfaceRowBackgroundBrush",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "TabularSurfaceGridLineBrush");
    theme.set(
        TemplatedControl::border_thickness_property(),
        Thickness::new(0.0, 0.0, 0.0, 1.0),
    );
    theme.template(table_view_row_theme_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundPointerOverBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundPointerOverBrush",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(Border::background_property(), "TabularSurfaceRowBackgroundPressedBrush");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundPressedBrush",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundDisabledBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundDisabledBrush",
            );
            Ok(())
        },
    )?;
    // Selected
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundSelectedBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundSelectedBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Rectangle>()
            .name("PART_SelectionIndicator"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        },
    )?;
    // SelectedPointerOver
    theme.style(
        nesting()
            .class(":selected")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundSelectedPointerOverBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pointerover")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundSelectedPointerOverBrush",
            );
            Ok(())
        },
    )?;
    // SelectedPressed
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundSelectedPressedBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundSelectedPressedBrush",
            );
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
            .name("PART_RootBorder"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "TabularSurfaceRowBackgroundDisabledBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":disabled")
            .template()
            .of::<Panel>()
            .name("PART_CellForegroundPresenter"),
        |style| {
            style.dynamic(
                TextElement::foreground_property(),
                "TabularSurfaceRowForegroundDisabledBrush",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":disabled")
            .template()
            .of::<Rectangle>()
            .name("PART_SelectionIndicator"),
        |style| {
            style.dynamic(Shape::fill_property(), "TabularSurfaceSelectionIndicatorDisabledBrush");
            Ok(())
        },
    )?;
    Ok(())
}
