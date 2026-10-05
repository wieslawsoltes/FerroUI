use super::test_theme::create_test_theme;
use super::{MockWindowingPlatform, TestServices};
use ferroui_base::input::{IKeyboardDevice, IKeyboardNavigationHandler, InputManager, KeyboardDevice, KeyboardNavigationHandler};
use ferroui_base::media::text_formatting::testing::{
    TestFont, TestFontManagerImpl, TestTextShaperImpl, CJK_FAMILY, DEFAULT_FAMILY, EMOJI_FAMILY, GLYPH_ADVANCE,
};
use ferroui_base::platform::StandardAssetLoader;
use ferroui_base::rendering::testing::{DrawingLog, MockPlatformRenderInterface};
use std::rc::Rc;

/// The presets that need the mock windowing platform and the test theme.
impl TestServices {
    /// The services of tests that show windows: the mock windowing
    /// platform, the test theme, and the render interface, font backend and
    /// text shaper of tests (what the text test scope of the base crate
    /// binds), so that a shown window lays out its text.
    pub fn styled_window() -> Self {
        Self {
            asset_loader: Some(Rc::new(StandardAssetLoader::new(None))),
            platform: Some(Rc::new(ferroui_base::platform::StandardRuntimePlatform::new())),
            render_interface: Some(MockPlatformRenderInterface::new(DrawingLog::new())),
            standard_cursor_factory: Some(Rc::new(HeadlessCursorFactoryStub)),
            theme: Some(Rc::new(create_test_theme)),
            font_manager_impl: Some(test_font_manager_impl()),
            text_shaper_impl: Some(Rc::new(TestTextShaperImpl::new())),
            windowing_platform: Some(MockWindowingPlatform::new()),
            ..Self::default()
        }
    }

    /// The services of tests that only need a windowing platform.
    pub fn mock_windowing_platform() -> Self {
        Self { windowing_platform: Some(MockWindowingPlatform::new()), ..Self::default() }
    }

    /// The services of tests that show windows and move the keyboard focus.
    pub fn focusable_window() -> Self {
        Self {
            keyboard_device: Some(Rc::new(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))),
            keyboard_navigation: Some(Rc::new(|| {
                Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>)
            })),
            input_manager: Some(Rc::new(InputManager::new())),
            asset_loader: Some(Rc::new(StandardAssetLoader::new(None))),
            standard_cursor_factory: Some(Rc::new(HeadlessCursorFactoryStub)),
            theme: Some(Rc::new(create_test_theme)),
            windowing_platform: Some(MockWindowingPlatform::new()),
            ..Self::default()
        }
    }
}

/// The font backend of the window presets: the fonts of the text test scope
/// of the base crate (the default family, a CJK family and an emoji family).
fn test_font_manager_impl() -> Rc<TestFontManagerImpl> {
    Rc::new(TestFontManagerImpl::new(vec![
        TestFont::new(DEFAULT_FAMILY)
            .with_ranges(&[(0x20, 0x7E), (0xA0, 0x24F), (0x300, 0x36F), (0x590, 0x6FF), (0x2000, 0x206F)]),
        TestFont::new(CJK_FAMILY).with_ranges(&[(0x3000, 0x30FF), (0x4E00, 0x9FFF)]),
        TestFont::new(EMOJI_FAMILY).with_ranges(&[(0x1F300, 0x1FAFF)]).with_advance(GLYPH_ADVANCE * 2),
    ]))
}

/// A cursor factory whose cursors do nothing.
pub struct HeadlessCursorFactoryStub;

struct CursorStub;

impl ferroui_base::platform::ICursorImpl for CursorStub {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ferroui_base::platform::ICursorFactory for HeadlessCursorFactoryStub {
    fn get_cursor(&self, _cursor_type: ferroui_base::input::StandardCursorType) -> Rc<dyn ferroui_base::platform::ICursorImpl> {
        Rc::new(CursorStub)
    }

    fn create_cursor(
        &self,
        _cursor: &ferroui_base::media::imaging::Bitmap,
        _hot_spot: ferroui_base::PixelPoint,
    ) -> Rc<dyn ferroui_base::platform::ICursorImpl> {
        Rc::new(CursorStub)
    }
}
