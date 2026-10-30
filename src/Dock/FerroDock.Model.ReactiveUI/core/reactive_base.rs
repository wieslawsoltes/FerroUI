// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_reactiveui::{reactive_object, ReactiveObject};

/// The shared reactive part of the abstract model base.
#[derive(Default)]
pub struct ReactiveBase {
    reactive: ReactiveObject,
}

reactive_object!(ReactiveBase);

impl AsRef<ReactiveObject> for ReactiveBase {
    fn as_ref(&self) -> &ReactiveObject {
        &self.reactive
    }
}
