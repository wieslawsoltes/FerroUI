use super::{references::Tokens, ValueResult};
use serde_json::{json, Value};

pub(super) fn finite(value: &Value) -> ValueResult<f64> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "Expected a finite number".into())
}

pub(super) fn literal_number(value: &str) -> ValueResult<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| "Expected a finite literal number".into())
}

pub(super) fn dimension(
    tokens: &Tokens<'_>,
    value: &Value,
    pixels_per_rem: Option<f64>,
    note: &mut Option<String>,
) -> std::result::Result<f64, String> {
    let length = finite(tokens.property(&value["value"])?)?;
    let pixels = match tokens.property(&value["unit"])?.as_str() {
        Some("px") => length,
        Some("rem") => {
            let scale =
                pixels_per_rem.ok_or("Supply pixelsPerRem to convert relative dimensions")?;
            append_note(
                note,
                &format!("{length} rem at {scale} logical pixels per rem"),
            );
            length * scale
        }
        _ => return Err("Expected dimension unit px or rem".into()),
    };
    if !pixels.is_finite() {
        return Err("Dimension is outside the finite XAML range".into());
    }
    Ok(pixels)
}

pub(super) fn color(
    tokens: &Tokens<'_>,
    value: &Value,
    note: &mut Option<String>,
) -> std::result::Result<String, String> {
    if tokens.property(&value["colorSpace"])?.as_str() != Some("srgb") {
        return Err("Only sRGB colors can be emitted as XAML colors".into());
    }
    let components = tokens.property(&value["components"])?;
    let rgb = components
        .as_array()
        .filter(|values| values.len() == 3)
        .ok_or("Expected three sRGB components")?;
    let mut channels = [255; 4];
    for (byte, component) in channels[1..].iter_mut().zip(rgb) {
        *byte = color_channel(tokens.property(component)?, note)?;
    }
    if let Some(alpha) = value.get("alpha") {
        channels[0] = color_channel(tokens.property(alpha)?, note)?;
    }
    if channels[0] == 255 {
        Ok(format!(
            "#{:02X}{:02X}{:02X}",
            channels[1], channels[2], channels[3]
        ))
    } else {
        Ok(format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            channels[0], channels[1], channels[2], channels[3]
        ))
    }
}

fn color_channel(value: &Value, note: &mut Option<String>) -> std::result::Result<u8, String> {
    let channel = finite(value)?;
    if !(0.0..=1.0).contains(&channel) {
        return Err("Color channels must be between zero and one".into());
    }
    let scaled = channel * 255.0;
    if (scaled - scaled.round()).abs() > 1e-8
        && !note
            .as_deref()
            .is_some_and(|text| text.contains("8-bit channels"))
    {
        append_note(note, "Rounded to 8-bit channels");
    }
    Ok(scaled.round() as u8)
}

pub(super) fn duration(tokens: &Tokens<'_>, value: &Value) -> std::result::Result<String, String> {
    let scalar = finite(tokens.property(&value["value"])?)?;
    let ticks = match tokens.property(&value["unit"])?.as_str() {
        Some("ms") => scalar * 10_000.0,
        Some("s") => scalar * 10_000_000.0,
        _ => return Err("Expected duration unit ms or s".into()),
    }
    .round();
    if ticks < 0.0 || ticks >= i64::MAX as f64 {
        return Err("Duration is outside the positive TimeSpan range".into());
    }
    let ticks = ticks as i64;
    let seconds = ticks / 10_000_000;
    Ok(format!(
        "{}.{:02}:{:02}:{:02}.{:07}",
        seconds / 86_400,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
        ticks % 10_000_000
    ))
}

pub(super) fn append_note(note: &mut Option<String>, detail: &str) {
    match note {
        Some(note) if !note.split(" · ").any(|existing| existing == detail) => {
            note.push_str(" · ");
            note.push_str(detail);
        }
        Some(_) => {}
        None => *note = Some(detail.into()),
    }
}

pub(super) fn hex_color(value: &str) -> ValueResult<Value> {
    let digits = value
        .trim()
        .strip_prefix('#')
        .ok_or("Use a literal hexadecimal sRGB color")?;
    if !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
        return Err("Invalid hexadecimal sRGB color".into());
    }
    let hex = u32::from_str_radix(digits, 16).map_err(|_| "Invalid hexadecimal sRGB color")?;
    let (alpha, red, green, blue) = match digits.len() {
        3 => (
            255,
            (hex >> 8 & 15) * 17,
            (hex >> 4 & 15) * 17,
            (hex & 15) * 17,
        ),
        4 => (
            (hex >> 12 & 15) * 17,
            (hex >> 8 & 15) * 17,
            (hex >> 4 & 15) * 17,
            (hex & 15) * 17,
        ),
        6 => (255, hex >> 16 & 255, hex >> 8 & 255, hex & 255),
        8 => (hex >> 24 & 255, hex >> 16 & 255, hex >> 8 & 255, hex & 255),
        _ => return Err("Invalid hexadecimal sRGB color".into()),
    };
    let normalize = |value: u32| f64::from(value) / 255.0;
    Ok(
        json!({"colorSpace":"srgb","components":[normalize(red),normalize(green),normalize(blue)],
        "alpha":normalize(alpha),"hex":format!("#{red:02X}{green:02X}{blue:02X}")}),
    )
}

pub(super) fn duration_ms(value: &str) -> ValueResult<f64> {
    let components = value.split(':').collect::<Vec<_>>();
    let [first, minutes, seconds] = components.as_slice() else {
        return Err("Expected a literal TimeSpan with hours, minutes and seconds".into());
    };
    let (days, hours) = first.split_once('.').unwrap_or(("0", first));
    let days = days.parse::<u32>().map_err(|_| "Invalid TimeSpan days")?;
    let hours = hours.parse::<u32>().map_err(|_| "Invalid TimeSpan hours")?;
    let minutes = minutes
        .parse::<u32>()
        .map_err(|_| "Invalid TimeSpan minutes")?;
    let seconds = seconds
        .parse::<f64>()
        .map_err(|_| "Invalid TimeSpan seconds")?;
    let total = (f64::from(days) * 86_400.0
        + f64::from(hours) * 3600.0
        + f64::from(minutes) * 60.0
        + seconds)
        * 1000.0;
    if hours >= 24
        || minutes >= 60
        || !(0.0..60.0).contains(&seconds)
        || total >= (i64::MAX as f64) / 10_000.0
    {
        return Err("TimeSpan is outside the supported duration range".into());
    }
    Ok(total)
}
