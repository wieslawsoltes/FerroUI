//! Source RibbonColorDialog.ShowAsync's retained content and HSL/hex state machine. Copyright
//! Wiesław Šoltés, MIT.
use crate::theming::{RibbonTheme, to_color, to_ribbon_color};
use ferroribbon_core::{RibbonString, theming::RibbonColor};
use ferroui_base::{
    CornerRadius, Ref, Thickness,
    interactivity::RoutedEventHandlerToken,
    layout::{HorizontalAlignment, Orientation},
    media::{Color, SolidColorBrush},
};
use ferroui_controls::{
    Border, Control, Slider, StackPanel, TextBlock, TextBox, primitives::RangeBase,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Private content only. No public ShowAsync or fabricated cancellation result is exposed.
pub(crate) struct RibbonColorDialogContent {
    pub(crate) root: Ref<StackPanel>,
    pub(crate) hue: Ref<Slider>,
    pub(crate) saturation: Ref<Slider>,
    pub(crate) lightness: Ref<Slider>,
    pub(crate) hex: Ref<TextBox>,
    pub(crate) preview: Ref<Border>,
    #[cfg(test)]
    pub(crate) original: Ref<Border>,
    current: Cell<Color>,
    updating: Cell<bool>,
    tokens: RefCell<Vec<(Ref<RangeBase>, RoutedEventHandlerToken)>>,
    text_token: RefCell<Option<RoutedEventHandlerToken>>,
}
fn swatch() -> Ref<Border> {
    let b = Border::new();
    b.set_width(96.);
    b.set_height(64.);
    b.set_corner_radius(CornerRadius::uniform(4.));
    b
}
fn slider(header: &str, maximum: f64, value: f64) -> Ref<Slider> {
    let s = Slider::new();
    s.set_minimum(0.);
    s.set_maximum(maximum);
    s.set_range_value(value);
    s.set_width(260.);
    ferroui_controls::automation::AutomationProperties::set_name(&s, Some(header));
    s
}
// The selected native Slider/TextBox lack the source Header property. Retained
// native label presenters preserve content.
fn labeled(header: &str, control: Ref<Control>) -> Ref<Control> {
    let label = TextBlock::new();
    label.set_text(Some(header));
    let panel = StackPanel::new();
    panel.children().add(label.upcast::<Control>());
    panel.children().add(control);
    panel.upcast::<Control>()
}
impl RibbonColorDialogContent {
    pub(crate) fn new(initial: Color) -> Rc<Self> {
        let (h, s, l) = to_ribbon_color(initial).to_hsl();
        let preview = swatch();
        preview.set_border_thickness(Thickness::uniform(1.));
        RibbonTheme::set_theme_brush(
            Some(&preview),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonSwatchBorderBrush"),
        )
        .expect("dialog preview theme brush");
        let original = swatch();
        original.set_background(Some(SolidColorBrush::with_color(initial).into()));
        let hue = slider("Hue", 360., h);
        let saturation = slider("Saturation", 100., s * 100.);
        let lightness = slider("Lightness", 100., l * 100.);
        let hex = TextBox::new();
        hex.set_text(Some(&to_ribbon_color(initial).to_hex(false)));
        hex.set_width(120.);
        hex.set_horizontal_alignment(HorizontalAlignment::Left);
        ferroui_controls::automation::AutomationProperties::set_name(&hex, Some("Hex"));
        let root = StackPanel::new();
        root.set_spacing(10.);
        for (name, control) in [
            ("Hue", hue.clone().upcast::<Control>()),
            ("Saturation", saturation.clone().upcast::<Control>()),
            ("Lightness", lightness.clone().upcast::<Control>()),
            ("Hex", hex.clone().upcast::<Control>()),
        ] {
            root.children().add(labeled(name, control));
        }
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(12.);
        row.children()
            .add(labeled("New", preview.clone().upcast::<Control>()));
        row.children()
            .add(labeled("Current", original.clone().upcast::<Control>()));
        root.children().add(row.upcast::<Control>());
        let this = Rc::new(Self {
            root,
            hue,
            saturation,
            lightness,
            hex,
            preview,
            #[cfg(test)]
            original,
            current: Cell::new(initial),
            updating: Cell::new(false),
            tokens: RefCell::default(),
            text_token: RefCell::default(),
        });
        for s in [&this.hue, &this.saturation, &this.lightness] {
            let weak = Rc::downgrade(&this);
            let token = s.value_changed(move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.update_from_sliders();
                }
            });
            this.tokens
                .borrow_mut()
                .push((s.clone().upcast::<RangeBase>(), token));
        }
        let weak = Rc::downgrade(&this);
        let token = this.hex.text_changed(move |_, _| {
            if let Some(c) = weak.upgrade() {
                c.update_from_hex();
            }
        });
        this.text_token.replace(Some(token));
        this.update_from_sliders();
        this
    }
    pub(crate) fn current(&self) -> Color {
        self.current.get()
    }
    fn update_from_sliders(&self) {
        if self.updating.get() {
            return;
        }
        self.updating.set(true);
        let current = to_color(RibbonColor::from_hsl(
            self.hue.value(),
            self.saturation.value() / 100.,
            self.lightness.value() / 100.,
        ));
        self.current.set(current);
        self.preview
            .set_background(Some(SolidColorBrush::with_color(current).into()));
        self.hex
            .set_text(Some(&to_ribbon_color(current).to_hex(false)));
        self.updating.set(false);
    }
    fn update_from_hex(&self) {
        if self.updating.get() {
            return;
        }
        let text = self.hex.text().map(RibbonString::from);
        let mut parsed = RibbonColor::default();
        if !RibbonColor::try_parse(text.as_ref(), &mut parsed) {
            return;
        }
        self.updating.set(true);
        let current = to_color(parsed);
        self.current.set(current);
        self.preview
            .set_background(Some(SolidColorBrush::with_color(current).into()));
        let (h, s, l) = parsed.to_hsl();
        self.hue.set_range_value(h);
        self.saturation.set_range_value(s * 100.);
        self.lightness.set_range_value(l * 100.);
        self.updating.set(false);
    }
}
impl Drop for RibbonColorDialogContent {
    fn drop(&mut self) {
        for (slider, token) in self.tokens.get_mut().drain(..) {
            slider.remove_handler(RangeBase::value_changed_event(), token);
        }
        if let Some(token) = self.text_token.get_mut().take() {
            self.hex
                .remove_handler(TextBox::text_changed_event(), token);
        }
    }
}
