//! The screen type of the backend and the providers of the raw screen
//! information (the port of `X11Screen.Providers.cs`): the monitors of
//! RandR 1.5, and the root window as the one screen of a server without
//! them.
//!
//! The reference nests these types in its screens class; here they are the
//! items of this module.

use super::x11_screens_scaling::{get_scaling_provider, IScalingProvider};
use crate::dispatching::EventHandler;
use crate::event::Event;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{RandrEvent, RandrEventMask};
use crate::x11_window_info::X11WindowInfo;
use crate::xlib::{self, Atom, OutputMode, WindowProperty, XEvent, XGeometry, XID};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{PixelRect, Size};
use ferroui_controls::platform::{PlatformHandle, PlatformScreen};
use std::cell::{Cell, RefCell};
use std::ffi::{c_long, c_ulong};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// The flag of a mode that is interlaced (`RRModeFlags.RR_Interlace`).
const RR_INTERLACE: c_ulong = 0x0000_0010;
/// The flag of a mode that scans every line twice
/// (`RRModeFlags.RR_DoubleScan`).
const RR_DOUBLE_SCAN: c_ulong = 0x0000_0020;

/// A screen of the X11 backend, identified by the atom of the name of its
/// RandR monitor.
///
/// The reference derives the screen of the fallback provider
/// (`FallBackScreen`) from this class, with a refresh that does nothing;
/// here it is the same type, made by [`X11Screen::new_fall_back`]: such a
/// screen has no scaling provider, without which a refresh does nothing.
pub struct X11Screen {
    base: PlatformScreen,
    physical_size: Cell<Option<Size>>,
    x11: Rc<X11Info>,
    scaling_provider: Option<Rc<dyn IScalingProvider>>,
    id: i32,
}

impl X11Screen {
    pub fn new(
        info: &MonitorInfo,
        x11: Rc<X11Info>,
        scaling_provider: Option<Rc<dyn IScalingProvider>>,
        id: i32,
    ) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(PlatformHandle::new(info.name as isize, Some("XRandRMonitorName")))),
            physical_size: Cell::new(None),
            x11,
            scaling_provider,
            id,
        }
    }

    /// The screen of a server without monitors: the root window
    /// (`FallBackScreen`).
    pub fn new_fall_back(pixel_rect: PixelRect, x11: Rc<X11Info>) -> Self {
        let this = Self::new(&MonitorInfo::default(), x11, None, 0);
        this.base.set_bounds(pixel_rect);
        this.base.set_display_name(Some("Default".to_string()));
        this.base.set_is_primary(true);
        this.physical_size.set(Some(pixel_rect.size().to_size(this.base.scaling())));
        this.update_work_area();
        this
    }

    /// The size of the screen in millimetres, when it is known.
    pub fn physical_size(&self) -> Option<Size> {
        self.physical_size.get()
    }

    pub fn set_physical_size(&self, value: Option<Size>) {
        self.physical_size.set(value);
    }

    /// Takes the values of the monitor as they are now.
    pub fn refresh(&self, new_info: &MonitorInfo) {
        let Some(scaling_provider) = self.scaling_provider.clone() else {
            return;
        };

        let name = xlib::get_atom_name(self.x11.display(), new_info.name);
        self.base.set_display_name(name);
        self.base.set_is_primary(new_info.is_primary);
        self.base.set_bounds(PixelRect::new(new_info.x, new_info.y, new_info.width, new_info.height));
        self.physical_size.set(new_info.physical_size);
        self.update_work_area();
        self.base.set_scaling(scaling_provider.get_scaling(self, self.id));
    }

    fn update_work_area(&self) {
        let property = xlib::x_get_window_property(
            self.x11.display(),
            self.x11.root_window(),
            self.x11.atoms()._NET_WORKAREA,
            0,
            128,
            false,
            xlib::ANY_PROPERTY_TYPE,
        );
        self.base.set_working_area(working_area(self.base.bounds(), &property));
    }
}

impl Deref for X11Screen {
    type Target = PlatformScreen;

    fn deref(&self) -> &PlatformScreen {
        &self.base
    }
}

impl AsRef<PlatformScreen> for X11Screen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

/// The working area of a screen with the bounds `bounds`, given the
/// `_NET_WORKAREA` property of the root window: the part of the bounds
/// inside the work area of the first desktop, and the bounds themselves
/// (the fallback value) when the property is missing, was not read to its
/// end, does not have four items for every desktop, or leaves nothing of
/// the bounds.
///
/// The reference reads the first four items without looking at their
/// number or format, which is a read of memory it does not own for a
/// property with fewer; such a property is the fallback value here.
///
/// # Panics
/// Panics when an item does not fit 32 bits, as the conversion of the
/// reference throws.
pub fn working_area(bounds: PixelRect, property: &WindowProperty) -> PixelRect {
    if property.status != 0
        || property.actual_type == 0
        || property.actual_format == 0
        || property.bytes_after != 0
        || property.nitems % 4 != 0
    {
        return bounds;
    }

    let pwa = property.longs();
    if pwa.len() < 4 {
        return bounds;
    }
    let to_i32 =
        |item: c_ulong| i32::try_from(item as c_long).expect("Arithmetic operation resulted in an overflow.");
    let wa = PixelRect::new(to_i32(pwa[0]), to_i32(pwa[1]), to_i32(pwa[2]), to_i32(pwa[3]));

    let working_area = bounds.intersect(wa);
    if working_area.width <= 0 || working_area.height <= 0 {
        return bounds;
    }
    working_area
}

/// Provides the raw information about the screens of the server
/// (`IX11RawScreenInfoProvider`).
pub trait IX11RawScreenInfoProvider {
    /// The keys of the screens: the atoms of the names of the monitors.
    fn screen_keys(&self) -> Vec<Atom>;

    /// Adds a handler to the event that says the screens changed
    /// (`Changed += handler`).
    fn subscribe_changed(&self, handler: Rc<dyn Fn()>);

    fn create_screen_from_key(&self, key: Atom) -> X11Screen;

    fn get_monitor_info_by_key(&self, key: Atom) -> MonitorInfo;

    /// The provider as one that knows refresh rates, when it is one (the
    /// reference tests for the interface).
    fn as_provider_with_refresh_rate(&self) -> Option<&dyn IX11RawScreenInfoProviderWithRefreshRate> {
        None
    }
}

/// A provider that knows the refresh rates of the screens
/// (`IX11RawScreenInfoProviderWithRefreshRate`).
pub trait IX11RawScreenInfoProviderWithRefreshRate: IX11RawScreenInfoProvider {
    fn max_refresh_rate(&self) -> i32;
}

/// What a provider knows about a monitor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MonitorInfo {
    /// The atom of the name of the monitor.
    pub name: Atom,
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub refresh_rate: i32,
    /// The size in millimetres.
    pub physical_size: Option<Size>,
    /// The lowest refresh rate of the outputs of the monitor.
    pub shared_refresh_rate: i32,
}

/// Creates a window that is never mapped and whose events go to `handler`
/// (`XLib.CreateEventWindow`).
fn create_event_window(platform: &FerroX11Platform, handler: EventHandler) -> XID {
    let win =
        xlib::x_create_simple_window(platform.display(), platform.info().default_root_window(), 0, 0, 1, 1, 0, 0, 0);
    platform.set_window(win, X11WindowInfo::new(handler, None));
    win
}

/// The vertical refresh rate of a mode in hertz, rounded to the nearest
/// integer (to the even one from a half): the dot clock over the total
/// size of a frame, doubled for an interlaced mode and halved for a mode
/// that scans every line twice. `None` for a mode without a dot clock or
/// without a total size.
pub fn refresh_rate_of_mode(dot_clock: u64, h_total: u32, v_total: u32, mode_flags: u64) -> Option<i32> {
    let mut multiplier = 1.0_f64;
    if mode_flags & RR_INTERLACE as u64 != 0 {
        multiplier *= 2.0;
    }
    if mode_flags & RR_DOUBLE_SCAN as u64 != 0 {
        multiplier /= 2.0;
    }
    if h_total == 0 || v_total == 0 || dot_clock == 0 {
        return None;
    }
    let hz = dot_clock as f64 / (h_total as f64 * v_total as f64) * multiplier;
    Some(hz.round_ties_even() as i32)
}

/// The refresh rate of an output: `None` when the output has no controller
/// that shows a mode (`modes` holds the mode of every output that has
/// one), or when the rate of its mode cannot be computed.
pub fn refresh_rate_for_output(modes: &[OutputMode], output: XID) -> Option<i32> {
    let mode = modes.iter().find(|mode| mode.output == output)?;
    refresh_rate_of_mode(mode.dot_clock as u64, mode.h_total, mode.v_total, mode.mode_flags as u64)
}

/// The refresh rate the outputs of a monitor share: the lowest of their
/// rates, and the default rate when none of them has one.
pub fn shared_refresh_rate_for_outputs(modes: &[OutputMode], outputs: &[XID]) -> i32 {
    let mut min_rate: Option<i32> = None;
    for output in outputs {
        let rate = refresh_rate_for_output(modes, *output);
        if let Some(rate) = rate {
            min_rate = Some(match min_rate {
                Some(min_rate) => min_rate.min(rate),
                None => rate,
            });
        }
    }

    min_rate.unwrap_or(FerroX11Platform::DEFAULT_FPS)
}

/// The highest of the refresh rates of the monitors, and the default rate
/// when there is no monitor.
pub fn max_shared_refresh_rate(monitors: &[MonitorInfo]) -> i32 {
    monitors.iter().map(|x| x.shared_refresh_rate).max().unwrap_or(FerroX11Platform::DEFAULT_FPS)
}

/// The length of an EDID block in bytes. The reference asks the server for
/// 32 units of four bytes of the property.
const EDID_STRUCTURE_LENGTH: usize = 32 * 4;

/// The physical size in millimetres an EDID block states, `None` when the
/// block is too short or states no size.
///
/// # Panics
/// Panics for a block of exactly 22 bytes: the reference accepts that
/// length and then reads the byte at index 22, which throws.
pub fn physical_monitor_size_from_edid_bytes(edid: &[u8]) -> Option<Size> {
    if edid.len() < 22 {
        return None;
    }
    let width = edid[21]; // 0x15 1 Max. Horizontal Image Size cm.
    let height = edid[22]; // 0x16 1 Max. Vertical Image Size cm.
    if width == 0 && height == 0 {
        return None;
    }
    Some(Size::new(f64::from(width) * 10.0, f64::from(height) * 10.0))
}

/// The physical size the `EDID` property of an output states: `None` when
/// the property is not an array of bytes of the type `INTEGER` (`integer`
/// is that atom), or does not state a size.
pub fn physical_monitor_size_from_edid_property(property: &WindowProperty, integer: Atom) -> Option<Size> {
    if property.actual_type != integer {
        return None;
    }
    if property.actual_format != 8 {
        // Expecting a byte array
        return None;
    }

    // Length of an EDID block (128 bytes): the reference reads no more of
    // the property.
    let length = property.data.len().min(EDID_STRUCTURE_LENGTH);
    physical_monitor_size_from_edid_bytes(&property.data[..length])
}

/// The monitors of RandR 1.5, with the refresh rates and the physical
/// sizes of their outputs.
pub struct Randr15ScreensImpl {
    this: Weak<Randr15ScreensImpl>,
    cache: RefCell<Option<Rc<[MonitorInfo]>>>,
    x11: Rc<X11Info>,
    window: XID,
    scaling_provider: Rc<dyn IScalingProvider>,
    changed: Event,
}

impl Randr15ScreensImpl {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let x11 = platform.info().clone();
        let this = Rc::new_cyclic(|weak: &Weak<Self>| {
            let handler_weak = weak.clone();
            let window = create_event_window(
                platform,
                Rc::new(move |ev: &mut XEvent| {
                    if let Some(this) = handler_weak.upgrade() {
                        this.on_event(ev);
                    }
                }),
            );
            Self {
                this: weak.clone(),
                cache: RefCell::new(None),
                x11: x11.clone(),
                window,
                scaling_provider: get_scaling_provider(platform),
                changed: Event::new(),
            }
        });
        xlib::xrr_select_input(
            x11.display(),
            this.window,
            (RandrEventMask::RR_SCREEN_CHANGE_NOTIFY
                | RandrEventMask::RR_OUTPUT_CHANGE_NOTIFY_MASK
                | RandrEventMask::RR_OUTPUT_PROPERTY_NOTIFY_MASK
                | RandrEventMask::RR_CRTC_CHANGE_NOTIFY_MASK)
                .bits(),
        );

        if let Some(settings_changed) = this.scaling_provider.settings_changed() {
            let weak = Rc::downgrade(&this);
            settings_changed.subscribe(move |()| {
                if let Some(this) = weak.upgrade() {
                    this.changed.raise();
                }
            });
        }

        let weak = Rc::downgrade(&this);
        platform.globals().root_property_changed.subscribe(move |atom| {
            if let Some(this) = weak.upgrade() {
                this.on_root_property_changed(atom);
            }
        });
        this
    }

    /// Raises the changed event from a job of the dispatcher.
    fn post_changed(&self) {
        let weak = self.this.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.changed.raise();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn on_root_property_changed(&self, atom: Atom) {
        if atom == self.x11.atoms()._NET_WORKAREA {
            self.post_changed();
        }
    }

    fn on_event(&self, ev: &XEvent) {
        let event = xlib::event_type(ev) - self.x11.randr_event_base();
        if event == RandrEvent::RRScreenChangeNotify as i32 || event == RandrEvent::RRNotify as i32 {
            let previous = self.cache.borrow_mut().take();
            drop(previous);
            // Delay triggering the update event
            self.post_changed();
        }
    }

    fn monitor_infos(&self) -> Rc<[MonitorInfo]> {
        if let Some(cache) = self.cache.borrow().as_ref() {
            return cache.clone();
        }
        let display = self.x11.display();
        let monitors = xlib::xrr_get_monitors(display, self.window);
        // The mode of every output of the screen resources whose
        // controller shows one: what the reference reads output by output
        // from the resources it gets here.
        let modes = xlib::xrr_get_output_modes(display, self.window);

        let screens: Rc<[MonitorInfo]> = monitors
            .iter()
            .map(|mon| MonitorInfo {
                name: mon.name,
                is_primary: mon.primary,
                x: mon.x,
                y: mon.y,
                width: mon.width,
                height: mon.height,
                refresh_rate: 0,
                physical_size: self.get_physical_monitor_size_from_first_eligible_output(&mon.outputs),
                shared_refresh_rate: shared_refresh_rate_for_outputs(&modes, &mon.outputs),
            })
            .collect();

        *self.cache.borrow_mut() = Some(screens.clone());
        screens
    }

    fn get_physical_monitor_size_from_first_eligible_output(&self, outputs: &[XID]) -> Option<Size> {
        outputs.iter().find_map(|output| self.get_physical_monitor_size_from_edid(*output))
    }

    fn get_physical_monitor_size_from_edid(&self, rr_output: XID) -> Option<Size> {
        if rr_output == 0 {
            return None;
        }
        let atoms = self.x11.atoms();
        let properties = xlib::xrr_list_output_properties_as_array(self.x11.display(), rr_output);
        let has_edid = properties.iter().any(|property| *property == atoms.EDID);

        if !has_edid {
            return None;
        }

        let property = xlib::xrr_get_output_property(self.x11.display(), rr_output, atoms.EDID);
        physical_monitor_size_from_edid_property(&property, atoms.INTEGER)
    }
}

impl IX11RawScreenInfoProvider for Randr15ScreensImpl {
    fn screen_keys(&self) -> Vec<Atom> {
        self.monitor_infos().iter().map(|x| x.name).collect()
    }

    fn subscribe_changed(&self, handler: Rc<dyn Fn()>) {
        self.changed.subscribe(move |()| handler());
    }

    /// # Panics
    /// Panics when no monitor has the key.
    fn create_screen_from_key(&self, key: Atom) -> X11Screen {
        let infos = self.monitor_infos();
        for (i, info) in infos.iter().enumerate() {
            if info.name == key {
                return X11Screen::new(info, self.x11.clone(), Some(self.scaling_provider.clone()), i as i32);
            }
        }

        panic!("Specified argument was out of the range of valid values. (Parameter 'key')");
    }

    /// # Panics
    /// Panics when no monitor has the key.
    fn get_monitor_info_by_key(&self, key: Atom) -> MonitorInfo {
        let infos = self.monitor_infos();
        for info in infos.iter() {
            if info.name == key {
                return *info;
            }
        }

        panic!("Specified argument was out of the range of valid values. (Parameter 'key')");
    }

    fn as_provider_with_refresh_rate(&self) -> Option<&dyn IX11RawScreenInfoProviderWithRefreshRate> {
        Some(self)
    }
}

impl IX11RawScreenInfoProviderWithRefreshRate for Randr15ScreensImpl {
    fn max_refresh_rate(&self) -> i32 {
        max_shared_refresh_rate(&self.monitor_infos())
    }
}

/// The root window as the one screen of a server without RandR 1.5.
pub struct FallbackScreensImpl {
    info: Rc<X11Info>,
    geo: Cell<XGeometry>,
}

impl FallbackScreensImpl {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let this = Rc::new(Self { info: platform.info().clone(), geo: Cell::new(XGeometry::default()) });
        if this.update_root_window_geometry() {
            let weak = Rc::downgrade(&this);
            platform.globals().root_geometry_changed_changed.subscribe(move |()| {
                if let Some(this) = weak.upgrade() {
                    this.update_root_window_geometry();
                }
            });
        }
        this
    }

    /// Reads the geometry of the root window; a geometry that cannot be
    /// read leaves the one that is known.
    fn update_root_window_geometry(&self) -> bool {
        match xlib::x_get_geometry(self.info.display(), self.info.root_window()) {
            Some(geo) => {
                self.geo.set(geo);
                true
            }
            None => false,
        }
    }
}

impl IX11RawScreenInfoProvider for FallbackScreensImpl {
    fn screen_keys(&self) -> Vec<Atom> {
        vec![0]
    }

    /// The screens of this provider never change: the handler is not kept
    /// (the event of the reference has empty accessors).
    fn subscribe_changed(&self, _handler: Rc<dyn Fn()>) {}

    fn create_screen_from_key(&self, _key: Atom) -> X11Screen {
        let geo = self.geo.get();
        X11Screen::new_fall_back(PixelRect::new(0, 0, geo.width, geo.height), self.info.clone())
    }

    fn get_monitor_info_by_key(&self, _key: Atom) -> MonitorInfo {
        MonitorInfo::default()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn work_area_property(items: &[c_long]) -> WindowProperty {
        WindowProperty {
            status: 0,
            actual_type: 6,
            actual_format: 32,
            nitems: items.len() as c_ulong,
            bytes_after: 0,
            data: items.iter().flat_map(|item| item.to_ne_bytes()).collect(),
        }
    }

    fn mode(output: XID, dot_clock: c_ulong, h_total: u32, v_total: u32, mode_flags: c_ulong) -> OutputMode {
        OutputMode { output, dot_clock, h_total, v_total, mode_flags }
    }

    #[test]
    fn the_working_area_is_the_part_of_the_bounds_in_the_work_area() {
        let bounds = PixelRect::new(0, 0, 1920, 1080);
        assert_eq!(working_area(bounds, &work_area_property(&[0, 32, 1920, 1048])), PixelRect::new(0, 32, 1920, 1048));
        // The work area spans all monitors; a monitor gets its part.
        let second = PixelRect::new(1920, 0, 1280, 1024);
        assert_eq!(working_area(second, &work_area_property(&[0, 32, 3200, 1048])), PixelRect::new(1920, 32, 1280, 992));
        // Only the first desktop is looked at.
        assert_eq!(
            working_area(bounds, &work_area_property(&[64, 0, 1856, 1080, 0, 0, 10, 10])),
            PixelRect::new(64, 0, 1856, 1080)
        );
    }

    #[test]
    fn a_work_area_outside_of_the_bounds_is_the_bounds() {
        let bounds = PixelRect::new(1920, 0, 1280, 1024);
        assert_eq!(working_area(bounds, &work_area_property(&[0, 0, 1920, 1080])), bounds);
        assert_eq!(working_area(bounds, &work_area_property(&[0, 0, 0, 0])), bounds);
        // Bounds of no size have no part in any work area.
        let empty = PixelRect::default();
        assert_eq!(working_area(empty, &work_area_property(&[0, 0, 1920, 1080])), empty);
    }

    #[test]
    fn a_missing_or_malformed_work_area_is_the_bounds() {
        let bounds = PixelRect::new(0, 0, 1920, 1080);
        let good = work_area_property(&[0, 32, 1920, 1048]);
        // The property does not exist.
        assert_eq!(working_area(bounds, &WindowProperty::default()), bounds);
        // The call failed.
        assert_eq!(working_area(bounds, &WindowProperty { status: 1, ..good.clone() }), bounds);
        assert_eq!(working_area(bounds, &WindowProperty { actual_type: 0, ..good.clone() }), bounds);
        assert_eq!(working_area(bounds, &WindowProperty { actual_format: 0, ..good.clone() }), bounds);
        // More of the property was left on the server.
        assert_eq!(working_area(bounds, &WindowProperty { bytes_after: 4, ..good.clone() }), bounds);
        // Not four items for every desktop.
        assert_eq!(working_area(bounds, &work_area_property(&[0, 32, 1920])), bounds);
        assert_eq!(working_area(bounds, &work_area_property(&[0, 32, 1920, 1048, 0])), bounds);
        // No items at all.
        assert_eq!(working_area(bounds, &work_area_property(&[])), bounds);
        // Four items of one byte are not four items of the work area.
        let bytes = WindowProperty { actual_format: 8, nitems: 4, data: vec![0, 32, 100, 100], ..good };
        assert_eq!(working_area(bounds, &bytes), bounds);
    }

    // Where a long has 32 bits every item fits.
    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "overflow")]
    fn an_item_of_the_work_area_beyond_32_bits_fails() {
        let item: c_long = 1 << 40;
        working_area(PixelRect::new(0, 0, 1920, 1080), &work_area_property(&[0, 0, item, 1080]));
    }

    #[test]
    fn refresh_rates_of_modes() {
        // 1920x1080 at 60 Hz: 148.5 MHz over 2200 by 1125.
        assert_eq!(refresh_rate_of_mode(148_500_000, 2200, 1125, 0), Some(60));
        // 59.94 Hz rounds to 60.
        assert_eq!(refresh_rate_of_mode(148_350_000, 2200, 1125, 0), Some(60));
        // 2560x1440 at 143.9 Hz.
        assert_eq!(refresh_rate_of_mode(586_000_000, 2720, 1497, 0), Some(144));
        // An interlaced mode shows two fields for a frame.
        assert_eq!(refresh_rate_of_mode(74_250_000, 2200, 1125, RR_INTERLACE as u64), Some(60));
        // A mode that scans every line twice shows half the lines.
        assert_eq!(refresh_rate_of_mode(25_175_000, 800, 449, RR_DOUBLE_SCAN as u64), Some(35));
        // Both flags cancel each other; other flags do not matter.
        assert_eq!(
            refresh_rate_of_mode(148_500_000, 2200, 1125, (RR_INTERLACE | RR_DOUBLE_SCAN | 0x1 | 0x4) as u64),
            Some(60)
        );
    }

    #[test]
    fn a_rate_of_a_half_rounds_to_the_even_integer() {
        // 60.5 Hz and 61.5 Hz.
        assert_eq!(refresh_rate_of_mode(121, 2, 1, 0), Some(60));
        assert_eq!(refresh_rate_of_mode(123, 2, 1, 0), Some(62));
    }

    #[test]
    fn a_mode_without_timings_has_no_refresh_rate() {
        assert_eq!(refresh_rate_of_mode(0, 2200, 1125, 0), None);
        assert_eq!(refresh_rate_of_mode(148_500_000, 0, 1125, 0), None);
        assert_eq!(refresh_rate_of_mode(148_500_000, 2200, 0, 0), None);
        assert_eq!(refresh_rate_of_mode(0, 0, 0, RR_INTERLACE as u64), None);
    }

    #[test]
    fn the_outputs_of_a_monitor_share_their_lowest_rate() {
        let modes = [
            mode(10, 148_500_000, 2200, 1125, 0),
            mode(11, 586_000_000, 2720, 1497, 0),
            mode(12, 0, 0, 0, 0),
        ];
        assert_eq!(refresh_rate_for_output(&modes, 10), Some(60));
        assert_eq!(refresh_rate_for_output(&modes, 11), Some(144));
        assert_eq!(refresh_rate_for_output(&modes, 12), None);
        assert_eq!(refresh_rate_for_output(&modes, 13), None);

        assert_eq!(shared_refresh_rate_for_outputs(&modes, &[11]), 144);
        assert_eq!(shared_refresh_rate_for_outputs(&modes, &[11, 10]), 60);
        // Outputs without a rate do not count.
        assert_eq!(shared_refresh_rate_for_outputs(&modes, &[12, 11, 13]), 144);
    }

    #[test]
    fn outputs_without_a_rate_share_the_default_rate() {
        let modes = [mode(12, 0, 0, 0, 0)];
        assert_eq!(shared_refresh_rate_for_outputs(&modes, &[]), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(shared_refresh_rate_for_outputs(&modes, &[12, 13]), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(shared_refresh_rate_for_outputs(&[], &[10]), FerroX11Platform::DEFAULT_FPS);
    }

    #[test]
    fn the_highest_rate_of_the_monitors() {
        let monitor = |shared_refresh_rate| MonitorInfo { shared_refresh_rate, ..MonitorInfo::default() };
        assert_eq!(max_shared_refresh_rate(&[]), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(max_shared_refresh_rate(&[monitor(60), monitor(144), monitor(75)]), 144);
        // Below the default rate: the screens class raises it.
        assert_eq!(max_shared_refresh_rate(&[monitor(30)]), 30);
    }

    fn edid(width_cm: u8, height_cm: u8, length: usize) -> Vec<u8> {
        let mut edid = vec![0u8; length];
        edid[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
        edid[21] = width_cm;
        if length > 22 {
            edid[22] = height_cm;
        }
        edid
    }

    #[test]
    fn the_physical_size_of_an_edid_block_is_in_millimetres() {
        assert_eq!(physical_monitor_size_from_edid_bytes(&edid(60, 34, 128)), Some(Size::new(600.0, 340.0)));
        assert_eq!(physical_monitor_size_from_edid_bytes(&edid(255, 255, 128)), Some(Size::new(2550.0, 2550.0)));
        // One of the two is enough.
        assert_eq!(physical_monitor_size_from_edid_bytes(&edid(0, 34, 128)), Some(Size::new(0.0, 340.0)));
        assert_eq!(physical_monitor_size_from_edid_bytes(&edid(60, 0, 23)), Some(Size::new(600.0, 0.0)));
    }

    #[test]
    fn an_edid_block_without_a_size_or_too_short_has_no_physical_size() {
        // A projector states no size.
        assert_eq!(physical_monitor_size_from_edid_bytes(&edid(0, 0, 128)), None);
        assert_eq!(physical_monitor_size_from_edid_bytes(&[]), None);
        assert_eq!(physical_monitor_size_from_edid_bytes(&[0u8; 21]), None);
    }

    #[test]
    #[should_panic]
    fn an_edid_block_of_22_bytes_fails() {
        physical_monitor_size_from_edid_bytes(&edid(60, 0, 22));
    }

    #[test]
    fn the_edid_property_is_an_array_of_bytes_of_the_integer_type() {
        const INTEGER: Atom = 19;
        let property = |actual_type, actual_format, data: Vec<u8>| WindowProperty {
            status: 0,
            actual_type,
            actual_format,
            nitems: data.len() as c_ulong,
            bytes_after: 0,
            data,
        };
        let size = Some(Size::new(600.0, 340.0));
        assert_eq!(physical_monitor_size_from_edid_property(&property(INTEGER, 8, edid(60, 34, 128)), INTEGER), size);
        // A block with extension blocks: the first block is read.
        assert_eq!(physical_monitor_size_from_edid_property(&property(INTEGER, 8, edid(60, 34, 256)), INTEGER), size);
        // Another type, another format.
        assert_eq!(physical_monitor_size_from_edid_property(&property(31, 8, edid(60, 34, 128)), INTEGER), None);
        assert_eq!(physical_monitor_size_from_edid_property(&property(INTEGER, 32, edid(60, 34, 128)), INTEGER), None);
        // The property could not be read.
        assert_eq!(physical_monitor_size_from_edid_property(&WindowProperty::default(), INTEGER), None);
        assert_eq!(physical_monitor_size_from_edid_property(&property(INTEGER, 8, Vec::new()), INTEGER), None);
    }
}
