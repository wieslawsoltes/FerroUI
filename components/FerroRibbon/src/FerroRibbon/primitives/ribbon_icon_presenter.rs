//! Supported native branches of the original RibbonIconPresenter.cs.
use crate::compat::{
    BitmapIcon, BitmapIconSource, FontIcon, FontIconSource, IconSource, ImageIcon, ImageIconSource,
    PathIconSource, SymbolIcon, SymbolIconSource,
};
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind, RibbonIconLayer},
    text::{digit, ordinal_equals_ascii, trim},
    theming::RibbonColor,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{
    FontFamily, FontWeight, Geometry, IBrush, IImage, PenLineCap, PenLineJoin, SolidColorBrush,
    Stretch, StretchDirection, TextAlignment,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, instantiate,
};
use ferroui_controls::Application;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    Canvas, Control, ControlImpl, Grid, IconElement, Image, PanelImpl, PathIcon, TextBlock, Viewbox,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonIconPresenter {
    base: Grid,
    element: RefCell<Option<Ref<Control>>>,
    fixed_foreground: Cell<bool>,
}
ferro_class!(RibbonIconPresenter:Grid);
ferro_impl_classes!(RibbonIconPresenter:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonIconPresenter {
    new:RibbonIconPresenter::new,
    markup:{namespace:"FerroRibbon.Controls.Primitives",content:Children,properties:[
        HasIcon:bool {get:RibbonIconPresenter::has_icon},
    ]},
});
ferro_properties! {impl RibbonIconPresenter {
    pub fn icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<RibbonIconPresenter,_>("Icon",None)}
    pub fn icon_size_property()->StyledProperty<f64> {FerroProperty::register::<RibbonIconPresenter,_>("IconSize",16.)}
    pub fn foreground_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<RibbonIconPresenter,_>("Foreground",None)}
}}
ferroui_base::ferro_thread_local! {
    static THEMED_FILL:BoxedValue=Rc::new(());
    static THEMED_STROKE:BoxedValue=Rc::new(());
}
impl RibbonIconPresenter {
    fn static_constructor() {
        Layoutable::horizontal_alignment_property()
            .override_default_value::<Self>(HorizontalAlignment::Center);
        Self::icon_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.rebuild())
            .detach();
        Self::icon_size_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.rebuild())
            .detach();
        Self::foreground_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.apply_foreground())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: Grid::construct(),
            element: RefCell::new(None),
            fixed_foreground: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }
    pub fn icon_size(&self) -> f64 {
        self.get_value(Self::icon_size_property())
    }
    pub fn set_icon_size(&self, value: f64) {
        self.set_value(Self::icon_size_property(), value)
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }
    pub fn has_icon(&self) -> bool {
        self.element.borrow().is_some()
    }

    fn rebuild(&self) {
        // Release the child before discarding its box. Reusing the same icon on a
        // size change otherwise fails both the logical and visual parent checks.
        let old = self.element.borrow().clone();
        if let Some(old) = old.and_then(|e| e.cast::<Viewbox>()) {
            old.set_child(None);
        }
        self.children().clear();
        self.element.replace(None);
        self.fixed_foreground.set(false);
        let size = self.icon_size();
        let Some(icon) = self.icon() else {
            self.set_width(0.);
            self.set_height(0.);
            return;
        };
        self.set_width(size);
        self.set_height(size);
        let (element, fixed) = create_element(&icon, size);
        self.fixed_foreground.set(fixed);
        self.element.replace(element.clone());
        if let Some(element) = element {
            self.children().add(element);
        }
        self.apply_foreground();
    }
    fn apply_foreground(&self) {
        if self.fixed_foreground.get() {
            return;
        }
        let Some(element) = self.element.borrow().clone() else {
            return;
        };
        let Some(brush) = self.foreground() else {
            return;
        };
        if apply_brush(&element, &brush) {
            return;
        }
        if let Some(viewbox) = element.cast::<Viewbox>()
            && let Some(child) = viewbox.child()
        {
            if apply_brush(&child, &brush) {
                return;
            }
            if let Some(canvas) = child.cast::<Canvas>() {
                for child in canvas.children().snapshot().iter() {
                    if let Some(path) = child.cast::<Path>()
                        && let Some(tag) = path.tag()
                    {
                        if THEMED_STROKE.with(|sentinel| Rc::ptr_eq(&tag, sentinel)) {
                            path.set_stroke(Some(brush.clone()));
                        } else if THEMED_FILL.with(|sentinel| Rc::ptr_eq(&tag, sentinel)) {
                            path.set_fill(Some(brush.clone()));
                        }
                    }
                }
            }
        }
    }
    /// Original CLR string classifier, including UTF-16 lengths and lone surrogates.
    pub fn classify(value: &RibbonString) -> RibbonIconKind {
        let text = trim(value.as_utf16());
        let starts = |ascii: &str| {
            text.get(..ascii.len())
                .is_some_and(|s| ordinal_equals_ascii(s, ascii))
        };
        let ends = |ascii: &str| {
            text.len()
                .checked_sub(ascii.len())
                .is_some_and(|at| ordinal_equals_ascii(&text[at..], ascii))
        };
        if starts("ms-appx:")
            || starts("http")
            || starts("ms-appdata:")
            || ends(".png")
            || ends(".svg")
        {
            return RibbonIconKind::Image;
        }
        if text.len() > 6
            && (matches!(text[0], 77 | 109 | 70)
                || text[0] == 123 && text.contains(&125)
                || text[0] == 91 && text.contains(&93))
            && text.iter().any(|&c| digit(c))
        {
            return RibbonIconKind::Path;
        }
        if text.len() <= 2
            && text
                .iter()
                // The two source ranges (surrogates and private-use) are adjacent.
                .all(|&c| matches!(c, 0xd800..=0xf8ff))
        {
            RibbonIconKind::Glyph
        } else {
            RibbonIconKind::Text
        }
    }
    /// Layered geometry built through the selected FerroUI path parser. Its grammar
    /// has not yet been proven identical to WinUI XamlReader (see the parity ledger).
    pub fn create_path(
        data: Option<&RibbonString>,
        view_box_size: f64,
        size: f64,
        fill: Option<Rc<dyn IBrush>>,
    ) -> Ref<Control> {
        let canvas = Canvas::new();
        canvas.set_width(view_box_size);
        canvas.set_height(view_box_size);
        for layer in RibbonIconLayer::parse(data).iter() {
            let layer = layer.expect("source parsed layer");
            let data = layer.data().as_ref().map(utf8).unwrap_or_default();
            let Ok(geometry) = Geometry::parse(&data) else {
                continue;
            };
            let path = Path::new();
            path.set_data(geometry);
            let layer_brush = color_brush(layer.color().as_ref());
            let brush = layer_brush.clone().or_else(|| fill.clone());
            if layer.stroke_thickness() > 0. {
                path.set_stroke_thickness(layer.stroke_thickness());
                path.set_stroke_line_cap(PenLineCap::Round);
                path.set_stroke_join(PenLineJoin::Round);
                path.set_fill(None);
                path.set_stroke(brush);
                if layer_brush.is_none() {
                    path.set_tag(Some(THEMED_STROKE.with(Clone::clone)));
                }
            } else {
                path.set_fill(brush);
                if layer_brush.is_none() {
                    path.set_tag(Some(THEMED_FILL.with(Clone::clone)));
                }
            }
            if layer.opacity() < 1. {
                path.set_opacity(layer.opacity());
            }
            canvas.children().add(path);
        }
        boxed(canvas.upcast(), size, StretchDirection::Both).upcast()
    }
}
impl FerroObjectImpl for RibbonIconPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_hit_test_visible(false);
        this.set_vertical_alignment(VerticalAlignment::Center);
        // Original constructor does not rebuild: Width/Height remain Auto here.
    }
}
fn utf8(value: &RibbonString) -> String {
    value.to_utf8().expect("FerroRibbon icon rendering: FerroUI UTF-8 text/geometry cannot represent an unpaired UTF-16 surrogate")
}
fn color_brush(value: Option<&RibbonString>) -> Option<Rc<dyn IBrush>> {
    let mut color = RibbonColor::default();
    RibbonColor::try_parse(value, &mut color).then(|| {
        SolidColorBrush::from_uint32(u32::from_be_bytes([color.a, color.r, color.g, color.b]))
            .into()
    })
}
fn boxed(child: Ref<Control>, size: f64, direction: StretchDirection) -> Ref<Viewbox> {
    let viewbox = Viewbox::new();
    viewbox.set_width(size);
    viewbox.set_height(size);
    viewbox.set_stretch(Stretch::Uniform);
    viewbox.set_stretch_direction(direction);
    viewbox.set_child(child);
    viewbox
}
fn apply_brush(element: &Ref<Control>, brush: &Rc<dyn IBrush>) -> bool {
    if let Some(icon) = element.cast::<IconElement>() {
        icon.set_foreground(Some(brush.clone()));
        true
    } else if let Some(text) = element.cast::<TextBlock>() {
        text.set_foreground(Some(brush.clone()));
        true
    } else {
        false
    }
}
fn create_text(text: &RibbonString, size: f64, brush: Option<Rc<dyn IBrush>>) -> Ref<Control> {
    let block = TextBlock::new();
    block.set_text(Some(&utf8(text)));
    block.set_font_size(size * 0.8);
    block.set_font_weight(FontWeight::SemiBold);
    block.set_horizontal_alignment(HorizontalAlignment::Center);
    block.set_vertical_alignment(VerticalAlignment::Center);
    block.set_text_alignment(TextAlignment::Center);
    if brush.is_some() {
        block.set_foreground(brush);
    }
    if text.len() <= 2 {
        block.upcast()
    } else {
        boxed(block.upcast(), size, StretchDirection::DownOnly).upcast()
    }
}
fn free(element: &Control) -> bool {
    element.visual_parent().is_none() && element.parent().is_none()
}
fn convert<T: ferroui_base::PropertyValue>(value: &BoxedValue) -> Option<T> {
    value.downcast_ref::<T>().cloned().or_else(|| {
        ValueTypes::try_convert(Some(value), ValueType::of::<T>())
            .flatten()
            .and_then(|v| v.downcast_ref::<T>().cloned())
    })
}
fn create_element(value: &BoxedValue, size: f64) -> (Option<Ref<Control>>, bool) {
    if let Some(text) = value
        .downcast_ref::<String>()
        .map(|s| RibbonString::from(s.as_str()))
        .or_else(|| value.downcast_ref::<RibbonString>().cloned())
    {
        if text.is_empty() {
            return (None, false);
        }
        let kind = RibbonIconPresenter::classify(&text);
        let viewbox = if kind == RibbonIconKind::Path {
            RibbonIconLayer::parse_view_box(Some(&text)).unwrap_or(24.)
        } else {
            24.
        };
        return create_icon(
            &RibbonIcon::new_with_options(kind, Some(text), None, None, viewbox),
            size,
        );
    }
    if let Some(icon) = value.downcast_ref::<RibbonIcon>() {
        return create_icon(icon, size);
    }
    if let Some(icon) = value.downcast_ref::<Rc<RibbonIcon>>() {
        return create_icon(icon, size);
    }
    if let Some(source) = ValueTypes::as_object(&**value).and_then(|o| o.cast::<IconSource>()) {
        return create_from_icon_source(&source, size);
    }
    if let Some(control) = convert::<Ref<Control>>(value) {
        if let Some(icon) = control.cast::<IconElement>() {
            let property = TemplatedControl::foreground_property();
            // Source ReadLocalValue detects local ownership even when an animation
            // overrides the effective value. The clone snapshots the effective brush.
            // An installed local binding is local even before its first value arrives.
            let fixed = icon.has_local_value_binding(property.as_property())
                || icon.get_local_value(property).is_some();
            let clone = if let Some(font) = icon.cast::<FontIcon>() {
                let clone = FontIcon::new();
                clone.set_glyph(font.glyph());
                clone.set_font_size(size);
                clone.set_font_family(font.font_family());
                clone.set_font_weight(font.font_weight());
                clone.set_font_style(font.font_style());
                clone.upcast::<IconElement>()
            } else if let Some(symbol) = icon.cast::<SymbolIcon>() {
                SymbolIcon::with_symbol(symbol.symbol())
                    .expect("SymbolIcon native text boundary")
                    .upcast::<IconElement>()
            } else if let Some(path) = icon.cast::<PathIcon>() {
                let clone = PathIcon::new();
                clone.set_data(path.data());
                clone.upcast::<IconElement>()
            } else if let Some(bitmap) = icon.cast::<BitmapIcon>() {
                let clone = BitmapIcon::new();
                clone.set_uri_source(bitmap.uri_source());
                clone.set_show_as_monochrome(bitmap.show_as_monochrome());
                clone.upcast::<IconElement>()
            } else if let Some(image) = icon.cast::<ImageIcon>() {
                let clone = ImageIcon::new();
                clone.set_source(image.source());
                clone.upcast::<IconElement>()
            } else if free(&icon) {
                icon.clone()
            } else {
                return (None, fixed);
            };
            if fixed {
                clone.set_foreground(icon.foreground());
            }
            if clone.cast::<FontIcon>().is_some() {
                return (Some(clone.upcast()), fixed);
            }
            clone.set_width(size);
            clone.set_height(size);
            return (
                Some(boxed(clone.upcast(), size, StretchDirection::Both).upcast()),
                fixed,
            );
        }
        if free(&control) {
            return (
                Some(boxed(control, size, StretchDirection::Both).upcast()),
                true,
            );
        }
        return (None, false);
    }
    if let Some(image) = convert::<Rc<dyn IImage>>(value) {
        let element = Image::new();
        element.set_source(Some(image));
        element.set_width(size);
        element.set_height(size);
        element.set_stretch(Stretch::Uniform);
        return (Some(element.upcast()), false);
    }
    (None, false)
}
fn create_from_icon_source(source: &IconSource, size: f64) -> (Option<Ref<Control>>, bool) {
    let fixed = source.foreground().is_some();
    let element: Ref<Control> = if let Some(font) = source.to_ref().cast::<FontIconSource>() {
        let icon = FontIcon::new();
        icon.set_glyph(font.glyph());
        icon.set_font_size(size);
        icon.set_font_family(font.font_family().unwrap_or_else(symbol_font));
        icon.set_font_weight(font.font_weight());
        icon.upcast()
    } else if let Some(bitmap) = source.to_ref().cast::<BitmapIconSource>() {
        let clone = BitmapIcon::new();
        clone.set_uri_source(bitmap.uri_source());
        clone.set_show_as_monochrome(bitmap.show_as_monochrome());
        clone.upcast::<Control>()
    } else if let Some(symbol) = source.to_ref().cast::<SymbolIconSource>() {
        SymbolIcon::with_symbol(symbol.symbol())
            .expect("SymbolIcon native text boundary")
            .upcast()
    } else if let Some(path) = source.to_ref().cast::<PathIconSource>() {
        let icon = PathIcon::new();
        icon.set_data(path.data());
        icon.upcast()
    } else if let Some(image) = source.to_ref().cast::<ImageIconSource>() {
        let icon = Image::new();
        icon.set_source(image.image_source());
        icon.set_stretch(Stretch::Uniform);
        icon.upcast()
    } else {
        return (None, fixed);
    };
    element.set_width(size);
    element.set_height(size);
    if let Some(icon) = element.cast::<IconElement>()
        && source.foreground().is_some()
    {
        icon.set_foreground(source.foreground());
    }
    if element.cast::<PathIcon>().is_some() || element.cast::<SymbolIcon>().is_some() {
        (
            Some(boxed(element, size, StretchDirection::Both).upcast()),
            fixed,
        )
    } else {
        (Some(element), fixed)
    }
}
fn create_icon(icon: &RibbonIcon, size: f64) -> (Option<Ref<Control>>, bool) {
    let brush = color_brush(icon.foreground.as_ref());
    let fixed = brush.is_some();
    let value = icon.value.as_ref();
    let element = match icon.kind {
        RibbonIconKind::Path => {
            RibbonIconPresenter::create_path(value, icon.view_box_size, size, brush)
        }
        RibbonIconKind::Image => {
            let element = Image::new();
            element.set_width(size);
            element.set_height(size);
            element.set_stretch(Stretch::Uniform);
            element.set_source(RibbonIconPresenter::create_display_uri_source(
                value.expect("RibbonIcon.Value is null"),
                Some(element.downgrade()),
            ));
            element.upcast()
        }
        RibbonIconKind::Glyph
            if icon.font_family.is_some()
                || RibbonIconPresenter::classify(value.expect("RibbonIcon.Value is null"))
                    != RibbonIconKind::Text =>
        {
            let font = FontIcon::new();
            font.set_glyph(value.map(utf8));
            font.set_font_size(size);
            font.set_font_family(
                icon.font_family
                    .as_ref()
                    .map(|family| FontFamily::new(&utf8(family)))
                    .unwrap_or_else(symbol_font),
            );
            if brush.is_some() {
                font.set_foreground(brush);
            }
            font.upcast()
        }
        _ => create_text(value.expect("RibbonIcon.Value is null"), size, brush),
    };
    (Some(element), fixed)
}
fn symbol_font() -> FontFamily {
    Application::current()
        .and_then(|app| {
            app.resources()
                .try_get_resource(&"SymbolThemeFontFamily".into(), None)
                .flatten()
        })
        .and_then(|font| font.downcast_ref::<FontFamily>().cloned())
        .unwrap_or_else(|| FontFamily::new("Segoe Fluent Icons,Segoe MDL2 Assets"))
}
