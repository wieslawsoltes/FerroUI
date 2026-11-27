//! Source: RibbonSpace.Uno/Primitives/RibbonScrollPanel.cs (pinned reference).
use ferroribbon_core::localization::RibbonStrings;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{InputElement, InputElementImpl, KeyboardNavigation};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, FontFamily, RectangleGeometry};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Nullable, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Application, Button, Control, ControlImpl, Panel, PanelImpl, TextBlock};
use std::{cell::Cell, rc::Rc};

/// Natural width after a constrained measure. Interface handles retain the actual element.
pub trait IRibbonScrollContent {
    fn extent_width(&self) -> f64;
    fn as_object(&self) -> Ref<FerroObject>;
}
impl PartialEq for dyn IRibbonScrollContent {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonScrollContent as "IRibbonScrollContent" {
    namespace: "FerroRibbon.Controls.Primitives",
    handles: [Rc<dyn IRibbonScrollContent>, Option<Rc<dyn IRibbonScrollContent>>],
    this: Rc<dyn IRibbonScrollContent>,
    properties: [ExtentWidth: f64 {get: |this: &Rc<dyn IRibbonScrollContent>| this.extent_width()}],
});

#[repr(C)]
pub struct RibbonScrollPanel {
    base: Panel,
    left: Ref<Button>,
    right: Ref<Button>,
    offset: Cell<f64>,
    extent: Cell<f64>,
    has_overflow: Cell<bool>,
}
ferro_class!(RibbonScrollPanel: Panel);
ferro_impl_classes!(RibbonScrollPanel: StyledElementImpl, VisualImpl, InteractiveImpl, ControlImpl, PanelImpl);
ferro_class_info!(RibbonScrollPanel {
    new: RibbonScrollPanel::new,
    markup: {
        namespace: "FerroRibbon.Controls.Primitives",
        content: Children,
        properties: [
            HasOverflow: bool {get: RibbonScrollPanel::has_overflow},
            Offset: f64 {get: RibbonScrollPanel::offset},
        ],
        methods: [
            fn ScrollBy(f64) => RibbonScrollPanel::scroll_by,
            fn BringIntoView(Ref<Control>) => |this: &Ref<RibbonScrollPanel>, element: Ref<Control>| this.bring_into_view(&element),
        ],
    },
});
ferro_properties! { impl RibbonScrollPanel {
    pub fn content_property() -> StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<RibbonScrollPanel, _>("Content", None)
    }
    pub fn measure_infinite_property() -> StyledProperty<bool> {
        FerroProperty::register::<RibbonScrollPanel, _>("MeasureInfinite", false)
    }
}}
impl RibbonScrollPanel {
    fn static_constructor() {
        Self::content_property()
            .changed()
            .add_class_handler::<Self>(|this, e| this.content_changed(e))
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            left: create_arrow("\u{e76b}"),
            right: create_arrow("\u{e76c}"),
            offset: Cell::new(0.),
            extent: Cell::new(0.),
            has_overflow: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::content_property())
    }
    pub fn set_content(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::content_property(), value.into().0);
    }
    pub fn measure_infinite(&self) -> bool {
        self.get_value(Self::measure_infinite_property())
    }
    pub fn set_measure_infinite(&self, value: bool) {
        self.set_value(Self::measure_infinite_property(), value);
    }
    pub fn has_overflow(&self) -> bool {
        self.has_overflow.get()
    }
    pub fn offset(&self) -> f64 {
        self.offset.get()
    }
    pub fn scroll_by(&self, delta: f64) {
        self.offset.set(clamp(
            self.offset.get() + delta,
            0.,
            max(0., self.extent.get() - self.bounds().width),
        ));
        self.invalidate_arrange();
    }
    pub fn bring_into_view(&self, element: &Ref<Control>) {
        let Some(content) = self.content() else {
            return;
        };
        if !self.has_overflow() || element.bounds().width <= 0. {
            return;
        }
        // Native transform failure corresponds to the source ArgumentException catch only.
        let Some(p) = element.translate_point(Point::new(0., 0.), &content) else {
            return;
        };
        if p.x < self.offset.get() {
            self.offset.set(max(0., p.x - 24.));
        } else if p.x + element.bounds().width > self.offset.get() + self.bounds().width {
            self.offset.set(min(
                self.extent.get() - self.bounds().width,
                p.x + element.bounds().width - self.bounds().width + 24.,
            ));
        }
        self.invalidate_arrange();
    }
    fn content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old, new) = e.get_old_and_new_value::<Option<Ref<Control>>>();
        if let Some(old) = old {
            self.children().remove(&old);
        }
        if let Some(new) = new {
            self.children().insert(0, new);
        }
        self.offset.set(0.);
        self.invalidate_measure();
    }
}
fn wheel_scroll_distance(e: &ferroui_base::input::PointerWheelEventArgs) -> f64 {
    use ferroui_base::input::PlatformWheelDelta;
    let (x, y) = match e.platform_delta() {
        Some(PlatformWheelDelta::MacOS { delta, is_precise }) => {
            let factor = if is_precise { 1. } else { 10. };
            (delta.x * factor, delta.y * factor)
        }
        Some(PlatformWheelDelta::Win32 { delta }) => (delta.x, delta.y),
        None => return normalized_wheel_scroll_distance(e),
    };
    integer_wheel_scroll_distance(x, y)
}
fn integer_wheel_scroll_distance(x: f64, y: f64) -> f64 {
    // The source selects after int32 truncation and negates the integer before
    // conversion to double. Minimum-value negation wraps. Rust casts define
    // saturation/NaN handling outside the source native cast's finite range.
    let y = y as i32;
    let scalar = if y != 0 { y } else { x as i32 };
    f64::from(scalar.wrapping_neg())
}
#[cfg(target_os = "macos")]
fn normalized_wheel_scroll_distance(e: &ferroui_base::input::PointerWheelEventArgs) -> f64 {
    // Delta-only synthetic events cannot recover original adjacent doubles.
    integer_wheel_scroll_distance(e.delta().x * 50., e.delta().y * 50.)
}
#[cfg(target_os = "windows")]
fn normalized_wheel_scroll_distance(e: &ferroui_base::input::PointerWheelEventArgs) -> f64 {
    // Synthetic delta-only events use pointer-protocol directions. Both actual
    // Win32 protocols supply tagged original data above, including their sign.
    integer_wheel_scroll_distance(e.delta().x * 120., e.delta().y * 120.)
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn normalized_wheel_scroll_distance(e: &ferroui_base::input::PointerWheelEventArgs) -> f64 {
    // Other backend mappings still require their source/platform audit.
    assert_eq!(
        e.delta().x,
        0.,
        "RibbonScrollPanel horizontal wheel mapping is incomplete on this platform"
    );
    -e.delta().y * 120.
}
impl FerroObjectImpl for RibbonScrollPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let strings = RibbonStrings::current();
        let left = strings.scroll_left().map(|s| {
            s.to_utf8()
                .expect("native automation names cannot represent unpaired UTF-16")
        });
        let right = strings.scroll_right().map(|s| {
            s.to_utf8()
                .expect("native automation names cannot represent unpaired UTF-16")
        });
        AutomationProperties::set_name(&this.left, left.as_deref());
        AutomationProperties::set_name(&this.right, right.as_deref());
        this.children().add(this.left.clone());
        this.children().add(this.right.clone());
        for (button, direction) in [(&this.left, -1.), (&this.right, 1.)] {
            let weak = this.to_ref().downgrade();
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.scroll_by(direction * max(120., this.bounds().width * 0.6));
                }
            });
        }
        // Instance handler matches the source routed subscription, without a parent cycle.
        this.add_handler(InputElement::pointer_wheel_changed_event(), |sender, e| {
            if let Some(this) = sender.downcast_ref::<RibbonScrollPanel>()
                && this.has_overflow()
            {
                this.scroll_by(wheel_scroll_distance(e));
                e.set_handled(true);
            }
        });
        this.set_background(Some(Brushes::transparent()));
    }
}
ferro_impl_classes!(RibbonScrollPanel: InputElementImpl);
impl LayoutableImpl for RibbonScrollPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let content = this.content();
        this.left.measure(available);
        this.right.measure(available);
        let Some(content) = content else {
            return Size::new(0., 0.);
        };
        let width = if available.width.is_infinite() {
            f64::INFINITY
        } else {
            available.width
        };
        content.measure(Size::new(
            if this.measure_infinite() {
                f64::INFINITY
            } else {
                width
            },
            available.height,
        ));
        let value = Control::boxed(&content);
        let scroll_content = ValueTypes::try_convert(
            Some(&value),
            ValueType::of::<Rc<dyn IRibbonScrollContent>>(),
        )
        .flatten()
        .and_then(|value| {
            value
                .downcast_ref::<Rc<dyn IRibbonScrollContent>>()
                .cloned()
        });
        let extent = if let Some(sc) = scroll_content {
            max(sc.extent_width(), content.desired_size().width)
        } else {
            content.desired_size().width
        };
        this.extent.set(extent);
        Size::new(
            if width.is_infinite() {
                extent
            } else {
                min(extent, width)
            },
            content.desired_size().height,
        )
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let content = this.content();
        let extent = this.extent.get();
        let overflow = extent > size.width + 0.5;
        this.has_overflow.set(overflow);
        let offset = if overflow {
            clamp(this.offset.get(), 0., extent - size.width)
        } else {
            0.
        };
        this.offset.set(offset);
        if let Some(content) = content {
            content.arrange(Rect::new(-offset, 0., max(extent, size.width), size.height));
        }
        this.left
            .set_is_visible(this.has_overflow() && this.offset.get() > 0.5);
        this.right.set_is_visible(
            this.has_overflow() && this.offset.get() < this.extent.get() - size.width - 0.5,
        );
        this.left.arrange(Rect::new(0., 0., 22., size.height));
        this.right
            .arrange(Rect::new(size.width - 22., 0., 22., size.height));
        this.set_clip(RectangleGeometry::with_rect(Rect::from_size(size)));
        size
    }
}
fn create_arrow(glyph: &str) -> Ref<Button> {
    let icon = TextBlock::new();
    icon.set_text(Some(glyph));
    icon.set_font_size(10.);
    // The private WinUI FontIcon maps to a native text glyph.
    icon.set_font_family(FontFamily::new("Segoe Fluent Icons"));
    let button = Button::new();
    button.set_content(Some(Control::boxed(icon)));
    button.set_width(22.);
    button.set_min_width(0.);
    button.set_min_height(0.);
    button.set_padding(Thickness::default());
    button.set_border_thickness(Thickness::default());
    button.set_vertical_alignment(VerticalAlignment::Stretch);
    button.set_is_visible(false);
    KeyboardNavigation::set_is_tab_stop(&button, false);
    if let Some(value) = Application::current()
        .and_then(|app| {
            app.resources()
                .try_get_resource(&"RibbonScrollButtonStyle".into(), None)
        })
        .flatten()
        && let Some(theme) = ValueTypes::as_object(&*value).and_then(|o| o.cast::<ControlTheme>())
    {
        button.set_theme(theme);
    }
    button
}
// Preserve Math.Max/Min NaN and signed-zero behavior, unlike f64::{max,min}.
fn max(a: f64, b: f64) -> f64 {
    ferroui_base::utilities::math_utilities::max(a, b)
}
fn min(a: f64, b: f64) -> f64 {
    ferroui_base::utilities::math_utilities::min(a, b)
}
fn clamp(value: f64, lower: f64, upper: f64) -> f64 {
    ferroui_base::utilities::MathUtilities::clamp(value, lower, upper)
}
