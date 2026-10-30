// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native docking attached behaviors.
mod visual_tree_lifecycle_behavior;
pub use visual_tree_lifecycle_behavior::VisualTreeLifecycleBehavior;

#[cfg(test)]
mod lifecycle_tests;
