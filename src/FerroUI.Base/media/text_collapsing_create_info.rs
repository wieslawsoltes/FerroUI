use std::rc::Rc;

use crate::media::text_formatting::TextRunProperties;
use crate::media::FlowDirection;

/// The values a [`TextTrimming`](crate::media::TextTrimming) creates
/// collapsing properties from.
#[derive(Clone)]
pub struct TextCollapsingCreateInfo {
    pub width: f64,
    pub text_run_properties: Rc<dyn TextRunProperties>,
    pub flow_direction: FlowDirection,
}

impl TextCollapsingCreateInfo {
    pub fn new(width: f64, text_run_properties: Rc<dyn TextRunProperties>, flow_direction: FlowDirection) -> Self {
        Self { width, text_run_properties, flow_direction }
    }
}

impl PartialEq for TextCollapsingCreateInfo {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && (Rc::ptr_eq(&self.text_run_properties, &other.text_run_properties)
                || *self.text_run_properties == *other.text_run_properties)
            && self.flow_direction == other.flow_direction
    }
}
