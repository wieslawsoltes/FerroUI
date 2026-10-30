// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Control-specific projection over the shared native object boundary.
pub use ferrodock_runtime::{register_value_type, to_source_value as to_recycling_value};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;
use std::{any::Any, rc::Rc};
/// Recovers the original native value, preserving raw control handles as before.
pub fn from_recycling_value(value: &Rc<dyn Any>) -> BoxedValue {
    if let Ok(control) = value.clone().downcast::<Ref<Control>>() {
        return control;
    }
    ferrodock_runtime::from_source_value(value)
}
