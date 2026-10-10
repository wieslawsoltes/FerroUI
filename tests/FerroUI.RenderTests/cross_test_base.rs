//! Port of upstream's `CrossTestBase.cs` of the Skia render tests: the base
//! of the tests that describe their scene through the cross UI and compare
//! the result with an image another UI framework rendered from the same
//! description (`<name>.wpf.png`).
//!
//! `CrossFact` and `CrossTheory` are plain facts and theories in this
//! configuration. The output is written under the output directory of the
//! tests (upstream writes it next to the expected image); the flavor of the
//! file name is the one of this configuration.

use crate::cross_ui::{CrossControl, FerroCrossControl};
use crate::test_render_helper;
use ferroui_base::threading::UnitTestDispatcherScope;
use std::rc::Rc;

pub struct CrossTestBase {
    group_name: String,
    scope: Option<UnitTestDispatcherScope>,
}

impl CrossTestBase {
    pub fn new(group_name: &str) -> CrossTestBase {
        let scope = Some(test_render_helper::begin_test());
        CrossTestBase { group_name: group_name.to_string(), scope }
    }

    #[track_caller]
    pub fn render_and_compare(&self, root: CrossControl, test_name: &str) {
        self.render_and_compare_with_dpi(root, test_name, 96.0);
    }

    #[track_caller]
    pub fn render_and_compare_with_dpi(&self, root: CrossControl, test_name: &str, dpi: f64) {
        assert!(!test_name.is_empty(), "testName");

        let dir = test_render_helper::get_tests_directory().join("TestFiles").join("CrossTests").join(&self.group_name);
        let output_dir = test_render_helper::get_output_directory().join("CrossTests").join(&self.group_name);
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir).expect("the output directory is created");
        }
        let flavor = "skia";
        let render_path = output_dir.join(format!("{test_name}.{flavor}.out.png"));
        let compare_with = dir.join(format!("{test_name}.wpf.png"));
        let control = FerroCrossControl::new(Rc::new(root));
        test_render_helper::render_to_file(&control.upcast(), &render_path, false, dpi);

        test_render_helper::assert_compare_images(&render_path, &compare_with);
    }
}

impl Drop for CrossTestBase {
    fn drop(&mut self) {
        let Some(scope) = self.scope.take() else { return };
        if std::thread::panicking() {
            return;
        }
        test_render_helper::end_test(scope);
    }
}
