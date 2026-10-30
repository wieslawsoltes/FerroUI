// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::core::*;
use crate::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// The source `result is T ? typed : default` operation with explicit Rust nullability.
/// Value types return their zero/default value on mismatch; reference results use
/// Option<Rc<T>> and retain identity. Applications may implement it for their own
/// value/result types. Automatic CLR subtype/interface casts remain a metadata gap.
pub trait DialogResult: Clone + 'static {
    /// Converts the nullable boxed result using exact source type membership.
    fn from_dialog_result(result: Option<BoxedValue>) -> Self;
}
macro_rules! values {
    ($($type:ty),* $(,)?) => { $(
        impl DialogResult for $type {
            fn from_dialog_result(result:Option<BoxedValue>)->Self {
                result.and_then(|value| value.downcast_ref::<Self>().cloned()).unwrap_or_default()
            }
        }
        impl DialogResult for Option<$type> {
            fn from_dialog_result(result:Option<BoxedValue>)->Self { result.and_then(|value| value.downcast_ref::<$type>().cloned()) }
        }
    )* };
}
values!(
    bool,
    char,
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
    u64,
    i64,
    u128,
    i128,
    usize,
    isize,
    f32,
    f64,
    (),
    DockPoint,
    DockRect,
    DockOperation,
    DockMode,
    Orientation,
    Alignment,
    GripMode,
    DocumentTabLayout,
    DocumentLayoutMode,
    DocumentCloseButtonShowMode,
    DockWindowState,
    DockWindowOwnerMode,
    DockFloatingWindowHostMode,
    PinnedDockDisplayMode,
    SplitViewDisplayMode,
    SplitViewPanePlacement,
    GridResizeDirection
);
impl<T: Any> DialogResult for Option<Rc<T>> {
    fn from_dialog_result(result: Option<BoxedValue>) -> Self {
        let value = result?;
        if let Some(reference) = value.downcast_ref::<Rc<T>>() {
            return Some(reference.clone());
        }
        if let Some(dockable) = value.downcast_ref::<Rc<dyn IDockable>>() {
            return crate::native_reference::cast_dockable(dockable.clone());
        }
        value.downcast::<T>().ok()
    }
}
impl DialogResult for Option<BoxedValue> {
    fn from_dialog_result(result: Option<BoxedValue>) -> Self {
        result
    }
}
macro_rules! interfaces {
    ($($type:ty),* $(,)?) => { $(
        impl DialogResult for Option<Rc<$type>> {
            fn from_dialog_result(result:Option<BoxedValue>)->Self { result.and_then(|value| {
                value.downcast_ref::<Rc<$type>>().cloned().or_else(|| {
                    let dockable=value.downcast_ref::<Rc<dyn IDockable>>()?;
                    crate::native_reference::cast_dockable::<$type>(dockable.clone())
                })
            }) }
        }
    )* };
}
interfaces!(
    dyn IDockable,
    dyn IDock,
    dyn crate::controls::IDocument,
    dyn crate::controls::ITool,
    dyn crate::controls::IRootDock,
    dyn crate::controls::IDocumentDock,
    dyn crate::controls::IToolDock
);
