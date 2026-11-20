//! `Controls/PopupRoot.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: the popup host of WinUI 3 is part of the framework and has no style in
//! microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The template is the one FerroUI's PopupRoot requires
//! (src/FerroUI.Themes.Fluent/Controls/PopupRoot.xaml); the text values are the resources of a
//! WinUI 3 window, so that a popup shows text as the window does (docs/porting/winui-theme.md,
//! section 6), with the placement of the text of text blocks in the height layout rounding adds of
//! Window.xaml. The presenter of the popup (FlyoutPresenter, MenuFlyoutPresenter, ToolTip) draws
//! its background.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::media::FontStyle;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextDecorationCollection;
use ferroui_controls::documents::Inline;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::PopupRoot;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::VisualLayerManager;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::LayoutTransformControl;
use ferroui_controls::Panel;
use ferroui_controls::TextBlock;
use ferroui_controls::TextLayoutRoundingPlacement;
use ferroui_controls::TopLevel;
use ferroui_controls::WindowTransparencyLevelCollection;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<PopupRoot>(), popup_root_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `PopupRoot` of `popup_root_theme`.
fn popup_root_theme_template(t: &Template<PopupRoot>) -> Result<Ref<Control>> {
    Ok(t.make(LayoutTransformControl::new())
        .template_bind(
            LayoutTransformControl::layout_transform_property(),
            PopupRoot::transform_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Panel::new()).children([
                t.make(Border::new())
                    .name("PART_TransparencyFallback")?
                    .set(InputElement::is_hit_test_visible_property(), false)
                    .control(),
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
                    )
                    .control(),
            ]),
        )
        .control())
}

/// The default control theme of `PopupRoot`.
fn popup_root_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PopupRoot>();
    theme.set_null(TemplatedControl::background_property());
    theme.set(
        TopLevel::transparency_level_hint_property(),
        literal(WindowTransparencyLevelCollection::__markup_parse(String::from(
            "Transparent",
        )))?,
    );
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
    theme.template(popup_root_theme_template);
    Ok(())
}
