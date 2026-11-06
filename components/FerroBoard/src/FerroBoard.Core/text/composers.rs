// Derived from Text/Composers.cs; see ../NOTICE.md.
use super::{Utf16Text, composer_data::*};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
/// Shared native rule map, retained by identity. HashMap iteration order is unspecified.
/// Composers capture a longest-suffix snapshot; equal-length keys use UTF-16 lexical order
/// for deterministic caches. Actions never acquire the retained map's lock.
pub type ComposerRules = Arc<RwLock<HashMap<Utf16Text, Utf16Text>>>;
pub trait IComposer {
    fn id(&self) -> Utf16Text;
    fn label(&self) -> Utf16Text;
    fn to_read(&self) -> i32;
    fn get_actions(&self, preceding_text: &Utf16Text, to_insert: &Utf16Text) -> (i32, Utf16Text);
}
pub struct AppenderComposer;
impl AppenderComposer {
    pub fn instance() -> Arc<Self> {
        static INSTANCE: LazyLock<Arc<AppenderComposer>> =
            LazyLock::new(|| Arc::new(AppenderComposer));
        INSTANCE.clone()
    }
}
impl IComposer for AppenderComposer {
    fn id(&self) -> Utf16Text {
        "appender".into()
    }
    fn label(&self) -> Utf16Text {
        "Appender".into()
    }
    fn to_read(&self) -> i32 {
        0
    }
    fn get_actions(&self, _: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        (0, insert.clone())
    }
}
fn index(chars: &[u16], c: u16) -> i32 {
    chars.iter().position(|x| *x == c).map_or(-1, |i| i as i32)
}
fn syllable(ini: i32, med: i32, fin: i32) -> u16 {
    (ini * 588 + med * 28 + fin + 44032) as u16
}
fn changed(units: &[u16]) -> (i32, Utf16Text) {
    (1, Utf16Text::from_units(units.to_vec()))
}
fn compose(table: &[(u16, u16, u16)], a: u16, b: u16) -> Option<u16> {
    table
        .iter()
        .find(|(x, y, _)| *x == a && *y == b)
        .map(|(_, _, c)| *c)
}
pub struct HangulComposer;
impl IComposer for HangulComposer {
    fn id(&self) -> Utf16Text {
        "hangul-unicode".into()
    }
    fn label(&self) -> Utf16Text {
        "Hangul Unicode".into()
    }
    fn to_read(&self) -> i32 {
        1
    }
    fn get_actions(&self, preceding: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        let fallback = || (0, insert.clone());
        let Some(&last) = preceding.units().last() else {
            return fallback();
        };
        let Some(&c) = insert.units().first() else {
            return fallback();
        };
        let initial = index(INITIALS, last);
        let medial = index(MEDIALS, c);
        if initial >= 0 && medial >= 0 {
            return changed(&[syllable(initial, medial, 0)]);
        }
        if (44032..=55203).contains(&last) {
            let ord = i32::from(last);
            let ini = (ord - 44032) / 588;
            let med = (ord - 44032 - ini * 588) / 28;
            let fin = (ord - 44032) % 28;
            if c == b'_' as u16 {
                return fallback();
            }
            let final_index = index(FINALS, c);
            if fin == 0 && final_index > 0 {
                return changed(&[syllable(ini, med, final_index)]);
            }
            let last_final = FINALS[fin as usize];
            if let Some(combined) = compose(FINALCOMP, last_final, c) {
                return changed(&[syllable(ini, med, index(FINALS, combined))]);
            }
            let reverse = FINALCOMP
                .iter()
                .find(|(_, _, combined)| *combined == last_final);
            if fin != 0 && reverse.is_none() && medial >= 0 && index(INITIALS, last_final) >= 0 {
                return changed(&[
                    syllable(ini, med, 0),
                    syllable(index(INITIALS, last_final), medial, 0),
                ]);
            }
            if let Some(&(first, second, _)) = reverse
                && medial >= 0
            {
                return changed(&[
                    syllable(ini, med, index(FINALS, first)),
                    syllable(index(INITIALS, second), medial, 0),
                ]);
            }
            if let Some(combined) = compose(MEDIALCOMP, MEDIALS[med as usize], c)
                && fin == 0
            {
                return changed(&[syllable(ini, index(MEDIALS, combined), 0)]);
            }
        } else if let Some(combined) = compose(MEDIALCOMP, last, c) {
            return changed(&[combined]);
        } else if let Some(combined) = compose(FINALCOMP, last, c) {
            return changed(&[combined]);
        }
        fallback()
    }
}
pub struct KanaComposer;
fn toggle(table: &[(u16, u16)], last: u16) -> Option<u16> {
    table
        .iter()
        .find(|(a, _)| *a == last)
        .map(|(_, b)| *b)
        .or_else(|| table.iter().find(|(_, b)| *b == last).map(|(a, _)| *a))
}
impl IComposer for KanaComposer {
    fn id(&self) -> Utf16Text {
        "kana-unicode".into()
    }
    fn label(&self) -> Utf16Text {
        "Kana Unicode".into()
    }
    fn to_read(&self) -> i32 {
        1
    }
    fn get_actions(&self, preceding: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        let fallback = || (0, insert.clone());
        let Some(&c) = insert.units().first() else {
            return fallback();
        };
        let daku = matches!(c, 0x3099 | 0x309b);
        let handaku = matches!(c, 0x309a | 0x309c);
        let small = c == 0x3013;
        let Some(&last) = preceding.units().last() else {
            return if daku || handaku || small {
                (0, Utf16Text::default())
            } else {
                fallback()
            };
        };
        if daku {
            if let Some(c) = toggle(DAKU, last) {
                return changed(&[c]);
            }
            return fallback();
        }
        if handaku {
            if let Some(c) = toggle(HANDAKU, last) {
                return changed(&[c]);
            }
            return fallback();
        }
        if small {
            if let Some(c) = toggle(SMALL, last) {
                return changed(&[c]);
            }
            return (0, Utf16Text::default());
        }
        fallback()
    }
}
/// Suffix-replacement composer, preserving the source constructor snapshot of rules.
pub struct RulesComposer {
    id: Utf16Text,
    label: Utf16Text,
    to_read: i32,
    rules: ComposerRules,
    ordered_rules: Vec<(Utf16Text, Utf16Text)>,
}
impl RulesComposer {
    pub fn new(id: Utf16Text, label: Utf16Text, rules: ComposerRules) -> Self {
        let mut ordered_rules: Vec<_> = rules
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        ordered_rules.sort_by(|(left, _), (right, _)| {
            right
                .len()
                .cmp(&left.len())
                .then_with(|| left.units().cmp(right.units()))
        });
        // Source enumerates Rules.Keys separately after capturing _orderedRules.
        let to_read = rules
            .read()
            .keys()
            .map(|k| k.len())
            .max()
            .unwrap_or(0)
            .saturating_sub(1) as i32;
        Self {
            id,
            label,
            to_read,
            rules,
            ordered_rules,
        }
    }
    pub fn rules(&self) -> ComposerRules {
        self.rules.clone()
    }
}
impl IComposer for RulesComposer {
    fn id(&self) -> Utf16Text {
        self.id.clone()
    }
    fn label(&self) -> Utf16Text {
        self.label.clone()
    }
    fn to_read(&self) -> i32 {
        self.to_read
    }
    fn get_actions(&self, preceding: &Utf16Text, insert: &Utf16Text) -> (i32, Utf16Text) {
        use super::TextCulture;
        let text = Utf16Text::from_units(
            preceding
                .units()
                .iter()
                .chain(insert.units())
                .copied()
                .collect::<Vec<_>>(),
        );
        let lower = TextCulture::to_lower_invariant(&text);
        for (key, value) in &self.ordered_rules {
            if !key.is_empty() && lower.units().ends_with(key.units()) {
                let first = Utf16Text::from_units(vec![text.units()[text.len() - key.len()]]);
                let value = if TextCulture::invariant().to_upper(&first) == first
                    && TextCulture::to_lower_invariant(&first) != first
                {
                    TextCulture::invariant().to_upper(value)
                } else {
                    value.clone()
                };
                return (key.len() as i32 - 1, value);
            }
        }
        (0, insert.clone())
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn native_rule_cache_keeps_source_entries_with_deterministic_equal_length_order() {
        fn text(v: &Value) -> Utf16Text {
            Utf16Text::from_units(
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            )
        }
        let rows = include_str!("../../../tests/fixtures/composer-ownership.jsonl")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows[0]["phase"], "input");
        for row in &rows[1..] {
            // The first composer retains its initial cache after both mutations. The rebuilt
            // and bundled composers capture the current dictionary at their own construction.
            let original = match row["phase"].as_str().unwrap() {
                "initial" | "worker-mutated" | "cleared" => &rows[0]["rules"],
                _ => &row["rules"],
            };
            let rules = ComposerRules::default();
            for entry in original.as_array().unwrap() {
                rules
                    .write()
                    .insert(text(&entry["key"]), text(&entry["value"]));
            }
            let composer = RulesComposer::new("id".into(), "Label".into(), rules);
            let ordered = composer
                .ordered_rules
                .iter()
                .map(|(key, value)| json!({"key":key.units(),"value":value.units()}))
                .collect::<Vec<_>>();
            // Native maps have no source dictionary slot order. Canonicalize only equal-length
            // cache entries; distinct keys of equal length cannot match the same suffix.
            let mut expected = row["ordered"].as_array().unwrap().clone();
            expected.sort_by(|left, right| {
                let left = text(&left["key"]);
                let right = text(&right["key"]);
                right
                    .len()
                    .cmp(&left.len())
                    .then_with(|| left.units().cmp(right.units()))
            });
            assert_eq!(json!(ordered), json!(expected), "{}", row["phase"]);
            assert_eq!(json!(composer.to_read), row["toRead"]);
        }
    }
}
