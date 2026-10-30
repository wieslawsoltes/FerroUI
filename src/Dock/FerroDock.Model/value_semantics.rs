// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Native untyped equality for the ported scalar, geometry, enum and canonical reference values.
/// Custom boxed value types/Equals overrides still require shared value metadata integration.
/// Unknown reference values compare by identity, matching source reference-class defaults.
pub fn boxed_values_equal(left: &BoxedValue, right: &BoxedValue) -> bool {
    if Rc::ptr_eq(left, right) {
        return true;
    }
    macro_rules! value_equals {
        ($($type:ty),+ $(,)?) => { $(
            if let (Some(a), Some(b)) = (left.downcast_ref::<$type>(), right.downcast_ref::<$type>()) { return a == b; }
        )+ };
    }
    value_equals!(
        String,
        bool,
        char,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        DockPoint,
        DockRect,
        DockMode,
        DockingWindowState,
        DockOperation,
        DockOperationMask,
        DockCapability,
        DockCapabilityValueSource,
        DockWindowState,
        DockWindowOwnerMode,
        DockFloatingWindowHostMode,
        Orientation,
        Alignment,
        GridResizeDirection,
        GripMode,
        DocumentLayoutMode,
        DocumentTabLayout,
        DocumentCloseButtonShowMode,
        MdiWindowState,
        PinnedDockDisplayMode,
        SplitViewDisplayMode,
        SplitViewPanePlacement,
        TrackingMode,
        DockTrackingChangeReason,
        DragAction,
        Rc<dyn IDockable>,
        Rc<dyn IFactory>,
        Rc<dyn ferroui_reactiveui::ICommand>
    );
    if let (Some(a), Some(b)) = (left.downcast_ref::<f64>(), right.downcast_ref::<f64>()) {
        return a == b || (a.is_nan() && b.is_nan());
    }
    if let (Some(a), Some(b)) = (left.downcast_ref::<f32>(), right.downcast_ref::<f32>()) {
        return a == b || (a.is_nan() && b.is_nan());
    }
    false
}

#[derive(Clone)]
pub(crate) struct SourceItemKey(pub(crate) BoxedValue);
impl PartialEq for SourceItemKey {
    fn eq(&self, other: &Self) -> bool {
        boxed_values_equal(&self.0, &other.0)
    }
}
impl Eq for SourceItemKey {}
fn double_bits(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else if value == 0.0 {
        0
    } else {
        value.to_bits()
    }
}
impl Hash for SourceItemKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().type_id().hash(state);
        macro_rules! hash_value {
            ($($type:ty),+ $(,)?) => { $(
                if let Some(value) = self.0.downcast_ref::<$type>() { value.hash(state); return; }
            )+ };
        }
        hash_value!(
            String,
            bool,
            char,
            i8,
            i16,
            i32,
            i64,
            i128,
            isize,
            u8,
            u16,
            u32,
            u64,
            u128,
            usize,
            DockMode,
            DockingWindowState,
            DockOperation,
            DockOperationMask,
            DockCapability,
            DockCapabilityValueSource,
            DockWindowState,
            DockWindowOwnerMode,
            DockFloatingWindowHostMode,
            Orientation,
            Alignment,
            GridResizeDirection,
            GripMode,
            DocumentLayoutMode,
            DocumentTabLayout,
            DocumentCloseButtonShowMode,
            MdiWindowState,
            PinnedDockDisplayMode,
            SplitViewDisplayMode,
            SplitViewPanePlacement,
            TrackingMode,
            DockTrackingChangeReason,
            DragAction,
        );
        if let Some(value) = self.0.downcast_ref::<f64>() {
            double_bits(*value).hash(state);
            return;
        }
        if let Some(value) = self.0.downcast_ref::<f32>() {
            let bits = if value.is_nan() {
                f32::NAN.to_bits()
            } else if *value == 0.0 {
                0
            } else {
                value.to_bits()
            };
            bits.hash(state);
            return;
        }
        if let Some(value) = self.0.downcast_ref::<DockPoint>() {
            double_bits(value.x).hash(state);
            double_bits(value.y).hash(state);
            return;
        }
        if let Some(value) = self.0.downcast_ref::<DockRect>() {
            for component in [value.x, value.y, value.width, value.height] {
                double_bits(component).hash(state);
            }
            return;
        }
        macro_rules! hash_reference {
            ($($type:ty),+ $(,)?) => { $(
                if let Some(value) = self.0.downcast_ref::<Rc<$type>>() { (Rc::as_ptr(value) as *const () as usize).hash(state); return; }
            )+ };
        }
        hash_reference!(
            dyn IDockable,
            dyn IFactory,
            dyn ferroui_reactiveui::ICommand
        );
        (Rc::as_ptr(&self.0) as *const () as usize).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::factory_parameter_tests::RecordingFactory;
    use crate::test_support::{TestCommand, TestNode};
    use ferroui_system::collections::generic::Dictionary;
    use std::collections::hash_map::DefaultHasher;

    fn same_hash_and_lookup<T: Clone + 'static>(value: T) {
        let left = SourceItemKey(Rc::new(value.clone()));
        let right = SourceItemKey(Rc::new(value));
        assert!(boxed_values_equal(&left.0, &right.0));
        let mut a = DefaultHasher::new();
        let mut b = DefaultHasher::new();
        left.hash(&mut a);
        right.hash(&mut b);
        assert_eq!(a.finish(), b.finish());
        let mut map = Dictionary::new();
        map.insert(left, 42);
        assert_eq!(map.get(&right), Some(&42));
    }

    #[test]
    fn scalar_and_defined_docking_value_types_have_consistent_hash_equality() {
        macro_rules! defaults {
            ($($type:ty),+ $(,)?) => { $(same_hash_and_lookup(<$type>::default());)+ };
        }
        defaults!(
            String,
            bool,
            char,
            i8,
            i16,
            i32,
            i64,
            i128,
            isize,
            u8,
            u16,
            u32,
            u64,
            u128,
            usize,
            DockMode,
            DockingWindowState,
            DockOperation,
            DockOperationMask,
            DockCapability,
            DockCapabilityValueSource,
            DockWindowState,
            DockWindowOwnerMode,
            DockFloatingWindowHostMode,
            Orientation,
            Alignment,
            GridResizeDirection,
            GripMode,
            DocumentLayoutMode,
            DocumentTabLayout,
            DocumentCloseButtonShowMode,
            MdiWindowState,
            PinnedDockDisplayMode,
            SplitViewDisplayMode,
            SplitViewPanePlacement,
            TrackingMode,
            DockTrackingChangeReason,
            DragAction
        );
        same_hash_and_lookup("item".to_owned());
        same_hash_and_lookup(123_i32);
        same_hash_and_lookup(-123_i64);
        same_hash_and_lookup('λ');
    }

    #[test]
    fn canonical_erased_reference_boxes_compare_the_underlying_object() {
        let dockable: Rc<dyn IDockable> = TestNode::new(None);
        let factory: Rc<dyn IFactory> = RecordingFactory::with_base();
        let command: Rc<dyn ferroui_reactiveui::ICommand> = Rc::new(TestCommand);
        same_hash_and_lookup(dockable);
        same_hash_and_lookup(factory);
        same_hash_and_lookup(command);
    }

    #[test]
    fn plain_reference_classes_keep_identity_and_different_numeric_types_are_not_equal() {
        struct ReferenceClass;
        let left: BoxedValue = Rc::new(ReferenceClass);
        let right: BoxedValue = Rc::new(ReferenceClass);
        assert!(boxed_values_equal(&left, &left.clone()));
        assert!(!boxed_values_equal(&left, &right));
        let mut map = Dictionary::new();
        map.insert(SourceItemKey(left.clone()), 42);
        assert_eq!(map.get(&SourceItemKey(left)), Some(&42));
        assert_eq!(map.get(&SourceItemKey(right)), None);
        assert!(!boxed_values_equal(
            &(Rc::new(0_i32) as BoxedValue),
            &(Rc::new(0_i64) as BoxedValue)
        ));
    }
}
