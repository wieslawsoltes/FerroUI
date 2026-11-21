//! Deliberately failing source-parity reproducer, separate from ordinary consumers.
use ferroui_base::utilities::{CompareOptions, CultureInfo};
#[test]
fn source_current_culture_ignore_case_orders_accented_label_before_z() {
    // The frozen unchanged-C# en-US probe is -1; the Python runner verifies it.
    let comparison = CultureInfo::get_culture_info("en-US")
        .compare_info()
        .compare("áb", "zb", CompareOptions::IGNORE_CASE);
    assert_eq!(
        comparison,
        std::cmp::Ordering::Less,
        "Original C# comparer orders áb before zb; selected provider-less native comparison is ordinal. This is a framework culture-data gap, not a passing parity check."
    );
}
#[test]
fn source_gallery_filter_canonical_equivalence_requires_culture_data() {
    CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-US"));
    let data: serde_json::Value = serde_json::from_str(include_str!("../gallery.json")).unwrap();
    let case = data["corpus"]["filtering"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["culture"] == "en-US" && c["query"] == "é")
        .unwrap();
    let actual = ferroui_base::utilities::StringComparison::CurrentCultureIgnoreCase.contains(
        case["label"].as_str().unwrap(),
        case["query"].as_str().unwrap(),
    );
    assert_eq!(
        actual,
        case["matches"].as_bool().unwrap(),
        "Source Gallery CurrentCultureIgnoreCase matches decomposed and composed accents; provider-less framework comparison is ordinal. Gallery refuses this missing service."
    );
}
