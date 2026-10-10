//! Port of upstream's `TestBase.cs` of the render tests.
//!
//! A test names itself (`[CallerMemberName]` upstream): the name is the
//! upstream method name, which is the base name of the expected image.
//! `expected_path` is the directory of the expected images inside the test
//! crate and `output_path` the directory of the outputs under the build
//! directory; upstream has one directory for both.

use crate::test_render_helper::{self, Rgba32Image, ALLOWED_ERROR};
use ferroui_base::media::FontFamily;
use ferroui_base::threading::UnitTestDispatcherScope;
use ferroui_base::{IntoRef, Ref};
use ferroui_controls::Control;
use std::path::{PathBuf, MAIN_SEPARATOR};

const FONT_URI: &str = "resm:FerroUI.Skia.RenderTests.Assets?assembly=ferroui-render-tests#Noto Mono";

/// `TestBase.TestFontFamily`.
pub fn test_font_family() -> FontFamily {
    FontFamily::new(FONT_URI)
}

pub struct TestBase {
    expected_path: PathBuf,
    output_path: PathBuf,
    scope: Option<UnitTestDispatcherScope>,
}

/// What `CompareImages` leaves out.
#[derive(Clone, Copy, Default)]
pub struct CompareOptions {
    /// Skips the immediate renderer output comparison.
    pub skip_immediate: bool,
    /// Skips all composited output comparisons.
    pub skip_compositor: bool,
}

impl TestBase {
    pub fn new(output_path: &str) -> TestBase {
        let output_path = output_path.replace('\\', &MAIN_SEPARATOR.to_string());
        let test_path = test_render_helper::get_tests_directory();
        let test_files = test_path.join("TestFiles");
        let expected_path = test_files.join("Skia").join(&output_path);
        let output_path = test_render_helper::get_output_directory().join("Skia").join(&output_path);

        let scope = Some(test_render_helper::begin_test());

        TestBase { expected_path, output_path, scope }
    }

    /// The directory of the expected images.
    pub fn expected_path(&self) -> &PathBuf {
        &self.expected_path
    }

    /// The directory the outputs are written to.
    pub fn output_path(&self) -> &PathBuf {
        if !self.output_path.exists() {
            std::fs::create_dir_all(&self.output_path).expect("the output directory is created");
        }
        &self.output_path
    }

    pub fn render_to_file(&self, target: impl IntoRef<Control>, test_name: &str) {
        self.render_to_file_with_dpi(target, test_name, 96.0);
    }

    pub fn render_to_file_with_dpi(&self, target: impl IntoRef<Control>, test_name: &str, dpi: f64) {
        let target: Ref<Control> = target.into_ref();
        let output_path = self.output_path();

        let immediate_path = output_path.join(format!("{test_name}.immediate.out.png"));
        let composited_path = output_path.join(format!("{test_name}.composited.out.png"));
        test_render_helper::render_to_file(&target, &immediate_path, true, dpi);
        test_render_helper::render_to_file(&target, &composited_path, false, dpi);
    }

    #[track_caller]
    pub fn compare_images(&self, test_name: &str) {
        self.compare_images_with(test_name, CompareOptions::default());
    }

    #[track_caller]
    pub fn compare_images_with(&self, test_name: &str, options: CompareOptions) {
        let expected_path = self.expected_path.join(format!("{test_name}.expected.png"));

        let expected = Rgba32Image::load(&expected_path);

        // Upstream fails at the first output that is beyond the allowed error. Here every output is
        // measured first, so that the record of the run (`results.tsv`) has the error of each, and the test
        // then fails as upstream does, with the outputs that are beyond the allowed error in its message.
        let mut failures = Vec::new();
        let mut compare = |output_type: &str, allowed_error: f64| {
            let actual_path = self.output_path.join(format!("{test_name}.{output_type}.out.png"));
            if let Err(message) = test_render_helper::compare_output(&actual_path, &expected, allowed_error) {
                failures.push(message);
            }
        };

        if !options.skip_immediate {
            compare("immediate", ALLOWED_ERROR);
        }

        if !options.skip_compositor {
            compare("composited", ALLOWED_ERROR);
        }

        if !failures.is_empty() {
            panic!("{} (expected: {})", failures.join("; "), expected_path.display());
        }
    }

    #[track_caller]
    pub fn compare_images_no_renderer(&self, test_name: &str, expected_name: Option<&str>) {
        let expected_path = self.expected_path.join(format!("{}.expected.png", expected_name.unwrap_or(test_name)));
        let actual_path = self.output_path.join(format!("{test_name}.out.png"));
        test_render_helper::assert_compare_images(&actual_path, &expected_path);
    }
}

impl Drop for TestBase {
    fn drop(&mut self) {
        let Some(scope) = self.scope.take() else { return };
        if std::thread::panicking() {
            return;
        }
        test_render_helper::end_test(scope);
    }
}
