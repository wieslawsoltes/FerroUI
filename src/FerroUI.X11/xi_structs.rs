//! The enumerations of the X Input extension (the port of the enumerations of `XIStructs.cs`).
//!
//! The structures of that file are the ones of the bindings (`crate::xlib::xi2`) and the
//! copies `crate::xlib` makes of them.

/// `XiScrollType`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiScrollType {
    Vertical = 1,
    Horizontal = 2,
}

impl XiScrollType {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiScrollType> {
        match value {
            1 => Some(XiScrollType::Vertical),
            2 => Some(XiScrollType::Horizontal),
            _ => None,
        }
    }
}

/// `XiDeviceType`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiDeviceType {
    XIMasterPointer = 1,
    XIMasterKeyboard = 2,
    XISlavePointer = 3,
    XISlaveKeyboard = 4,
    XIFloatingSlave = 5,
}

impl XiDeviceType {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiDeviceType> {
        match value {
            1 => Some(XiDeviceType::XIMasterPointer),
            2 => Some(XiDeviceType::XIMasterKeyboard),
            3 => Some(XiDeviceType::XISlavePointer),
            4 => Some(XiDeviceType::XISlaveKeyboard),
            5 => Some(XiDeviceType::XIFloatingSlave),
            _ => None,
        }
    }
}

/// `XiPredefinedDeviceId`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiPredefinedDeviceId {
    XIAllDevices = 0,
    XIAllMasterDevices = 1,
}

impl XiPredefinedDeviceId {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiPredefinedDeviceId> {
        match value {
            0 => Some(XiPredefinedDeviceId::XIAllDevices),
            1 => Some(XiPredefinedDeviceId::XIAllMasterDevices),
            _ => None,
        }
    }
}

/// `XiDeviceClass`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiDeviceClass {
    XIKeyClass = 0,
    XIButtonClass = 1,
    XIValuatorClass = 2,
    XIScrollClass = 3,
    XITouchClass = 8,
}

impl XiDeviceClass {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiDeviceClass> {
        match value {
            0 => Some(XiDeviceClass::XIKeyClass),
            1 => Some(XiDeviceClass::XIButtonClass),
            2 => Some(XiDeviceClass::XIValuatorClass),
            3 => Some(XiDeviceClass::XIScrollClass),
            8 => Some(XiDeviceClass::XITouchClass),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `XiDeviceEventFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XiDeviceEventFlags: i32 {
        const NONE = 0;
        const XI_POINTER_EMULATED = 0x10000;
    }
}

/// `XiDeviceChangeReason`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiDeviceChangeReason {
    XISlaveSwitch = 1,
    XIDeviceChange = 2,
}

impl XiDeviceChangeReason {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiDeviceChangeReason> {
        match value {
            1 => Some(XiDeviceChangeReason::XISlaveSwitch),
            2 => Some(XiDeviceChangeReason::XIDeviceChange),
            _ => None,
        }
    }
}

/// `XiEventType`. Some of its members share a value, so it is a set of constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct XiEventType(pub i32);

#[allow(non_upper_case_globals)]
impl XiEventType {
    pub const XI_DeviceChanged: XiEventType = XiEventType(1);
    pub const XI_KeyPress: XiEventType = XiEventType(2);
    pub const XI_KeyRelease: XiEventType = XiEventType(3);
    pub const XI_ButtonPress: XiEventType = XiEventType(4);
    pub const XI_ButtonRelease: XiEventType = XiEventType(5);
    pub const XI_Motion: XiEventType = XiEventType(6);
    pub const XI_Enter: XiEventType = XiEventType(7);
    pub const XI_Leave: XiEventType = XiEventType(8);
    pub const XI_FocusIn: XiEventType = XiEventType(9);
    pub const XI_FocusOut: XiEventType = XiEventType(10);
    pub const XI_HierarchyChanged: XiEventType = XiEventType(11);
    pub const XI_PropertyEvent: XiEventType = XiEventType(12);
    pub const XI_RawKeyPress: XiEventType = XiEventType(13);
    pub const XI_RawKeyRelease: XiEventType = XiEventType(14);
    pub const XI_RawButtonPress: XiEventType = XiEventType(15);
    pub const XI_RawButtonRelease: XiEventType = XiEventType(16);
    pub const XI_RawMotion: XiEventType = XiEventType(17);
    pub const XI_TouchBegin: XiEventType = XiEventType(18);
    pub const XI_TouchUpdate: XiEventType = XiEventType(19);
    pub const XI_TouchEnd: XiEventType = XiEventType(20);
    pub const XI_TouchOwnership: XiEventType = XiEventType(21);
    pub const XI_RawTouchBegin: XiEventType = XiEventType(22);
    pub const XI_RawTouchUpdate: XiEventType = XiEventType(23);
    pub const XI_RawTouchEnd: XiEventType = XiEventType(24);
    pub const XI_BarrierHit: XiEventType = XiEventType(25);
    pub const XI_BarrierLeave: XiEventType = XiEventType(26);
    pub const XI_LASTEVENT: XiEventType = XiEventType(26);
}

/// `XiEnterLeaveDetail`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XiEnterLeaveDetail {
    XINotifyAncestor = 0,
    XINotifyVirtual = 1,
    XINotifyInferior = 2,
    XINotifyNonlinear = 3,
    XINotifyNonlinearVirtual = 4,
    XINotifyPointer = 5,
    XINotifyPointerRoot = 6,
    XINotifyDetailNone = 7,
}

impl XiEnterLeaveDetail {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XiEnterLeaveDetail> {
        match value {
            0 => Some(XiEnterLeaveDetail::XINotifyAncestor),
            1 => Some(XiEnterLeaveDetail::XINotifyVirtual),
            2 => Some(XiEnterLeaveDetail::XINotifyInferior),
            3 => Some(XiEnterLeaveDetail::XINotifyNonlinear),
            4 => Some(XiEnterLeaveDetail::XINotifyNonlinearVirtual),
            5 => Some(XiEnterLeaveDetail::XINotifyPointer),
            6 => Some(XiEnterLeaveDetail::XINotifyPointerRoot),
            7 => Some(XiEnterLeaveDetail::XINotifyDetailNone),
            _ => None,
        }
    }
}
