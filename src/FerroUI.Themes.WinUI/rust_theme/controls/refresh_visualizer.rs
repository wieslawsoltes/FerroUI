//! `Controls/RefreshVisualizer.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/PullToRefresh/RefreshVisualizer/RefreshVisualizer.xaml (the implicit style; the
//!   resources of controls/dev/PullToRefresh/RefreshVisualizer/RefreshVisualizer_themeresources.xaml
//!   are Resources/RefreshVisualizer_themeresources.xaml)
//!   controls/dev/PullToRefresh/RefreshVisualizer/RefreshVisualizer.cpp (the default content)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Root is PART_Root, the grid into which FerroUI's RefreshVisualizer puts its content, as
//!   WinUI's.
//! - The default content: WinUI's control creates a SymbolIcon of Symbol.Refresh (the glyph E72C
//!   of the symbol font at the 20 px of a SymbolIcon) of 30 x 30 when it has no content. FerroUI's
//!   control creates none; the theme sets it, as the Fluent theme does: a Grid of 30 x 30 with a
//!   Path of GlyphGeometryE72C, 16 px wide, filled with the foreground of the visualizer (which a
//!   SymbolIcon inherits).
//! - The FerroUI contract: the visualizer is not hit-testable (IsHitTestVisible="False"), as in the
//!   Fluent theme, so that it does not take the input of the content it lies over.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Stretch;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::RefreshVisualizer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<RefreshVisualizer>(), refresh_visualizer_theme);
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

/// The value of `Content` of `refresh_visualizer_theme`, made for each control.
fn refresh_visualizer_theme_content_template(t: &Template<Control>) -> Result<Option<ferroui_base::BoxedValue>> {
    Ok(value(
        t.make(Grid::new())
            .set(Layoutable::width_property(), 30.0)
            .set(Layoutable::height_property(), 30.0)
            .children([t
                .make(Path::new())
                .static_resource(Path::data_property(), "GlyphGeometryE72C")?
                .bind(
                    Shape::fill_property(),
                    path()
                        .ancestor_of::<RefreshVisualizer>(0)
                        .prop(TemplatedControl::foreground_property())
                        .binding(),
                )
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 16.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .control()])
            .end(),
    ))
}

/// The template of `RefreshVisualizer` of `refresh_visualizer_theme`.
fn refresh_visualizer_theme_template(t: &Template<RefreshVisualizer>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("PART_Root")?
        .set(Layoutable::min_height_property(), 80.0)
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .control())
}

/// The default control theme of `RefreshVisualizer`.
fn refresh_visualizer_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RefreshVisualizer>();
    theme.dynamic(TemplatedControl::background_property(), "RefreshVisualizerBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "RefreshVisualizerForeground");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(InputElement::is_hit_test_visible_property(), false);
    theme.set(Layoutable::height_property(), 100.0);
    // The SymbolIcon(Symbol.Refresh) of 30 x 30 the control of WinUI creates.
    theme.set_template(
        ContentControl::content_property(),
        refresh_visualizer_theme_content_template,
    );
    theme.template(refresh_visualizer_theme_template);
    Ok(())
}
