/// Describes a graphics device adapter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlatformGraphicsDeviceAdapterDescription {
    pub description: Option<String>,
    pub device_luid: Option<Vec<u8>>,
    pub device_uuid: Option<Vec<u8>>,
}
