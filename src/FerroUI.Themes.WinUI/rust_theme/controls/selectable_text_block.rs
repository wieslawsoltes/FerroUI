//! `Controls/SelectableTextBlock.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI has no SelectableTextBlock; a TextBlock of WinUI with
//! IsTextSelectionEnabled="True" is selectable, and it has no style in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). Its selection is drawn by the text selection
//! manager of the framework (dxaml/xcp/core/text/common/TextSelectionManager.cpp): the background
//! is the SelectionHighlightColor of the TextBlock, whose default is the accent colour of the
//! system (GetDefaultSelectionHighlightColor), and in a high contrast theme the highlight colour
//! of the system; the selected text is drawn in the highlight text colour of the system
//! (GetSystemTextSelectionForegroundBrush). Over selectable text the pointer is the I-beam.
//!
//! This theme gives FerroUI's SelectableTextBlock those values: SelectionBrush is
//! SystemControlHighlightAccentBrush (SystemAccentColor, and SystemColorHighlightColor in the
//! HighContrast dictionary), SelectionForegroundBrush is SystemColorHighlightTextColor, and the
//! cursor of an enabled block is IBeam.
//!
//! Pending (docs/porting/winui-theme.md, section 7, batch B3): the context flyout. WinUI's is the
//! TextCommandBarFlyout of the text controls (waived); FerroUI's text controls get a menu flyout
//! of the same commands with the menu flyouts of batch B3, the text box and this block together.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_controls::SelectableTextBlock;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<SelectableTextBlock>(), selectable_text_block_theme);
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

/// The default control theme of `SelectableTextBlock`.
fn selectable_text_block_theme(theme: &StyleBuilder) -> Result {
    theme.target::<SelectableTextBlock>();
    theme.dynamic(
        SelectableTextBlock::selection_brush_property(),
        "SystemControlHighlightAccentBrush",
    );
    theme.dynamic(
        SelectableTextBlock::selection_foreground_brush_property(),
        "SystemColorHighlightTextColor",
    );
    theme.style(
        nesting().property_is(InputElement::is_enabled_property(), true),
        |style| {
            style.set(InputElement::cursor_property(), Some(literal(Cursor::parse("IBeam"))?));
            Ok(())
        },
    )?;
    Ok(())
}
