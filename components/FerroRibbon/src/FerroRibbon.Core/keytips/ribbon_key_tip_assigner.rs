//! Port of RibbonKeyTipAssigner.cs; source private helper projection is pinned separately.
use super::keytip_data;
use crate::collections::RibbonFixedItems;
use crate::collections::RibbonItems;
use crate::{
    RibbonString,
    collections::{RibbonArray, RibbonList},
    model::RibbonItemsSource,
    text::{empty, letter_or_digit, upper_char, whitespace},
};
use std::{collections::HashSet, rc::Rc};
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct RibbonKeyTipRequest {
    pub label: Option<RibbonString>,
    pub explicit: Option<RibbonString>,
}
impl RibbonKeyTipRequest {
    pub fn new(label: Option<RibbonString>, explicit: Option<RibbonString>) -> Self {
        Self { label, explicit }
    }
    pub fn to_ribbon_string(&self) -> RibbonString {
        let mut units = "RibbonKeyTipRequest { Label = "
            .encode_utf16()
            .collect::<Vec<_>>();
        if let Some(label) = &self.label {
            units.extend_from_slice(label.as_utf16());
        }
        units.extend(", Explicit = ".encode_utf16());
        if let Some(tip) = &self.explicit {
            units.extend_from_slice(tip.as_utf16());
        }
        units.extend(" }".encode_utf16());
        RibbonString::from_utf16(units)
    }
    pub fn deconstruct(&self) -> (Option<RibbonString>, Option<RibbonString>) {
        (self.label.clone(), self.explicit.clone())
    }
}
/// Source IReadOnlyList adapter. Count/indexer calls stay live and ordered; custom
/// implementations can reproduce source mutation/exception behavior during Assign.
pub trait RibbonKeyTipRequests {
    fn count(&self) -> i32;
    fn get(&self, index: i32) -> RibbonKeyTipRequest;
}
impl<const N: usize> RibbonKeyTipRequests for [RibbonKeyTipRequest; N] {
    fn count(&self) -> i32 {
        i32::try_from(N).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> RibbonKeyTipRequest {
        self[index as usize].clone()
    }
}
impl RibbonKeyTipRequests for RibbonList<RibbonKeyTipRequest> {
    fn count(&self) -> i32 {
        i32::try_from(self.len()).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> RibbonKeyTipRequest {
        RibbonItems::get(self.as_ref(), index as usize)
    }
}
impl RibbonKeyTipRequests for Vec<RibbonKeyTipRequest> {
    fn count(&self) -> i32 {
        i32::try_from(self.len()).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> RibbonKeyTipRequest {
        self[index as usize].clone()
    }
}
impl RibbonKeyTipRequests for RibbonArray<RibbonKeyTipRequest> {
    fn count(&self) -> i32 {
        i32::try_from(self.len()).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> RibbonKeyTipRequest {
        self.as_ref().get(index as usize)
    }
}
pub type RibbonKeyTipAssignments = RibbonArray<Option<RibbonString>>;
pub type RibbonQuickAccessKeyTips = RibbonList<Option<RibbonString>>;
pub struct RibbonKeyTipAssigner;
const SINGLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ123456789";
const PAIR: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
#[derive(Default)]
struct Used {
    order: Vec<RibbonString>,
    set: HashSet<RibbonString>,
}
impl Used {
    fn add(&mut self, tip: RibbonString) {
        if self.set.insert(tip.clone()) {
            self.order.push(tip);
        }
    }
    fn available(&self, tip: &RibbonString) -> bool {
        self.order.iter().all(|e| {
            !e.as_utf16().starts_with(tip.as_utf16()) && !tip.as_utf16().starts_with(e.as_utf16())
        })
    }
}
impl RibbonKeyTipAssigner {
    pub fn assign(
        requests: Option<&dyn RibbonKeyTipRequests>,
        reserved: Option<RibbonItemsSource<Option<RibbonString>>>,
    ) -> RibbonKeyTipAssignments {
        let requests = requests.expect("ArgumentNull: requests");
        let mut used = Used::default();
        if let Some(reserved) = reserved {
            for tip in reserved {
                used.add(Self::normalize(tip.as_ref()));
            }
        }
        let count = usize::try_from(requests.count()).expect("OverflowException: array length");
        let mut result = vec![None; count];
        let mut i = 0;
        while i < requests.count() {
            let tip = Self::normalize(requests.get(i).explicit.as_ref());
            if !tip.is_empty() && used.available(&tip) {
                used.add(tip.clone());
                *result
                    .get_mut(i as usize)
                    .expect("IndexOutOfRangeException: result") = Some(tip);
            }
            i += 1;
        }
        let pending = result.iter().filter(|v| v.is_none()).count() as i32;
        let free = SINGLE.iter().filter(|&&c| used.available(&one(c))).count() as i32;
        let prefixes = free + i32::from(used.available(&one(b'0')));
        let mut budget = if pending <= free {
            pending
        } else {
            ((36 * prefixes - pending) / 35).clamp(0, free)
        };
        let mut i = 0;
        while i < requests.count() && budget > 0 {
            if result
                .get(i as usize)
                .expect("IndexOutOfRangeException: result")
                .is_none()
            {
                for candidate in single_candidates(requests.get(i).label.as_ref()) {
                    if used.available(&candidate) {
                        used.add(candidate.clone());
                        result[i as usize] = Some(candidate);
                        budget -= 1;
                        break;
                    }
                }
            }
            i += 1;
        }
        let mut i = 0;
        while i < requests.count() && budget > 0 {
            if result
                .get(i as usize)
                .expect("IndexOutOfRangeException: result")
                .is_none()
                && let Some(digit) = (b'1'..=b'9').map(one).find(|v| used.available(v))
            {
                used.add(digit.clone());
                result[i as usize] = Some(digit);
                budget -= 1;
            }
            i += 1;
        }
        let mut i = 0;
        while i < requests.count() {
            if result
                .get(i as usize)
                .expect("IndexOutOfRangeException: result")
                .is_none()
            {
                let tip = double_candidates(requests.get(i).label.as_ref())
                    .find(|v| used.available(v))
                    .unwrap_or_else(|| fallback(&used));
                used.add(tip.clone());
                result[i as usize] = Some(tip);
            }
            i += 1;
        }
        RibbonFixedItems::new(result)
    }
    pub fn normalize(key_tip: Option<&RibbonString>) -> RibbonString {
        let Some(value) = key_tip else { return empty() };
        let units = value
            .as_utf16()
            .iter()
            .copied()
            .filter(|&c| letter_or_digit(c))
            .map(upper_char)
            .collect::<Vec<_>>();
        if units.is_empty() {
            empty()
        } else {
            RibbonString::from_utf16(units)
        }
    }
    pub fn assign_quick_access(count: i32) -> RibbonQuickAccessKeyTips {
        let count = count.max(0) as usize;
        let mut tips = Vec::with_capacity(count);
        for i in 1..=9.min(count) {
            tips.push(Some(one(b'0' + i as u8)));
        }
        let mut prefix = vec![b'0' as u16];
        while tips.len() < count {
            for i in (1..=9).rev() {
                if tips.len() >= count {
                    break;
                }
                let mut tip = prefix.clone();
                tip.push(b'0' as u16 + i);
                tips.push(Some(RibbonString::from_utf16(tip)));
            }
            for c in b'A'..=b'Z' {
                if tips.len() >= count {
                    break;
                }
                let mut tip = prefix.clone();
                tip.push(c as u16);
                tips.push(Some(RibbonString::from_utf16(tip)));
            }
            prefix.push(b'0' as u16);
        }
        RibbonItems::from_values(tips)
    }
}
fn one(unit: u8) -> RibbonString {
    RibbonString::from_utf16(vec![unit as u16])
}
pub(super) fn letters(label: Option<&RibbonString>) -> Vec<u16> {
    let Some(label) = label else { return vec![] };
    if label.as_utf16().iter().all(|&c| whitespace(c)) {
        return vec![];
    }
    let mut result = vec![];
    for scalar in char::decode_utf16(label.as_utf16().iter().copied()) {
        let scalar = scalar.expect("ArgumentException: invalid UTF16 label") as u32;
        assert!(
            keytip_data::INVALID.binary_search(&scalar).is_err(),
            "ArgumentException: invalid normalization character"
        );
        if scalar < 128 {
            let c = scalar as u8;
            if c.is_ascii_alphanumeric() {
                result.push(c.to_ascii_uppercase() as u16);
            } else if whitespace(c as u16) || matches!(c, b'&' | b'-' | b'/' | b'(' | b')') {
                result.push(32);
            }
        } else if let Ok(i) = keytip_data::PROJECTION.binary_search_by_key(&scalar, |&(c, _)| c) {
            result.extend_from_slice(keytip_data::PROJECTION[i].1);
        }
    }
    result
}
fn single_candidates(label: Option<&RibbonString>) -> impl Iterator<Item = RibbonString> + use<> {
    let letters: Rc<[u16]> = letters(label).into();
    let chars = letters.clone();
    (0..letters.len())
        .filter_map(move |i| {
            if letters[i] != 32 && (i == 0 || letters[i - 1] == 32) {
                Some(RibbonString::from_utf16(vec![letters[i]]))
            } else {
                None
            }
        })
        .chain((0..chars.len()).filter_map(move |i| {
            if chars[i] != 32 {
                Some(RibbonString::from_utf16(vec![chars[i]]))
            } else {
                None
            }
        }))
}
fn double_candidates(label: Option<&RibbonString>) -> impl Iterator<Item = RibbonString> + use<> {
    let letters = letters(label);
    let words = letters
        .split(|&c| c == 32)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    let initials =
        (words.len() >= 2).then(|| RibbonString::from_utf16(vec![words[0][0], words[1][0]]));
    let compact: Rc<[u16]> = letters
        .into_iter()
        .filter(|&c| c != 32)
        .collect::<Vec<_>>()
        .into();
    let digits = compact.clone();
    let len = compact.len();
    initials
        .into_iter()
        .chain((0..len).flat_map(move |i| {
            let chars = compact.clone();
            ((i + 1)..len).map(move |j| RibbonString::from_utf16(vec![chars[i], chars[j]]))
        }))
        .chain((0..len).flat_map(move |i| {
            let chars = digits.clone();
            (b'1'..=b'9').map(move |d| RibbonString::from_utf16(vec![chars[i], d as u16]))
        }))
}
fn fallback(used: &Used) -> RibbonString {
    let mut level = std::iter::once(b'0')
        .chain(SINGLE.iter().copied())
        .map(one)
        .collect::<Vec<_>>();
    loop {
        let mut next = vec![];
        for prefix in level {
            for &c in PAIR {
                let mut units = prefix.as_utf16().to_vec();
                units.push(c as u16);
                let tip = RibbonString::from_utf16(units);
                if used.available(&tip) {
                    return tip;
                }
                next.push(tip);
            }
        }
        level = next.into_iter().filter(|t| !used.set.contains(t)).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn corpus() -> Value {
        serde_json::from_str(include_str!("../../../tests/fixtures/keytips.json")).unwrap()
    }
    fn units(v: &Value) -> Vec<u16> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect()
    }
    #[test]
    fn all_scalar_projections_and_combining_mixtures_match_source() {
        let data = corpus();
        let rows = data["unicode"]["projection"].as_array().unwrap();
        let mut at = 0;
        for scalar in 128..=0x10ffff {
            let Some(c) = char::from_u32(scalar) else {
                continue;
            };
            let label = RibbonString::from_utf16(
                std::iter::once(120)
                    .chain(c.encode_utf16(&mut [0; 2]).iter().copied())
                    .chain(std::iter::once(121))
                    .collect::<Vec<_>>(),
            );
            if data["unicode"]["errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(scalar as u64))
            {
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| letters(Some(
                        &label
                    ))))
                    .is_err()
                );
                continue;
            }
            let mut expected = vec![88];
            if at < rows.len() && rows[at]["scalar"].as_u64() == Some(scalar as u64) {
                expected.extend(units(&rows[at]["units"]));
                at += 1;
            }
            expected.push(89);
            assert_eq!(letters(Some(&label)), expected, "{scalar:x}");
        }
        assert_eq!(at, rows.len());
        for row in data["unicode"]["mixtures"].as_array().unwrap() {
            let input = RibbonString::from_utf16(units(&row["input"]));
            assert_eq!(letters(Some(&input)), units(&row["letters"]));
        }
    }
    #[test]
    fn malformed_labels_and_all_bmp_keytip_units_match_distinct_source_apis() {
        let data = corpus();
        let input = RibbonString::from_utf16((0..=u16::MAX).collect::<Vec<_>>());
        assert_eq!(
            RibbonKeyTipAssigner::normalize(Some(&input)).as_utf16(),
            units(&data["unicode"]["normalizedBmp"])
        );
        for row in data["unicode"]["malformed"].as_array().unwrap() {
            let input = RibbonString::from_utf16(units(&row["input"]));
            assert_eq!(
                RibbonKeyTipAssigner::normalize(Some(&input)).as_utf16(),
                units(&row["normalized"])
            );
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| letters(Some(&input))));
            assert_eq!(result.is_err(), !row["error"].is_null());
            if let Ok(value) = result {
                assert_eq!(value, units(&row["letters"]));
            }
        }
    }
}
