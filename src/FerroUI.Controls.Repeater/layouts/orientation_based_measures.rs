// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use ferroui_base::{Point, Rect, Size};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// Major is the scrolling axis; minor is the axis along a line.
#[derive(Clone, Copy, Debug, Default)]
pub struct OrientationBasedMeasures {
    pub orientation: ScrollOrientation,
}
impl OrientationBasedMeasures {
    pub fn new(orientation: ScrollOrientation) -> Self {
        Self { orientation }
    }
    pub fn major(&self, size: Size) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            size.height
        } else {
            size.width
        }
    }
    pub fn minor(&self, size: Size) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            size.width
        } else {
            size.height
        }
    }
    pub fn major_point(&self, point: Point) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            point.y
        } else {
            point.x
        }
    }
    pub fn minor_point(&self, point: Point) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            point.x
        } else {
            point.y
        }
    }
    pub fn major_size(&self, rect: Rect) -> f64 {
        self.major(rect.size())
    }
    pub fn minor_size(&self, rect: Rect) -> f64 {
        self.minor(rect.size())
    }
    pub fn major_start(&self, rect: Rect) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            rect.y
        } else {
            rect.x
        }
    }
    pub fn minor_start(&self, rect: Rect) -> f64 {
        if self.orientation == ScrollOrientation::Vertical {
            rect.x
        } else {
            rect.y
        }
    }
    pub fn major_end(&self, rect: Rect) -> f64 {
        self.major_start(rect) + self.major_size(rect)
    }
    pub fn minor_end(&self, rect: Rect) -> f64 {
        self.minor_start(rect) + self.minor_size(rect)
    }
    pub fn rect(&self, minor: f64, major: f64, minor_size: f64, major_size: f64) -> Rect {
        if self.orientation == ScrollOrientation::Vertical {
            Rect::new(minor, major, minor_size, major_size)
        } else {
            Rect::new(major, minor, major_size, minor_size)
        }
    }
    pub fn point(&self, minor: f64, major: f64) -> Point {
        if self.orientation == ScrollOrientation::Vertical {
            Point::new(minor, major)
        } else {
            Point::new(major, minor)
        }
    }
    pub fn size(&self, minor: f64, major: f64) -> Size {
        if self.orientation == ScrollOrientation::Vertical {
            Size::new(minor, major)
        } else {
            Size::new(major, minor)
        }
    }
    pub fn with_major_start(&self, rect: Rect, value: f64) -> Rect {
        self.rect(
            self.minor_start(rect),
            value,
            self.minor_size(rect),
            self.major_size(rect),
        )
    }
    pub fn with_minor_start(&self, rect: Rect, value: f64) -> Rect {
        self.rect(
            value,
            self.major_start(rect),
            self.minor_size(rect),
            self.major_size(rect),
        )
    }
    pub fn with_major_size(&self, rect: Rect, value: f64) -> Rect {
        self.rect(
            self.minor_start(rect),
            self.major_start(rect),
            self.minor_size(rect),
            value,
        )
    }
    pub fn with_minor_size(&self, rect: Rect, value: f64) -> Rect {
        self.rect(
            self.minor_start(rect),
            self.major_start(rect),
            value,
            self.major_size(rect),
        )
    }
}
