//! `Controls/OverlayPopupHost.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: the popup host of WinUI 3 is part of the framework and has no style in
//! microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The template is the one FerroUI's OverlayPopupHost
//! requires (src/FerroUI.Themes.Fluent/Controls/OverlayPopupHost.xaml); the text values are the
//! resources of a WinUI 3 window and the placement of the text of text blocks, as PopupRoot.xaml
//! (docs/porting/winui-theme.md, section 6).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::FontStyle;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextDecorationCollection;
use ferroui_controls::documents::Inline;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::OverlayPopupHost;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::VisualLayerManager;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::LayoutTransformControl;
use ferroui_controls::TextBlock;
use ferroui_controls::TextLayoutRoundingPlacement;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<OverlayPopupHost>(), overlay_popup_host_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `OverlayPopupHost` of `overlay_popup_host_theme`.
fn overlay_popup_host_theme_template(t: &Template<OverlayPopupHost>) -> Result<Ref<Control>> {
    Ok(t.make(LayoutTransformControl::new())
        .template_bind(
            LayoutTransformControl::layout_transform_property(),
            OverlayPopupHost::transform_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(VisualLayerManager::new())
                .name("PART_VisualLayerManager")?
                .child(
                    Decorator::child_property(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .template_bind(
                            ContentPresenter::background_property(),
                            TemplatedControl::background_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::padding_property(),
                            TemplatedControl::padding_property(),
                        ),
                ),
        )
        .control())
}

/// The default control theme of `OverlayPopupHost`.
fn overlay_popup_host_theme(theme: &StyleBuilder) -> Result {
    theme.target::<OverlayPopupHost>();
    theme.dynamic(TemplatedControl::foreground_property(), "TextFillColorPrimaryBrush");
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(
        TextBlock::layout_rounding_placement_property(),
        TextLayoutRoundingPlacement::Above,
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(TemplatedControl::font_style_property(), FontStyle::Normal);
    theme.set(
        Inline::text_decorations_property(),
        Some(literal(TextDecorationCollection::parse(""))?),
    );
    theme.template(overlay_popup_host_theme_template);
    Ok(())
}
