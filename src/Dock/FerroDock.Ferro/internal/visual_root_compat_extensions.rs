// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{Ref, Visual};
/// Original nullable visual-root compatibility extension.
pub(crate) struct VisualRootCompatExtensions;
impl VisualRootCompatExtensions {
    pub(crate) fn get_visual_root(visual: Option<&Visual>) -> Option<Ref<Visual>> {
        visual.and_then(Visual::visual_root)
    }
}
