//! What the tests of the WinUI icons (`font_icon_tests`, `symbol_icon_tests`,
//! `icon_source_tests`) share: the text part of the template of the WinUI theme, and fonts.
//!
//! The controls have no template of their own (the WinUI theme has it,
//! `src/FerroUI.Themes.WinUI/Controls/FontIcon.xaml` and `SymbolIcon.xaml`), so a test gives an
//! icon the part of that template the control relies on: a text block named `PART_Text`.

use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{IconElement, TextBlock};
use ferroui_base::layout::{Layoutable, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::{TestFont, TextTestScope};
use ferroui_base::media::TextAlignment;
use ferroui_base::{Rect, Ref, Size};

/// The fonts of the tests: one named as the symbol font of the WinUI theme and another.
pub(crate) fn fonts() -> TextTestScope {
    TextTestScope::with_fonts(vec![
        TestFont::new("Segoe Fluent Icons").with_ranges(&[(0x20, 0xF8FF)]),
        TestFont::new("Custom Icons").with_ranges(&[(0x20, 0xF8FF)]),
    ])
}

/// Gives `icon` the text part of the template of the WinUI theme.
pub(crate) fn apply_text_template(icon: &TemplatedControl) {
    icon.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let text = TextBlock::new();
        text.set_name(Some("PART_Text".to_string()));
        text.set_text_alignment(TextAlignment::Center);
        text.set_vertical_alignment(VerticalAlignment::Center);
        AutomationProperties::set_accessibility_view(&text, AccessibilityView::Raw);
        text.register_in_name_scope(&**scope).upcast()
    })));
}

/// The text part of the template of `icon`.
pub(crate) fn text_part(icon: &IconElement) -> Ref<TextBlock> {
    icon.visual_children_snapshot().expect("the template is applied")[0].cast::<TextBlock>().expect("PART_Text")
}

/// Measures and arranges `control` at its desired size.
pub(crate) fn layout(control: &Layoutable) {
    control.measure(Size::new(1000.0, 1000.0));
    let size = control.desired_size();
    control.arrange(Rect::new(0.0, 0.0, size.width, size.height));
}
