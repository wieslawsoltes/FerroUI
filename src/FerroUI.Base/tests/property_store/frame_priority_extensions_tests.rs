//! Port of the upstream frame priority tests.

use crate::data::BindingPriority;
use crate::property_store::{FramePriority, FrameType};

const PRIORITIES: [BindingPriority; 4] =
    [BindingPriority::Animation, BindingPriority::StyleTrigger, BindingPriority::Template, BindingPriority::Style];

const TYPES: [FrameType; 3] = [FrameType::Style, FrameType::TemplatedParentTheme, FrameType::Theme];

#[test]
fn binding_priority_to_frame_priority() {
    // The frame priorities in upstream enumeration order:
    // Animation, AnimationTemplatedParentTheme, AnimationTheme, StyleTrigger,
    // StyleTriggerTemplatedParentTheme, StyleTriggerTheme, Template,
    // TemplateTemplatedParentTheme, TemplateTheme, Style,
    // StyleTemplatedParentTheme, StyleTheme.
    let mut expected = 0;
    let mut previous: Option<FramePriority> = None;

    for priority in PRIORITIES {
        for type_ in TYPES {
            let result = FramePriority::new(priority, type_);

            assert_eq!(format!("FramePriority({expected})"), format!("{result:?}"));
            assert_eq!(priority, result.to_binding_priority());
            if let Some(previous) = previous {
                assert!(previous < result);
            }

            previous = Some(result);
            expected += 1;
        }
    }
}

#[test]
fn frame_priority_is_frame_type() {
    for priority in PRIORITIES {
        for type_ in TYPES {
            let frame_priority = FramePriority::new(priority, type_);

            for other in TYPES {
                assert_eq!(other == type_, frame_priority.is_type(other), "{priority:?} {type_:?} {other:?}");
            }
        }
    }
}
