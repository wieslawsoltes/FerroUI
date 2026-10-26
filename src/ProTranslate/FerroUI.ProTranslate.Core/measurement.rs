use crate::{Culture, FormatArgument, RegionProfile, TranslationError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MeasurementSystem {
    #[default]
    Metric,
    USCustomary,
    Imperial,
    Custom,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementUnitCategory {
    Length,
    Temperature,
    Mass,
    Volume,
    Speed,
}
macro_rules! units {
    ($( $unit:ident, $category:ident, $symbol:literal, $scale:expr; )*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum MeasurementUnit { $( $unit, )* }
        impl MeasurementUnit {
            pub const ALL: &'static [Self] = &[$(Self::$unit,)*];
            pub fn category(self) -> MeasurementUnitCategory { match self { $(Self::$unit => MeasurementUnitCategory::$category,)* } }
            pub fn symbol(self) -> &'static str { match self { $(Self::$unit => $symbol,)* } }
            fn scale(self) -> f64 { match self { $(Self::$unit => $scale,)* } }
        }
    }
}
units! {
    Meter, Length, "m", 1.0;
    Kilometer, Length, "km", 1000.0;
    Centimeter, Length, "cm", 0.01;
    Millimeter, Length, "mm", 0.001;
    Inch, Length, "in", 0.0254;
    Foot, Length, "ft", 0.3048;
    Yard, Length, "yd", 0.9144;
    Mile, Length, "mi", 1609.344;
    Celsius, Temperature, "°C", 1.0;
    Fahrenheit, Temperature, "°F", 5.0 / 9.0;
    Kelvin, Temperature, "K", 1.0;
    Gram, Mass, "g", 0.001;
    Kilogram, Mass, "kg", 1.0;
    Ounce, Mass, "oz", 0.028349523125;
    Pound, Mass, "lb", 0.45359237;
    Stone, Mass, "st", 6.35029318;
    Milliliter, Volume, "mL", 0.001;
    Liter, Volume, "L", 1.0;
    USFluidOunce, Volume, "us fl oz", 0.0295735295625;
    USCup, Volume, "cup", 0.2365882365;
    USPint, Volume, "us pt", 0.473176473;
    USQuart, Volume, "us qt", 0.946352946;
    USGallon, Volume, "gal", 3.785411784;
    ImperialFluidOunce, Volume, "imp fl oz", 0.0284130625;
    ImperialPint, Volume, "imp pt", 0.56826125;
    ImperialQuart, Volume, "imp qt", 1.1365225;
    ImperialGallon, Volume, "imp gal", 4.54609;
    MetersPerSecond, Speed, "m/s", 1.0;
    KilometersPerHour, Speed, "km/h", 1000.0 / 3600.0;
    MilesPerHour, Speed, "mph", 1609.344 / 3600.0;
    FeetPerSecond, Speed, "ft/s", 0.3048;
}
impl MeasurementUnit {
    pub fn from_symbol(symbol: &str, system: MeasurementSystem) -> Result<Self, TranslationError> {
        match symbol {
            "C" => Ok(Self::Celsius),
            "F" => Ok(Self::Fahrenheit),
            "ml" => Ok(Self::Milliliter),
            "l" => Ok(Self::Liter),
            "pt" => Ok(Self::USPint),
            "qt" => Ok(Self::USQuart),
            "us cup" => Ok(Self::USCup),
            "gal" if system == MeasurementSystem::Imperial => Ok(Self::ImperialGallon),
            value => Self::ALL
                .iter()
                .find(|unit| unit.symbol() == value)
                .copied()
                .ok_or_else(|| TranslationError::UnsupportedProfileUnit(value.into())),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasurementValue {
    pub value: f64,
    pub unit: MeasurementUnit,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementSystemProfile {
    pub system: MeasurementSystem,
    pub length_unit: String,
    pub temperature_unit: String,
    pub mass_unit: String,
    pub volume_unit: String,
    pub speed_unit: String,
}
impl MeasurementSystemProfile {
    pub fn for_system(system: MeasurementSystem) -> Self {
        let units = match system {
            MeasurementSystem::Metric => ["m", "C", "kg", "l", "km/h"],
            MeasurementSystem::USCustomary => ["ft", "F", "lb", "gal", "mph"],
            MeasurementSystem::Imperial => ["ft", "C", "st", "imp gal", "mph"],
            MeasurementSystem::Custom => ["", "", "", "", ""],
        };
        Self {
            system,
            length_unit: units[0].into(),
            temperature_unit: units[1].into(),
            mass_unit: units[2].into(),
            volume_unit: units[3].into(),
            speed_unit: units[4].into(),
        }
    }
    pub fn unit_for(
        &self,
        category: MeasurementUnitCategory,
    ) -> Result<MeasurementUnit, TranslationError> {
        let symbol = match category {
            MeasurementUnitCategory::Length => &self.length_unit,
            MeasurementUnitCategory::Temperature => &self.temperature_unit,
            MeasurementUnitCategory::Mass => &self.mass_unit,
            MeasurementUnitCategory::Volume => &self.volume_unit,
            MeasurementUnitCategory::Speed => &self.speed_unit,
        };
        let unit = MeasurementUnit::from_symbol(symbol, self.system)?;
        if unit.category() != category {
            return Err(TranslationError::IncompatibleUnits);
        }
        Ok(unit)
    }
}
pub trait MeasurementSystemResolver {
    fn resolve(
        &self,
        region: &RegionProfile,
        override_system: Option<MeasurementSystem>,
    ) -> MeasurementSystemProfile;
}
#[derive(Default)]
pub struct DefaultMeasurementSystemResolver;
impl MeasurementSystemResolver for DefaultMeasurementSystemResolver {
    fn resolve(
        &self,
        region: &RegionProfile,
        override_system: Option<MeasurementSystem>,
    ) -> MeasurementSystemProfile {
        MeasurementSystemProfile::for_system(override_system.unwrap_or({
            match region.two_letter_iso_region_name.as_str() {
                "US" | "LR" | "MM" => MeasurementSystem::USCustomary,
                "GB" => MeasurementSystem::Imperial,
                _ if region.is_metric => MeasurementSystem::Metric,
                _ => MeasurementSystem::Custom,
            }
        }))
    }
}
pub trait UnitConversionService {
    fn get_category(&self, unit: MeasurementUnit) -> MeasurementUnitCategory {
        unit.category()
    }
    fn convert(
        &self,
        value: f64,
        from: MeasurementUnit,
        to: MeasurementUnit,
    ) -> Result<f64, TranslationError>;
    fn convert_to_profile(
        &self,
        value: f64,
        from: MeasurementUnit,
        profile: &MeasurementSystemProfile,
    ) -> Result<MeasurementValue, TranslationError> {
        let unit = profile.unit_for(self.get_category(from))?;
        Ok(MeasurementValue {
            value: self.convert(value, from, unit)?,
            unit,
        })
    }
}
#[derive(Default)]
pub struct DefaultUnitConversionService;
impl UnitConversionService for DefaultUnitConversionService {
    fn convert(
        &self,
        value: f64,
        from: MeasurementUnit,
        to: MeasurementUnit,
    ) -> Result<f64, TranslationError> {
        if from.category() != to.category() {
            return Err(TranslationError::IncompatibleUnits);
        }
        if from == to {
            return Ok(value);
        }
        if from.category() == MeasurementUnitCategory::Temperature {
            let celsius = match from {
                MeasurementUnit::Fahrenheit => (value - 32.0) * 5.0 / 9.0,
                MeasurementUnit::Kelvin => value - 273.15,
                _ => value,
            };
            return Ok(match to {
                MeasurementUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
                MeasurementUnit::Kelvin => celsius + 273.15,
                _ => celsius,
            });
        }
        Ok(value * from.scale() / to.scale())
    }
}
pub trait LocalizedUnitFormatter {
    fn format(
        &self,
        value: MeasurementValue,
        culture: &Culture,
        format: Option<&str>,
    ) -> Result<String, TranslationError>;
}
#[derive(Default)]
pub struct DefaultLocalizedUnitFormatter;
impl LocalizedUnitFormatter for DefaultLocalizedUnitFormatter {
    fn format(
        &self,
        value: MeasurementValue,
        culture: &Culture,
        format: Option<&str>,
    ) -> Result<String, TranslationError> {
        Ok(format!(
            "{} {}",
            FormatArgument::Number(value.value).format(format.unwrap_or("G"), culture)?,
            value.unit.symbol()
        ))
    }
}
