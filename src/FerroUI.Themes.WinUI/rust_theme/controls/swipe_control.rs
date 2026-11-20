//! `Controls/SwipeControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/SwipeControl/SwipeControl.xaml (DefaultSwipeControlStyle and the implicit style)
//!   controls/dev/SwipeControl/SwipeControl_themeresources.xaml (SwipeItemStyle; its resources are
//!   Resources/SwipeControl_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's SwipeControl, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-calculator-gaps.md, SW-1); the Fluent theme has no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The parts keep their names: RootGrid, SwipeContentRoot, SwipeContentStackPanel, ContentRoot,
//!   ContentPresenter and InputEater. The content presenter's background, border and corner radius
//!   are template bindings, as in WinUI; ContentTransitions is not a FerroUI property.
//! - ListViewItemMinHeight and ListViewItemMinWidth are the sizes of WinUI's list items (40 and 88);
//!   the theme has no ListView yet, so the values are written here.
//! - SwipeItemStyle targets CommandBarButton, FerroUI's AppBarButton: the control makes one button
//!   per SwipeItem with this theme. Its Pressed state (the Root background
//!   SwipeItemBackgroundPressed) is the :pressed pseudo-class. AutomationProperties.AccessibilityView
//!   Raw is kept on the label.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::Thickness;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::CommandBarButton;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::RowDefinitions;
use ferroui_controls::StackPanel;
use ferroui_controls::SwipeControl;
use ferroui_controls::TextBlock;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("SwipeItemStyle", swipe_item_style);
    resources.control_theme("DefaultSwipeControlStyle", default_swipe_control_style);
    resources.control_theme(type_key::<SwipeControl>(), swipe_control_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `CommandBarButton` of `swipe_item_style`.
fn swipe_item_style_template(t: &Template<CommandBarButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .name("ContentRoot")?
                .set(Layoutable::margin_property(), Thickness::new(4.0, 4.0, 4.0, 2.0))
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .with(|e| {
                    Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*"))?);
                    Ok(())
                })?
                // Font Size 12pt == 16px in the redlines
                .children([
                    t.make(Viewbox::new())
                        .set(Layoutable::max_height_property(), 16.0)
                        .set(Layoutable::min_width_property(), 16.0)
                        .child(
                            Viewbox::child_property(),
                            t.make(ContentPresenter::new())
                                .name("Content")?
                                .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 0.0, 2.0))
                                .template_bind(ContentPresenter::content_property(), CommandBarButton::icon_property())
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                ),
                        )
                        .control(),
                    t.make(TextBlock::new())
                        .name("TextLabel")?
                        .set(Grid::row_property(), 1)
                        .template_bind(TextBlock::text_property(), CommandBarButton::label_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .set(TextBlock::font_size_property(), 12.0)
                        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
                        .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                        .set(
                            AutomationProperties::accessibility_view_property(),
                            AccessibilityView::Raw,
                        )
                        .control(),
                ]),
        )
        .control())
}

/// `SwipeItemStyle`: a control theme of `CommandBarButton`.
fn swipe_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<CommandBarButton>();
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(Layoutable::min_width_property(), 68.0);
    theme.set(Layoutable::width_property(), f64::NAN);
    theme.set(Layoutable::min_height_property(), 40.0);
    theme.template(swipe_item_style_template);
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("Root"),
        |style| {
            style.dynamic(Border::background_property(), "SwipeItemBackgroundPressed");
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `SwipeControl` of `default_swipe_control_style`.
fn default_swipe_control_style_template(t: &Template<SwipeControl>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .children([
            t.make(Grid::new())
                .name("SwipeContentRoot")?
                .children([t.make(StackPanel::new()).name("SwipeContentStackPanel")?.control()])
                .control(),
            t.make(Grid::new())
                .name("ContentRoot")?
                .children([
                    t.make(ContentPresenter::new())
                        .name("ContentPresenter")?
                        .template_bind(
                            ContentPresenter::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            ContentPresenter::border_brush_property(),
                            TemplatedControl::border_brush_property(),
                        )
                        .template_bind(
                            ContentPresenter::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            ContentPresenter::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .template_bind(
                            ContentPresenter::padding_property(),
                            TemplatedControl::padding_property(),
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .set(
                            ContentPresenter::horizontal_content_alignment_property(),
                            HorizontalAlignment::Stretch,
                        )
                        .set(
                            ContentPresenter::vertical_content_alignment_property(),
                            VerticalAlignment::Stretch,
                        )
                        .set(
                            AutomationProperties::accessibility_view_property(),
                            AccessibilityView::Raw,
                        )
                        .control(),
                    t.make(Grid::new()).name("InputEater")?.control(),
                ])
                .control(),
        ])
        .control())
}

/// `DefaultSwipeControlStyle`: a control theme of `SwipeControl`.
fn default_swipe_control_style(theme: &StyleBuilder) -> Result {
    theme.target::<SwipeControl>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(Layoutable::min_height_property(), 40.0);
    theme.set(Layoutable::min_width_property(), 88.0);
    theme.template(default_swipe_control_style_template);
    Ok(())
}

/// The default control theme of `SwipeControl`.
fn swipe_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<SwipeControl>();
    theme.based_on("DefaultSwipeControlStyle")?;
    Ok(())
}
