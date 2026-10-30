// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use std::rc::Rc;
/// Original empty Tool2 context model, retaining reference identity.
#[derive(Default)]
pub struct Tool2;
impl Tool2 {
    /// Creates a distinct source context object.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl PartialEq for Tool2 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
