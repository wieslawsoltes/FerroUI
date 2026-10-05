//! Port of `Xaml/WindowTests.cs`.

use ferroui_base::Ref;
use ferroui_controls::{Window, WindowTransparencyLevel};

use crate::support::app::mock_windowing_platform_application;
use crate::support::loader::parse;

#[test]
fn can_specify_transparency_level_hint() {
    let _app = mock_windowing_platform_application();
    let xaml = "<Window xmlns='https://github.com/ferroui' TransparencyLevelHint='Blur,Transparent,None'/>";

    let target = parse::<Ref<Window>>(xaml);

    assert_eq!(
        &[WindowTransparencyLevel::blur(), WindowTransparencyLevel::transparent(), WindowTransparencyLevel::none()][..],
        &*target.transparency_level_hint()
    );
}
