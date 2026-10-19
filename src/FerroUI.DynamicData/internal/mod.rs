//! The internal machinery shared by the cache and list sides (`DynamicData/Internal`).

mod delivery_queue;

pub(crate) use delivery_queue::DeliveryQueue;
