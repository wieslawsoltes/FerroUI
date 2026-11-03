//! Port of upstream's `Media/TextFormatting/BidiReordererTests.cs` of the
//! Skia unit tests.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::{
    BidiReorderer, GenericTextRunProperties, ShapedTextRun, TextRun, TextRunProperties, TextShaper,
    TextShaperOptions,
};
use ferroui_base::media::{FlowDirection, Typeface};
use ferroui_base::utilities::{current_locale, ReadOnlyMemory};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

/// Each case lists the bidi level of every run in logical order and the expected
/// logical index of the run at each visual position.
fn level_cases() -> Vec<(FlowDirection, Vec<i8>, Vec<usize>)> {
    vec![
        // Single level-0 run, nothing to reorder.
        (FlowDirection::LeftToRight, vec![0], vec![0]),
        // Two LTR runs inside an RTL paragraph keep their relative order and sit left of the RTL run (3-cycle).
        (FlowDirection::RightToLeft, vec![1, 2, 2], vec![1, 2, 0]),
        (FlowDirection::RightToLeft, vec![2, 2, 1], vec![2, 0, 1]),
        (FlowDirection::RightToLeft, vec![1, 2, 2, 1], vec![3, 1, 2, 0]),
        (FlowDirection::RightToLeft, vec![2, 2, 2, 1], vec![3, 0, 1, 2]),
        // LTR runs mixed with RTL runs in an LTR paragraph.
        (FlowDirection::LeftToRight, vec![0, 1, 1, 0], vec![0, 2, 1, 3]),
        (FlowDirection::LeftToRight, vec![0, 1, 2, 1, 0], vec![0, 3, 2, 1, 4]),
        (FlowDirection::LeftToRight, vec![1, 1, 0, 0], vec![1, 0, 2, 3]),
    ]
}

#[test]
fn should_map_logical_runs_to_visual_run_index() {
    for (flow_direction, levels, visual_to_logical) in level_cases() {
        let _scope = start();

        let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let logical_runs: Vec<Rc<dyn TextRun>> = levels
            .iter()
            .enumerate()
            .map(|(i, &level)| {
                let text: String = std::iter::repeat_n((b'a' + i as u8) as char, i + 1).collect();
                let options = TextShaperOptions::with_all(
                    Typeface::default_typeface().glyph_typeface(),
                    10.0,
                    level,
                    Some(current_locale()),
                    0.0,
                    0.0,
                    None,
                );
                let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(&text), &options);
                Rc::new(ShapedTextRun::new(shaped_buffer, properties.clone())) as Rc<dyn TextRun>
            })
            .collect();

        let mut text_runs = logical_runs.clone();

        let indexed_runs = BidiReorderer::bidi_reorder(&mut text_runs, flow_direction, 5);

        assert_eq!(levels.len(), indexed_runs.len());

        // text_runs is rewritten into visual order.
        for (visual, &logical) in visual_to_logical.iter().enumerate() {
            assert!(Rc::ptr_eq(&logical_runs[logical], &text_runs[visual]), "{levels:?}: visual {visual}");
        }

        // IndexedTextRun stays in logical order and run_index points at the run's visual position.
        let mut expected_source_index = 5;

        for (logical, indexed) in indexed_runs.iter().enumerate() {
            let indexed_text_run = indexed.text_run.borrow().clone().expect("a text run");

            assert!(Rc::ptr_eq(&logical_runs[logical], &indexed_text_run), "{levels:?}: logical {logical}");
            assert_eq!(expected_source_index, indexed.text_source_character_index);
            let expected_run_index = visual_to_logical.iter().position(|&l| l == logical).map_or(-1, |p| p as i32);
            assert_eq!(expected_run_index, indexed.run_index.get(), "{levels:?}: logical {logical}");
            assert!(Rc::ptr_eq(&indexed_text_run, &text_runs[indexed.run_index.get() as usize]));

            expected_source_index += logical_runs[logical].length();
        }
    }
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::new(None, None)))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}
