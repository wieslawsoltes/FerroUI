//! Port of the upstream `FlowDirectionTests`.
//!
//! The parent of the last test is a visual where upstream's is a decorator
//! (the control library is another crate).

use crate::media::FlowDirection;
use crate::*;

#[test]
fn has_mirror_transform_should_be_true() {
    let target = Visual::new();
    target.set_flow_direction(FlowDirection::RightToLeft);

    assert!(target.has_mirror_transform());
}

#[test]
fn has_mirror_transform_of_ltr_children_should_be_true_for_rtl_parent() {
    let child = Visual::new();
    child.set_flow_direction(FlowDirection::LeftToRight);

    let target = Visual::new();
    target.set_flow_direction(FlowDirection::RightToLeft);
    target.visual_children().add(child.clone());

    child.invalidate_mirror_transform();

    assert!(target.has_mirror_transform());
    assert!(child.has_mirror_transform());
}

#[test]
fn has_mirror_transform_of_children_is_updated_after_parent_changed() {
    let child = Visual::new();
    child.set_flow_direction(FlowDirection::LeftToRight);

    let target = Visual::new();
    target.set_flow_direction(FlowDirection::LeftToRight);
    target.visual_children().add(child.clone());

    assert!(!target.has_mirror_transform());
    assert!(!child.has_mirror_transform());

    target.set_flow_direction(FlowDirection::RightToLeft);

    assert!(target.has_mirror_transform());
    assert!(child.has_mirror_transform());
}
