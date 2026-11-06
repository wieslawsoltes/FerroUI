//! iOS locale operations through public CoreFoundation APIs, not private ICU symbols.
use super::{CollationError, Ordering, length, pointer};
use std::{ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};

#[repr(C)]
struct Range {
    location: isize,
    length: isize,
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCharacters(
        allocator: *const c_void,
        characters: *const u16,
        length: isize,
    ) -> *const c_void;
    fn CFLocaleCreate(allocator: *const c_void, identifier: *const c_void) -> *const c_void;
    fn CFRelease(object: *const c_void);
    fn CFStringCompareWithOptionsAndLocale(
        left: *const c_void,
        right: *const c_void,
        range: Range,
        options: usize,
        locale: *const c_void,
    ) -> isize;
    fn CFStringFindWithOptionsAndLocale(
        text: *const c_void,
        pattern: *const c_void,
        range: Range,
        options: usize,
        locale: *const c_void,
        result: *mut Range,
    ) -> u8;
}
struct OwnedCf(NonNull<c_void>);
impl OwnedCf {
    fn created(raw: *const c_void, operation: &'static str) -> Result<Self, CollationError> {
        NonNull::new(raw.cast_mut())
            .map(Self)
            .ok_or(CollationError::Native { operation, code: 0 })
    }
    fn raw(&self) -> *const c_void {
        self.0.as_ptr()
    }
}
impl Drop for OwnedCf {
    fn drop(&mut self) {
        // SAFETY: each successful Create result has exactly one owning RAII handle.
        unsafe { CFRelease(self.raw()) }
    }
}
struct NativeString {
    object: OwnedCf,
    length: isize,
}
impl NativeString {
    fn new(text: &[u16]) -> Result<Self, CollationError> {
        // Keep the engine's signed UTF16 length bound, even on a 64-bit CFIndex ABI.
        let length = length(text)? as isize;
        // SAFETY: checked explicit length, readable UTF16 buffer; Create copies it.
        let raw = unsafe { CFStringCreateWithCharacters(std::ptr::null(), pointer(text), length) };
        Ok(Self {
            object: OwnedCf::created(raw, "CFStringCreateWithCharacters")?,
            length,
        })
    }
    fn range(&self) -> Range {
        Range {
            location: 0,
            length: self.length,
        }
    }
}
// Canonical equivalence and explicit Apple locale tailoring; case follows the caller.
fn options(ignore_case: bool) -> usize {
    const NONLITERAL: usize = 16;
    const LOCALIZED: usize = 32;
    NONLITERAL | LOCALIZED | usize::from(ignore_case)
}
pub(super) struct Collator {
    locale: OwnedCf,
    _owner_thread: PhantomData<Rc<()>>,
}
impl Collator {
    pub(super) fn new(locale: &str) -> Result<Self, CollationError> {
        if locale.contains('\0') {
            return Err(CollationError::InvalidLocale);
        }
        let identifier: Vec<_> = locale.encode_utf16().collect();
        let identifier = NativeString::new(&identifier)?;
        // SAFETY: live immutable CFString identifier; Create owns its locale result.
        let raw = unsafe { CFLocaleCreate(std::ptr::null(), identifier.object.raw()) };
        Ok(Self {
            locale: OwnedCf::created(raw, "CFLocaleCreate")?,
            _owner_thread: PhantomData,
        })
    }
    pub(super) fn compare(
        &self,
        left: &[u16],
        right: &[u16],
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        let left = NativeString::new(left)?;
        let right = NativeString::new(right)?;
        // SAFETY: live CF strings/locale, exact complete left range, documented flag bits.
        let result = unsafe {
            CFStringCompareWithOptionsAndLocale(
                left.object.raw(),
                right.object.raw(),
                left.range(),
                options(ignore_case),
                self.locale.raw(),
            )
        };
        Ok(result.cmp(&0))
    }
    pub(super) fn contains(
        &self,
        text: &[u16],
        pattern: &[u16],
        ignore_case: bool,
    ) -> Result<bool, CollationError> {
        // CF's finder rejects empty patterns; the keyboard's established contract matches them.
        if pattern.is_empty() || self.compare(pattern, &[], ignore_case)? == Ordering::Equal {
            return Ok(true);
        }
        if text.is_empty() {
            return Ok(false);
        }
        let text = NativeString::new(text)?;
        let pattern = NativeString::new(pattern)?;
        // SAFETY: live CF strings/locale, exact text range; null result requests a boolean only.
        let found = unsafe {
            CFStringFindWithOptionsAndLocale(
                text.object.raw(),
                pattern.object.raw(),
                text.range(),
                options(ignore_case),
                self.locale.raw(),
                std::ptr::null_mut(),
            )
        };
        Ok(found != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
    #[test]
    fn public_cf_locale_sorts_and_searches_keyboard_labels() {
        let locale = Collator::new("en-US").unwrap();
        for (left, right, ignore, expected) in [
            ("Alpha", "alpha", true, Ordering::Equal),
            ("a", "b", false, Ordering::Less),
            ("café", "cafe\u{301}", true, Ordering::Equal),
            ("café", "cafe", true, Ordering::Greater),
        ] {
            assert_eq!(
                locale.compare(&units(left), &units(right), ignore).unwrap(),
                expected
            );
        }
        for (text, pattern, ignore, expected) in [
            ("English (United States)", "UNITED", true, true),
            ("English", "ENGLISH", false, false),
            ("café", "cafe\u{301}", true, true),
            ("café", "cafe", true, false),
            ("😀English👩‍💻", "English", true, true),
            ("A\0B", "\0B", false, true),
            ("", "", true, true),
            ("", "English", true, false),
            ("English", "", true, true),
        ] {
            assert_eq!(
                locale
                    .contains(&units(text), &units(pattern), ignore)
                    .unwrap(),
                expected,
                "{text:?}/{pattern:?}"
            );
        }
        assert!(matches!(
            Collator::new("en\0US"),
            Err(CollationError::InvalidLocale)
        ));
    }
    #[test]
    fn public_cf_locale_uses_explicit_turkish_and_swedish_tailoring() {
        let tr = Collator::new("tr-TR").unwrap();
        assert_eq!(
            tr.compare(&units("I"), &units("ı"), true).unwrap(),
            Ordering::Equal
        );
        assert_ne!(
            tr.compare(&units("I"), &units("i"), true).unwrap(),
            Ordering::Equal
        );
        assert!(
            tr.contains(&units("İstanbul"), &units("istanbul"), true)
                .unwrap()
        );
        let sv = Collator::new("sv-SE").unwrap();
        assert_eq!(
            sv.compare(&units("z"), &units("å"), true).unwrap(),
            Ordering::Less
        );
    }
    #[test]
    fn public_cf_locale_keeps_explicit_utf16_units_and_owned_lifetimes() {
        let locale = Collator::new("").unwrap();
        let text = [0xd800, b'A' as u16, 0xdc00];
        assert_eq!(
            locale.compare(&text, &text, false).unwrap(),
            Ordering::Equal
        );
        assert!(locale.contains(&text, &[0xd800], false).unwrap());
        assert!(locale.contains(&text, &units("a"), true).unwrap());
        for _ in 0..20 {
            assert!(
                locale
                    .contains(&units("Danish"), &units("dan"), true)
                    .unwrap()
            );
            assert!(
                !locale
                    .contains(&units("Swedish"), &units("dan"), true)
                    .unwrap()
            );
        }
    }
    #[test]
    fn public_cf_locale_measures_original_comparison_corpus() {
        use std::collections::BTreeMap;
        let decode = |text: &str| -> Vec<u16> {
            if text.is_empty() {
                vec![]
            } else {
                text.split(',')
                    .map(|v| u16::from_str_radix(v, 16).unwrap())
                    .collect()
            }
        };
        let mut locales = BTreeMap::new();
        let mut report = Vec::new();
        let source = include_str!("../../../../tests/fixtures/culture-comparison.tsv");
        assert_eq!(source.lines().count(), 150);
        for (index, line) in source.lines().enumerate() {
            let row: Vec<_> = line.split('\t').collect();
            let locale = locales
                .entry(row[0])
                .or_insert_with(|| Collator::new(row[0]).unwrap());
            let (left, right) = (decode(row[3]), decode(row[4]));
            let ignore = row[2] == "True";
            let actual = if row[1] == "compare" {
                match locale.compare(&left, &right, ignore).unwrap() {
                    Ordering::Less => "-1",
                    Ordering::Equal => "0",
                    Ordering::Greater => "1",
                }
            } else if locale.contains(&left, &right, ignore).unwrap() {
                "True"
            } else {
                "False"
            };
            if actual != row[5] {
                report.push(serde_json::json!({"index":index,"source":line,"native":actual}));
            }
        }
        if let Ok(path) = std::env::var("FERROBOARD_CF_CORPUS_REPORT") {
            std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        }
        let reviewed: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/apple-locale-differences.json"
        ))
        .unwrap();
        let expected: Vec<_> = reviewed["differences"].as_array().unwrap().iter().map(|row| {
            serde_json::json!({"index":row["index"], "source":row["source"], "native":row["native"]})
        }).collect();
        assert_eq!(
            report, expected,
            "unreviewed Apple comparison/search behavior"
        );
        assert_eq!(reviewed["total_cases"], 150);
        assert_eq!(report.len(), 25);
    }
}
