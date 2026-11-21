//! Native descriptors for the existing UI-independent Core values.
//! Record synthesis/out parameters and UTF16 UI limits remain documented separately.
use ferroribbon_core::theming::{RibbonColor, RibbonThemePalette};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[doc(hidden)]
pub struct RibbonColorMembers;
impl RibbonColorMembers {
    pub fn parse(value: &str) -> Result<RibbonColor, String> {
        Self::parse_nullable(Some(value.to_owned()))
    }
    pub fn parse_nullable(value: Option<String>) -> Result<RibbonColor, String> {
        RibbonColor::parse(value.map(Into::into).as_ref())
            .map_err(|e| e.message.to_utf8().expect("native input is valid UTF16"))
    }
}
ferroui_base::ferro_markup_type!(struct RibbonColorMembers as "RibbonColor" {
    namespace:"FerroRibbon.Theming",this:RibbonColor,handles:[RibbonColor],
    parse:RibbonColorMembers::parse,
    constructors:[()=>RibbonColor::default,(u8,u8,u8,u8)=>RibbonColor::new],
    properties:[
        A:u8 {get:|c:&RibbonColor|c.a}, R:u8 {get:|c:&RibbonColor|c.r},
        G:u8 {get:|c:&RibbonColor|c.g}, B:u8 {get:|c:&RibbonColor|c.b},
        Luminance:f64 {get:|c:&RibbonColor|c.luminance()},
        Lightness:f64 {get:|c:&RibbonColor|c.lightness()},
        PrefersDarkForeground:bool {get:|c:&RibbonColor|c.prefers_dark_foreground()},
    ],
    static_properties:[
        Transparent:RibbonColor {get:||RibbonColor::TRANSPARENT},
        White:RibbonColor {get:||RibbonColor::WHITE},
        Black:RibbonColor {get:||RibbonColor::BLACK},
    ],
    methods:[
        static fn FromRgb(u8,u8,u8)->RibbonColor => RibbonColor::from_rgb,
        static try fn Parse(Option<String>)->RibbonColor => RibbonColorMembers::parse_nullable,
        static fn ContrastRatio(RibbonColor,RibbonColor)->f64 => RibbonColor::contrast_ratio,
        fn WithAlpha(u8)->RibbonColor => |c:&RibbonColor,a:u8|c.with_alpha(a),
        fn Lighten(f64)->RibbonColor => |c:&RibbonColor,a:f64|c.lighten(a),
        fn Darken(f64)->RibbonColor => |c:&RibbonColor,a:f64|c.darken(a),
        fn Blend(RibbonColor,f64)->RibbonColor => |c:&RibbonColor,b:RibbonColor,a:f64|c.blend(b,a),
        fn ToHsl()->(f64,f64,f64) => |c:&RibbonColor|c.to_hsl(),
        static fn FromHsl(f64,f64,f64)->RibbonColor => RibbonColor::from_hsl,
        static fn FromHsl(f64,f64,f64,u8)->RibbonColor => RibbonColor::from_hsl_with_alpha,
        fn ToHex()->String => |c:&RibbonColor|c.to_hex(false),
        fn ToHex(bool)->String => |c:&RibbonColor,a:bool|c.to_hex(a),
        fn ToString()->String => |c:&RibbonColor|c.to_string(),
    ],
});
/// A descriptor wrapper must name the nullable VALUE, not its marker type.
#[doc(hidden)]
pub struct RibbonColorMetadata;
impl MarkupTyped for RibbonColorMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut meta = *RibbonColorMembers::MARKUP;
            meta.nullable = Some(ValueType::of::<Option<RibbonColor>>);
            meta
        };
        &META
    };
}

#[doc(hidden)]
pub struct RibbonThemePaletteMetadata;
impl RibbonThemePaletteMetadata {
    pub fn create(
        name: Option<String>,
        accent: RibbonColor,
        accent_dark: RibbonColor,
    ) -> Rc<RibbonThemePalette> {
        RibbonThemePalette::new(name.map(Into::into), accent, accent_dark)
    }
    pub fn name(
        palette: &Rc<RibbonThemePalette>,
    ) -> Result<Option<String>, std::string::FromUtf16Error> {
        palette.name.as_ref().map(|s| s.to_utf8()).transpose()
    }
    pub fn from_accent_named(accent: RibbonColor, name: Option<String>) -> Rc<RibbonThemePalette> {
        RibbonThemePalette::from_accent_named(accent, name.map(Into::into))
    }
}
ferroui_base::ferro_markup_type!(class RibbonThemePaletteMetadata as "RibbonThemePalette" {
    namespace:"FerroRibbon.Theming",this:Rc<RibbonThemePalette>,
    handles:[RibbonThemePalette,Rc<RibbonThemePalette>,Option<Rc<RibbonThemePalette>>],
    constructors:[(Option<String>,RibbonColor,RibbonColor)=>RibbonThemePaletteMetadata::create],
    properties:[
        Name:Option<String> {try_get:RibbonThemePaletteMetadata::name},
        Accent:RibbonColor {get:|p:&Rc<RibbonThemePalette>|p.accent},
        AccentDark:RibbonColor {get:|p:&Rc<RibbonThemePalette>|p.accent_dark},
    ],
    static_properties:[
        Word:Rc<RibbonThemePalette> {get:RibbonThemePalette::word},
        Excel:Rc<RibbonThemePalette> {get:RibbonThemePalette::excel},
        PowerPoint:Rc<RibbonThemePalette> {get:RibbonThemePalette::power_point},
        Outlook:Rc<RibbonThemePalette> {get:RibbonThemePalette::outlook},
        OneNote:Rc<RibbonThemePalette> {get:RibbonThemePalette::one_note},
        Access:Rc<RibbonThemePalette> {get:RibbonThemePalette::access},
        Visio:Rc<RibbonThemePalette> {get:RibbonThemePalette::visio},
        Project:Rc<RibbonThemePalette> {get:RibbonThemePalette::project},
        Publisher:Rc<RibbonThemePalette> {get:RibbonThemePalette::publisher},
        Teams:Rc<RibbonThemePalette> {get:RibbonThemePalette::teams},
        Graphite:Rc<RibbonThemePalette> {get:RibbonThemePalette::graphite},
        Cad:Rc<RibbonThemePalette> {get:RibbonThemePalette::cad},
        Presets:Rc<[Rc<RibbonThemePalette>]> {get:RibbonThemePalette::presets},
    ],
    methods:[
        static fn FromAccent(RibbonColor)->Rc<RibbonThemePalette> => RibbonThemePalette::from_accent,
        static fn FromAccent(RibbonColor,Option<String>)->Rc<RibbonThemePalette> => RibbonThemePaletteMetadata::from_accent_named,
        fn GetAccent(bool)->RibbonColor => RibbonThemePalette::get_accent,
        fn GetAccentSubtle(bool)->RibbonColor => RibbonThemePalette::get_accent_subtle,
        fn GetAccentSubtleStrong(bool)->RibbonColor => RibbonThemePalette::get_accent_subtle_strong,
        fn GetOnAccent(bool)->RibbonColor => RibbonThemePalette::get_on_accent,
    ],
});
pub(crate) fn register() {
    MarkupType::register_all(&[
        RibbonColorMetadata::MARKUP,
        RibbonThemePaletteMetadata::MARKUP,
        RibbonThemePalettePresetsMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<RibbonColor>();
    ValueTypes::register_display::<RibbonColor>();
    ValueTypes::register_reference::<RibbonThemePalette>();
}
/// Closed read-only slice specialization. Its identity remains the Core Rc slice.
#[doc(hidden)]
pub struct RibbonThemePalettePresetsMetadata;
impl RibbonThemePalettePresetsMetadata {
    pub fn item(
        presets: &Rc<[Rc<RibbonThemePalette>]>,
        index: i32,
    ) -> Result<Rc<RibbonThemePalette>, String> {
        usize::try_from(index)
            .ok()
            .and_then(|i| presets.get(i))
            .cloned()
            .ok_or_else(|| {
                "IndexOutOfRangeException: Index was outside the bounds of the array.".into()
            })
    }
}
ferroui_base::ferro_markup_type!(class RibbonThemePalettePresetsMetadata as "RibbonThemePalettePresets" {
    namespace:"FerroRibbon.Internal",this:Rc<[Rc<RibbonThemePalette>]>,handles:[Rc<[Rc<RibbonThemePalette>]>],
    properties:[Count:i32 {get:|p:&Rc<[Rc<RibbonThemePalette>]>|i32::try_from(p.len()).expect("Core presets are a fixed 12-element list")}],
    indexers:[(i32)->Rc<RibbonThemePalette> {try_get:RibbonThemePalettePresetsMetadata::item}],
});
