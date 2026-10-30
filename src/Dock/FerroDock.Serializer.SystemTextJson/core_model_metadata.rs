// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{JsonPropertyInfo, JsonTypeInfo, JsonValuePropertyInfo};
use ferrodock_model::core::{
    DockCapabilityOverrides, DockCapabilityPolicy, DockCapabilityPolicyImpl,
    DockFloatingWindowHostMode, DockManagerOptions, DockMode, DockOperationMask, DockPoint,
    DockRect, DockWindowOptions, DockWindowOwnerMode, DockWindowState, DockingWindowState,
    DragAction, IDockCapabilityPolicy, IDockWindow, PinnedDockDisplayMode,
};
use std::rc::Rc;
type Policy = Rc<dyn DockCapabilityPolicyImpl>;
type Contract = Rc<dyn IDockCapabilityPolicy>;
fn identity<T: ?Sized>(value: &Rc<T>) -> usize {
    Rc::as_ptr(value) as *const () as usize
}
fn properties<T: IDockCapabilityPolicy + ?Sized + 'static>() -> Vec<JsonPropertyInfo<Rc<T>>> {
    vec![
        JsonPropertyInfo::new(
            "CanClose",
            |v: &Rc<T>| v.can_close(),
            |v, x| v.set_can_close(x),
        ),
        JsonPropertyInfo::new("CanPin", |v: &Rc<T>| v.can_pin(), |v, x| v.set_can_pin(x)),
        JsonPropertyInfo::new(
            "CanFloat",
            |v: &Rc<T>| v.can_float(),
            |v, x| v.set_can_float(x),
        ),
        JsonPropertyInfo::new(
            "CanDrag",
            |v: &Rc<T>| v.can_drag(),
            |v, x| v.set_can_drag(x),
        ),
        JsonPropertyInfo::new(
            "CanDrop",
            |v: &Rc<T>| v.can_drop(),
            |v, x| v.set_can_drop(x),
        ),
        JsonPropertyInfo::new(
            "CanDockAsDocument",
            |v: &Rc<T>| v.can_dock_as_document(),
            |v, x| v.set_can_dock_as_document(x),
        ),
    ]
}
/// Compiled metadata for source Core values, options, signed flags and capability policies,
/// with their nullable dependencies. Registration is explicit: supplied contexts gain no
/// implicit model catalog.
/// All class codecs retain actual model handles and original constructors/accessors.
/// The policy interface has no fabricated constructor; preserved references can upcast.
pub fn core_model_type_metadata() -> Vec<Rc<JsonTypeInfo>> {
    let rectangle = JsonTypeInfo::value_object(
        DockRect::default,
        vec![
            JsonValuePropertyInfo::new("X", |v: &DockRect| v.x, |v, x| v.x = x),
            JsonValuePropertyInfo::new("Y", |v: &DockRect| v.y, |v, x| v.y = x),
            JsonValuePropertyInfo::new("Width", |v: &DockRect| v.width, |v, x| v.width = x),
            JsonValuePropertyInfo::new("Height", |v: &DockRect| v.height, |v, x| v.height = x),
        ],
    );
    let concrete = JsonTypeInfo::object(
        DockCapabilityPolicy::new,
        identity,
        properties::<DockCapabilityPolicy>(),
    );
    let concrete =
        JsonTypeInfo::with_reference_cast::<Rc<DockCapabilityPolicy>, Policy>(concrete, |v| {
            v.clone()
        });
    let concrete =
        JsonTypeInfo::with_reference_cast::<Rc<DockCapabilityPolicy>, Contract>(concrete, |v| {
            v.clone()
        });
    let policy = JsonTypeInfo::object(
        || DockCapabilityPolicy::new() as Policy,
        identity,
        properties::<dyn DockCapabilityPolicyImpl>(),
    );
    let policy = JsonTypeInfo::with_reference_cast::<Policy, Contract>(policy, |v| v.clone());
    let overrides = JsonTypeInfo::object(
        DockCapabilityOverrides::new,
        identity,
        properties::<DockCapabilityOverrides>(),
    );
    let overrides =
        JsonTypeInfo::with_reference_cast::<Rc<DockCapabilityOverrides>, Policy>(overrides, |v| {
            v.clone()
        });
    let overrides = JsonTypeInfo::with_reference_cast::<Rc<DockCapabilityOverrides>, Contract>(
        overrides,
        |v| v.clone(),
    );
    vec![
        rectangle,
        JsonTypeInfo::optional::<DockRect>(),
        JsonTypeInfo::value_object(
            DockPoint::default,
            vec![
                JsonValuePropertyInfo::readonly("X", |v: &DockPoint| v.x),
                JsonValuePropertyInfo::readonly("Y", |v: &DockPoint| v.y),
            ],
        ),
        JsonTypeInfo::optional::<DockPoint>(),
        JsonTypeInfo::object(
            DockManagerOptions::new,
            identity,
            vec![JsonPropertyInfo::new(
                "IsDockingEnabled",
                |v: &Rc<DockManagerOptions>| v.is_docking_enabled(),
                |v, x| v.set_is_docking_enabled(x),
            )],
        ),
        JsonTypeInfo::optional::<Rc<DockManagerOptions>>(),
        JsonTypeInfo::numeric_enum(
            DockingWindowState::bits,
            DockingWindowState::from_bits_retain,
        ),
        JsonTypeInfo::optional::<DockingWindowState>(),
        JsonTypeInfo::numeric_enum(DockOperationMask::bits, DockOperationMask::from_bits_retain),
        JsonTypeInfo::optional::<DockOperationMask>(),
        JsonTypeInfo::numeric_enum(DragAction::bits, DragAction::from_bits_retain),
        JsonTypeInfo::optional::<DragAction>(),
        JsonTypeInfo::numeric_enum(
            |v: &DockWindowOwnerMode| v.to_raw(),
            DockWindowOwnerMode::from_raw,
        ),
        JsonTypeInfo::optional::<DockWindowOwnerMode>(),
        JsonTypeInfo::numeric_enum(|v: &DockWindowState| v.to_raw(), DockWindowState::from_raw),
        JsonTypeInfo::optional::<DockWindowState>(),
        JsonTypeInfo::numeric_enum(|v: &DockMode| v.to_raw(), DockMode::from_raw),
        JsonTypeInfo::optional::<DockMode>(),
        JsonTypeInfo::numeric_enum(
            |v: &PinnedDockDisplayMode| v.to_raw(),
            PinnedDockDisplayMode::from_raw,
        ),
        JsonTypeInfo::optional::<PinnedDockDisplayMode>(),
        JsonTypeInfo::numeric_enum(
            |v: &DockFloatingWindowHostMode| v.to_raw(),
            DockFloatingWindowHostMode::from_raw,
        ),
        JsonTypeInfo::optional::<DockFloatingWindowHostMode>(),
        JsonTypeInfo::object(
            DockWindowOptions::new,
            identity,
            vec![
                JsonPropertyInfo::new(
                    "OwnerMode",
                    |v: &Rc<DockWindowOptions>| v.owner_mode(),
                    |v, x| v.set_owner_mode(x),
                ),
                JsonPropertyInfo::new(
                    "ParentWindow",
                    |v: &Rc<DockWindowOptions>| v.parent_window(),
                    |v, x| v.set_parent_window(x),
                ),
                JsonPropertyInfo::new(
                    "IsModal",
                    |v: &Rc<DockWindowOptions>| v.is_modal(),
                    |v, x| v.set_is_modal(x),
                ),
                JsonPropertyInfo::new(
                    "ShowInTaskbar",
                    |v: &Rc<DockWindowOptions>| v.show_in_taskbar(),
                    |v, x| v.set_show_in_taskbar(x),
                ),
            ],
        ),
        JsonTypeInfo::optional::<Rc<DockWindowOptions>>(),
        JsonTypeInfo::optional::<Rc<dyn IDockWindow>>(),
        JsonTypeInfo::optional::<bool>(),
        concrete,
        JsonTypeInfo::optional::<Rc<DockCapabilityPolicy>>(),
        policy,
        JsonTypeInfo::optional::<Policy>(),
        overrides,
        JsonTypeInfo::optional::<Rc<DockCapabilityOverrides>>(),
        JsonTypeInfo::abstract_object(identity, properties::<dyn IDockCapabilityPolicy>()),
        JsonTypeInfo::optional::<Contract>(),
    ]
}
