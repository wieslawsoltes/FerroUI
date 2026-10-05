//! Markup metadata of the value types of this crate and of the types that
//! convert from text: their `Parse(string)`, their public constructors and
//! their static values, as the managed original declares them.

use crate::animation::easings::Easing;
use crate::animation::{Cue, IterationCount, IterationType, TimeSpan};
use crate::data::core::ValueTypes;
use crate::utilities::{CultureInfo, DateTime, DateTimeOffset, Decimal, NumberStyles};
use crate::ferro_markup_type;
use crate::input::{Cursor, Key, KeyGesture, KeyModifiers, StandardCursorType};
use crate::media::{
    BoxShadow, BoxShadows, Color, FontFamily, FontFeature, FontFeatureCollection, FontWeight, HslColor, HsvColor,
    TextDecorationCollection, TextDecorations,
};
use crate::metadata::{MarkupType, MarkupTyped};
use crate::styling::ThemeVariant;
use crate::utilities::{FormatError, Uri, UriKind};
use crate::{
    CornerRadius, Matrix, PixelPoint, PixelRect, PixelSize, PixelVector, Point, Rect, RelativePoint, RelativeRect,
    RelativeScalar, RelativeUnit, RoundedRect, Size, Thickness, Vector, Vector3D,
};
use std::rc::Rc;

// FerroUI

ferro_markup_type!(struct Thickness {
    namespace: "FerroUI",
    handles: [Thickness],
    parse: Thickness::parse,
    constructors: [
        () => Thickness::default,
        (f64) => Thickness::uniform,
        (f64, f64) => Thickness::symmetric,
        (f64, f64, f64, f64) => Thickness::new,
    ],
    properties: [
        Left: f64 { get: |t: &Thickness| t.left },
        Top: f64 { get: |t: &Thickness| t.top },
        Right: f64 { get: |t: &Thickness| t.right },
        Bottom: f64 { get: |t: &Thickness| t.bottom },
        IsUniform: bool { get: |t: &Thickness| t.is_uniform() },
    ],
});

ferro_markup_type!(struct Point {
    namespace: "FerroUI",
    handles: [Point],
    parse: Point::parse,
    constructors: [() => Point::default, (f64, f64) => Point::new],
    properties: [
        X: f64 { get: |p: &Point| p.x },
        Y: f64 { get: |p: &Point| p.y },
    ],
});

ferro_markup_type!(struct Vector {
    namespace: "FerroUI",
    handles: [Vector],
    parse: Vector::parse,
    constructors: [() => Vector::default, (f64, f64) => Vector::new],
    properties: [
        X: f64 { get: |v: &Vector| v.x },
        Y: f64 { get: |v: &Vector| v.y },
        Length: f64 { get: |v: &Vector| v.length() },
        SquaredLength: f64 { get: |v: &Vector| v.squared_length() },
    ],
});

ferro_markup_type!(struct Size {
    namespace: "FerroUI",
    handles: [Size],
    parse: Size::parse,
    constructors: [() => Size::default, (f64, f64) => Size::new],
    properties: [
        Width: f64 { get: |s: &Size| s.width },
        Height: f64 { get: |s: &Size| s.height },
        AspectRatio: f64 { get: |s: &Size| s.aspect_ratio() },
    ],
});

ferro_markup_type!(struct Rect {
    namespace: "FerroUI",
    handles: [Rect],
    parse: Rect::parse,
    constructors: [
        () => Rect::default,
        (f64, f64, f64, f64) => Rect::new,
        (Size) => Rect::from_size,
        (Point, Size) => Rect::from_position_size,
        (Point, Point) => Rect::from_points,
    ],
    properties: [
        X: f64 { get: |r: &Rect| r.x },
        Y: f64 { get: |r: &Rect| r.y },
        Width: f64 { get: |r: &Rect| r.width },
        Height: f64 { get: |r: &Rect| r.height },
        Position: Point { get: |r: &Rect| r.position() },
        Size: Size { get: |r: &Rect| r.size() },
        Right: f64 { get: |r: &Rect| r.right() },
        Bottom: f64 { get: |r: &Rect| r.bottom() },
        Left: f64 { get: |r: &Rect| r.left() },
        Top: f64 { get: |r: &Rect| r.top() },
        TopLeft: Point { get: |r: &Rect| r.top_left() },
        TopRight: Point { get: |r: &Rect| r.top_right() },
        BottomLeft: Point { get: |r: &Rect| r.bottom_left() },
        BottomRight: Point { get: |r: &Rect| r.bottom_right() },
        Center: Point { get: |r: &Rect| r.center() },
    ],
});

ferro_markup_type!(struct Matrix {
    namespace: "FerroUI",
    handles: [Matrix],
    parse: Matrix::parse,
    constructors: [
        () => Matrix::default,
        (f64, f64, f64, f64, f64, f64) => Matrix::new,
        (f64, f64, f64, f64, f64, f64, f64, f64, f64) => Matrix::new_3x3,
    ],
    properties: [
        M11: f64 { get: |m: &Matrix| m.m11 },
        M12: f64 { get: |m: &Matrix| m.m12 },
        M13: f64 { get: |m: &Matrix| m.m13 },
        M21: f64 { get: |m: &Matrix| m.m21 },
        M22: f64 { get: |m: &Matrix| m.m22 },
        M23: f64 { get: |m: &Matrix| m.m23 },
        M31: f64 { get: |m: &Matrix| m.m31 },
        M32: f64 { get: |m: &Matrix| m.m32 },
        M33: f64 { get: |m: &Matrix| m.m33 },
        IsIdentity: bool { get: |m: &Matrix| m.is_identity() },
        HasInverse: bool { get: |m: &Matrix| m.has_inverse() },
    ],
});

ferro_markup_type!(struct CornerRadius {
    namespace: "FerroUI",
    handles: [CornerRadius],
    parse: CornerRadius::parse,
    constructors: [
        () => CornerRadius::default,
        (f64) => CornerRadius::uniform,
        (f64, f64) => CornerRadius::top_bottom,
        (f64, f64, f64, f64) => CornerRadius::new,
    ],
    properties: [
        TopLeft: f64 { get: |c: &CornerRadius| c.top_left },
        TopRight: f64 { get: |c: &CornerRadius| c.top_right },
        BottomRight: f64 { get: |c: &CornerRadius| c.bottom_right },
        BottomLeft: f64 { get: |c: &CornerRadius| c.bottom_left },
        IsUniform: bool { get: |c: &CornerRadius| c.is_uniform() },
    ],
});

ferro_markup_type!(struct RelativePoint {
    namespace: "FerroUI",
    handles: [RelativePoint],
    parse: RelativePoint::parse,
    constructors: [
        () => RelativePoint::default,
        (f64, f64, RelativeUnit) => RelativePoint::new,
        (Point, RelativeUnit) => RelativePoint::from_point,
    ],
    properties: [
        Point: Point { get: |r: &RelativePoint| r.point },
        Unit: RelativeUnit { get: |r: &RelativePoint| r.unit },
    ],
});

ferro_markup_type!(struct RelativeRect {
    namespace: "FerroUI",
    handles: [RelativeRect],
    parse: RelativeRect::parse,
    constructors: [
        () => RelativeRect::default,
        (f64, f64, f64, f64, RelativeUnit) => RelativeRect::new,
        (Rect, RelativeUnit) => RelativeRect::from_rect,
        (Size, RelativeUnit) => RelativeRect::from_size,
        (Point, Size, RelativeUnit) => RelativeRect::from_position_size,
        (Point, Point, RelativeUnit) => RelativeRect::from_points,
    ],
    properties: [
        Unit: RelativeUnit { get: |r: &RelativeRect| r.unit },
        Rect: Rect { get: |r: &RelativeRect| r.rect },
    ],
});

ferro_markup_type!(struct RelativeScalar {
    namespace: "FerroUI",
    handles: [RelativeScalar],
    parse: RelativeScalar::parse,
    constructors: [() => RelativeScalar::default, (f64, RelativeUnit) => RelativeScalar::new],
    properties: [
        Scalar: f64 { get: |r: &RelativeScalar| r.scalar },
        Unit: RelativeUnit { get: |r: &RelativeScalar| r.unit },
    ],
});

ferro_markup_type!(struct PixelPoint {
    namespace: "FerroUI",
    handles: [PixelPoint],
    parse: PixelPoint::parse,
    constructors: [() => PixelPoint::default, (i32, i32) => PixelPoint::new],
    properties: [
        X: i32 { get: |p: &PixelPoint| p.x },
        Y: i32 { get: |p: &PixelPoint| p.y },
    ],
});

ferro_markup_type!(struct PixelSize {
    namespace: "FerroUI",
    handles: [PixelSize],
    parse: PixelSize::parse,
    constructors: [() => PixelSize::default, (i32, i32) => PixelSize::new],
    properties: [
        Width: i32 { get: |p: &PixelSize| p.width },
        Height: i32 { get: |p: &PixelSize| p.height },
        AspectRatio: f64 { get: |p: &PixelSize| p.aspect_ratio() },
    ],
});

ferro_markup_type!(struct PixelRect {
    namespace: "FerroUI",
    handles: [PixelRect],
    parse: PixelRect::parse,
    constructors: [
        () => PixelRect::default,
        (i32, i32, i32, i32) => PixelRect::new,
        (PixelSize) => PixelRect::from_size,
        (PixelPoint, PixelSize) => PixelRect::from_position_size,
        (PixelPoint, PixelPoint) => PixelRect::from_points,
    ],
    properties: [
        X: i32 { get: |p: &PixelRect| p.x },
        Y: i32 { get: |p: &PixelRect| p.y },
        Width: i32 { get: |p: &PixelRect| p.width },
        Height: i32 { get: |p: &PixelRect| p.height },
        Position: PixelPoint { get: |p: &PixelRect| p.position() },
        Size: PixelSize { get: |p: &PixelRect| p.size() },
        Right: i32 { get: |p: &PixelRect| p.right() },
        Bottom: i32 { get: |p: &PixelRect| p.bottom() },
        TopLeft: PixelPoint { get: |p: &PixelRect| p.top_left() },
        TopRight: PixelPoint { get: |p: &PixelRect| p.top_right() },
        BottomLeft: PixelPoint { get: |p: &PixelRect| p.bottom_left() },
        BottomRight: PixelPoint { get: |p: &PixelRect| p.bottom_right() },
        Center: PixelPoint { get: |p: &PixelRect| p.center() },
    ],
});

ferro_markup_type!(struct PixelVector {
    namespace: "FerroUI",
    handles: [PixelVector],
    constructors: [() => PixelVector::default, (i32, i32) => PixelVector::new],
    properties: [
        X: i32 { get: |p: &PixelVector| p.x },
        Y: i32 { get: |p: &PixelVector| p.y },
        Length: f64 { get: |p: &PixelVector| p.length() },
    ],
});

ferro_markup_type!(struct Vector3D {
    namespace: "FerroUI",
    handles: [Vector3D],
    parse: Vector3D::parse,
    constructors: [() => Vector3D::default, (f64, f64, f64) => Vector3D::new],
    properties: [
        X: f64 { get: |v: &Vector3D| v.x },
        Y: f64 { get: |v: &Vector3D| v.y },
        Z: f64 { get: |v: &Vector3D| v.z },
        Length: f64 { get: |v: &Vector3D| v.length() },
    ],
});

ferro_markup_type!(struct RoundedRect {
    namespace: "FerroUI",
    handles: [RoundedRect],
    constructors: [
        () => RoundedRect::default,
        (Rect) => RoundedRect::from_rect,
        (Rect, f64) => RoundedRect::from_radius,
        (Rect, f64, f64) => RoundedRect::from_radius_xy,
        (Rect, Vector) => RoundedRect::from_uniform_radii,
        (Rect, CornerRadius) => RoundedRect::from_corner_radius,
    ],
});

// FerroUI.Media

ferro_markup_type!(struct Color {
    namespace: "FerroUI.Media",
    handles: [Color],
    parse: Color::parse,
    constructors: [() => Color::default, (u8, u8, u8, u8) => Color::new],
    methods: [
        static fn FromArgb(u8, u8, u8, u8) -> Color => Color::from_argb,
        static fn FromRgb(u8, u8, u8) -> Color => Color::from_rgb,
        static fn FromUInt32(u32) -> Color => Color::from_uint32,
    ],
    properties: [
        A: u8 { get: |c: &Color| c.a },
        R: u8 { get: |c: &Color| c.r },
        G: u8 { get: |c: &Color| c.g },
        B: u8 { get: |c: &Color| c.b },
    ],
});

ferro_markup_type!(struct HslColor {
    namespace: "FerroUI.Media",
    handles: [HslColor],
    parse: HslColor::parse,
    constructors: [
        () => HslColor::default,
        (f64, f64, f64, f64) => HslColor::new,
        (Color) => HslColor::from_color,
    ],
    methods: [
        static fn FromAhsl(f64, f64, f64, f64) -> HslColor => HslColor::from_ahsl,
        static fn FromHsl(f64, f64, f64) -> HslColor => HslColor::from_hsl,
    ],
    properties: [
        A: f64 { get: |h: &HslColor| h.a },
        H: f64 { get: |h: &HslColor| h.h },
        S: f64 { get: |h: &HslColor| h.s },
        L: f64 { get: |h: &HslColor| h.l },
    ],
});

ferro_markup_type!(struct HsvColor {
    namespace: "FerroUI.Media",
    handles: [HsvColor],
    parse: HsvColor::parse,
    constructors: [
        () => HsvColor::default,
        (f64, f64, f64, f64) => HsvColor::new,
        (Color) => HsvColor::from_color,
    ],
    methods: [
        static fn FromAhsv(f64, f64, f64, f64) -> HsvColor => HsvColor::from_ahsv,
        static fn FromHsv(f64, f64, f64) -> HsvColor => HsvColor::from_hsv,
    ],
    properties: [
        A: f64 { get: |h: &HsvColor| h.a },
        H: f64 { get: |h: &HsvColor| h.h },
        S: f64 { get: |h: &HsvColor| h.s },
        V: f64 { get: |h: &HsvColor| h.v },
    ],
});

// An enumeration in the managed original (any weight from 1 to 999 is a
// value of it); the named weights are its members.
ferro_markup_type!(struct FontWeight {
    namespace: "FerroUI.Media",
    handles: [FontWeight],
    parse: |s: &str| s.parse::<FontWeight>(),
    fields: [
        Thin: FontWeight => || FontWeight::Thin,
        ExtraLight: FontWeight => || FontWeight::ExtraLight,
        UltraLight: FontWeight => || FontWeight::UltraLight,
        Light: FontWeight => || FontWeight::Light,
        SemiLight: FontWeight => || FontWeight::SemiLight,
        Normal: FontWeight => || FontWeight::Normal,
        Regular: FontWeight => || FontWeight::Regular,
        Medium: FontWeight => || FontWeight::Medium,
        DemiBold: FontWeight => || FontWeight::DemiBold,
        SemiBold: FontWeight => || FontWeight::SemiBold,
        Bold: FontWeight => || FontWeight::Bold,
        ExtraBold: FontWeight => || FontWeight::ExtraBold,
        UltraBold: FontWeight => || FontWeight::UltraBold,
        Black: FontWeight => || FontWeight::Black,
        Heavy: FontWeight => || FontWeight::Heavy,
        Solid: FontWeight => || FontWeight::Solid,
        ExtraBlack: FontWeight => || FontWeight::ExtraBlack,
        UltraBlack: FontWeight => || FontWeight::UltraBlack,
    ],
    constructors: [() => FontWeight::default],
});

ferro_markup_type!(class FontFamily {
    namespace: "FerroUI.Media",
    handles: [FontFamily, Option<FontFamily>],
    parse: FontFamily::parse,
    constructors: [
        (String) => |name: String| FontFamily::new(&name),
        (Option<Uri>, String) => |base_uri: Option<Uri>, name: String| FontFamily::with_base_uri(base_uri.as_ref(), &name),
    ],
    static_properties: [Default: FontFamily { get: FontFamily::default_family }],
});

ferro_markup_type!(class FontFeature {
    namespace: "FerroUI.Media",
    handles: [FontFeature, Option<FontFeature>],
    parse: |s: &str| Ok::<_, FormatError>(FontFeature::parse(s)),
});

ferro_markup_type!(class FontFeatureCollection {
    namespace: "FerroUI.Media",
    handles: [FontFeatureCollection, Option<FontFeatureCollection>],
    base: crate::collections::FerroList<FontFeature>,
    parse: FontFeatureCollection::parse,
    constructors: [() => FontFeatureCollection::new],
    methods: [fn Add(FontFeature) => |c: &FontFeatureCollection, item: FontFeature| c.add(item)],
});

ferro_markup_type!(class TextDecorationCollection {
    namespace: "FerroUI.Media",
    handles: [TextDecorationCollection, Option<TextDecorationCollection>],
    base: crate::collections::FerroList<crate::Ref<crate::media::TextDecoration>>,
    parse: TextDecorationCollection::parse,
    constructors: [() => TextDecorationCollection::new],
    methods: [
        fn Add(crate::Ref<crate::media::TextDecoration>) =>
            |c: &TextDecorationCollection, item: crate::Ref<crate::media::TextDecoration>| c.add(item),
    ],
});

ferro_markup_type!(static TextDecorations {
    namespace: "FerroUI.Media",
    static_properties: [
        Underline: TextDecorationCollection { get: TextDecorations::underline },
        Strikethrough: TextDecorationCollection { get: TextDecorations::strikethrough },
        Overline: TextDecorationCollection { get: TextDecorations::overline },
        Baseline: TextDecorationCollection { get: TextDecorations::baseline },
    ],
});

ferro_markup_type!(struct BoxShadow {
    namespace: "FerroUI.Media",
    handles: [BoxShadow],
    parse: BoxShadow::parse,
    properties: [
        OffsetX: f64 { get: |b: &BoxShadow| b.offset_x },
        OffsetY: f64 { get: |b: &BoxShadow| b.offset_y },
        Blur: f64 { get: |b: &BoxShadow| b.blur },
        Spread: f64 { get: |b: &BoxShadow| b.spread },
        Color: Color { get: |b: &BoxShadow| b.color },
        IsInset: bool { get: |b: &BoxShadow| b.is_inset },
    ],
    constructors: [() => BoxShadow::default],
});

ferro_markup_type!(struct BoxShadows {
    namespace: "FerroUI.Media",
    handles: [BoxShadows],
    parse: BoxShadows::parse,
    constructors: [() => BoxShadows::default, (BoxShadow) => BoxShadows::new],
});

// FerroUI.Input

ferro_markup_type!(class KeyGesture {
    namespace: "FerroUI.Input",
    handles: [KeyGesture, Option<KeyGesture>],
    parse: KeyGesture::parse,
    constructors: [
        (Key) => KeyGesture::from_key,
        (Key, KeyModifiers) => KeyGesture::new,
    ],
    properties: [
        Key: Key { get: |k: &KeyGesture| k.key() },
        KeyModifiers: KeyModifiers { get: |k: &KeyGesture| k.key_modifiers() },
    ],
});

ferro_markup_type!(class Cursor {
    this: Rc<Cursor>,
    namespace: "FerroUI.Input",
    handles: [Rc<Cursor>, Option<Rc<Cursor>>],
    parse: Cursor::parse,
    constructors: [(StandardCursorType) => Cursor::new],
    fields: [Default: Rc<Cursor> => Cursor::default_cursor],
});

// FerroUI.Animation

ferro_markup_type!(struct Cue {
    namespace: "FerroUI.Animation",
    handles: [Cue],
    parse: Cue::parse,
    properties: [
        CueValue: f64 { get: |c: &Cue| c.cue_value() },
    ],
    constructors: [() => Cue::default],
});

ferro_markup_type!(struct IterationCount {
    namespace: "FerroUI.Animation",
    handles: [IterationCount],
    parse: IterationCount::parse,
    constructors: [
        () => IterationCount::default,
        (u64) => IterationCount::new,
        (u64, IterationType) => IterationCount::with_type,
    ],
    properties: [
        Value: u64 { get: |i: &IterationCount| i.value() },
        RepeatType: IterationType { get: |i: &IterationCount| i.repeat_type() },
        IsInfinite: bool { get: |i: &IterationCount| i.is_infinite() },
    ],
});

// FerroUI.Animation.Easings

ferro_markup_type!(class Easing {
    namespace: "FerroUI.Animation.Easings",
    handles: [Easing, Option<Easing>],
    parse: Easing::parse,
});

// FerroUI.Styling

ferro_markup_type!(class ThemeVariant {
    namespace: "FerroUI.Styling",
    handles: [ThemeVariant, Option<ThemeVariant>],
    parse: |s: &str| s.parse::<ThemeVariant>(),
    constructors: [
        // The key is any object in the managed original.
        try (Option<crate::BoxedValue>, Option<ThemeVariant>) =>
            |key: Option<crate::BoxedValue>, inherit_variant: Option<ThemeVariant>| {
                super::plain::resource_key(key).map(|key| ThemeVariant::new(key, inherit_variant))
            },
    ],
    properties: [
        // The key is any object in the managed original.
        Key: Option<crate::BoxedValue> { get: |variant: &ThemeVariant| super::plain::resource_key_value(variant.key()) },
        InheritVariant: Option<ThemeVariant> { get: ThemeVariant::inherit_variant },
    ],
    static_properties: [
        Default: ThemeVariant { get: ThemeVariant::default },
        Light: ThemeVariant { get: ThemeVariant::light },
        Dark: ThemeVariant { get: ThemeVariant::dark },
    ],
});

// System

ferro_markup_type!(struct TimeSpan {
    namespace: "System",
    handles: [TimeSpan],
    parse: TimeSpan::parse,
    constructors: [() => TimeSpan::default, (i64) => TimeSpan::from_ticks],
    methods: [
        static fn FromTicks(i64) -> TimeSpan => TimeSpan::from_ticks,
        static fn FromSeconds(f64) -> TimeSpan => TimeSpan::from_seconds,
        static fn FromMilliseconds(f64) -> TimeSpan => TimeSpan::from_milliseconds,
        static fn FromMinutes(f64) -> TimeSpan => TimeSpan::from_minutes,
    ],
    properties: [
        Ticks: i64 { get: |t: &TimeSpan| t.ticks() },
        TotalSeconds: f64 { get: |t: &TimeSpan| t.total_seconds() },
        TotalMilliseconds: f64 { get: |t: &TimeSpan| t.total_milliseconds() },
    ],
});

// The dates of the runtime library, written in the invariant culture.
ferro_markup_type!(struct DateTime {
    namespace: "System",
    handles: [DateTime],
    parse: |s: &str| DateTime::parse(s, &CultureInfo::invariant_culture()),
    constructors: [() => DateTime::default],
    // `static DateTime Now { get; }` and the like are static properties of the managed
    // original; `MinValue` and `MaxValue` are its static read-only fields.
    fields: [
        MinValue: DateTime => || DateTime::MIN_VALUE,
        MaxValue: DateTime => || DateTime::MAX_VALUE,
    ],
    static_properties: [
        Now: DateTime { get: DateTime::now },
        UtcNow: DateTime { get: DateTime::utc_now },
        Today: DateTime { get: DateTime::today },
    ],
});

ferro_markup_type!(struct DateTimeOffset {
    namespace: "System",
    handles: [DateTimeOffset],
    parse: |s: &str| DateTimeOffset::parse(s, &CultureInfo::invariant_culture()),
    constructors: [() => DateTimeOffset::default],
});

ferro_markup_type!(class Uri {
    namespace: "System",
    handles: [Uri, Option<Uri>],
    parse: |s: &str| Uri::new(s, UriKind::RelativeOrAbsolute),
    constructors: [
        try (String) => |uri: String| Uri::new(&uri, UriKind::Absolute),
        try (String, UriKind) => |uri: String, kind: UriKind| Uri::new(&uri, kind),
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <Thickness as MarkupTyped>::MARKUP,
    <Point as MarkupTyped>::MARKUP,
    <Vector as MarkupTyped>::MARKUP,
    <Size as MarkupTyped>::MARKUP,
    <Rect as MarkupTyped>::MARKUP,
    <Matrix as MarkupTyped>::MARKUP,
    <CornerRadius as MarkupTyped>::MARKUP,
    <RelativePoint as MarkupTyped>::MARKUP,
    <RelativeRect as MarkupTyped>::MARKUP,
    <RelativeScalar as MarkupTyped>::MARKUP,
    <PixelPoint as MarkupTyped>::MARKUP,
    <PixelSize as MarkupTyped>::MARKUP,
    <PixelRect as MarkupTyped>::MARKUP,
    <PixelVector as MarkupTyped>::MARKUP,
    <Vector3D as MarkupTyped>::MARKUP,
    <RoundedRect as MarkupTyped>::MARKUP,
    <Color as MarkupTyped>::MARKUP,
    <HslColor as MarkupTyped>::MARKUP,
    <HsvColor as MarkupTyped>::MARKUP,
    <FontWeight as MarkupTyped>::MARKUP,
    <FontFamily as MarkupTyped>::MARKUP,
    <FontFeature as MarkupTyped>::MARKUP,
    <FontFeatureCollection as MarkupTyped>::MARKUP,
    <TextDecorationCollection as MarkupTyped>::MARKUP,
    <TextDecorations as MarkupTyped>::MARKUP,
    <BoxShadow as MarkupTyped>::MARKUP,
    <BoxShadows as MarkupTyped>::MARKUP,
    <KeyGesture as MarkupTyped>::MARKUP,
    <Cursor as MarkupTyped>::MARKUP,
    <Cue as MarkupTyped>::MARKUP,
    <IterationCount as MarkupTyped>::MARKUP,
    <Easing as MarkupTyped>::MARKUP,
    <ThemeVariant as MarkupTyped>::MARKUP,
    <TimeSpan as MarkupTyped>::MARKUP,
    <DateTime as MarkupTyped>::MARKUP,
    <DateTimeOffset as MarkupTyped>::MARKUP,
    // Declared next to the types.
    <Decimal as MarkupTyped>::MARKUP,
    <NumberStyles as MarkupTyped>::MARKUP,
    <Uri as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the value types with the untyped value
/// conversions of the current thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Thickness>();
    ValueTypes::register_nullable::<Point>();
    ValueTypes::register_nullable::<Vector>();
    ValueTypes::register_nullable::<Size>();
    ValueTypes::register_nullable::<Rect>();
    ValueTypes::register_nullable::<Matrix>();
    ValueTypes::register_nullable::<CornerRadius>();
    ValueTypes::register_nullable::<RelativePoint>();
    ValueTypes::register_nullable::<RelativeRect>();
    ValueTypes::register_nullable::<RelativeScalar>();
    ValueTypes::register_nullable::<PixelPoint>();
    ValueTypes::register_nullable::<PixelSize>();
    ValueTypes::register_nullable::<PixelRect>();
    ValueTypes::register_nullable::<PixelVector>();
    ValueTypes::register_nullable::<Vector3D>();
    ValueTypes::register_nullable::<RoundedRect>();
    ValueTypes::register_nullable::<Color>();
    ValueTypes::register_nullable::<HslColor>();
    ValueTypes::register_nullable::<HsvColor>();
    ValueTypes::register_nullable::<FontWeight>();
    ValueTypes::register_nullable::<FontFamily>();
    ValueTypes::register_nullable::<FontFeature>();
    ValueTypes::register_nullable::<FontFeatureCollection>();
    ValueTypes::register_nullable::<TextDecorationCollection>();
    ValueTypes::register_nullable::<BoxShadow>();
    ValueTypes::register_nullable::<BoxShadows>();
    ValueTypes::register_nullable::<KeyGesture>();
    ValueTypes::register_nullable::<Cue>();
    ValueTypes::register_nullable::<IterationCount>();
    ValueTypes::register_nullable::<Easing>();
    ValueTypes::register_nullable::<ThemeVariant>();
    ValueTypes::register_nullable::<TimeSpan>();
    ValueTypes::register_nullable::<Uri>();
    ValueTypes::register_nullable::<Rc<Cursor>>();

    // The text of a value (its `ToString()` in the managed original, what a string format
    // or a binding to text shows): the value types that define one.
    ValueTypes::register_display::<Thickness>();
    ValueTypes::register_display::<Point>();
    ValueTypes::register_display::<Vector>();
    ValueTypes::register_display::<Size>();
    ValueTypes::register_display::<Rect>();
    ValueTypes::register_display::<Matrix>();
    ValueTypes::register_display::<CornerRadius>();
    ValueTypes::register_display::<RelativePoint>();
    ValueTypes::register_display::<RelativeScalar>();
    ValueTypes::register_display::<PixelPoint>();
    ValueTypes::register_display::<PixelSize>();
    ValueTypes::register_display::<PixelRect>();
    ValueTypes::register_display::<PixelVector>();
    ValueTypes::register_display::<Vector3D>();
    ValueTypes::register_display::<Color>();
    ValueTypes::register_display::<HslColor>();
    ValueTypes::register_display::<HsvColor>();
    ValueTypes::register_display::<FontWeight>();
    ValueTypes::register_display::<FontFamily>();
    ValueTypes::register_display::<FontFeature>();
    ValueTypes::register_display::<BoxShadow>();
    ValueTypes::register_display::<BoxShadows>();
    ValueTypes::register_display::<KeyGesture>();
    ValueTypes::register_display::<IterationCount>();
    ValueTypes::register_display::<ThemeVariant>();
    ValueTypes::register_display::<TimeSpan>();
    ValueTypes::register_display::<DateTime>();
    ValueTypes::register_display::<DateTimeOffset>();
    ValueTypes::register_display::<Decimal>();
    ValueTypes::register_nullable::<DateTime>();
    ValueTypes::register_nullable::<DateTimeOffset>();
    ValueTypes::register_nullable::<Decimal>();
    ValueTypes::register_nullable::<NumberStyles>();
    ValueTypes::register_display::<Uri>();
    ValueTypes::register_display::<Rc<Cursor>>();
}
