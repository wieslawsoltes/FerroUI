//! Reusable Android vibration backend. Its Java source is a declared library asset.
use super::HapticDevice;
use ferroui_android::{
    FerroAndroidApplication,
    interop::java::{JavaClass, JavaValue, call_static_boolean},
};

#[derive(Default)]
pub struct AndroidHaptics;
impl AndroidHaptics {
    pub fn new() -> Self {
        Self
    }
}
impl HapticDevice for AndroidHaptics {
    fn vibrate(&self, duration_ms: i32) -> bool {
        let context = FerroAndroidApplication::context();
        let class = JavaClass::find("org/ferroboard/controls/Haptics");
        call_static_boolean(
            &class,
            "vibrate",
            "(Landroid/content/Context;I)Z",
            &[
                JavaValue::Object(Some(&context)),
                JavaValue::Int(duration_ms),
            ],
        )
    }
}
