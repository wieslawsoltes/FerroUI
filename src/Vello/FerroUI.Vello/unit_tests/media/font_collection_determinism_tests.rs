//! Port of upstream's `Media/FontCollectionDeterminismTests.cs` of the Skia
//! unit tests.
//!
//! Verifies that `FontCollectionBase.TryMatchCharacter` resolves the same
//! family regardless of the order in which fonts were added to the
//! collection, and is stable across repeated invocations.
//!
//! Upstream shuffles with `System.Random(seed)`; [`Random`] ports its seeded
//! generator (the subtractive generator .NET keeps for seeded instances), so
//! the orderings are upstream's.

use crate::unit_tests::mock_platform_render_interface;
use crate::FontManagerImpl;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, IFontCollection};
use ferroui_base::media::{FontStretch, FontStyle, FontWeight, Typeface};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::testing::UnitTestApplication;
use std::rc::Rc;

const ASSETS_NAMESPACE: &str = "FerroUI.Vello.UnitTests.Assets";

// A set of Latin-covering test fonts. All cover ASCII 'A'.
fn s_latin_font_assets() -> Vec<String> {
    [
        "Inter-Regular.ttf",
        "Inter-Bold.ttf",
        "Manrope-Light.ttf",
        "NotoMono-Regular.ttf",
        "NotoSans-Italic.ttf",
        "SourceSerif4_36pt-Italic.ttf",
    ]
    .iter()
    .map(|file_name| format!("{ASSETS_NAMESPACE}.{file_name}"))
    .collect()
}

#[test]
fn try_match_character_returns_same_family_regardless_of_add_order() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let latin_font_assets = s_latin_font_assets();

    let orderings = [
        latin_font_assets.clone(),
        latin_font_assets.iter().rev().cloned().collect(),
        shuffle(&latin_font_assets, 1),
        shuffle(&latin_font_assets, 17),
        shuffle(&latin_font_assets, 42),
        shuffle(&latin_font_assets, 1337),
    ];

    let mut expected: Option<String> = None;

    for ordering in &orderings {
        let collection = build_collection(ordering);

        let matched = collection
            .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
            .unwrap_or_else(|| panic!("{ordering:?}"));

        let family_name = extract_family_name(&matched).to_owned();

        match &expected {
            None => expected = Some(family_name),
            Some(expected) => assert_eq!(expected, &family_name, "{ordering:?}"),
        }
    }
}

#[test]
fn try_match_character_is_stable_across_repeated_invocations() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let collection = build_collection(&s_latin_font_assets());

    let first_match = collection
        .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    let expected = extract_family_name(&first_match);

    for i in 0..50 {
        let matched = collection
            .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
            .expect("a match");

        assert_eq!(expected, extract_family_name(&matched), "{i}");
    }
}

#[test]
fn try_match_character_result_is_independent_of_concurrent_cache_population() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    // Touch a variety of typefaces in different orders before the fallback call,
    // so each collection's _glyphTypefaceCache has different ConcurrentDictionary
    // insertion / hash-bucket order.
    let mut results_a: Vec<String> = Vec::new();
    let mut results_b: Vec<String> = Vec::new();

    for _ in 0..5 {
        let a = build_collection(&s_latin_font_assets());
        let b = build_collection(&s_latin_font_assets());

        // Different warm-up order on purpose.
        warmup(&a, &["Inter", "Manrope Light", "Noto Mono", "Noto Sans", "Source Serif 4 36pt"]);
        warmup(&b, &["Source Serif 4 36pt", "Noto Sans", "Noto Mono", "Manrope Light", "Inter"]);

        let ma = a
            .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
            .expect("a match");
        let mb = b
            .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
            .expect("a match");

        results_a.push(extract_family_name(&ma).to_owned());
        results_b.push(extract_family_name(&mb).to_owned());
    }

    // Every iteration of A produces the same family.
    assert_eq!(1, distinct(&results_a).len());

    // Every iteration of B produces the same family.
    assert_eq!(1, distinct(&results_b).len());

    // And the chosen family is the same for both warm-up orders.
    assert_eq!(results_a[0], results_b[0]);
}

#[test]
fn try_match_character_cached_script_fallback_lookup_returns_same_family() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let collection = build_collection(&s_latin_font_assets());

    // First call populates the script/culture fallback cache; subsequent calls must hit
    // it and still return the same family.
    let first = collection
        .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    let second = collection
        .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    assert_eq!(extract_family_name(&first), extract_family_name(&second));
}

fn build_collection(asset_paths: &[String]) -> Rc<CustomFontCollection> {
    let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
    let collection = CustomFontCollection::new(Uri::new("fonts:determinism", UriKind::Absolute).unwrap());

    for path in asset_paths {
        let uri = Uri::new(&format!("resm:{path}?assembly=ferroui-vello"), UriKind::Absolute).unwrap();

        let mut stream = asset_loader.open(&uri, None).expect("the font stream");

        assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(&*collection, &mut *stream).is_some(), "{path}");
    }

    collection
}

fn warmup(collection: &CustomFontCollection, family_names: &[&str]) {
    for name in family_names {
        collection.try_get_glyph_typeface(name, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal);
    }
}

fn shuffle(source: &[String], seed: i32) -> Vec<String> {
    let mut rng = Random::new(seed);
    let mut copy = source.to_vec();

    for i in (1..copy.len()).rev() {
        let j = rng.next(i as i32 + 1) as usize;
        copy.swap(i, j);
    }

    copy
}

fn extract_family_name(typeface: &Typeface) -> &str {
    // Fallback Typefaces are built with a FontFamily of the form "<collection-key>#<familyName>".
    // The plain family name is what we want to compare across orderings.
    let name = typeface.font_family().name();

    match name.rfind('#') {
        Some(hash_index) => &name[hash_index + 1..],
        None => name,
    }
}

/// The distinct values, in order of first appearance (C# `Distinct()`).
fn distinct(values: &[String]) -> Vec<&String> {
    let mut result: Vec<&String> = Vec::new();

    for value in values {
        if !result.contains(&value) {
            result.push(value);
        }
    }

    result
}

struct CustomFontCollection {
    base: FontCollectionBase,
    key: Uri,
}

impl CustomFontCollection {
    fn new(key: Uri) -> Rc<Self> {
        Rc::new(Self { base: FontCollectionBase::new(), key })
    }
}

impl FontCollectionBaseImpl for CustomFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }
}

/// `System.Random` created with a seed: Knuth's subtractive generator, as
/// .NET implements it for seeded instances.
struct Random {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl Random {
    const MSEED: i32 = 161803398;

    fn new(seed: i32) -> Self {
        let mut seed_array = [0i32; 56];

        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = Self::MSEED - subtraction;
        seed_array[55] = mj;
        let mut mk = 1;
        let mut ii = 0;

        for _ in 1..55 {
            ii += 21;

            if ii >= 55 {
                ii -= 55;
            }

            seed_array[ii] = mk;
            mk = mj - mk;

            if mk < 0 {
                mk += i32::MAX;
            }

            mj = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;

                if n >= 55 {
                    n -= 55;
                }

                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);

                if seed_array[i] < 0 {
                    seed_array[i] = seed_array[i].wrapping_add(i32::MAX);
                }
            }
        }

        Self { seed_array, inext: 0, inextp: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;

        if loc_inext >= 56 {
            loc_inext = 1;
        }

        let mut loc_inextp = self.inextp + 1;

        if loc_inextp >= 56 {
            loc_inextp = 1;
        }

        let mut ret_val = self.seed_array[loc_inext].wrapping_sub(self.seed_array[loc_inextp]);

        if ret_val == i32::MAX {
            ret_val -= 1;
        }

        if ret_val < 0 {
            ret_val = ret_val.wrapping_add(i32::MAX);
        }

        self.seed_array[loc_inext] = ret_val;
        self.inext = loc_inext;
        self.inextp = loc_inextp;

        ret_val
    }

    fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 * (1.0 / i32::MAX as f64)
    }

    /// A non-negative random integer less than `max_value`.
    fn next(&mut self, max_value: i32) -> i32 {
        (self.sample() * max_value as f64) as i32
    }
}
