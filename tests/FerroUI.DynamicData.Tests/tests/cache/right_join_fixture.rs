//! `RightJoinFixture` (`DynamicData.Tests/Cache/RightJoinFixture.cs`): each metadata with its
//! device, if any.

use crate::utilities::{CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeReason, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::EnumerableEx;
use ferroui_system::IDisposable;
use std::cell::RefCell;
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

/// `DeviceMetaData`: its name can change; equal by all its properties.
struct DeviceMetaData {
    is_auto_connect: bool,
    key: i32,
    name: RefCell<String>,
}

impl DeviceMetaData {
    /// `new DeviceMetaData(int key, string name, bool isAutoConnect = false)`.
    fn new(key: i32, name: &str) -> Rc<Self> {
        Rc::new(Self { is_auto_connect: false, key, name: RefCell::new(name.to_string()) })
    }

    fn key(&self) -> i32 {
        self.key
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, name: &str) {
        *self.name.borrow_mut() = name.to_string();
    }
}

impl PartialEq for DeviceMetaData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || (self.is_auto_connect == other.is_auto_connect && self.key == other.key && *self.name.borrow() == *other.name.borrow())
    }
}

impl fmt::Debug for DeviceMetaData {
    /// `ToString`: `Key: {Key}. Metadata: {Name}. IsAutoConnect = {IsAutoConnect}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key: {}. Metadata: {}. IsAutoConnect = {}", self.key, self.name.borrow(), if self.is_auto_connect { "True" } else { "False" })
    }
}

/// `DeviceWithMetadata`: equal by key.
struct DeviceWithMetadata {
    device: Option<Rc<Device>>,
    key: i32,
    meta_data: Rc<DeviceMetaData>,
}

impl DeviceWithMetadata {
    fn new(key: i32, device: Option<Rc<Device>>, meta_data: Rc<DeviceMetaData>) -> Rc<Self> {
        Rc::new(Self { device, key, meta_data })
    }

    fn device(&self) -> Option<Rc<Device>> {
        self.device.clone()
    }

    fn meta_data(&self) -> Rc<DeviceMetaData> {
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
    result: ChangeSetAggregator<Rc<DeviceWithMetadata>, i32>,
    right: SourceCache<Rc<DeviceMetaData>, i32>,
}

impl Fixture {
    fn new() -> Self {
        let left = SourceCache::new(|device: &Rc<Device>| device.name());
        let right = SourceCache::new(|device: &Rc<DeviceMetaData>| device.key());

        let result = left
            .connect()
            .right_join_with_key(right.connect(), |meta: &Rc<DeviceMetaData>| meta.name(), |key: &i32, device: Option<&Rc<Device>>, meta: &Rc<DeviceMetaData>| {
                DeviceWithMetadata::new(*key, device.cloned(), meta.clone())
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

    /// One edit of the right side that adds the metadata.
    fn add_meta_data(&self, items: &[(i32, &str)]) {
        self.right.edit(|inner_cache| {
            for (key, name) in items {
                inner_cache.add_or_update(DeviceMetaData::new(*key, name));
            }
        });
    }

    /// `Items.Select(pair => (pair.Device.ValueOrDefault()?.Name, pair.MetaData.Key))`.
    fn pairs(&self) -> Vec<(Option<String>, i32)> {
        self.result.data().items().iter().map(|pair| (pair.device().map(|device| device.name()), pair.meta_data().key())).collect()
    }

    /// The number of results without a device (`Device == Optional<Device>.None`).
    fn count_without_device(&self) -> usize {
        self.result.data().items().iter().filter(|dwm| dwm.device().is_none()).count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.left.dispose();
        self.right.dispose();
        self.result.dispose();
    }
}

fn pair(device_name: Option<&str>, meta_key: i32) -> (Option<String>, i32) {
    (device_name.map(str::to_string), meta_key)
}

#[test]
fn add_left_only() {
    let fixture = Fixture::new();
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 0);
}

#[test]
fn add_left_then_right() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);

    assert_eq!(fixture.result.data().count(), 3);

    assert_eq!(fixture.count_without_device(), 0);
}

#[test]
fn add_right_only() {
    let fixture = Fixture::new();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.result.data().lookup(&1).is_some());
    assert!(fixture.result.data().lookup(&2).is_some());
    assert!(fixture.result.data().lookup(&3).is_some());

    assert!(fixture.result.data().items().iter().all(|dwm| dwm.device().is_none()));
}

#[test]
fn add_right_then_left() {
    let fixture = Fixture::new();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 3);

    assert_eq!(fixture.count_without_device(), 0);
}

#[test]
fn refresh_right_key() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);

    let refresh_item = fixture.right.lookup(&2).expect("the item of key 2");

    // Change pairing
    refresh_item.set_name("Device3");
    fixture.right.refresh(&refresh_item);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(Some("Device2"), 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device3"), 2)));

    // Remove pairing
    refresh_item.set_name("Device4");
    fixture.right.refresh(&refresh_item);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(Some("Device3"), 2)));
    assert!(fixture.pairs().contains(&pair(None, 2)));

    // Restore pairing
    refresh_item.set_name("Device2");
    fixture.right.refresh(&refresh_item);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(None, 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device2"), 2)));

    // No change
    fixture.right.refresh(&refresh_item);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(None, 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device2"), 2)));
}

#[test]
fn remove_various() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);

    fixture.right.remove_key(3);

    assert_eq!(fixture.result.data().count(), 2);
    assert_eq!(fixture.result.data().items().iter().filter(|dwm| dwm.device().is_some()).count(), 2);

    fixture.left.remove_key("Device1".to_string());
    assert!(fixture.result.data().lookup(&1).is_some());
    assert_eq!(fixture.count_without_device(), 1);
}

#[test]
fn update_right() {
    let fixture = Fixture::new();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);
    fixture.add_devices();

    assert_eq!(fixture.result.data().count(), 3);

    assert_eq!(fixture.count_without_device(), 0);
}

#[test]
fn update_right_key() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3")]);

    // Change pairing
    fixture.right.add_or_update(DeviceMetaData::new(2, "Device3"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(Some("Device2"), 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device3"), 2)));

    // Remove pairing
    fixture.right.add_or_update(DeviceMetaData::new(2, "Device4"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(Some("Device3"), 2)));
    assert!(fixture.pairs().contains(&pair(None, 2)));

    // Restore pairing
    fixture.right.add_or_update(DeviceMetaData::new(2, "Device2"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(None, 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device2"), 2)));

    // No change
    fixture.right.add_or_update(DeviceMetaData::new(2, "Device2"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair(None, 2)));
    assert!(fixture.pairs().contains(&pair(Some("Device2"), 2)));
}

#[test]
fn multiple_right() {
    let fixture = Fixture::new();
    fixture.add_devices();
    // deliberate!
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device3"), (3, "Device3")]);

    assert_eq!(fixture.result.data().count(), 3);

    assert_eq!(fixture.count_without_device(), 0);
}

#[test]
fn more_right() {
    let fixture = Fixture::new();
    fixture.add_devices();
    fixture.add_meta_data(&[(1, "Device1"), (2, "Device2"), (3, "Device3"), (4, "Device4")]);

    assert_eq!(fixture.result.data().count(), 4);

    assert!(fixture.result.data().lookup(&4).is_some());
    assert_eq!(fixture.count_without_device(), 1);
}

#[test]
fn initialization_waits_for_both_sources() {
    let left = vec![1, 2, 3];
    let right = vec![4, 6, 2];

    let (_subscription, results) = left
        .as_observable_change_set_cache(|left: &i32| 2 * left)
        .right_join(right.as_observable_change_set_cache(|right: &i32| *right), |right: &i32| *right, |left: Option<&i32>, right: &i32| (left.copied(), *right))
        .validate_synchronization()
        .validate_cache_change_sets(|pair: &(Option<i32>, i32)| pair.1)
        .record_cache_items();

    assert!(results.error().is_none());

    assert_eq!(results.recorded_change_sets().len(), 1, "Initialization should only emit one changeset.");
    assert!(results.recorded_change_sets()[0].iter().all(|change| change.reason() == ChangeReason::Add), "Initialization should only emit Add changes.");

    assert!(results.recorded_items_by_key().values().all(|pair| pair.0.map(|left| 2 * left) == Some(pair.1)), "Source items should have been joined correctly");
}
