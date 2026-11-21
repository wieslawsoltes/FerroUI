// SPDX-License-Identifier: Apache-2.0
// Copyright (c) Uno Platform Inc.
// Native FerroUI adaptation.
use super::{Symbol, symbol_glyphs::symbol_glyph};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledElementImpl, StyledProperty, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::{FontFamily, FontStyle, SolidColorBrush, TextAlignment},
};
use ferroui_controls::{
    ControlImpl, Grid, IconElement, TextBlock,
    automation::{AccessibilityView, AutomationProperties},
    primitives::{TemplatedControl, TemplatedControlImpl},
};
use std::cell::OnceCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolIconError {
    pub glyph: u16,
}
impl std::fmt::Display for SymbolIconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol glyph U+{:04X} is an unpaired UTF-16 unit unsupported by the native TextBlock String boundary",
            self.glyph
        )
    }
}
impl std::error::Error for SymbolIconError {}

#[repr(C)]
/// Native symbol control. The mapping retains one UTF-16 unit, but the
/// selected SDK's TextBlock text property requires a Rust string. Checked
/// setters return an error for an unpaired surrogate before changing Symbol.
/// This framework-only reproducer shows that boundary without a ribbon shim:
///
/// ```compile_fail,E0308
/// let text = ferroui_controls::TextBlock::new();
/// text.set_text(Some(&[0xd800_u16]));
/// ```
pub struct SymbolIcon {
    base: IconElement,
    initial_symbol: Symbol,
    root: OnceCell<Ref<Grid>>,
    text: OnceCell<Ref<TextBlock>>,
}
ferro_class!(SymbolIcon:IconElement);
ferro_class_info!(SymbolIcon {new:SymbolIcon::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_impl_classes!(SymbolIcon:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl SymbolIcon {
    pub fn symbol_property()->StyledProperty<Symbol> {FerroProperty::register::<Self,_>("Symbol",Symbol::Emoji)}
}}
impl SymbolIcon {
    pub const FONT_FAMILY: &'static str =
        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols";
    fn construct(symbol: Symbol) -> Self {
        Self {
            base: IconElement::construct(),
            initial_symbol: symbol,
            root: OnceCell::new(),
            text: OnceCell::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct(Symbol::Emoji))
    }
    pub fn with_symbol(symbol: Symbol) -> Result<Ref<Self>, SymbolIconError> {
        Self::glyph_text(symbol)?;
        Ok(instantiate(Self::construct(symbol)))
    }
    pub fn symbol(&self) -> Symbol {
        self.get_value(Self::symbol_property())
    }
    pub fn set_symbol(&self, value: Symbol) -> Result<(), SymbolIconError> {
        Self::glyph_text(value)?;
        self.set_value(Self::symbol_property(), value);
        Ok(())
    }
    pub const fn convert_symbol_value_to_glyph(value: i32) -> u16 {
        symbol_glyph(value)
    }
    fn glyph_text(value: Symbol) -> Result<String, SymbolIconError> {
        let glyph = symbol_glyph(value.raw());
        char::from_u32(u32::from(glyph))
            .map(|c| c.to_string())
            .ok_or(SymbolIconError { glyph })
    }
    fn set_symbol_text(&self) {
        if let Some(text) = self.text.get() {
            text.set_text(Some(
                &Self::glyph_text(self.symbol()).expect("SymbolIcon native text boundary"),
            ));
        }
    }
}
impl FerroObjectImpl for SymbolIcon {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let text = TextBlock::new();
        let root = Grid::new();
        root.set_background(Some(SolidColorBrush::from_uint32(0).into()));
        root.children().add(text.clone());
        root.set_parent(this.to_ref());
        this.visual_children().add(root.clone().upcast());
        this.logical_children().add(root.clone().upcast());
        this.root.set(root).expect("SymbolIcon initialized once");
        this.text
            .set(text.clone())
            .expect("SymbolIcon text initialized once");
        this.set_symbol(this.initial_symbol)
            .expect("validated SymbolIcon constructor");
        text.set_text_alignment(TextAlignment::Center);
        text.set_horizontal_alignment(HorizontalAlignment::Stretch);
        text.set_vertical_alignment(VerticalAlignment::Center);
        text.set_font_size(20.);
        text.set_font_style(FontStyle::Normal);
        text.set_font_family(FontFamily::new(Self::FONT_FAMILY));
        text.set_value(
            AutomationProperties::accessibility_view_property(),
            AccessibilityView::Raw,
        );
        this.set_symbol_text();
        text.set_foreground(this.foreground());
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == Self::symbol_property().as_property() {
            this.set_symbol_text();
        } else if e.property() == TemplatedControl::foreground_property().as_property()
            && let Some(text) = this.text.get()
        {
            text.set_foreground(this.foreground());
        }
    }
}
impl LayoutableImpl for SymbolIcon {
    fn apply_template(this: &Self) {
        assert!(
            this.template().is_none(),
            "SymbolIcon has no source Template property"
        );
    }
    fn measure_override(this: &Self, size: Size) -> Size {
        let root = this.root.get().expect("SymbolIcon root");
        root.measure(size);
        root.desired_size()
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        this.root
            .get()
            .expect("SymbolIcon root")
            .arrange(Rect::from_size(size));
        size
    }
}
