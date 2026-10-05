// RichTextKit
// Copyright © 2019 Topten Software. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this product except in compliance with the License. You may obtain
// a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// Ported from: https://github.com/foliojs/linebreak
// Copied from: https://github.com/toptensoftware/RichTextKit

/// Information about a potential line break position
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LineBreak {
    position_measure: usize,
    position_wrap: usize,
    required: bool,
}

impl LineBreak {
    /// Constructor
    ///
    /// * `position_measure` - The code point index to measure to
    /// * `position_wrap` - The code point index to actually break the line at
    /// * `required` - True if this is a required line break; otherwise false
    #[inline]
    pub const fn new(position_measure: usize, position_wrap: usize, required: bool) -> Self {
        Self { position_measure, position_wrap, required }
    }

    /// The break position, before any trailing whitespace
    ///
    /// This doesn't include trailing whitespace
    #[inline]
    pub const fn position_measure(&self) -> usize {
        self.position_measure
    }

    /// The break position, after any trailing whitespace
    ///
    /// This includes trailing whitespace
    #[inline]
    pub const fn position_wrap(&self) -> usize {
        self.position_wrap
    }

    /// True if there should be a forced line break here
    #[inline]
    pub const fn required(&self) -> bool {
        self.required
    }
}
