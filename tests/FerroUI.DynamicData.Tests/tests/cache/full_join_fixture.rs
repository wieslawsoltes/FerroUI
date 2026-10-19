//! `FullJoinFixture` (`DynamicData.Tests/Cache/FullJoinFixture.cs`): the devices and the metadata
//! joined by name, with either side missing.

use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
use std::fmt;
use std::rc::Rc;

/// `Device`: equal by name.
struct Device {
    name: String,
}

impl Device {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_string() })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.name == other.name
    }
}

impl fmt::Debug for Device {
    /// `ToString`: `{Name}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// `DeviceMetaData`: equal by name and `IsAutoConnect`.
struct DeviceMetaData {
    is_auto_connect: bool,
    name: String,
}

impl DeviceMetaData {
    /// `new DeviceMetaData(string name, bool isAutoConnect = false)`.
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { is_auto_connect: false, name: name.to_string() })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl PartialEq for DeviceMetaData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || (self.name == other.name && self.is_auto_connect == other.is_auto_connect)
    }
}

impl fmt::Debug for DeviceMetaData {
    /// `ToString`: `Metadata: {Name}. IsAutoConnect = {IsAutoConnect}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Metadata: {}. IsAutoConnect = {}", self.name, if self.is_auto_connect { "True" } else { "False" })
    }
}

/// `DeviceWithMetadata`: equal by key.
struct DeviceWithMetadata {
    device: Option<Rc<Device>>,
    key: String,
    meta_data: Option<Rc<DeviceMetaData>>,
}

impl DeviceWithMetadata {
    fn new(key: String, device: Option<Rc<Device>>, meta_data: Option<Rc<DeviceMetaData>>) -> Rc<Self> {
        Rc::new(Self { device, key, meta_data })
    }

    fn device(&self) -> Option<Rc<Device>> {
        self.device.clone()
    }

    fn meta_data(&self) -> Option<Rc<DeviceMetaData>> {
        self.meta_data.clone()
    }
}

impl PartialEq for DeviceWithMetadata {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.key == other.key
    }
}

impl fmt::Debug for DeviceWithMetadata {
    /// `ToString`: `{Key}: {Device} ({MetaData})`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?} ({:?})", self.key, self.device, self.meta_data)
    }
}

/// The fixture: the two sources and the aggregator of the join, disposed with it (`Dispose`).
struct Fixture {
    left: SourceCache<Rc<Device>, String>,
    result: ChangeSetAggregator<Rc<DeviceWithMetadata>, String>,
    right: SourceCache<Rc<DeviceMetaData>, String>,
}

impl Fixture {
    fn new() -> Self {
        let left = SourceCache::new(|device: &Rc<Device>| device.name());
        let right = SourceCache::new(|device: &Rc<DeviceMetaData>| device.name());

        let result = left
            .connect()
            .full_join_with_key(right.connect(), |meta: &Rc<DeviceMetaData>| meta.name(), |key: &String, device: Option<&Rc<Device>>, meta: Option<&Rc<DeviceMetaData>>| {
                DeviceWithMetadata::new(key.clone(), device.cloned(), meta.cloned())
            })
            .as_aggregator();
        Self { left, result, right }
    }

    /// The edit of the left side that adds `Device1`, `Device2` and `Device3`.
    fn add_devices(&self) {
        self.left.edit(|inner_cache| {
            inner_cache.add_or_update(Device::new("Device1"));
            inner_cache.add_or_update(Device::new("Device2"));
            inner_cache.add_or_update(Device::new("Device3"));
        });
    }

    /// The edit of the right side that adds the metadata of `Device1`, `Device2` and `Device3`.
    fn add_meta_data(&self) {
        self.right.edit(|inner_cache| {
            inner_cache.add_or_update(DeviceMetaData::new("Device1"));
            inner_cache.add_or_update(DeviceMetaData::new("Device2"));
            inner_cache.add_or_update(DeviceMetaData::new("Device3"));
        });
    }

    fn has_result(&self, key: &str) -> bool {
        self.result.data().lookup(&key.to_string()).is_some()
    }

    fn items(&self) -> Vec<Rc<DeviceWithMetadata>> {
        self.result.data().items()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.left.dispose();
        self.right.dispose();
        self.result.dispose();
    }
}

#[test]
fn add_left_only() {
    let fixture = Fixture::new();
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.has_result("Device1"));
    assert!(fixture.has_result("Device2"));
    assert!(fixture.has_result("Device3"));

    assert!(fixture.items().iter().all(|dwm| dwm.meta_data().is_none()));
    assert!(fixture.items().iter().all(|dwm| dwm.device().is_some()));
}

#[test]
fn add_let_then_right() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data();

    assert_eq!(fixture.result.data().count(), 3);

    assert!(fixture.items().iter().all(|dwm| dwm.meta_data().is_some()));
}

#[test]
fn add_right_only() {
    let fixture = Fixture::new();
    fixture.add_meta_data();

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.has_result("Device1"));
    assert!(fixture.has_result("Device2"));
    assert!(fixture.has_result("Device3"));
    assert!(fixture.items().iter().all(|dwm| dwm.meta_data().is_some()));
    assert!(fixture.items().iter().all(|dwm| dwm.device().is_none()));
}

#[test]
fn add_right_then_left() {
    let fixture = Fixture::new();
    fixture.add_meta_data();
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 3);

    assert!(fixture.items().iter().all(|dwm| dwm.meta_data().is_some()));
}

#[test]
fn remove_various() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data();
    assert!(fixture.has_result("Device1"));
    assert!(fixture.has_result("Device2"));
    assert!(fixture.has_result("Device3"));

    fixture.right.remove_key("Device3".to_string());

    assert_eq!(fixture.result.data().count(), 3);
    assert_eq!(fixture.items().iter().filter(|dwm| dwm.meta_data().is_some()).count(), 2);

    fixture.left.remove_key("Device1".to_string());
    assert!(fixture.has_result("Device1"));
    assert!(fixture.has_result("Device2"));
    assert!(fixture.has_result("Device3"));
}

#[test]
fn update_right() {
    let fixture = Fixture::new();
    fixture.add_meta_data();
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 3);

    assert!(fixture.items().iter().all(|dwm| dwm.meta_data().is_some()));
}
