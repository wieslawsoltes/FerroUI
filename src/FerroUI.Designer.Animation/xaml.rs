use crate::{
    require, AnimationError, AnimationValue, Easing, FillMode, Keyframe, PlaybackBehavior,
    PlaybackDirection, Repeat, Result, Timeline, Track, ValueKind,
};
use std::collections::BTreeMap;

/// Runtime properties are typed. Hosts should supply hints for custom properties and the live
/// underlying values of tracks whose endpoints are omitted. Without an underlying value,
/// strict import rejects an implicit endpoint rather than silently changing animation behavior.
#[derive(Clone, Debug)]
pub struct ImportOptions {
    pub property_kinds: BTreeMap<String, ValueKind>,
    pub base_values: BTreeMap<String, AnimationValue>,
    /// Host-resolved literals, keyed by (property name, decoded literal text), e.g. named colors.
    pub literal_values: BTreeMap<(String, String), AnimationValue>,
    /// Native value types that need a host animator instead of the portable literal codec.
    /// Extension codecs may replace this entry when they implement that interpolation.
    pub interpolation_errors: BTreeMap<String, String>,
    pub strict: bool,
}
impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            property_kinds: BTreeMap::new(),
            base_values: BTreeMap::new(),
            literal_values: BTreeMap::new(),
            interpolation_errors: BTreeMap::new(),
            strict: true,
        }
    }
}
#[derive(Clone, Debug)]
pub struct AnimationImport {
    pub timeline: Timeline,
    pub warnings: Vec<String>,
}

impl Timeline {
    /// Export one target as native FerroUI Animation markup. Authored tracks without an
    /// underlying value receive explicit hold endpoints. Imported tracks with a captured
    /// underlying value keep native implicit endpoints: the host must retain that base property
    /// on the target. Materializing those endpoints would alter reverse initial-delay fill.
    /// XML is encoded once, preserving markup-extension text as an attribute value.
    pub fn to_xaml(&self, target: u64) -> Result<String> {
        self.validate()?;
        require(
            native_ticks(self.duration)? > 0,
            "Native animation duration must be at least one 100 ns tick",
        )?;
        native_ticks(self.delay)?;
        native_ticks(self.delay_between_iterations)?;
        let tracks = self
            .tracks
            .iter()
            .filter(|t| t.target == target && !t.muted && !t.keyframes.is_empty())
            .collect::<Vec<_>>();
        require(
            !tracks.is_empty()
                || self
                    .tracks
                    .iter()
                    .all(|track| track.muted || track.keyframes.is_empty()),
            "No enabled tracks for this target",
        )?;
        let repeat = match self.repeat {
            Repeat::Count(n) => n.to_string(),
            Repeat::Infinite => "Infinite".into(),
        };
        let easing_element = self.easing.property_element();
        let easing_attribute = if easing_element.is_none() {
            format!(" Easing=\"{}\"", escape(&self.easing.to_xaml()))
        } else {
            String::new()
        };
        let mut result = format!(
            "<Animation Duration=\"{}\" Delay=\"{}\" DelayBetweenIterations=\"{}\" PlaybackBehavior=\"{:?}\" SpeedRatio=\"{}\" IterationCount=\"{}\" PlaybackDirection=\"{:?}\" FillMode=\"{:?}\"{}>\n",
            time_string(self.duration),
            time_string(self.delay),
            time_string(self.delay_between_iterations),
            self.playback_behavior,
            self.speed_ratio,
            repeat,
            self.playback_direction,
            self.fill,
            easing_attribute
        );
        if let Some(element) = easing_element {
            result.push_str(&element);
            result.push('\n');
        }
        let mut keys: Vec<(f64, &str, AnimationValue, Easing)> = vec![];
        for track in tracks {
            let first = &track.keyframes[0];
            let last = track.keyframes.last().expect("nonempty track");
            if first.time > 0.0 && track.base_value.is_none() {
                keys.push((
                    0.0,
                    &track.property,
                    if matches!(self.fill, FillMode::Backward | FillMode::Both) {
                        first.value.clone()
                    } else {
                        track.base_value.as_ref().unwrap_or(&first.value).clone()
                    },
                    Easing::Linear,
                ));
            }
            for key in &track.keyframes {
                keys.push((
                    key.time,
                    &track.property,
                    key.value.clone(),
                    key.easing.clone(),
                ));
            }
            if last.time < self.duration && track.base_value.is_none() {
                keys.push((
                    self.duration,
                    &track.property,
                    if matches!(self.fill, FillMode::Forward | FillMode::Both) {
                        last.value.clone()
                    } else {
                        track.base_value.as_ref().unwrap_or(&last.value).clone()
                    },
                    last.easing.clone(),
                ));
            }
        }
        keys.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        let mut native_times = std::collections::BTreeSet::new();
        for (time, property, value, easing) in keys {
            require(
                native_times.insert((property, native_ticks(time)?)),
                "Keyframe times collide at native 100 ns precision",
            )?;
            let spline = if matches!(easing, Easing::Linear) {
                String::new()
            } else {
                format!(" KeySpline=\"{}\"", escape(&easing.to_xaml()))
            };
            result.push_str(&format!("  <KeyFrame KeyTime=\"{}\"{spline}><Setter Property=\"{}\" Value=\"{}\" /></KeyFrame>\n",time_string(time),escape(property),escape(&literal_value(&value))));
        }
        result.push_str("</Animation>");
        Ok(result)
    }
    /// Place this inside the owning control's .Styles property with an explicit named selector.
    pub fn to_style_xaml(&self, target: u64, selector: &str) -> Result<String> {
        require(
            !selector.is_empty()
                && selector.chars().all(|ch| {
                    matches!(ch, '\t' | '\n' | '\r')
                        || (ch >= ' ' && !matches!(ch, '\u{FFFE}' | '\u{FFFF}'))
                }),
            "An animation style requires an XML-valid selector",
        )?;
        Ok(format!(
            "<Style Selector=\"{}\"><Style.Animations>\n{}\n</Style.Animations></Style>",
            escape(selector),
            self.to_xaml(target)?
        ))
    }
    pub fn from_xaml(source: &str, target: u64) -> Result<Self> {
        Ok(Self::import_xaml(source, target, &ImportOptions::default())?.timeline)
    }
    pub fn import_xaml(
        source: &str,
        target: u64,
        options: &ImportOptions,
    ) -> Result<AnimationImport> {
        require(
            source.len() <= 16 * 1024 * 1024,
            "Animation markup exceeds the import limit",
        )?;
        let document =
            roxmltree::Document::parse(source).map_err(|e| AnimationError(e.to_string()))?;
        let root = document.root_element();
        require(
            root.tag_name().name() == "Animation" && native_namespace(root.tag_name().namespace()),
            "Expected an Animation root element",
        )?;
        for attr in root.attributes() {
            require(
                attr.namespace().is_none()
                    && matches!(
                        attr.name(),
                        "Duration"
                            | "Delay"
                            | "DelayBetweenIterations"
                            | "PlaybackBehavior"
                            | "SpeedRatio"
                            | "IterationCount"
                            | "PlaybackDirection"
                            | "FillMode"
                            | "Easing"
                    ),
                format!(
                    "Unsupported Animation attribute {}; retain it in the source editor",
                    attr.name()
                ),
            )?;
        }
        let mut timeline = Self::new(parse_time(
            root.attribute("Duration")
                .ok_or_else(|| AnimationError("Animation.Duration is required".into()))?,
        )?)?;
        if let Some(value) = root.attribute("Delay") {
            timeline.delay = parse_time(value)?;
        }
        if let Some(value) = root.attribute("DelayBetweenIterations") {
            timeline.delay_between_iterations = parse_time(value)?;
        }
        timeline.playback_behavior = match root.attribute("PlaybackBehavior").unwrap_or("Auto") {
            "Auto" => PlaybackBehavior::Auto,
            "Always" => PlaybackBehavior::Always,
            "OnlyIfVisible" => PlaybackBehavior::OnlyIfVisible,
            _ => return Err(AnimationError("Invalid PlaybackBehavior".into())),
        };
        if let Some(value) = root.attribute("SpeedRatio") {
            timeline.speed_ratio = parse_number(value)?;
        }
        timeline.repeat = match root.attribute("IterationCount").unwrap_or("1") {
            value if value.eq_ignore_ascii_case("Infinite") => Repeat::Infinite,
            value => Repeat::Count(
                value
                    .parse()
                    .map_err(|_| AnimationError("Invalid IterationCount".into()))?,
            ),
        };
        timeline.playback_direction = match root.attribute("PlaybackDirection").unwrap_or("Normal")
        {
            "Normal" => PlaybackDirection::Normal,
            "Reverse" => PlaybackDirection::Reverse,
            "Alternate" => PlaybackDirection::Alternate,
            "AlternateReverse" => PlaybackDirection::AlternateReverse,
            _ => return Err(AnimationError("Invalid PlaybackDirection".into())),
        };
        timeline.fill = match root.attribute("FillMode").unwrap_or("None") {
            "None" => FillMode::None,
            "Forward" => FillMode::Forward,
            "Backward" => FillMode::Backward,
            "Both" => FillMode::Both,
            _ => return Err(AnimationError("Invalid FillMode".into())),
        };
        timeline.easing = import_easing(root)?;
        let mut tracks: BTreeMap<String, Track> = BTreeMap::new();
        for node in root.children().filter(|n| n.is_element()) {
            if node.tag_name().name() == "Animation.Easing"
                && native_namespace(node.tag_name().namespace())
            {
                continue;
            }
            require(
                node.tag_name().name() == "KeyFrame" && native_namespace(node.tag_name().namespace()),
                "Only literal KeyFrame children are supported; custom animation objects remain in source",
            )?;
            for attr in node.attributes() {
                require(
                    attr.namespace().is_none()
                        && matches!(attr.name(), "Cue" | "KeyTime" | "KeySpline"),
                    format!("Unsupported KeyFrame attribute {}", attr.name()),
                )?;
            }
            require(
                !(node.attribute("Cue").is_some() && node.attribute("KeyTime").is_some()),
                "KeyFrame cannot specify both Cue and KeyTime",
            )?;
            let time = if let Some(cue) = node.attribute("Cue") {
                let cue = cue.trim();
                let fraction = if let Some(percent) = cue.strip_suffix('%') {
                    parse_number(percent)? / 100.0
                } else {
                    parse_number(cue)?
                };
                fraction * timeline.duration
            } else if let Some(time) = node.attribute("KeyTime") {
                parse_time(time)?
            } else {
                return Err(AnimationError("KeyFrame requires Cue or KeyTime".into()));
            };
            let easing = if let Some(spline) = node.attribute("KeySpline") {
                Easing::parse(spline)?
            } else {
                Easing::Linear
            };
            require(
                !matches!(easing, Easing::Named(_)),
                "KeySpline must specify four Bezier controls",
            )?;
            for setter in node.children().filter(|n| n.is_element()) {
                require(
                    setter.tag_name().name() == "Setter" && native_namespace(setter.tag_name().namespace())
                        && setter.children().all(|n| !n.is_element()),
                    "Only literal Setter attributes can be imported; custom or binding objects remain in source",
                )?;
                for attr in setter.attributes() {
                    require(
                        attr.namespace().is_none() && matches!(attr.name(), "Property" | "Value"),
                        format!("Unsupported Setter attribute {}", attr.name()),
                    )?;
                }
                let property = setter
                    .attribute("Property")
                    .ok_or_else(|| AnimationError("Setter.Property is required".into()))?;
                if let Some(reason) = options.interpolation_errors.get(property) {
                    return Err(AnimationError(reason.clone()));
                }
                let source = setter
                    .attribute("Value")
                    .ok_or_else(|| AnimationError("Setter.Value is required".into()))?;
                let source = if let Some(literal) = source.strip_prefix("{}") {
                    literal
                } else {
                    require(
                        !source.starts_with('{'),
                        "Resolve animation binding/resource values through the host before importing into a typed timeline",
                    )?;
                    source
                };
                let kind = options
                    .property_kinds
                    .get(property)
                    .copied()
                    .or_else(|| {
                        options
                            .literal_values
                            .get(&(property.to_owned(), source.to_owned()))
                            .map(AnimationValue::kind)
                    })
                    .or_else(|| options.base_values.get(property).map(AnimationValue::kind))
                    .unwrap_or_else(|| infer_kind(property, source));
                let value = if let Some(value) = options
                    .literal_values
                    .get(&(property.to_owned(), source.to_owned()))
                {
                    value.validate()?;
                    require(
                        value.kind() == kind,
                        "Resolved literal differs from the property type",
                    )?;
                    value.clone()
                } else {
                    AnimationValue::parse(source, kind)?
                };
                let track = tracks
                    .entry(property.into())
                    .or_insert_with(|| Track::new(target, property));
                require(
                    !track.keyframes.iter().any(|k| k.time == time),
                    "Duplicate property keyframe time",
                )?;
                track.keyframes.push(Keyframe {
                    time,
                    value,
                    easing: easing.clone(),
                });
            }
        }
        let mut warnings = vec![];
        for (_, mut track) in tracks {
            track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
            track.base_value = options.base_values.get(&track.property).cloned();
            let requires_base = track.keyframes.first().is_some_and(|key| key.time > 0.0)
                && !matches!(timeline.fill, FillMode::Backward | FillMode::Both)
                || track
                    .keyframes
                    .last()
                    .is_some_and(|key| key.time < timeline.duration)
                    && !matches!(timeline.fill, FillMode::Forward | FillMode::Both);
            if requires_base && track.base_value.is_none() {
                let message = format!(
                    "{} has implicit endpoint values; provide the underlying live value",
                    track.property
                );
                if options.strict {
                    return Err(AnimationError(message));
                }
                warnings.push(message);
            }
            timeline.tracks.push(track);
        }
        timeline.validate()?;
        Ok(AnimationImport { timeline, warnings })
    }
}
fn parse_number(source: &str) -> Result<f64> {
    let value = source
        .trim()
        .parse::<f64>()
        .map_err(|_| AnimationError(format!("Invalid number: {source}")))?;
    require(value.is_finite(), "Number must be finite")?;
    Ok(value)
}
// Keep the native invariant TimeSpan grammar. A bare integer means DAYS,
// not seconds; fractional seconds require the hh:mm:ss.fffffff form.
pub(crate) fn parse_time(source: &str) -> Result<f64> {
    let invalid = || AnimationError(format!("Invalid native TimeSpan: {source}"));
    let number = |part: &str| -> Result<u64> {
        require(
            !part.is_empty() && part.len() <= 9 && part.bytes().all(|byte| byte.is_ascii_digit()),
            format!("Invalid native TimeSpan component: {part}"),
        )?;
        part.parse().map_err(|_| invalid())
    };
    let split_days = |part: &str| -> Result<(u64, u64)> {
        if let Some((days, hours)) = part.split_once('.') {
            Ok((number(days)?, number(hours)?))
        } else {
            Ok((0, number(part)?))
        }
    };
    let parts: Vec<_> = source.trim().split(':').collect();
    let (days, hours, minutes, seconds) = match parts.as_slice() {
        [days] => (number(days)?, 0, 0, None),
        [hours, minutes] => {
            let (days, hours) = split_days(hours)?;
            (days, hours, number(minutes)?, None)
        }
        [hours, minutes, seconds] => {
            let (days, hours) = split_days(hours)?;
            (days, hours, number(minutes)?, Some(*seconds))
        }
        [days, hours, minutes, seconds] => (
            number(days)?,
            number(hours)?,
            number(minutes)?,
            Some(*seconds),
        ),
        _ => return Err(invalid()),
    };
    let (seconds, fraction) = match seconds {
        None => (0, 0),
        Some(part) => {
            if let Some((whole, fraction)) = part.split_once('.') {
                require(
                    !fraction.is_empty()
                        && fraction.len() <= 7
                        && fraction.bytes().all(|byte| byte.is_ascii_digit()),
                    "Native TimeSpan supports one to seven fractional second digits",
                )?;
                let ticks = number(fraction)? * 10_u64.pow((7 - fraction.len()) as u32);
                (if whole.is_empty() { 0 } else { number(whole)? }, ticks)
            } else {
                (number(part)?, 0)
            }
        }
    };
    require(
        hours < 24 && minutes < 60 && seconds < 60,
        "Invalid native TimeSpan components",
    )?;
    let ticks = days
        .checked_mul(864_000_000_000)
        .and_then(|ticks| ticks.checked_add(hours * 36_000_000_000))
        .and_then(|ticks| ticks.checked_add(minutes * 600_000_000))
        .and_then(|ticks| ticks.checked_add(seconds * 10_000_000))
        .and_then(|ticks| ticks.checked_add(fraction))
        .ok_or_else(invalid)?;
    require(
        ticks <= i64::MAX as u64,
        "Native TimeSpan is outside Int64 tick range",
    )?;
    let result = ticks as f64 / 10_000_000.0;
    native_ticks(result)?;
    Ok(result)
}
pub(crate) fn native_ticks(time: f64) -> Result<u64> {
    let ticks = time * 10_000_000.0;
    require(
        time.is_finite() && time >= 0.0 && ticks < i64::MAX as f64,
        "Duration is outside native TimeSpan range",
    )?;
    Ok(ticks.round_ties_even() as u64)
}
pub(crate) fn time_string(time: f64) -> String {
    let ticks = native_ticks(time).expect("validated native duration");
    let days = ticks / 864_000_000_000;
    let hours = ticks / 36_000_000_000 % 24;
    let minutes = ticks / 600_000_000 % 60;
    let seconds = ticks / 10_000_000 % 60;
    let fraction = ticks % 10_000_000;
    let day_prefix = if days > 0 {
        format!("{days}.")
    } else {
        String::new()
    };
    if fraction == 0 {
        format!("{day_prefix}{hours}:{minutes}:{seconds}")
    } else {
        format!("{day_prefix}{hours}:{minutes}:{seconds}.{fraction:07}")
    }
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}
pub(crate) fn infer_kind(property: &str, source: &str) -> ValueKind {
    let property = property.rsplit('.').next().unwrap_or(property);
    if matches!(property, "Margin" | "Padding" | "BorderThickness") {
        ValueKind::Thickness
    } else if property == "CornerRadius" {
        ValueKind::CornerRadius
    } else if matches!(property, "RenderTransformOrigin") {
        ValueKind::RelativePoint
    } else if matches!(property, "Size" | "PageSize") {
        ValueKind::Size
    } else if matches!(property, "Rect" | "Bounds" | "Viewport") {
        ValueKind::Rect
    } else if matches!(property, "Center" | "StartPoint" | "EndPoint") {
        if source.contains('%') {
            ValueKind::RelativePoint
        } else {
            ValueKind::Point
        }
    } else if source.starts_with('#') {
        ValueKind::Color
    } else if source.trim().parse::<f64>().is_ok() {
        ValueKind::Number
    } else {
        ValueKind::Discrete
    }
}

pub(crate) fn import_easing(root: roxmltree::Node<'_, '_>) -> Result<Easing> {
    let properties = root
        .children()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "Animation.Easing"
                && native_namespace(node.tag_name().namespace())
        })
        .collect::<Vec<_>>();
    require(
        properties.len() <= 1 && (root.attribute("Easing").is_none() || properties.is_empty()),
        "Animation easing must be declared once",
    )?;
    let Some(property) = properties.first() else {
        return Easing::parse(root.attribute("Easing").unwrap_or("LinearEasing"));
    };
    require(
        property.attributes().len() == 0,
        "Unknown easing property attributes remain in source",
    )?;
    let children = property
        .children()
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    require(
        children.len() == 1,
        "Animation.Easing requires one easing object",
    )?;
    let node = children[0];
    require(
        native_namespace(node.tag_name().namespace())
            || node.tag_name().namespace().is_some_and(|namespace| {
                matches!(
                    namespace,
                    "using:FerroUI.Animation.Easings"
                        | "clr-namespace:FerroUI.Animation.Easings;assembly=FerroUI.Base"
                )
            }),
        "Extension easing objects require an animation codec",
    )?;
    require(
        !node.children().any(|child| {
            child.is_element() || child.is_text() && !child.text().unwrap_or("").trim().is_empty()
        }),
        "Unknown easing payload remains in source",
    )?;
    let numeric = |name: &str, default: f64| {
        node.attribute(name)
            .map(parse_number)
            .transpose()
            .map(|value| value.unwrap_or(default))
    };
    let (result, allowed): (Easing, &[&str]) = match node.tag_name().name() {
        "SpringEasing" => (
            Easing::Spring {
                mass: numeric("Mass", 0.0)?,
                stiffness: numeric("Stiffness", 0.0)?,
                damping: numeric("Damping", 0.0)?,
                initial_velocity: numeric("InitialVelocity", 0.0)?,
            },
            &["Mass", "Stiffness", "Damping", "InitialVelocity"],
        ),
        "SplineEasing" => (
            Easing::CubicBezier {
                x1: numeric("X1", 0.0)?,
                y1: numeric("Y1", 0.0)?,
                x2: numeric("X2", 1.0)?,
                y2: numeric("Y2", 1.0)?,
            },
            &["X1", "Y1", "X2", "Y2"],
        ),
        name => (Easing::parse(name)?, &[]),
    };
    require(
        node.attributes().all(|attribute| {
            attribute.namespace().is_none() && allowed.contains(&attribute.name())
        }),
        "Unknown easing object attributes remain in source",
    )?;
    result.validate()?;
    Ok(result)
}

pub(crate) fn easing_from_source(source: &str) -> Result<Easing> {
    let document =
        roxmltree::Document::parse(source).map_err(|error| AnimationError(error.to_string()))?;
    import_easing(document.root_element())
}

fn literal_value(value: &AnimationValue) -> String {
    let source = value.to_xaml();
    if matches!(value, AnimationValue::Discrete(_)) && source.starts_with('{') {
        format!("{{}}{source}")
    } else {
        source
    }
}

pub(crate) fn native_namespace(namespace: Option<&str>) -> bool {
    namespace.is_none_or(|namespace| {
        matches!(
            namespace,
            "https://github.com/ferroui"
                | "using:FerroUI.Animation"
                | "using:FerroUI.Styling"
                | "clr-namespace:FerroUI.Animation;assembly=FerroUI.Base"
                | "clr-namespace:FerroUI.Styling;assembly=FerroUI.Base"
        )
    })
}
