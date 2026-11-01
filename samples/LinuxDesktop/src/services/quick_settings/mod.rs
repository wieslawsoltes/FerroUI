//! Reads and mutations performed off the UI thread. System-bus proxies are
//! short-lived and uncached. The closed panel owns no service subscriptions.
mod audio;
pub use audio::{Audio, Mixer};
use std::{fs, io, path::Path, time::Duration};
use zbus::blocking::{connection::Builder, proxy::Builder as ProxyBuilder, Connection, Proxy};
use zbus::proxy::CacheProperties;

const NETWORK: &str = "org.freedesktop.NetworkManager";
const NETWORK_PATH: &str = "/org/freedesktop/NetworkManager";
const POWER: &str = "org.freedesktop.UPower";
const POWER_PATH: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const LOGIN: &str = "org.freedesktop.login1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Wifi {
    pub enabled: bool,
    pub hardware_enabled: bool,
    pub connection: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Backlight {
    pub device: String,
    pub percent: u8,
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub audio: Option<Audio>,
    pub wifi: Option<Wifi>,
    pub battery: Option<String>,
    pub backlights: Vec<Backlight>,
    pub login_available: bool,
}

pub enum Change {
    Refresh,
    Mute(Mixer, bool),
    Volume(Mixer, bool),
    SetLevel(Mixer, u16),
    Wifi(bool),
    Brightness(String, bool),
}

fn system_bus() -> zbus::Result<Connection> {
    Builder::system()?
        .method_timeout(Duration::from_secs(3))
        .build()
}

fn proxy<'a>(
    bus: &'a Connection,
    name: &'static str,
    path: &'a str,
    interface: &'static str,
) -> zbus::Result<Proxy<'a>> {
    ProxyBuilder::new(bus)
        .destination(name)?
        .path(path)?
        .interface(interface)?
        .cache_properties(CacheProperties::No)
        .build()
}

fn wifi(bus: &Connection) -> Option<Wifi> {
    let nm = proxy(bus, NETWORK, NETWORK_PATH, NETWORK).ok()?;
    let enabled = nm.get_property("WirelessEnabled").ok()?;
    let hardware_enabled = nm.get_property("WirelessHardwareEnabled").ok()?;
    let connection = nm
        .get_property::<zbus::zvariant::OwnedObjectPath>("PrimaryConnection")
        .ok()
        .filter(|path| path.as_str() != "/")
        .and_then(|path| {
            proxy(
                bus,
                NETWORK,
                path.as_str(),
                "org.freedesktop.NetworkManager.Connection.Active",
            )
            .and_then(|active| active.get_property::<String>("Id"))
            .ok()
        })
        .unwrap_or_default();
    Some(Wifi {
        enabled,
        hardware_enabled,
        connection,
    })
}

fn battery(bus: &Connection) -> Option<String> {
    let device = proxy(bus, POWER, POWER_PATH, "org.freedesktop.UPower.Device").ok()?;
    if !device.get_property::<bool>("IsPresent").ok()? {
        return None;
    }
    let percent: f64 = device.get_property("Percentage").ok()?;
    let state = match device.get_property::<u32>("State").ok()? {
        1 => "Charging",
        2 => "Discharging",
        3 => "Empty",
        4 => "Fully charged",
        5 => "Waiting to charge",
        6 => "Waiting to discharge",
        _ => "Battery",
    };
    Some(format!("{state} · {:.0}%", percent.clamp(0.0, 100.0)))
}

fn read_brightness(root: &Path, device: &str) -> Option<(u32, u32)> {
    if Path::new(device).file_name()?.to_str()? != device {
        return None;
    }
    let directory = root.join(device);
    let read = |name| {
        fs::read_to_string(directory.join(name))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()
    };
    let max = read("max_brightness")?;
    if max == 0 {
        return None;
    }
    Some((read("brightness")?.min(max), max))
}

fn backlights(root: &Path) -> Vec<Backlight> {
    let mut items: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let device = entry.file_name().into_string().ok()?;
            let (value, max) = read_brightness(root, &device)?;
            Some(Backlight {
                device,
                percent: ((u64::from(value) * 100 + u64::from(max) / 2) / u64::from(max)) as u8,
            })
        })
        .collect();
    items.sort_by(|a, b| a.device.cmp(&b.device));
    items
}

fn next_level(current: u32, maximum: u32, increase: bool) -> u32 {
    let step = maximum.div_ceil(10);
    if increase {
        current.saturating_add(step).min(maximum)
    } else {
        current.saturating_sub(step).max(1)
    }
}

pub fn update(change: Change) -> (Snapshot, Option<String>) {
    let bus = system_bus().ok();
    let root = Path::new("/sys/class/backlight");
    let apply = || -> io::Result<()> {
        let connected = || {
            bus.as_ref()
                .ok_or_else(|| io::Error::other("System bus is unavailable"))
        };
        match change {
            Change::Refresh => Ok(()),
            Change::Mute(mixer, value) => mixer.set_mute(value),
            Change::Volume(mixer, increase) => mixer.step(increase),
            Change::SetLevel(mixer, value) => mixer.set_level(value),
            Change::Wifi(value) => proxy(connected()?, NETWORK, NETWORK_PATH, NETWORK)
                .map_err(io::Error::other)?
                .set_property("WirelessEnabled", value)
                .map_err(io::Error::other),
            Change::Brightness(ref device, increase) => {
                let (current, max) = read_brightness(root, device)
                    .ok_or_else(|| io::Error::other("Backlight device disconnected"))?;
                let level = next_level(current, max, increase);
                proxy(
                    connected()?,
                    LOGIN,
                    "/org/freedesktop/login1/session/auto",
                    "org.freedesktop.login1.Session",
                )
                .map_err(io::Error::other)?
                .call::<_, _, ()>("SetBrightness", &("backlight", device.as_str(), level))
                .map_err(io::Error::other)
            }
        }
    };
    let error = apply().err().map(|e| e.to_string());
    let backlights = backlights(root);
    let snapshot = Snapshot {
        wifi: bus.as_ref().and_then(wifi),
        battery: bus.as_ref().and_then(battery),
        audio: audio::default_output(),
        login_available: !backlights.is_empty()
            && bus.as_ref().is_some_and(|bus| {
                proxy(
                    bus,
                    LOGIN,
                    "/org/freedesktop/login1/session/auto",
                    "org.freedesktop.login1.Session",
                )
                .and_then(|session| session.get_property::<String>("Id"))
                .is_ok()
            }),
        backlights,
    };
    (snapshot, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn light_steps_handle_small_and_large_devices() {
        assert_eq!(next_level(1, 2, false), 1);
        assert_eq!(next_level(1, 2, true), 2);
        assert_eq!(next_level(2, 2, true), 2);
        assert_eq!(next_level(u32::MAX, u32::MAX, true), u32::MAX);
        assert_eq!(next_level(50, 100, false), 40);
    }
    #[test]
    fn stale_or_relative_device_ids_cannot_escape_sysfs() {
        assert!(read_brightness(Path::new("/sys/class/backlight"), "../brightness").is_none());
    }
}
