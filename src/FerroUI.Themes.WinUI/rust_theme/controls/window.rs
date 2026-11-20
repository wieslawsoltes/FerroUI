//! `Controls/Window.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: Window is not a control in WinUI 3 and has no style in
//! microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The template is the one FerroUI's Window requires
//! (src/FerroUI.Themes.Fluent/Controls/Window.xaml); the values are the resources a WinUI 3 window
//! shows (docs/porting/winui-theme.md, section 6): the background of a window without its Mica
//! backdrop (SolidBackgroundFillColorBaseBrush, which WinUI's MicaBackdrop falls back to), the
//! primary text colour, and the font of the controls (ContentControlThemeFontFamily,
//! ControlContentThemeFontSize of dxaml/xcp/dxaml/themes/generic.xaml). The text blocks of the window
//! put the height layout rounding adds to them above their text, as WinUI's TextBlock does
//! (TextBlock.LayoutRoundingPlacement, inherited; docs/porting/text-rendering.md, section 6).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::Brush;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::VisualLayerManager;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_controls::TextBlock;
use ferroui_controls::TextLayoutRoundingPlacement;
use ferroui_controls::TopLevel;
use ferroui_controls::Window;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<Window>(), window_theme);
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

/// The template of `Window` of `window_theme`.
fn window_theme_template(t: &Template<Window>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(Border::new())
                .name("PART_TransparencyFallback")?
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
            t.make(Border::new())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
            t.make(Panel::new())
                .set(
                    Panel::background_property(),
                    Some(literal(Brush::parse("Transparent"))?),
                )
                .template_bind(
                    Layoutable::margin_property(),
                    Window::window_decoration_margin_property(),
                )
                .control(),
            t.make(VisualLayerManager::new())
                .name("PART_VisualLayerManager")?
                .child(
                    Decorator::child_property(),
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            ContentPresenter::horizontal_content_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentPresenter::vertical_content_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        ),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `Window`.
fn window_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Window>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "SolidBackgroundFillColorBaseBrush",
    );
    theme.dynamic(
        TopLevel::transparency_background_fallback_property(),
        "SolidBackgroundFillColorBaseBrush",
    );
    theme.dynamic(
        TopLevel::system_bar_color_property(),
        "SolidBackgroundFillColorBaseBrush",
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
    theme.template(window_theme_template);
    // A window with a system backdrop (Mica, desktop acrylic) draws no background of its own:
    // the backdrop is drawn behind the content (docs/porting/winui-materials.md).
    theme.style(nesting().class(":system-backdrop"), |style| {
        style.set(
            TemplatedControl::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        );
        Ok(())
    })?;
    Ok(())
}
