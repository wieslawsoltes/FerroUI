// Source-derived contracts and independent paragraph/layout/RTF regressions.
use super::{
    DocumentSnapshot, Paragraph, ParagraphFormat, RichTextDocument, TabAlignment, TabLeader,
    TextTabStop, TextTabStops,
};
use crate::test_support::test_scope;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, TextAlignment};
use ferroui_base::{Rect, Size};

#[test]
fn rich_paragraph_tabs_use_native_values_points_and_sorted_replacement() {
    assert_eq!(
        [
            TabAlignment::Left as i32,
            TabAlignment::Center as i32,
            TabAlignment::Right as i32,
            TabAlignment::Decimal as i32,
            TabAlignment::Bar as i32
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        [
            TabLeader::Spaces as i32,
            TabLeader::Dots as i32,
            TabLeader::Dashes as i32,
            TabLeader::Lines as i32,
            TabLeader::ThickLines as i32,
            TabLeader::Equals as i32
        ],
        [0, 1, 2, 3, 4, 5]
    );
    let mut format = ParagraphFormat::default();
    format
        .add_tab(72.0, TabAlignment::Right, TabLeader::Dots)
        .unwrap();
    format
        .add_tab(36.0, TabAlignment::Center, TabLeader::Dashes)
        .unwrap();
    format
        .add_tab(72.0, TabAlignment::Decimal, TabLeader::Equals)
        .unwrap();
    assert_eq!(format.tab_count(), 2);
    assert_eq!(
        format.get_tab(0).unwrap(),
        (36.0, TabAlignment::Center, TabLeader::Dashes)
    );
    assert_eq!(
        format.get_tab(1).unwrap(),
        (72.0, TabAlignment::Decimal, TabLeader::Equals)
    );
    assert_eq!(
        format.tab_stops.as_ref().unwrap().as_slice()[0].position,
        48.0
    );
    format.delete_tab(36.0).unwrap();
    format.delete_tab(36.0).unwrap();
    assert_eq!(format.tab_count(), 1);
    format.clear_all_tabs();
    assert_eq!(format.tab_count(), 0);
    assert!(format.get_tab(0).is_err());
}

#[test]
fn rich_paragraph_tabs_reject_invalid_edits_atomically_and_enforce_native_limit() {
    let mut format = ParagraphFormat::default();
    for i in 1..=63 {
        format
            .add_tab(i as f64, TabAlignment::Left, TabLeader::Spaces)
            .unwrap();
    }
    let original = format.clone();
    for value in [-1.0, f64::INFINITY, f64::NAN, f64::MAX] {
        assert!(format
            .add_tab(value, TabAlignment::Left, TabLeader::Spaces)
            .is_err());
        assert_eq!(format, original);
    }
    assert!(format
        .add_tab(64.0, TabAlignment::Left, TabLeader::Spaces)
        .is_err());
    assert_eq!(format, original);
    format
        .add_tab(63.0, TabAlignment::Right, TabLeader::Dots)
        .unwrap();
    assert_eq!(format.tab_count(), 63);
    let clone = format.clone();
    format.clear_all_tabs();
    assert_eq!(
        clone.tab_count(),
        63,
        "snapshot clones must remain immutable"
    );
}

