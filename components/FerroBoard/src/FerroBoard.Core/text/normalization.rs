// Canonical normalization bridge, Unicode 16.0.0 UAX #15 revision 56.
// Unicode data and test licenses: reference/unicode-16.0.0/LICENSE.txt.
use super::{Utf16Text, normalization_data::*};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizationError {
    InvalidUtf16,
    InvalidCodePoint,
}
impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidUtf16 => "normalization requires well-formed UTF-16",
            Self::InvalidCodePoint => "unsupported code point for normalization",
        })
    }
}
impl std::error::Error for NormalizationError {}
#[derive(Clone, Copy)]
pub(crate) enum CanonicalForm {
    D,
    C,
}
fn class(cp: u32) -> u8 {
    CLASSES
        .binary_search_by_key(&cp, |p| p.0)
        .map_or(0, |i| CLASSES[i].1)
}
fn decompose(cp: u32, output: &mut Vec<u32>) {
    if (0xac00..0xd7a4).contains(&cp) {
        let s = cp - 0xac00;
        output.push(0x1100 + s / 588);
        output.push(0x1161 + (s % 588) / 28);
        let t = s % 28;
        if t != 0 {
            output.push(0x11a7 + t);
        }
        return;
    }
    if let Ok(i) = DECOMPOSITIONS.binary_search_by_key(&cp, |p| p.0) {
        for &part in DECOMPOSITIONS[i].1 {
            decompose(part, output);
        }
    } else {
        output.push(cp);
    }
}
fn compose(a: u32, b: u32) -> Option<u32> {
    if (0x1100..0x1113).contains(&a) && (0x1161..0x1176).contains(&b) {
        return Some(0xac00 + ((a - 0x1100) * 21 + b - 0x1161) * 28);
    }
    if (0xac00..0xd7a4).contains(&a)
        && (a - 0xac00).is_multiple_of(28)
        && (0x11a8..0x11c3).contains(&b)
    {
        return Some(a + b - 0x11a7);
    }
    COMPOSITIONS
        .binary_search_by_key(&(a, b), |p| p.0)
        .ok()
        .map(|i| COMPOSITIONS[i].1)
}
pub(crate) fn normalize(
    text: &Utf16Text,
    form: CanonicalForm,
) -> Result<Utf16Text, NormalizationError> {
    let mut scalars = Vec::with_capacity(text.len());
    for c in char::decode_utf16(text.units().iter().copied()) {
        let cp = c.map_err(|_| NormalizationError::InvalidUtf16)? as u32;
        // .NET's normalization bridge rejects U+FFFE, independently of UCD validity.
        if cp == 0xfffe {
            return Err(NormalizationError::InvalidCodePoint);
        }
        decompose(cp, &mut scalars);
    }
    let mut start = 0;
    for i in 0..scalars.len() {
        if class(scalars[i]) == 0 {
            scalars[start..i].sort_by_key(|c| class(*c));
            start = i + 1;
        }
    }
    scalars[start..].sort_by_key(|c| class(*c));
    if matches!(form, CanonicalForm::C) {
        let mut result = Vec::with_capacity(scalars.len());
        let mut starter = None;
        let mut last_class = 0;
        for cp in scalars {
            let cc = class(cp);
            if let Some(index) = starter
                && (last_class < cc || last_class == 0)
                && let Some(composite) = compose(result[index], cp)
            {
                result[index] = composite;
                continue;
            }
            if cc == 0 {
                starter = Some(result.len());
            }
            result.push(cp);
            last_class = cc;
        }
        scalars = result;
    }
    let mut units = Vec::with_capacity(text.len());
    for cp in scalars {
        let mut buf = [0; 2];
        units.extend_from_slice(
            char::from_u32(cp)
                .expect("validated normalization data")
                .encode_utf16(&mut buf),
        );
    }
    if units == text.units() {
        Ok(text.clone())
    } else {
        Ok(Utf16Text::from_units(units))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode16_canonical_normalization_conformance() {
        for line in include_str!("../../../reference/unicode-16.0.0/NormalizationTest.txt").lines()
        {
            let s = line.split('#').next().unwrap().trim();
            if s.is_empty() || s.starts_with('@') {
                continue;
            }
            let values: Vec<Utf16Text> = s
                .split(';')
                .take(5)
                .map(|f| {
                    let mut units = Vec::new();
                    for p in f.split_whitespace() {
                        let mut b = [0; 2];
                        units.extend_from_slice(
                            char::from_u32(u32::from_str_radix(p, 16).unwrap())
                                .unwrap()
                                .encode_utf16(&mut b),
                        );
                    }
                    Utf16Text::from_units(units)
                })
                .collect();
            for (i, value) in values.iter().enumerate() {
                assert_eq!(
                    normalize(value, CanonicalForm::D).unwrap(),
                    values[if i < 3 { 2 } else { 4 }],
                    "NFD {line}"
                );
                assert_eq!(
                    normalize(value, CanonicalForm::C).unwrap(),
                    values[if i < 3 { 1 } else { 3 }],
                    "NFC {line}"
                );
            }
        }
    }
}
