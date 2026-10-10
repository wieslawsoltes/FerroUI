//! The screens of the device: the displays of the display manager.
//!
//! The reference asks the display objects of the system; here the Java
//! layer describes every display in one call ([`DisplayInfo`]) and the
//! screens are made from the descriptions, through a source the tests
//! replace.

use ferroui_base::{PixelPoint, PixelRect};
use ferroui_controls::platform::{
    ITopLevelImpl, PlatformHandle, PlatformScreen, Screen, ScreenOrientation, ScreensBase, ScreensBaseImpl,
    ScreensBaseImplExt,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// `Display.DEFAULT_DISPLAY`.
const DEFAULT_DISPLAY: i32 = 0;

/// `Surface.ROTATION_*`.
pub(crate) mod surface_orientation {
    pub const ROTATION_0: i32 = 0;
    pub const ROTATION_90: i32 = 1;
    pub const ROTATION_180: i32 = 2;
    pub const ROTATION_270: i32 = 3;
}

/// `Configuration.ORIENTATION_*`.
pub(crate) mod android_orientation {
    pub const PORTRAIT: i32 = 1;
    pub const LANDSCAPE: i32 = 2;
    pub const SQUARE: i32 = 3;
}

/// What the Java layer tells about a display.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayInfo {
    pub display_id: i32,
    pub name: String,
    /// `Display.getRotation`.
    pub rotation: i32,
    /// Whether the bounds are the maximum window metrics of the display
    /// (API 30 and later); otherwise they are its real metrics.
    pub has_window_metrics: bool,
    pub bounds: PixelRect,
    /// The density of the configuration over the default density, or the
    /// density of the real metrics.
    pub scaling: f64,
    /// The orientation of the configuration of the display; 0 when it is
    /// not known.
    pub orientation: i32,
}

/// The number of values the Java layer gives for each display.
const DISPLAY_LENGTH: usize = 9;

impl DisplayInfo {
    /// Reads the descriptions of `PlatformHelper.getDisplays` and the names
    /// of `PlatformHelper.getDisplayNames`.
    pub fn from_arrays(values: &[f32], names: &[String]) -> Vec<DisplayInfo> {
        values
            .chunks_exact(DISPLAY_LENGTH)
            .enumerate()
            .map(|(index, display)| DisplayInfo {
                display_id: display[0] as i32,
                name: names.get(index).cloned().unwrap_or_default(),
                rotation: display[1] as i32,
                has_window_metrics: display[2] != 0.0,
                bounds: PixelRect::new(display[3] as i32, display[4] as i32, display[5] as i32, display[6] as i32),
                scaling: f64::from(display[7]),
                orientation: display[8] as i32,
            })
            .collect()
    }

    /// The orientation of the display at rotation zero.
    pub(crate) fn natural_orientation(&self) -> ScreenOrientation {
        if !self.has_window_metrics {
            return ScreenOrientation::Portrait;
        }

        if self.orientation == android_orientation::SQUARE {
            ScreenOrientation::None
        } else if self.rotation == surface_orientation::ROTATION_0 || self.rotation == surface_orientation::ROTATION_180
        {
            if self.orientation == android_orientation::LANDSCAPE {
                ScreenOrientation::Landscape
            } else {
                ScreenOrientation::Portrait
            }
        } else if self.orientation == android_orientation::PORTRAIT {
            ScreenOrientation::Landscape
        } else {
            ScreenOrientation::Portrait
        }
    }

    pub(crate) fn current_orientation(&self) -> ScreenOrientation {
        use surface_orientation::{ROTATION_0, ROTATION_180, ROTATION_270, ROTATION_90};

        match (self.rotation, self.natural_orientation()) {
            (_, ScreenOrientation::None) => ScreenOrientation::None,
            (ROTATION_0, ScreenOrientation::Landscape) => ScreenOrientation::Landscape,
            (ROTATION_90, ScreenOrientation::Landscape) => ScreenOrientation::Portrait,
            (ROTATION_180, ScreenOrientation::Landscape) => ScreenOrientation::LandscapeFlipped,
            (ROTATION_270, ScreenOrientation::Landscape) => ScreenOrientation::PortraitFlipped,
            (ROTATION_0, _) => ScreenOrientation::Portrait,
            (ROTATION_90, _) => ScreenOrientation::Landscape,
            (ROTATION_180, _) => ScreenOrientation::PortraitFlipped,
            (ROTATION_270, _) => ScreenOrientation::LandscapeFlipped,
            _ => ScreenOrientation::Portrait,
        }
    }

    /// The working area: the bounds without the system bars of an empty set
    /// of insets, which is what the reference computes (it builds the
    /// insets it asks, so every inset is zero).
    pub(crate) fn working_area(&self) -> PixelRect {
        let (left, top, right, bottom) = (0, 0, 0, 0);
        PixelRect::new(
            self.bounds.x + left,
            self.bounds.y + top,
            self.bounds.width - (left + right),
            self.bounds.height - (top + bottom),
        )
    }
}

/// Where the screens come from.
pub(crate) trait IDisplaySource {
    /// The displays of the device now.
    fn displays(&self) -> Vec<DisplayInfo>;

    /// The id of the display a top-level is on; `None` when it is on none.
    fn display_of_top_level(&self, top_level: &dyn ITopLevelImpl) -> Option<i32>;
}

/// A screen of the device.
pub struct AndroidScreen {
    base: PlatformScreen,
    display_id: i32,
}

impl AndroidScreen {
    fn new(display_id: i32) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(PlatformHandle::new(display_id as isize, Some("DisplayId")))),
            display_id,
        }
    }

    /// The id of the display of the screen.
    pub fn display_id(&self) -> i32 {
        self.display_id
    }

    pub(crate) fn refresh(&self, display: &DisplayInfo) {
        self.base.set_display_name(Some(display.name.clone()));
        self.base.set_is_primary(display.display_id == DEFAULT_DISPLAY);

        if display.has_window_metrics {
            self.base.set_bounds(display.bounds);
            self.base.set_working_area(display.working_area());
            // The reference leaves the scaling alone when the context has no configuration.
            if display.scaling > 0.0 {
                self.base.set_scaling(display.scaling);
            }
        } else {
            self.base.set_scaling(display.scaling);
            self.base.set_bounds(display.bounds);
            self.base.set_working_area(display.bounds);
        }

        self.base.set_current_orientation(display.current_orientation());
    }
}

impl AsRef<PlatformScreen> for AndroidScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

/// The screens of the device, keyed by the id of their display.
pub struct AndroidScreens {
    base: ScreensBase<i32, AndroidScreen>,
    weak_self: Weak<AndroidScreens>,
    source: Box<dyn IDisplaySource>,
    /// The displays as the last enumeration found them, for the refresh of
    /// the screens that follows it.
    displays: RefCell<Vec<DisplayInfo>>,
}

impl AndroidScreens {
    pub(crate) fn with_source(source: Box<dyn IDisplaySource>) -> Rc<AndroidScreens> {
        Rc::new_cyclic(|weak_self| AndroidScreens {
            base: ScreensBase::new(),
            weak_self: weak_self.clone(),
            source,
            displays: RefCell::new(Vec::new()),
        })
    }

    /// The displays changed, or the configuration did.
    pub(crate) fn on_changed(&self) {
        if let Some(this) = self.weak_self.upgrade() {
            ScreensBaseImplExt::on_changed(&this);
        }
    }
}

impl ScreensBaseImpl for AndroidScreens {
    type Key = i32;
    type Screen = AndroidScreen;

    fn screens_base(&self) -> &ScreensBase<i32, AndroidScreen> {
        &self.base
    }

    fn get_all_screen_keys(&self) -> Vec<i32> {
        let displays = self.source.displays();
        let keys = displays.iter().map(|display| display.display_id).collect();
        *self.displays.borrow_mut() = displays;
        keys
    }

    fn create_screen_from_key(&self, key: &i32) -> Rc<AndroidScreen> {
        Rc::new(AndroidScreen::new(*key))
    }

    fn screen_changed(&self, screen: &Rc<AndroidScreen>) {
        let display = self.displays.borrow().iter().find(|display| display.display_id == screen.display_id).cloned();
        if let Some(display) = display {
            screen.refresh(&display);
        }
    }

    fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        let display = self.source.display_of_top_level(top_level)?;
        self.try_get_screen(&display).map(|screen| screen.base.screen().clone())
    }

    fn screen_from_point_core(&self, _point: PixelPoint) -> Option<Rc<Screen>> {
        None
    }

    fn screen_from_rect_core(&self, _rect: PixelRect) -> Option<Rc<Screen>> {
        None
    }
}

#[cfg(target_os = "android")]
mod imp {
    use super::{AndroidScreens, DisplayInfo, IDisplaySource};
    use crate::interop::java::{
        call_static_object, call_static_void, float_array_of, string_array_of, JavaClass, JavaObject, JavaValue,
    };
    use crate::interop::natives::{next_handle, PLATFORM_HELPER};
    use crate::platform::skia_platform::TopLevelImpl;
    use ferroui_controls::platform::ITopLevelImpl;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

    thread_local! {
        static SCREENS: RefCell<HashMap<i64, Weak<AndroidScreens>>> = RefCell::new(HashMap::new());
    }

    /// The displays of the display manager of a context, with the listener
    /// that reports their changes.
    struct ContextDisplaySource {
        context: JavaObject,
        handle: i64,
        listener: Option<JavaObject>,
    }

    impl IDisplaySource for ContextDisplaySource {
        fn displays(&self) -> Vec<DisplayInfo> {
            let helper = JavaClass::find(PLATFORM_HELPER);
            let context = [JavaValue::Object(Some(&self.context))];
            let values = call_static_object(&helper, "getDisplays", "(Landroid/content/Context;)[F", &context);
            let names = call_static_object(
                &helper,
                "getDisplayNames",
                "(Landroid/content/Context;)[Ljava/lang/String;",
                &context,
            );
            match (values, names) {
                (Some(values), Some(names)) => {
                    DisplayInfo::from_arrays(&float_array_of(&values), &string_array_of(&names))
                }
                _ => Vec::new(),
            }
        }

        fn display_of_top_level(&self, top_level: &dyn ITopLevelImpl) -> Option<i32> {
            top_level.as_any().downcast_ref::<TopLevelImpl>()?.display_id()
        }
    }

    impl Drop for ContextDisplaySource {
        fn drop(&mut self) {
            SCREENS.with(|screens| screens.borrow_mut().remove(&self.handle));
            if let Some(listener) = &self.listener {
                call_static_void(
                    &JavaClass::find(PLATFORM_HELPER),
                    "unregisterDisplayListener",
                    "(Landroid/content/Context;Ljava/lang/Object;)V",
                    &[JavaValue::Object(Some(&self.context)), JavaValue::Object(Some(listener))],
                );
            }
        }
    }

    impl AndroidScreens {
        /// The screens of the display manager of `context`, which follow
        /// its display listener.
        pub(crate) fn new(context: &JavaObject) -> Rc<AndroidScreens> {
            let handle = next_handle();
            let listener = call_static_object(
                &JavaClass::find(PLATFORM_HELPER),
                "registerDisplayListener",
                "(Landroid/content/Context;J)Ljava/lang/Object;",
                &[JavaValue::Object(Some(context)), JavaValue::Long(handle)],
            )
            .map(|listener| listener.to_global());
            let screens = AndroidScreens::with_source(Box::new(ContextDisplaySource {
                context: context.clone(),
                handle,
                listener,
            }));
            SCREENS.with(|all| all.borrow_mut().insert(handle, Rc::downgrade(&screens)));
            screens
        }

        /// The display listener registered with `handle` reported a change.
        pub(crate) fn on_displays_changed(handle: i64) {
            let screens = SCREENS.with(|all| all.borrow().get(&handle).and_then(Weak::upgrade));
            if let Some(screens) = screens {
                screens.on_changed();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the screens.
    use super::*;
    use ferroui_controls::platform::IScreenImpl;
    use std::cell::Cell;

    fn display(display_id: i32, rotation: i32, orientation: i32) -> DisplayInfo {
        DisplayInfo {
            display_id,
            name: format!("Display {display_id}"),
            rotation,
            has_window_metrics: true,
            bounds: PixelRect::new(0, 0, 1080, 2400),
            scaling: 2.625,
            orientation,
        }
    }

    struct Source {
        displays: Rc<RefCell<Vec<DisplayInfo>>>,
        top_level_display: Rc<Cell<Option<i32>>>,
    }

    impl IDisplaySource for Source {
        fn displays(&self) -> Vec<DisplayInfo> {
            self.displays.borrow().clone()
        }

        fn display_of_top_level(&self, _top_level: &dyn ITopLevelImpl) -> Option<i32> {
            self.top_level_display.get()
        }
    }

    #[test]
    fn the_arrays_of_the_java_layer_are_read_per_display() {
        let values = [
            0.0, 1.0, 1.0, 0.0, 0.0, 2400.0, 1080.0, 2.625, 2.0, //
            7.0, 0.0, 0.0, 0.0, 0.0, 1920.0, 1080.0, 1.0, 0.0,
        ];
        let displays = DisplayInfo::from_arrays(&values, &["Built-in".to_string()]);

        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0].display_id, 0);
        assert_eq!(displays[0].name, "Built-in");
        assert_eq!(displays[0].rotation, surface_orientation::ROTATION_90);
        assert!(displays[0].has_window_metrics);
        assert_eq!(displays[0].bounds, PixelRect::new(0, 0, 2400, 1080));
        assert_eq!(displays[0].scaling, 2.625);
        assert_eq!(displays[0].orientation, android_orientation::LANDSCAPE);
        assert_eq!(displays[1].display_id, 7);
        assert_eq!(displays[1].name, "");
        assert!(!displays[1].has_window_metrics);
    }

    #[test]
    fn a_phone_is_portrait_at_rotation_zero_and_landscape_when_turned() {
        use android_orientation::{LANDSCAPE, PORTRAIT};
        use surface_orientation::{ROTATION_0, ROTATION_180, ROTATION_270, ROTATION_90};

        // The configuration follows the rotation: a phone held upright is portrait.
        assert_eq!(display(0, ROTATION_0, PORTRAIT).current_orientation(), ScreenOrientation::Portrait);
        assert_eq!(display(0, ROTATION_90, LANDSCAPE).current_orientation(), ScreenOrientation::Landscape);
        assert_eq!(display(0, ROTATION_180, PORTRAIT).current_orientation(), ScreenOrientation::PortraitFlipped);
        assert_eq!(display(0, ROTATION_270, LANDSCAPE).current_orientation(), ScreenOrientation::LandscapeFlipped);
    }

    #[test]
    fn a_tablet_is_landscape_at_rotation_zero() {
        use android_orientation::{LANDSCAPE, PORTRAIT};
        use surface_orientation::{ROTATION_0, ROTATION_180, ROTATION_270, ROTATION_90};

        assert_eq!(display(0, ROTATION_0, LANDSCAPE).current_orientation(), ScreenOrientation::Landscape);
        assert_eq!(display(0, ROTATION_90, PORTRAIT).current_orientation(), ScreenOrientation::Portrait);
        assert_eq!(display(0, ROTATION_180, LANDSCAPE).current_orientation(), ScreenOrientation::LandscapeFlipped);
        assert_eq!(display(0, ROTATION_270, PORTRAIT).current_orientation(), ScreenOrientation::PortraitFlipped);
    }

    #[test]
    fn a_square_display_has_no_orientation_and_real_metrics_are_portrait() {
        assert_eq!(display(0, 1, android_orientation::SQUARE).current_orientation(), ScreenOrientation::None);

        let mut old = display(0, surface_orientation::ROTATION_90, 0);
        old.has_window_metrics = false;
        assert_eq!(old.natural_orientation(), ScreenOrientation::Portrait);
        assert_eq!(old.current_orientation(), ScreenOrientation::Landscape);
    }

    #[test]
    fn the_screens_follow_the_displays_of_the_source() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let displays = Rc::new(RefCell::new(vec![display(0, 0, android_orientation::PORTRAIT)]));
        let top_level_display = Rc::new(Cell::new(None));
        let screens = AndroidScreens::with_source(Box::new(Source {
            displays: displays.clone(),
            top_level_display: top_level_display.clone(),
        }));

        let all = screens.all_screens();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_primary());
        assert_eq!(all[0].display_name().as_deref(), Some("Display 0"));
        assert_eq!(all[0].bounds(), PixelRect::new(0, 0, 1080, 2400));
        assert_eq!(all[0].working_area(), PixelRect::new(0, 0, 1080, 2400));
        assert_eq!(all[0].scaling(), 2.625);
        assert_eq!(all[0].current_orientation(), ScreenOrientation::Portrait);

        // A second display appears and the first is turned.
        let changed = Rc::new(Cell::new(0));
        screens.set_changed(Some(Rc::new({
            let changed = changed.clone();
            move || changed.set(changed.get() + 1)
        })));
        {
            let mut displays = displays.borrow_mut();
            displays[0].rotation = surface_orientation::ROTATION_90;
            displays[0].orientation = android_orientation::LANDSCAPE;
            displays[0].bounds = PixelRect::new(0, 0, 2400, 1080);
            displays.push(display(3, 0, android_orientation::LANDSCAPE));
        }
        screens.on_changed();
        screens.on_changed();
        // The changes are gathered into one job of the dispatcher.
        assert_eq!(changed.get(), 0);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);

        assert_eq!(changed.get(), 1);
        let all = screens.all_screens();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].current_orientation(), ScreenOrientation::Landscape);
        assert_eq!(all[0].bounds(), PixelRect::new(0, 0, 2400, 1080));
        assert!(!all[1].is_primary());

        assert!(screens.screen_from_point(PixelPoint::new(10, 10)).is_none());
        assert!(screens.screen_from_rect(PixelRect::new(0, 0, 10, 10)).is_none());
    }
}
