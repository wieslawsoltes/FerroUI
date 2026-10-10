//! The scale factors of the screens (the port of `X11Screens.Scaling.cs`):
//! the providers that answer the scaling of a screen, and the choice among
//! them by the environment.
//!
//! The reference nests these types in its screens class; here they are the
//! items of this module.

use super::x11_screen_providers::X11Screen;
use crate::event::Event;
use crate::x11_platform::FerroX11Platform;
use crate::x_resources::XResources;
use ferroui_base::utilities::span_helpers::{self, NumberStyles};
use ferroui_base::{PixelRect, Size};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// The global scale factor of an application of this framework.
///
/// The reference reads a variable with the prefix of its own project name;
/// the rest of the name is the same.
pub const GLOBAL_SCALE_FACTOR_VARIABLE: &str = "FERROUI_GLOBAL_SCALE_FACTOR";
/// The scale factors per screen (`2;1.5` by index, or `DP-1=2;HDMI-1=1.5`
/// by name). See [`GLOBAL_SCALE_FACTOR_VARIABLE`] for the name.
pub const SCREEN_SCALE_FACTORS_VARIABLE: &str = "FERROUI_SCREEN_SCALE_FACTORS";
/// `1` derives the scaling from the physical size of a screen. See
/// [`GLOBAL_SCALE_FACTOR_VARIABLE`] for the name.
pub const USE_PHYSICAL_DPI_VARIABLE: &str = "FERROUI_USE_PHYSICAL_DPI";
/// `1` ignores the scaling variables of Qt. See
/// [`GLOBAL_SCALE_FACTOR_VARIABLE`] for the name.
pub const SCREEN_SCALE_IGNORE_QT_VARIABLE: &str = "FERROUI_SCREEN_SCALE_IGNORE_QT";

/// Answers the scaling of a screen (`IScalingProvider`).
pub trait IScalingProvider {
    fn get_scaling(&self, screen: &X11Screen, index: i32) -> f64;

    /// The event of a provider whose answer can change
    /// (`IScalingProviderWithChanges.SettingsChanged`); `None` for a
    /// provider that is not one (the reference tests for the interface).
    fn settings_changed(&self) -> Option<&Event> {
        None
    }
}

/// Multiplies the answer of another provider by a factor.
///
/// As in the reference, this provider does not forward the changes of the
/// provider it wraps: it is not a provider with changes.
pub struct PostMultiplyScalingProvider {
    inner: Rc<dyn IScalingProvider>,
    factor: f64,
}

impl PostMultiplyScalingProvider {
    pub fn new(inner: Rc<dyn IScalingProvider>, factor: f64) -> Self {
        Self { inner, factor }
    }
}

impl IScalingProvider for PostMultiplyScalingProvider {
    fn get_scaling(&self, screen: &X11Screen, index: i32) -> f64 {
        self.inner.get_scaling(screen, index) * self.factor
    }
}

/// Answers 1 for every screen.
pub struct NullScalingProvider;

impl IScalingProvider for NullScalingProvider {
    fn get_scaling(&self, _screen: &X11Screen, _index: i32) -> f64 {
        1.0
    }
}

/// A number in the invariant culture with every style but the hexadecimal
/// one (`double.TryParse(text, NumberStyles.Any, CultureInfo.InvariantCulture, ..)`).
///
/// The shared parser of the framework knows white space, signs, group
/// separators, the decimal point and the exponent. The two further
/// elements of the style are handled here in their common forms: one
/// currency symbol of the invariant culture at the start or the end, and a
/// pair of parentheses around the number, which make it negative.
pub fn try_parse_double_any(text: &str) -> Option<f64> {
    let styles = NumberStyles::FLOAT_THOUSANDS | NumberStyles::ALLOW_TRAILING_SIGN;
    if let Some(value) = span_helpers::try_parse_double(text, styles) {
        return Some(value);
    }

    fn trim_white(text: &str) -> &str {
        text.trim_matches(|c: char| c == ' ' || ('\u{9}'..='\u{d}').contains(&c))
    }

    const CURRENCY_SYMBOL: char = '\u{a4}';
    let mut body = trim_white(text);
    let mut has_currency = false;
    let mut negative = false;
    let mut has_parentheses = false;

    if let Some(rest) = body.strip_prefix(CURRENCY_SYMBOL) {
        has_currency = true;
        body = trim_white(rest);
    }
    if let Some(rest) = body.strip_prefix('(') {
        has_parentheses = true;
        negative = true;
        body = rest;
        if !has_currency {
            if let Some(rest) = body.strip_prefix(CURRENCY_SYMBOL) {
                has_currency = true;
                body = trim_white(rest);
            }
        }
    }
    if !has_currency {
        if let Some(rest) = body.strip_suffix(CURRENCY_SYMBOL) {
            has_currency = true;
            body = trim_white(rest);
        }
    }
    if has_parentheses {
        body = trim_white(body.strip_suffix(')')?);
        if !has_currency {
            if let Some(rest) = body.strip_suffix(CURRENCY_SYMBOL) {
                has_currency = true;
                body = trim_white(rest);
            }
        }
    }
    if !has_currency && !has_parentheses {
        // Nothing the shared parser did not already see.
        return None;
    }

    // Inside parentheses there is no sign: the parenthesis is the sign.
    let inner_styles = if has_parentheses {
        NumberStyles::ALLOW_DECIMAL_POINT | NumberStyles::ALLOW_THOUSANDS | NumberStyles::ALLOW_EXPONENT
    } else {
        NumberStyles::ALLOW_LEADING_SIGN
            | NumberStyles::ALLOW_TRAILING_SIGN
            | NumberStyles::ALLOW_DECIMAL_POINT
            | NumberStyles::ALLOW_THOUSANDS
            | NumberStyles::ALLOW_EXPONENT
    };
    // Only a number is left: the names of the values that are not finite
    // do not combine with a currency symbol or parentheses.
    let is_number = body.bytes().all(|c| c.is_ascii_digit() || matches!(c, b'.' | b',' | b'e' | b'E' | b'+' | b'-'));
    if !is_number {
        return None;
    }
    let value = span_helpers::try_parse_double(body, inner_styles)?;
    Some(if negative { -value } else { value })
}

/// The scale factor of the `Xft.dpi` resource: the resolution over 96, and
/// 1 when the resource is missing, blank or not a number.
pub fn xft_dpi_factor(resource: Option<&str>) -> f64 {
    let mut factor = 1.0;
    let string_value = resource.map(str::trim);
    if let Some(string_value) = string_value {
        if !string_value.trim().is_empty() {
            if let Some(parsed) = try_parse_double_any(string_value) {
                factor = parsed / 96.0;
            }
        }
    }
    factor
}

/// Answers the scaling of the `Xft.dpi` resource of the server for every
/// screen, and says when it changes.
pub struct XrdbScalingProvider {
    resources: Rc<XResources>,
    factor: Cell<f64>,
    settings_changed: Event,
}

impl XrdbScalingProvider {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let this = Rc::new(Self {
            resources: platform.resources().clone(),
            factor: Cell::new(1.0),
            settings_changed: Event::new(),
        });
        let weak = Rc::downgrade(&this);
        this.resources.resource_changed.subscribe(move |name: String| {
            if name == "Xft.dpi" {
                if let Some(this) = weak.upgrade() {
                    this.update();
                }
            }
        });
        this.update();
        this
    }

    fn update(&self) {
        let factor = xft_dpi_factor(self.resources.get_resource("Xft.dpi").as_deref());

        // The floats are compared by equality on purpose.
        if self.factor.get() != factor {
            self.factor.set(factor);
            self.settings_changed.raise();
        }
    }
}

impl IScalingProvider for XrdbScalingProvider {
    fn get_scaling(&self, _screen: &X11Screen, _index: i32) -> f64 {
        self.factor.get()
    }

    fn settings_changed(&self) -> Option<&Event> {
        Some(&self.settings_changed)
    }
}

const FULL_HD_WIDTH: i32 = 1920;
const FULL_HD_HEIGHT: i32 = 1080;

/// A scale factor for a screen of `pixel` pixels that measures `physical`
/// millimetres: one of 1, 1.25, 1.5, 1.75 and 2.
pub fn guess_pixel_density(pixel: PixelRect, physical: Size) -> f64 {
    let mut calculated_density = 1.0_f64;
    if physical.width > 0.0 {
        calculated_density = if pixel.width <= FULL_HD_WIDTH {
            1.0
        } else {
            f64::max(1.0, pixel.width as f64 / physical.width * 25.4 / 96.0)
        };
    } else if physical.height > 0.0 {
        calculated_density = if pixel.height <= FULL_HD_HEIGHT {
            1.0
        } else {
            f64::max(1.0, pixel.height as f64 / physical.height * 25.4 / 96.0)
        };
    }

    if calculated_density > 3.0 {
        1.0
    } else {
        let sane_pixel_densities = [1.0, 1.25, 1.50, 1.75, 2.0];
        for sane_density in sane_pixel_densities {
            if calculated_density <= sane_density + 0.20 {
                return sane_density;
            }
        }

        sane_pixel_densities[sane_pixel_densities.len() - 1]
    }
}

/// The scaling of a screen by its physical size: 1 when the size is not
/// known.
pub fn physical_dpi_scaling(bounds: PixelRect, physical_size: Option<Size>) -> f64 {
    match physical_size {
        None => 1.0,
        Some(physical_size) => guess_pixel_density(bounds, physical_size),
    }
}

/// Derives the scaling of a screen from its physical size.
pub struct PhysicalDpiScalingProvider;

impl IScalingProvider for PhysicalDpiScalingProvider {
    fn get_scaling(&self, screen: &X11Screen, _index: i32) -> f64 {
        physical_dpi_scaling(screen.bounds(), screen.physical_size())
    }
}

/// The scaling the user configured for a screen: by index when there is a
/// list of factors (1 for an index outside of it), else by name, else 1.
///
/// # Panics
/// Panics when the factors are by name and the screen has no name, as the
/// lookup of the reference throws for a null key.
pub fn user_configured_scaling(
    named_config: Option<&HashMap<String, f64>>,
    indexed_config: Option<&[f64]>,
    display_name: Option<&str>,
    index: i32,
) -> f64 {
    if let Some(indexed_config) = indexed_config {
        if index >= 0 && (index as usize) < indexed_config.len() {
            return indexed_config[index as usize];
        }
        return 1.0;
    }
    if let Some(named_config) = named_config {
        let display_name = display_name.expect("Value cannot be null. (Parameter 'key')");
        if let Some(scaling) = named_config.get(display_name) {
            return *scaling;
        }
    }

    1.0
}

/// Answers the scale factors the user configured.
pub struct UserConfiguredScalingProvider {
    named_config: Option<HashMap<String, f64>>,
    indexed_config: Option<Vec<f64>>,
}

impl UserConfiguredScalingProvider {
    pub fn new(named_config: Option<HashMap<String, f64>>, indexed_config: Option<Vec<f64>>) -> Self {
        Self { named_config, indexed_config }
    }
}

impl IScalingProvider for UserConfiguredScalingProvider {
    fn get_scaling(&self, screen: &X11Screen, index: i32) -> f64 {
        user_configured_scaling(
            self.named_config.as_ref(),
            self.indexed_config.as_deref(),
            screen.display_name().as_deref(),
            index,
        )
    }
}

/// The scale factors per screen of the user: by name or by index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserScalingConfiguration {
    pub named_config: Option<HashMap<String, f64>>,
    pub indexed_config: Option<Vec<f64>>,
}

/// What a set of environment variables configures.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvConfiguration {
    pub config: Option<UserScalingConfiguration>,
    pub global: f64,
    pub force_auto: bool,
}

fn is_null_or_white_space(text: Option<&str>) -> bool {
    text.is_none_or(|text| text.trim().is_empty())
}

/// The scale factors per screen of the value of a variable: `None` when
/// the value cannot be parsed (the reference catches the failure).
///
/// The items are separated by semicolons and blank items are skipped. When
/// the first item has an equals sign every item is `name=factor` (a name
/// twice, or a later item without an equals sign, is a failure); else
/// every item is a factor.
pub fn parse_screen_scale_factors(screen_factors_string: &str) -> Option<UserScalingConfiguration> {
    let split: Vec<&str> = screen_factors_string.split(';').filter(|x| !x.trim().is_empty()).collect();
    if split.first()?.contains('=') {
        let mut named_config = HashMap::new();
        for x in &split {
            let (name, factor) = x.split_once('=')?;
            let factor = span_helpers::parse_double(factor)?;
            if named_config.insert(name.to_string(), factor).is_some() {
                return None;
            }
        }
        Some(UserScalingConfiguration { named_config: Some(named_config), indexed_config: None })
    } else {
        let indexed_config = split.iter().map(|x| span_helpers::parse_double(x)).collect::<Option<Vec<f64>>>()?;
        Some(UserScalingConfiguration { named_config: None, indexed_config: Some(indexed_config) })
    }
}

/// What the variables of one toolkit configure; `None` when neither the
/// global factor nor the factors per screen are set. `get_env` answers the
/// value of an environment variable.
pub fn try_get_env_configuration(
    global_factor_name: &str,
    user_config_name: &str,
    auto_names: &[&str],
    get_env: &dyn Fn(&str) -> Option<String>,
) -> Option<EnvConfiguration> {
    let global_factor_string = get_env(global_factor_name);
    let screen_factors_string = get_env(user_config_name);
    let mut use_physical_dpi = false;
    for auto_name in auto_names {
        let env_value = get_env(auto_name);
        if env_value.as_deref() == Some("1") {
            use_physical_dpi = true;
        }
    }

    let mut global_factor = None;
    if !is_null_or_white_space(global_factor_string.as_deref()) {
        if let Some(parsed) = global_factor_string.as_deref().and_then(try_parse_double_any) {
            global_factor = Some(parsed);
        }
    }

    let mut user_config = None;
    if let Some(screen_factors_string) = screen_factors_string.as_deref() {
        if !screen_factors_string.trim().is_empty() {
            user_config = parse_screen_scale_factors(screen_factors_string);
            if user_config.is_none() {
                eprintln!("Unable to parse {user_config_name}={screen_factors_string}");
            }
        }
    }

    if global_factor_string.is_none() && screen_factors_string.is_none() {
        return None;
    }

    Some(EnvConfiguration { config: user_config, global: global_factor.unwrap_or(1.0), force_auto: use_physical_dpi })
}

/// The configuration of the first set of variables that configures
/// anything: the ones of this framework, then the ones of Qt unless they
/// are to be ignored. Without any: no factors, a global factor of 1 and
/// no derivation from the physical size.
pub fn get_env_scaling_configuration(get_env: &dyn Fn(&str) -> Option<String>) -> EnvConfiguration {
    const OWN_AUTO_NAMES: &[&str] = &[USE_PHYSICAL_DPI_VARIABLE];
    const QT_AUTO_NAMES: &[&str] = &["QT_AUTO_SCREEN_SCALE_FACTOR", "QT_USE_PHYSICAL_DPI"];

    let mut env_sets: Vec<(&str, &str, &[&str])> =
        vec![(GLOBAL_SCALE_FACTOR_VARIABLE, SCREEN_SCALE_FACTORS_VARIABLE, OWN_AUTO_NAMES)];

    if get_env(SCREEN_SCALE_IGNORE_QT_VARIABLE).as_deref() != Some("1") {
        env_sets.push(("QT_SCALE_FACTOR", "QT_SCREEN_SCALE_FACTORS", QT_AUTO_NAMES));
    }

    for (global_factor_name, user_config_name, auto_names) in env_sets {
        if let Some(env_config) = try_get_env_configuration(global_factor_name, user_config_name, auto_names, get_env)
        {
            return env_config;
        }
    }

    EnvConfiguration { config: None, global: 1.0, force_auto: false }
}

/// The value of an environment variable of the process.
fn environment_variable(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

/// The scaling provider the environment asks for: the factors of the user
/// when there are any, else the physical size when it is asked for, else
/// the `Xft.dpi` resource; multiplied by the global factor when it is
/// not 1.
pub fn get_scaling_provider(platform: &Rc<FerroX11Platform>) -> Rc<dyn IScalingProvider> {
    let EnvConfiguration { config, global, force_auto } = get_env_scaling_configuration(&environment_variable);

    let mut provider: Rc<dyn IScalingProvider> = if let Some(config) = config {
        Rc::new(UserConfiguredScalingProvider::new(config.named_config, config.indexed_config))
    } else if force_auto {
        Rc::new(PhysicalDpiScalingProvider)
    } else {
        XrdbScalingProvider::new(platform)
    };

    // The floats are compared by equality on purpose.
    if global != 1.0 {
        provider = Rc::new(PostMultiplyScalingProvider::new(provider, global));
    }

    provider
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn env<'a>(variables: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| variables.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string())
    }

    #[test]
    fn numbers_of_any_style() {
        assert_eq!(try_parse_double_any("96"), Some(96.0));
        assert_eq!(try_parse_double_any("  1.5\t"), Some(1.5));
        assert_eq!(try_parse_double_any("-2"), Some(-2.0));
        assert_eq!(try_parse_double_any("2-"), Some(-2.0));
        assert_eq!(try_parse_double_any("1e2"), Some(100.0));
        // Group separators are skipped, whatever the size of the groups.
        assert_eq!(try_parse_double_any("1,5"), Some(15.0));
        assert_eq!(try_parse_double_any("(2.5)"), Some(-2.5));
        assert_eq!(try_parse_double_any("\u{a4}3"), Some(3.0));
        assert_eq!(try_parse_double_any("3 \u{a4}"), Some(3.0));
        assert_eq!(try_parse_double_any("(-2)"), None);
        assert_eq!(try_parse_double_any("(2"), None);
        assert_eq!(try_parse_double_any(""), None);
        assert_eq!(try_parse_double_any("   "), None);
        assert_eq!(try_parse_double_any("two"), None);
        assert_eq!(try_parse_double_any("1.5x"), None);
        assert_eq!(try_parse_double_any("0x10"), None);
    }

    #[test]
    fn the_xft_resolution_is_a_factor_of_96() {
        assert_eq!(xft_dpi_factor(Some("96")), 1.0);
        assert_eq!(xft_dpi_factor(Some("144")), 1.5);
        assert_eq!(xft_dpi_factor(Some(" 192\t")), 2.0);
        assert_eq!(xft_dpi_factor(Some("120.0")), 1.25);
        // A resolution of zero is taken as it is.
        assert_eq!(xft_dpi_factor(Some("0")), 0.0);
    }

    #[test]
    fn a_missing_or_malformed_xft_resolution_is_a_factor_of_one() {
        assert_eq!(xft_dpi_factor(None), 1.0);
        assert_eq!(xft_dpi_factor(Some("")), 1.0);
        assert_eq!(xft_dpi_factor(Some("   ")), 1.0);
        assert_eq!(xft_dpi_factor(Some("high")), 1.0);
        assert_eq!(xft_dpi_factor(Some("96dpi")), 1.0);
    }

    #[test]
    fn densities_of_physical_sizes() {
        let uhd = PixelRect::new(0, 0, 3840, 2160);
        // Full HD and below is never scaled, whatever its size.
        assert_eq!(guess_pixel_density(PixelRect::new(0, 0, 1920, 1080), Size::new(100.0, 60.0)), 1.0);
        assert_eq!(guess_pixel_density(PixelRect::new(0, 0, 1366, 768), Size::new(10.0, 10.0)), 1.0);
        // 3840 pixels over 600 mm: 1.69, which is within 0.2 of 1.5.
        assert_eq!(guess_pixel_density(uhd, Size::new(600.0, 340.0)), 1.5);
        // 3840 pixels over 340 mm: 2.99, above every sane density.
        assert_eq!(guess_pixel_density(uhd, Size::new(340.0, 190.0)), 2.0);
        // 3840 pixels over 300 mm: 3.39, which is not believed.
        assert_eq!(guess_pixel_density(uhd, Size::new(300.0, 170.0)), 1.0);
        // A large screen with few pixels is not scaled below 1.
        assert_eq!(guess_pixel_density(PixelRect::new(0, 0, 2560, 1440), Size::new(2000.0, 1100.0)), 1.0);
    }

    #[test]
    fn the_height_is_used_when_the_width_is_not_known() {
        let uhd = PixelRect::new(0, 0, 3840, 2160);
        // 2160 pixels over 300 mm: 1.905, which is within 0.2 of 1.75.
        assert_eq!(guess_pixel_density(uhd, Size::new(0.0, 300.0)), 1.75);
        assert_eq!(guess_pixel_density(PixelRect::new(0, 0, 3840, 1080), Size::new(0.0, 100.0)), 1.0);
        // With a width, the height is not looked at.
        assert_eq!(guess_pixel_density(PixelRect::new(0, 0, 1920, 2160), Size::new(500.0, 300.0)), 1.0);
    }

    #[test]
    fn sizes_of_zero_are_a_density_of_one() {
        let uhd = PixelRect::new(0, 0, 3840, 2160);
        assert_eq!(guess_pixel_density(uhd, Size::new(0.0, 0.0)), 1.0);
        assert_eq!(guess_pixel_density(uhd, Size::new(-1.0, -1.0)), 1.0);
        assert_eq!(guess_pixel_density(PixelRect::default(), Size::new(0.0, 0.0)), 1.0);
        assert_eq!(guess_pixel_density(PixelRect::default(), Size::new(300.0, 200.0)), 1.0);
        assert_eq!(physical_dpi_scaling(uhd, None), 1.0);
        assert_eq!(physical_dpi_scaling(uhd, Some(Size::new(600.0, 340.0))), 1.5);
    }

    #[test]
    fn factors_of_the_user_by_index() {
        let indexed = [2.0, 1.5];
        assert_eq!(user_configured_scaling(None, Some(&indexed), Some("DP-1"), 0), 2.0);
        assert_eq!(user_configured_scaling(None, Some(&indexed), None, 1), 1.5);
        assert_eq!(user_configured_scaling(None, Some(&indexed), Some("DP-1"), 2), 1.0);
        assert_eq!(user_configured_scaling(None, Some(&indexed), Some("DP-1"), -1), 1.0);
        // The list wins over the names.
        let named = HashMap::from([("DP-1".to_string(), 3.0)]);
        assert_eq!(user_configured_scaling(Some(&named), Some(&[]), Some("DP-1"), 0), 1.0);
    }

    #[test]
    fn factors_of_the_user_by_name() {
        let named = HashMap::from([("DP-1".to_string(), 2.0), ("HDMI-1".to_string(), 1.25)]);
        assert_eq!(user_configured_scaling(Some(&named), None, Some("DP-1"), 5), 2.0);
        assert_eq!(user_configured_scaling(Some(&named), None, Some("HDMI-1"), 0), 1.25);
        assert_eq!(user_configured_scaling(Some(&named), None, Some("eDP-1"), 0), 1.0);
        assert_eq!(user_configured_scaling(None, None, None, 0), 1.0);
    }

    #[test]
    #[should_panic(expected = "Value cannot be null")]
    fn a_screen_without_a_name_fails_the_lookup_by_name() {
        let named = HashMap::from([("DP-1".to_string(), 2.0)]);
        user_configured_scaling(Some(&named), None, None, 0);
    }

    #[test]
    fn screen_factors_by_index() {
        let config = parse_screen_scale_factors("2;1.5").unwrap();
        assert_eq!(config.indexed_config, Some(vec![2.0, 1.5]));
        assert_eq!(config.named_config, None);
        // Blank items are skipped; white space around a number is fine.
        let config = parse_screen_scale_factors(";2; ; 1.25 ;").unwrap();
        assert_eq!(config.indexed_config, Some(vec![2.0, 1.25]));
    }

    #[test]
    fn screen_factors_by_name() {
        let config = parse_screen_scale_factors("DP-1=2;HDMI-1=1.5;").unwrap();
        assert_eq!(config.indexed_config, None);
        assert_eq!(
            config.named_config,
            Some(HashMap::from([("DP-1".to_string(), 2.0), ("HDMI-1".to_string(), 1.5)]))
        );
        // Only the first equals sign separates, and names are not trimmed.
        assert_eq!(parse_screen_scale_factors("a=b=2"), None);
        let config = parse_screen_scale_factors(" DP-1=2").unwrap();
        assert_eq!(config.named_config, Some(HashMap::from([(" DP-1".to_string(), 2.0)])));
    }

    #[test]
    fn malformed_screen_factors() {
        assert_eq!(parse_screen_scale_factors(";"), None);
        assert_eq!(parse_screen_scale_factors("two"), None);
        assert_eq!(parse_screen_scale_factors("2;x"), None);
        assert_eq!(parse_screen_scale_factors("DP-1=2;1.5"), None);
        assert_eq!(parse_screen_scale_factors("DP-1=2;DP-1=3"), None);
        assert_eq!(parse_screen_scale_factors("DP-1="), None);
        // A list by index whose later item has an equals sign.
        assert_eq!(parse_screen_scale_factors("2;DP-1=3"), None);
    }

    #[test]
    fn no_variables_no_configuration() {
        assert_eq!(try_get_env_configuration("G", "S", &["A"], &env(&[])), None);
        // The variables that ask for the physical size do not make a
        // configuration alone.
        assert_eq!(try_get_env_configuration("G", "S", &["A"], &env(&[("A", "1")])), None);
    }

    #[test]
    fn the_variables_of_one_set() {
        let config = try_get_env_configuration("G", "S", &["A", "B"], &env(&[("G", "1.5")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 1.5, force_auto: false });

        let config = try_get_env_configuration("G", "S", &["A", "B"], &env(&[("G", "2"), ("B", "1")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 2.0, force_auto: true });

        // Only the value `1` asks for the physical size.
        let config = try_get_env_configuration("G", "S", &["A", "B"], &env(&[("G", "2"), ("A", "true")])).unwrap();
        assert!(!config.force_auto);

        let config = try_get_env_configuration("G", "S", &[], &env(&[("S", "2;1")])).unwrap();
        assert_eq!(config.global, 1.0);
        assert_eq!(config.config.unwrap().indexed_config, Some(vec![2.0, 1.0]));
    }

    #[test]
    fn malformed_variables_of_one_set() {
        // A global factor that is not a number is a factor of 1, and the
        // set still counts as configured.
        let config = try_get_env_configuration("G", "S", &[], &env(&[("G", "big")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 1.0, force_auto: false });
        let config = try_get_env_configuration("G", "S", &[], &env(&[("G", "")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 1.0, force_auto: false });
        // Factors per screen that cannot be parsed are no factors.
        let config = try_get_env_configuration("G", "S", &[], &env(&[("S", "a;b"), ("G", "2")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 2.0, force_auto: false });
        let config = try_get_env_configuration("G", "S", &[], &env(&[("S", "  ")])).unwrap();
        assert_eq!(config, EnvConfiguration { config: None, global: 1.0, force_auto: false });
    }

    #[test]
    fn the_variables_of_this_framework_come_before_the_ones_of_qt() {
        let default = EnvConfiguration { config: None, global: 1.0, force_auto: false };
        assert_eq!(get_env_scaling_configuration(&env(&[])), default);

        let config = get_env_scaling_configuration(&env(&[("QT_SCALE_FACTOR", "2")]));
        assert_eq!(config.global, 2.0);

        let config = get_env_scaling_configuration(&env(&[
            ("QT_SCALE_FACTOR", "2"),
            ("QT_AUTO_SCREEN_SCALE_FACTOR", "1"),
            (GLOBAL_SCALE_FACTOR_VARIABLE, "1.5"),
        ]));
        assert_eq!(config, EnvConfiguration { config: None, global: 1.5, force_auto: false });

        let config = get_env_scaling_configuration(&env(&[
            ("QT_SCREEN_SCALE_FACTORS", "eDP-1=2"),
            ("QT_USE_PHYSICAL_DPI", "1"),
            (USE_PHYSICAL_DPI_VARIABLE, "1"),
        ]));
        assert!(config.force_auto);
        assert_eq!(config.config.unwrap().named_config, Some(HashMap::from([("eDP-1".to_string(), 2.0)])));

        // The variable that asks for the physical size is not a
        // configuration alone, for either toolkit.
        assert_eq!(get_env_scaling_configuration(&env(&[(USE_PHYSICAL_DPI_VARIABLE, "1")])), default);
        assert_eq!(get_env_scaling_configuration(&env(&[("QT_AUTO_SCREEN_SCALE_FACTOR", "1")])), default);
    }

    #[test]
    fn the_variables_of_qt_can_be_ignored() {
        let config = get_env_scaling_configuration(&env(&[
            ("QT_SCALE_FACTOR", "2"),
            (SCREEN_SCALE_IGNORE_QT_VARIABLE, "1"),
        ]));
        assert_eq!(config, EnvConfiguration { config: None, global: 1.0, force_auto: false });

        let config = get_env_scaling_configuration(&env(&[
            ("QT_SCALE_FACTOR", "2"),
            (SCREEN_SCALE_IGNORE_QT_VARIABLE, "0"),
        ]));
        assert_eq!(config.global, 2.0);
    }
}
