// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use std::rc::Rc;
/// Original empty DemoDocument context model, retaining reference identity.
#[derive(Default)]
pub struct DemoDocument;
impl DemoDocument {
    /// Creates a distinct source context object.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl PartialEq for DemoDocument {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
