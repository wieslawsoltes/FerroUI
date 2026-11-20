//! `Controls/ItemsControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: ItemsControl has no style in the merged theme of microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0); its default template is built in by the framework
//! and is an ItemsPresenter of the items in the ItemsPanel of the control, without background or
//! border. This is that template as FerroUI's ItemsControl requires it: PART_ItemsPresenter with
//! the ItemsPanel of the control, in a border of the Background, BorderBrush, BorderThickness,
//! CornerRadius and Padding of the control (none of them is set, so the border draws nothing by
//! default; a FerroUI application may set them, as for the ItemsControl of FerroUI's other
//! themes).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::BackgroundSizing;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ItemsControl>(), items_control_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ItemsControl` of `items_control_theme`.
fn items_control_theme_template(t: &Template<ItemsControl>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                ),
        )
        .control())
}

/// The default control theme of `ItemsControl`.
fn items_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ItemsControl>();
    theme.template(items_control_theme_template);
    Ok(())
}
