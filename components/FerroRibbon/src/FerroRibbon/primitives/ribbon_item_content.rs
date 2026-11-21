//! Native port of shared RibbonItemContent.cs.
use super::{RibbonIconPresenter, RibbonMetrics};
use crate::compat::FontIcon;
use ferroribbon_core::{RibbonItemSize, RibbonString, text::trim};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{FontFamily, IBrush, TextLineBounds, TextTrimming, TextWrapping};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Application, Control, ControlImpl, Panel, PanelImpl, TextBlock};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonItemContentPart(i32);
ferroui_base::styled_value!(RibbonItemContentPart);
#[allow(non_upper_case_globals)]
impl RibbonItemContentPart {
    pub const All: Self = Self(0);
    pub const IconOnly: Self = Self(1);
    pub const LabelAndChevron: Self = Self(2);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for RibbonItemContentPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::All => f.write_str("All"),
            Self::IconOnly => f.write_str("IconOnly"),
            Self::LabelAndChevron => f.write_str("LabelAndChevron"),
            _ => write!(f, "{}", self.0),
        }
    }
}
struct Parts {
    icon: Ref<RibbonIconPresenter>,
    line1: Ref<TextBlock>,
    line2: Ref<TextBlock>,
    chevron: Ref<FontIcon>,
    color_bar: Ref<Rectangle>,
}
#[cfg(test)]
type ArrangeObserver = Rc<dyn Fn(usize)>;
#[repr(C)]
pub struct RibbonItemContent {
    base: Panel,
    parts: Parts,
    split_for: RefCell<Option<String>>,
    split_font_size: Cell<f64>,
    text_factory: fn() -> Ref<TextBlock>,
    #[cfg(test)]
    pub(crate) arranged: RefCell<Vec<(usize, Rect)>>,
    #[cfg(test)]
    pub(crate) arrange_observer: RefCell<Option<ArrangeObserver>>,
}
ferro_class!(RibbonItemContent:Panel);
ferro_impl_classes!(RibbonItemContent:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonItemContent {new:RibbonItemContent::new,markup:{namespace:"FerroRibbon.Controls.Primitives",content:Children}});
ferro_properties! {impl RibbonItemContent {
    pub fn icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<RibbonItemContent,_>("Icon",None)}
    pub fn large_icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<RibbonItemContent,_>("LargeIcon",None)}
    pub fn label_property()->StyledProperty<Option<String>> {FerroProperty::register::<RibbonItemContent,_>("Label",None)}
    pub fn item_size_property()->StyledProperty<RibbonItemSize> {FerroProperty::register::<RibbonItemContent,_>("ItemSize",RibbonItemSize::Medium)}
    pub fn show_label_property()->StyledProperty<bool> {FerroProperty::register::<RibbonItemContent,_>("ShowLabel",true)}
    pub fn show_chevron_property()->StyledProperty<bool> {FerroProperty::register::<RibbonItemContent,_>("ShowChevron",false)}
    pub fn is_simplified_property()->StyledProperty<bool> {FerroProperty::register::<RibbonItemContent,_>("IsSimplified",false)}
    pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>> {FerroProperty::register::<RibbonItemContent,_>("Metrics",Some(RibbonMetrics::comfortable()))}
    pub fn foreground_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<RibbonItemContent,_>("Foreground",None)}
    pub fn icon_foreground_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<RibbonItemContent,_>("IconForeground",None)}
    pub fn part_property()->StyledProperty<RibbonItemContentPart> {FerroProperty::register::<RibbonItemContent,_>("Part",RibbonItemContentPart::All)}
    pub fn color_bar_property()->StyledProperty<Option<Rc<dyn IBrush>>> {FerroProperty::register::<RibbonItemContent,_>("ColorBar",None)}
}}
impl RibbonItemContent {
    fn static_constructor() {
        for property in [
            Self::icon_property().as_property(),
            Self::large_icon_property().as_property(),
            Self::label_property().as_property(),
            Self::item_size_property().as_property(),
            Self::show_label_property().as_property(),
            Self::show_chevron_property().as_property(),
            Self::is_simplified_property().as_property(),
            Self::metrics_property().as_property(),
            Self::foreground_property().as_property(),
            Self::icon_foreground_property().as_property(),
            Self::part_property().as_property(),
            Self::color_bar_property().as_property(),
        ] {
            property
                .changed()
                .add_class_handler::<Self>(|this, _| {
                    this.sync();
                    this.invalidate_measure();
                })
                .detach();
        }
    }
    pub fn construct() -> Self {
        Self::construct_with_text_factory(create_text)
    }
    pub(crate) fn construct_with_text_factory(text_factory: fn() -> Ref<TextBlock>) -> Self {
        let icon = RibbonIconPresenter::new();
        let line1 = text_factory();
        let line2 = text_factory();
        let chevron = FontIcon::new();
        chevron.set_glyph(Some("\u{e70d}".into()));
        chevron.set_font_size(8.);
        chevron.set_is_hit_test_visible(false);
        let color_bar = Rectangle::new();
        color_bar.set_height(4.);
        color_bar.set_is_hit_test_visible(false);
        Self {
            base: Panel::construct(),
            parts: Parts {
                icon,
                line1,
                line2,
                chevron,
                color_bar,
            },
            split_for: RefCell::new(None),
            split_font_size: Cell::new(0.),
            text_factory,
            #[cfg(test)]
            arranged: RefCell::new(Vec::new()),
            #[cfg(test)]
            arrange_observer: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), v)
    }
    pub fn large_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::large_icon_property())
    }
    pub fn set_large_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::large_icon_property(), v)
    }
    pub fn label(&self) -> Option<String> {
        self.get_value(Self::label_property())
    }
    pub fn set_label(&self, v: Option<String>) {
        self.set_value(Self::label_property(), v)
    }
    pub fn item_size(&self) -> RibbonItemSize {
        self.get_value(Self::item_size_property())
    }
    pub fn set_item_size(&self, v: RibbonItemSize) {
        self.set_value(Self::item_size_property(), v)
    }
    pub fn show_label(&self) -> bool {
        self.get_value(Self::show_label_property())
    }
    pub fn set_show_label(&self, v: bool) {
        self.set_value(Self::show_label_property(), v)
    }
    pub fn show_chevron(&self) -> bool {
        self.get_value(Self::show_chevron_property())
    }
    pub fn set_show_chevron(&self, v: bool) {
        self.set_value(Self::show_chevron_property(), v)
    }
    pub fn is_simplified(&self) -> bool {
        self.get_value(Self::is_simplified_property())
    }
    pub fn set_is_simplified(&self, v: bool) {
        self.set_value(Self::is_simplified_property(), v)
    }
    pub fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.get_value(Self::metrics_property())
    }
    pub fn set_metrics(&self, v: Option<Arc<RibbonMetrics>>) {
        self.set_value(Self::metrics_property(), v)
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), v)
    }
    pub fn icon_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::icon_foreground_property())
    }
    pub fn set_icon_foreground(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::icon_foreground_property(), v)
    }
    pub fn part(&self) -> RibbonItemContentPart {
        self.get_value(Self::part_property())
    }
    pub fn set_part(&self, v: RibbonItemContentPart) {
        self.set_value(Self::part_property(), v)
    }
    pub fn color_bar(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::color_bar_property())
    }
    pub fn set_color_bar(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::color_bar_property(), v)
    }
    fn is_large(&self) -> bool {
        self.item_size() == RibbonItemSize::Large && !self.is_simplified()
    }
    fn actual_metrics(&self) -> Arc<RibbonMetrics> {
        self.metrics().unwrap_or_else(RibbonMetrics::comfortable)
    }
    fn sync(&self) {
        let metrics = self.actual_metrics();
        let large = self.is_large();
        let part = self.part();
        let p = &self.parts;
        p.icon
            .set_icon(if part == RibbonItemContentPart::LabelAndChevron {
                None
            } else if large {
                self.large_icon().or_else(|| self.icon())
            } else {
                self.icon()
            });
        p.icon.set_icon_size(if large {
            metrics.large_icon_size()
        } else {
            metrics.small_icon_size()
        });
        p.icon
            .set_foreground(self.icon_foreground().or_else(|| self.foreground()));
        p.color_bar.set_fill(self.color_bar());
        p.color_bar.set_is_visible(
            self.color_bar().is_some() && part != RibbonItemContentPart::LabelAndChevron,
        );
        p.color_bar.set_width(p.icon.icon_size());
        p.color_bar.set_height(if large { 5. } else { 3. });
        for text in [&p.line1, &p.line2] {
            text.set_font_size(metrics.font_size());
            text.set_foreground(self.foreground());
            text.set_line_height(metrics.font_size() + 2.);
        }
        if let Some(foreground) = self.foreground() {
            p.chevron.set_foreground(Some(foreground));
        }
        p.chevron.set_font_size(if large { 8. } else { 7. });
        p.chevron
            .set_is_visible(self.show_chevron() && part != RibbonItemContentPart::IconOnly);
        let show_text = if part == RibbonItemContentPart::IconOnly
            || self.label().as_ref().is_none_or(String::is_empty)
        {
            false
        } else if large || !p.icon.has_icon() {
            true
        } else if self.item_size() == RibbonItemSize::Small && !self.is_simplified() {
            false
        } else {
            self.show_label()
        };
        if !show_text {
            // The source's chained assignments set line2 before line1.
            p.line2.set_text(Some(""));
            p.line1.set_text(Some(""));
            p.line2.set_is_visible(false);
            p.line1.set_is_visible(false);
            return;
        }
        p.line1.set_is_visible(true);
        if !large {
            p.line1
                .set_text(Some(self.label().as_deref().unwrap_or("")));
            p.line2.set_text(Some(""));
            p.line2.set_is_visible(false);
            return;
        }
        p.line2.set_is_visible(true);
        let cached_label = self.split_for.borrow().clone();
        if cached_label != self.label()
            || (self.split_font_size.get() - metrics.font_size()).abs() > 0.01
            || text_len(&p.line1) + text_len(&p.line2) == 0
        {
            self.split_label(self.label().as_deref().expect("visible large label"));
            // Text/visibility observers can reenter Sync and replace Label.
            // The source reads it again after SplitLabel, not before it.
            self.split_for.replace(self.label());
            self.split_font_size.set(metrics.font_size());
        }
    }
    fn split_label(&self, label: &str) {
        // The source creates its probe only for labels with at least two words.
        let mut probe = None;
        let (line1, line2) = split_label(&RibbonString::from(label), self.show_chevron(), |text| {
            let probe = probe.get_or_insert_with(|| {
                let probe = (self.text_factory)();
                probe.set_font_size(self.parts.line1.font_size());
                probe
            });
            probe.set_text(Some(
                &text
                    .to_utf8()
                    .expect("FerroRibbon label probe cannot render unpaired UTF-16"),
            ));
            probe.measure(Size::INFINITY);
            probe.desired_size().width
        });
        self.parts.line1.set_text(Some(
            &line1
                .to_utf8()
                .expect("FerroRibbon label cannot render unpaired UTF-16"),
        ));
        self.parts.line2.set_text(Some(
            &line2
                .to_utf8()
                .expect("FerroRibbon label cannot render unpaired UTF-16"),
        ));
    }
    fn arrange_child(&self, index: usize, child: &Control, rect: Rect) {
        #[cfg(test)]
        self.arranged.borrow_mut().push((index, rect));
        #[cfg(test)]
        {
            let observer = self.arrange_observer.borrow().clone();
            if let Some(observer) = observer {
                observer(index);
            }
        }
        #[cfg(not(test))]
        let _ = index;
        child.arrange(rect);
    }
}
impl FerroObjectImpl for RibbonItemContent {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let p = &this.parts;
        for child in [
            p.icon.clone().upcast::<Control>(),
            p.line1.clone().upcast(),
            p.line2.clone().upcast(),
            p.chevron.clone().upcast(),
            p.color_bar.clone().upcast(),
        ] {
            this.children().add(child);
        }
        p.chevron.set_font_family(
            Application::current()
                .and_then(|app| {
                    app.resources()
                        .try_get_resource(&"SymbolThemeFontFamily".into(), None)
                        .flatten()
                })
                .and_then(|font| font.downcast_ref::<FontFamily>().cloned())
                .unwrap_or_else(|| FontFamily::new("Segoe Fluent Icons")),
        );
    }
}
impl LayoutableImpl for RibbonItemContent {
    fn measure_override(this: &Self, _available: Size) -> Size {
        this.sync();
        let metrics = this.actual_metrics();
        let children = this.children();
        let snapshot = children.snapshot();
        for child in snapshot.iter() {
            assert!(
                Rc::ptr_eq(&snapshot, &children.snapshot()),
                "RibbonItemContent children changed during measurement"
            );
            child.measure(Size::INFINITY);
        }
        assert!(
            Rc::ptr_eq(&snapshot, &children.snapshot()),
            "RibbonItemContent children changed during measurement"
        );
        let p = &this.parts;
        let icon_w = if p.icon.has_icon() {
            p.icon.icon_size()
        } else {
            0.
        };
        let chevron_w = if p.chevron.is_visible() { 10. } else { 0. };
        let l1 = if p.line1.is_visible() {
            p.line1.desired_size()
        } else {
            Size::default()
        };
        let l2 = if p.line2.is_visible() {
            p.line2.desired_size()
        } else {
            Size::default()
        };
        let pad = metrics.item_padding();
        if this.is_large() {
            let second = if text_len(&p.line2) == 0 {
                chevron_w
            } else {
                l2.width + if chevron_w > 0. { chevron_w + 2. } else { 0. }
            };
            let width = max(max(icon_w, l1.width), second) + pad * 2. - 2.;
            return Size::new(
                max(width, metrics.large_item_min_width()),
                if this.part() == RibbonItemContentPart::IconOnly {
                    metrics.large_icon_size() + 6.
                } else if this.part() == RibbonItemContentPart::LabelAndChevron {
                    metrics.large_item_height() - metrics.large_icon_size() - 6.
                } else {
                    metrics.large_item_height()
                },
            );
        }
        let height = if this.is_simplified() {
            metrics.simplified_item_height()
        } else {
            metrics.row_height()
        };
        let text_w = if l1.width > 0. {
            l1.width + if icon_w > 0. { 5. } else { 0. }
        } else {
            0.
        };
        let w = icon_w + text_w + if chevron_w > 0. { chevron_w + 3. } else { 0. } + pad * 2.
            - if this.part() == RibbonItemContentPart::IconOnly {
                1.
            } else {
                0.
            };
        Size::new(max(w, height), height)
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let metrics = this.actual_metrics();
        let pad = metrics.item_padding();
        let p = &this.parts;
        let icon_w = if p.icon.has_icon() {
            p.icon.icon_size()
        } else {
            0.
        };
        let hidden = Rect::new(0., 0., 0., 0.);
        let arrange = |i, child: &Control, rect| this.arrange_child(i, child, rect);
        if this.is_large() {
            let has_bar = p.color_bar.is_visible();
            if this.part() != RibbonItemContentPart::LabelAndChevron {
                let top = if this.part() == RibbonItemContentPart::IconOnly {
                    (size.height - icon_w) / 2. + 1.
                } else {
                    3.
                };
                arrange(
                    0,
                    &p.icon,
                    Rect::new(
                        (size.width - icon_w) / 2.,
                        top - if has_bar { 2. } else { 0. },
                        icon_w,
                        icon_w,
                    ),
                );
                arrange(
                    4,
                    &p.color_bar,
                    if has_bar {
                        Rect::new((size.width - icon_w) / 2., top + icon_w - 3., icon_w, 5.)
                    } else {
                        hidden
                    },
                );
            } else {
                arrange(0, &p.icon, hidden);
                arrange(4, &p.color_bar, hidden);
            }
            let top = if this.part() == RibbonItemContentPart::LabelAndChevron {
                0.
            } else {
                metrics.large_icon_size() + 5.
            };
            if this.part() == RibbonItemContentPart::IconOnly {
                arrange(1, &p.line1, hidden);
                arrange(2, &p.line2, hidden);
                arrange(3, &p.chevron, hidden);
                return size;
            }
            let line_h = metrics.font_size() + 2.;
            arrange(
                1,
                &p.line1,
                Rect::new(
                    (size.width - p.line1.desired_size().width) / 2.,
                    top,
                    p.line1.desired_size().width,
                    line_h,
                ),
            );
            let chevron_w = if p.chevron.is_visible() { 10. } else { 0. };
            if text_len(&p.line2) == 0 {
                arrange(2, &p.line2, hidden);
                arrange(
                    3,
                    &p.chevron,
                    if chevron_w > 0. {
                        Rect::new(
                            (size.width - chevron_w) / 2.,
                            top + line_h,
                            chevron_w,
                            line_h,
                        )
                    } else {
                        hidden
                    },
                );
            } else {
                let total =
                    p.line2.desired_size().width + if chevron_w > 0. { chevron_w + 2. } else { 0. };
                let x = (size.width - total) / 2.;
                arrange(
                    2,
                    &p.line2,
                    Rect::new(x, top + line_h, p.line2.desired_size().width, line_h),
                );
                arrange(
                    3,
                    &p.chevron,
                    if chevron_w > 0. {
                        Rect::new(
                            x + p.line2.desired_size().width + 2.,
                            top + line_h,
                            chevron_w,
                            line_h,
                        )
                    } else {
                        hidden
                    },
                );
            }
            return size;
        }
        let cy = size.height / 2.;
        let mut x = pad
            - if this.part() == RibbonItemContentPart::IconOnly {
                1.
            } else {
                0.
            };
        let bar = p.color_bar.is_visible();
        arrange(
            0,
            &p.icon,
            if icon_w > 0. {
                Rect::new(
                    x,
                    cy - icon_w / 2. - if bar { 1.5 } else { 0. },
                    icon_w,
                    icon_w,
                )
            } else {
                hidden
            },
        );
        arrange(
            4,
            &p.color_bar,
            if bar {
                Rect::new(x, cy + icon_w / 2. - 2., icon_w, 3.)
            } else {
                hidden
            },
        );
        x += icon_w;
        if p.line1.is_visible() && text_len(&p.line1) > 0 {
            x += if icon_w > 0. { 5. } else { 0. };
            arrange(
                1,
                &p.line1,
                Rect::new(
                    x,
                    cy - p.line1.desired_size().height / 2.,
                    p.line1.desired_size().width,
                    p.line1.desired_size().height,
                ),
            );
            // Source updates an otherwise unused xPos here; retain the read/arithmetic.
            x += p.line1.desired_size().width;
            let _ = x;
        } else {
            arrange(1, &p.line1, hidden);
        }
        arrange(2, &p.line2, hidden);
        arrange(
            3,
            &p.chevron,
            if p.chevron.is_visible() {
                Rect::new(size.width - pad - 10., cy - 5., 10., 10.)
            } else {
                hidden
            },
        );
        size
    }
}
fn create_text() -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(""));
    text.set_text_wrapping(TextWrapping::NoWrap);
    text.set_text_trimming(<dyn TextTrimming>::none());
    text.set_text_line_bounds(TextLineBounds::Full);
    text.set_is_hit_test_visible(false);
    text
}
fn text_len(text: &TextBlock) -> usize {
    text.text().map_or(0, |s| s.encode_utf16().count())
}
/// Same UTF-16 algorithm, with native measurement supplied by the caller. This
/// seam lets the unchanged C# algorithm be compared without pretending font parity.
pub(crate) fn split_label(
    label: &RibbonString,
    show_chevron: bool,
    mut width: impl FnMut(&RibbonString) -> f64,
) -> (RibbonString, RibbonString) {
    let units = label.as_utf16();
    if let Some(at) = units.iter().position(|&u| u == 10) {
        return (trim(&units[..at]).into(), trim(&units[at + 1..]).into());
    }
    let words = units
        .split(|&u| u == 32)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    if words.len() < 2 {
        return (label.clone(), "".into());
    }
    let join = |words: &[&[u16]]| {
        let mut joined = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i > 0 {
                joined.push(32)
            }
            joined.extend_from_slice(w)
        }
        RibbonString::from_utf16(joined)
    };
    let mut best = 1;
    let mut best_width = f64::MAX;
    let chevron = if show_chevron { 12. } else { 0. };
    for i in 1..words.len() {
        let w = max(
            width(&join(&words[..i])),
            width(&join(&words[i..])) + chevron,
        );
        if w < best_width - 0.5 {
            best_width = w;
            best = i;
        }
    }
    (join(&words[..best]), join(&words[best..]))
}
