//! `Controls/EmbeddableControlRoot.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI 3 has no counterpart of FerroUI's EmbeddableControlRoot (the root
//! of FerroUI content hosted in another UI). The template is the one the control requires
//! (src/FerroUI.Themes.Fluent/Controls/EmbeddableControlRoot.xaml); the values are the resources of
//! a window of microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0), as Window.xaml (docs/porting/winui-theme.md,
//! section 6), and the placement of the text of text blocks in the height layout rounding adds,
//! as Window.xaml.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_controls::embedding::EmbeddableControlRoot;
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
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<EmbeddableControlRoot>(), embeddable_control_root_theme);
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

/// The template of `EmbeddableControlRoot` of `embeddable_control_root_theme`.
fn embeddable_control_root_theme_template(t: &Template<EmbeddableControlRoot>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(Border::new())
                .name("PART_TransparencyFallback")?
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
            t.make(Border::new())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .child(
                    Decorator::child_property(),
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
                                .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
                        ),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `EmbeddableControlRoot`.
fn embeddable_control_root_theme(theme: &StyleBuilder) -> Result {
    theme.target::<EmbeddableControlRoot>();
    theme.dynamic(TemplatedControl::foreground_property(), "TextFillColorPrimaryBrush");
    theme.dynamic(
        TemplatedControl::background_property(),
        "SolidBackgroundFillColorBaseBrush",
    );
    theme.dynamic(
        TopLevel::system_bar_color_property(),
        "SolidBackgroundFillColorBaseBrush",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(
        TextBlock::layout_rounding_placement_property(),
        TextLayoutRoundingPlacement::Above,
    );
    theme.template(embeddable_control_root_theme_template);
    Ok(())
}
