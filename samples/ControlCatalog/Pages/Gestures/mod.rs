//! The samples of the gestures gallery (directory `Pages/Gestures`): one module per upstream
//! code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod gesture_pinch_rotation_page;
mod gesture_pinch_zoom_page;

pub use gesture_pinch_rotation_page::GesturePinchRotationPage;
pub use gesture_pinch_zoom_page::GesturePinchZoomPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    GesturePinchRotationPage::TYPE,
    GesturePinchZoomPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &GesturePinchRotationPage::XAML_CLASS,
    &GesturePinchZoomPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
