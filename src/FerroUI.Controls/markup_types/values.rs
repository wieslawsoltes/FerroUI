//! Markup metadata of the value types of this crate and of the types that
//! convert from text: their `Parse(string)`, their public constructors and
//! their static values, as the managed original declares them.

use crate::{
    AcrylicPlatformCompensationLevels, GridLength, GridUnitType, TickList, WindowTransparencyLevel,
    WindowTransparencyLevelCollection,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

ferro_markup_type!(struct GridLength {
    namespace: "FerroUI.Controls",
    handles: [GridLength],
    parse: GridLength::parse,
    constructors: [
        () => GridLength::default,
        (f64) => GridLength::from_pixels,
        (f64, GridUnitType) => GridLength::new,
    ],
    static_properties: [
        Auto: GridLength { get: GridLength::auto },
        Star: GridLength { get: GridLength::star },
    ],
    properties: [
        Value: f64 { get: |g: &GridLength| g.value() },
        GridUnitType: GridUnitType { get: |g: &GridLength| g.grid_unit_type() },
        IsAbsolute: bool { get: |g: &GridLength| g.is_absolute() },
        IsAuto: bool { get: |g: &GridLength| g.is_auto() },
        IsStar: bool { get: |g: &GridLength| g.is_star() },
    ],
});

/// A transparency level from its name. (The managed original has no `Parse`:
/// its markup compiler looks the name up among the static values of the type.)
fn parse_transparency_level(s: &str) -> Result<WindowTransparencyLevel, String> {
    match s.trim() {
        "None" => Ok(WindowTransparencyLevel::none()),
        "Transparent" => Ok(WindowTransparencyLevel::transparent()),
        "Blur" => Ok(WindowTransparencyLevel::blur()),
        "AcrylicBlur" => Ok(WindowTransparencyLevel::acrylic_blur()),
        "Mica" => Ok(WindowTransparencyLevel::mica()),
        other => Err(format!("Unable to parse \"{other}\" as a window transparency level.")),
    }
}

/// A comma-separated list of transparency levels.
fn parse_transparency_levels(s: &str) -> Result<WindowTransparencyLevelCollection, String> {
    let levels: Result<Vec<_>, _> = s.split(',').filter(|part| !part.trim().is_empty()).map(parse_transparency_level).collect();
    levels.map(WindowTransparencyLevelCollection::new)
}

ferro_markup_type!(class WindowTransparencyLevelCollection {
    namespace: "FerroUI.Controls",
    handles: [WindowTransparencyLevelCollection, Option<WindowTransparencyLevelCollection>],
    parse: parse_transparency_levels,
});

ferro_markup_type!(struct WindowTransparencyLevel {
    namespace: "FerroUI.Controls",
    handles: [WindowTransparencyLevel],
    parse: parse_transparency_level,
    static_properties: [
        None: WindowTransparencyLevel { get: WindowTransparencyLevel::none },
        Transparent: WindowTransparencyLevel { get: WindowTransparencyLevel::transparent },
        Blur: WindowTransparencyLevel { get: WindowTransparencyLevel::blur },
        AcrylicBlur: WindowTransparencyLevel { get: WindowTransparencyLevel::acrylic_blur },
        Mica: WindowTransparencyLevel { get: WindowTransparencyLevel::mica },
    ],
    constructors: [() => WindowTransparencyLevel::none],
});

ferro_markup_type!(struct AcrylicPlatformCompensationLevels {
    namespace: "FerroUI.Controls",
    handles: [AcrylicPlatformCompensationLevels],
    constructors: [() => AcrylicPlatformCompensationLevels::default, (f64, f64, f64) => AcrylicPlatformCompensationLevels::new],
});

// The list of numbers of the `Ticks` properties.
ferro_markup_type!(class TickList {
    namespace: "FerroUI.Controls",
    handles: [TickList, Option<TickList>],
    base: ferroui_base::media::MediaCollection<f64>,
    constructors: [() => TickList::new],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <GridLength as MarkupTyped>::MARKUP,
    <WindowTransparencyLevel as MarkupTyped>::MARKUP,
    <WindowTransparencyLevelCollection as MarkupTyped>::MARKUP,
    <AcrylicPlatformCompensationLevels as MarkupTyped>::MARKUP,
    <TickList as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the value types with the untyped value
/// conversions of the current thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<GridLength>();
    ValueTypes::register_nullable::<WindowTransparencyLevel>();
    ValueTypes::register_nullable::<WindowTransparencyLevelCollection>();
    ValueTypes::register_nullable::<AcrylicPlatformCompensationLevels>();
    ValueTypes::register_nullable::<TickList>();
    // The text of a value (its `ToString()` in the managed original).
    ValueTypes::register_display::<GridLength>();
    ValueTypes::register_display::<WindowTransparencyLevel>();
}
