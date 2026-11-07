//! `Rect3D`: an axis-aligned box in 3D space.

use std::fmt;
use std::str::FromStr;

use ferroui_base::utilities::FormatError;

use crate::text::{is_keyword, parse_numbers, write_numbers};
use crate::{Point3D, Size3D, Size3DError, Vector3D};

/// An axis-aligned box in 3D space: a location (its corner of least coordinates) and a size, with
/// double-precision values (WPF's `System.Windows.Media.Media3D.Rect3D`). The bounds of the 3D
/// scene model are boxes.
///
/// [`Rect3D::EMPTY`] is the box of nothing: its location is positive infinity and its size is
/// [`Size3D::EMPTY`]. It contains nothing, intersects nothing, and is the identity of
/// [`union`](Rect3D::union). The members that WPF guards with exceptions return a [`Size3DError`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect3D {
    x: f64,
    y: f64,
    z: f64,
    size_x: f64,
    size_y: f64,
    size_z: f64,
}

impl Rect3D {
    /// The box of nothing (WPF's `Rect3D.Empty`).
    pub const EMPTY: Rect3D = Rect3D {
        x: f64::INFINITY,
        y: f64::INFINITY,
        z: f64::INFINITY,
        size_x: f64::NEG_INFINITY,
        size_y: f64::NEG_INFINITY,
        size_z: f64::NEG_INFINITY,
    };

    /// A box with the given location and size; [`Size3DError::NegativeDimension`] when a size is
    /// negative.
    pub fn new(x: f64, y: f64, z: f64, size_x: f64, size_y: f64, size_z: f64) -> Result<Self, Size3DError> {
        if size_x < 0.0 || size_y < 0.0 || size_z < 0.0 {
            return Err(Size3DError::NegativeDimension);
        }
        Ok(Self {
            x,
            y,
            z,
            size_x,
            size_y,
            size_z,
        })
    }

    /// A box at the location with the size; [`Rect3D::EMPTY`] when the size is empty.
    pub fn from_location_size(location: Point3D, size: Size3D) -> Self {
        if size.is_empty() {
            return Rect3D::EMPTY;
        }
        Self {
            x: location.x,
            y: location.y,
            z: location.z,
            size_x: size.x(),
            size_y: size.y(),
            size_z: size.z(),
        }
    }

    /// The smallest box that contains both points.
    pub(crate) fn from_points(point1: Point3D, point2: Point3D) -> Self {
        let x = point1.x.min(point2.x);
        let y = point1.y.min(point2.y);
        let z = point1.z.min(point2.z);
        Self {
            x,
            y,
            z,
            size_x: point1.x.max(point2.x) - x,
            size_y: point1.y.max(point2.y) - y,
            size_z: point1.z.max(point2.z) - z,
        }
    }

    /// Whether this is [`Rect3D::EMPTY`].
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size_x < 0.0
    }

    /// The X coordinate of the location.
    #[inline]
    pub fn x(&self) -> f64 {
        self.x
    }

    /// The Y coordinate of the location.
    #[inline]
    pub fn y(&self) -> f64 {
        self.y
    }

    /// The Z coordinate of the location.
    #[inline]
    pub fn z(&self) -> f64 {
        self.z
    }

    /// The extent along the X axis.
    #[inline]
    pub fn size_x(&self) -> f64 {
        self.size_x
    }

    /// The extent along the Y axis.
    #[inline]
    pub fn size_y(&self) -> f64 {
        self.size_y
    }

    /// The extent along the Z axis.
    #[inline]
    pub fn size_z(&self) -> f64 {
        self.size_z
    }

    /// The corner of least coordinates.
    #[inline]
    pub fn location(&self) -> Point3D {
        Point3D::new(self.x, self.y, self.z)
    }

    /// The size; [`Size3D::EMPTY`] for the empty box.
    pub fn size(&self) -> Size3D {
        if self.is_empty() {
            Size3D::EMPTY
        } else {
            Size3D::from_magnitudes(self.size_x, self.size_y, self.size_z)
        }
    }

    /// Sets the X coordinate of the location; the empty box cannot be moved.
    pub fn set_x(&mut self, value: f64) -> Result<(), Size3DError> {
        self.check_not_empty()?;
        self.x = value;
        Ok(())
    }

    /// Sets the Y coordinate of the location; the empty box cannot be moved.
    pub fn set_y(&mut self, value: f64) -> Result<(), Size3DError> {
        self.check_not_empty()?;
        self.y = value;
        Ok(())
    }

    /// Sets the Z coordinate of the location; the empty box cannot be moved.
    pub fn set_z(&mut self, value: f64) -> Result<(), Size3DError> {
        self.check_not_empty()?;
        self.z = value;
        Ok(())
    }

    /// Sets the extent along the X axis; the empty box cannot be changed, and a size is not
    /// negative.
    pub fn set_size_x(&mut self, value: f64) -> Result<(), Size3DError> {
        self.size_x = self.checked_size(value)?;
        Ok(())
    }

    /// Sets the extent along the Y axis; the empty box cannot be changed, and a size is not
    /// negative.
    pub fn set_size_y(&mut self, value: f64) -> Result<(), Size3DError> {
        self.size_y = self.checked_size(value)?;
        Ok(())
    }

    /// Sets the extent along the Z axis; the empty box cannot be changed, and a size is not
    /// negative.
    pub fn set_size_z(&mut self, value: f64) -> Result<(), Size3DError> {
        self.size_z = self.checked_size(value)?;
        Ok(())
    }

    /// Sets the location; the empty box cannot be moved.
    pub fn set_location(&mut self, location: Point3D) -> Result<(), Size3DError> {
        self.check_not_empty()?;
        self.x = location.x;
        self.y = location.y;
        self.z = location.z;
        Ok(())
    }

    /// Sets the size: the empty size makes this the empty box; another size cannot be given to
    /// the empty box.
    pub fn set_size(&mut self, size: Size3D) -> Result<(), Size3DError> {
        if size.is_empty() {
            *self = Rect3D::EMPTY;
            return Ok(());
        }
        self.check_not_empty()?;
        self.size_x = size.x();
        self.size_y = size.y();
        self.size_z = size.z();
        Ok(())
    }

    /// Whether the point is in the box or on its faces; never for the empty box.
    #[inline]
    pub fn contains(&self, point: Point3D) -> bool {
        self.contains_xyz(point.x, point.y, point.z)
    }

    /// Whether the point with the coordinates is in the box or on its faces; never for the empty
    /// box.
    pub fn contains_xyz(&self, x: f64, y: f64, z: f64) -> bool {
        if self.is_empty() {
            return false;
        }
        x >= self.x
            && x <= self.x + self.size_x
            && y >= self.y
            && y <= self.y + self.size_y
            && z >= self.z
            && z <= self.z + self.size_z
    }

    /// Whether the box is inside this one (faces may touch); never when either is empty.
    pub fn contains_rect(&self, rect: Rect3D) -> bool {
        if self.is_empty() || rect.is_empty() {
            return false;
        }
        self.x <= rect.x
            && self.y <= rect.y
            && self.z <= rect.z
            && self.x + self.size_x >= rect.x + rect.size_x
            && self.y + self.size_y >= rect.y + rect.size_y
            && self.z + self.size_z >= rect.z + rect.size_z
    }

    /// Whether the boxes share a point (touching faces count); never when either is empty.
    pub fn intersects_with(&self, rect: Rect3D) -> bool {
        if self.is_empty() || rect.is_empty() {
            return false;
        }
        rect.x <= self.x + self.size_x
            && rect.x + rect.size_x >= self.x
            && rect.y <= self.y + self.size_y
            && rect.y + rect.size_y >= self.y
            && rect.z <= self.z + self.size_z
            && rect.z + rect.size_z >= self.z
    }

    /// Makes this box the intersection of the two: [`Rect3D::EMPTY`] when they do not intersect.
    pub fn intersect(&mut self, rect: Rect3D) {
        if !self.intersects_with(rect) {
            *self = Rect3D::EMPTY;
            return;
        }
        let x = self.x.max(rect.x);
        let y = self.y.max(rect.y);
        let z = self.z.max(rect.z);
        self.size_x = (self.x + self.size_x).min(rect.x + rect.size_x) - x;
        self.size_y = (self.y + self.size_y).min(rect.y + rect.size_y) - y;
        self.size_z = (self.z + self.size_z).min(rect.z + rect.size_z) - z;
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// The intersection of two boxes (WPF's static `Rect3D.Intersect`).
    pub fn intersect_rects(rect1: Rect3D, rect2: Rect3D) -> Rect3D {
        let mut rect = rect1;
        rect.intersect(rect2);
        rect
    }

    /// Makes this box the smallest box that contains both. The empty box adds nothing.
    pub fn union(&mut self, rect: Rect3D) {
        if self.is_empty() {
            *self = rect;
            return;
        }
        if rect.is_empty() {
            return;
        }
        let x = self.x.min(rect.x);
        let y = self.y.min(rect.y);
        let z = self.z.min(rect.z);
        self.size_x = (self.x + self.size_x).max(rect.x + rect.size_x) - x;
        self.size_y = (self.y + self.size_y).max(rect.y + rect.size_y) - y;
        self.size_z = (self.z + self.size_z).max(rect.z + rect.size_z) - z;
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// Makes this box the smallest box that contains it and the point.
    #[inline]
    pub fn union_point(&mut self, point: Point3D) {
        self.union(Rect3D::from_points(point, point));
    }

    /// The smallest box that contains both boxes (WPF's static `Rect3D.Union`).
    pub fn union_rects(rect1: Rect3D, rect2: Rect3D) -> Rect3D {
        let mut rect = rect1;
        rect.union(rect2);
        rect
    }

    /// The smallest box that contains the box and the point (WPF's static `Rect3D.Union` of a
    /// box and a point).
    pub fn union_rect_point(rect: Rect3D, point: Point3D) -> Rect3D {
        let mut rect = rect;
        rect.union_point(point);
        rect
    }

    /// Moves the box by the vector; the empty box cannot be moved.
    #[inline]
    pub fn offset(&mut self, offset: Vector3D) -> Result<(), Size3DError> {
        self.offset_xyz(offset.x, offset.y, offset.z)
    }

    /// Moves the box by the given amounts; the empty box cannot be moved.
    pub fn offset_xyz(&mut self, offset_x: f64, offset_y: f64, offset_z: f64) -> Result<(), Size3DError> {
        self.check_not_empty()?;
        self.x += offset_x;
        self.y += offset_y;
        self.z += offset_z;
        Ok(())
    }

    /// The box moved by the vector (WPF's static `Rect3D.Offset`); the empty box cannot be moved.
    pub fn offset_rect(rect: Rect3D, offset: Vector3D) -> Result<Rect3D, Size3DError> {
        let mut rect = rect;
        rect.offset(offset)?;
        Ok(rect)
    }

    fn check_not_empty(&self) -> Result<(), Size3DError> {
        if self.is_empty() {
            Err(Size3DError::ModifiesEmpty)
        } else {
            Ok(())
        }
    }

    fn checked_size(&self, value: f64) -> Result<f64, Size3DError> {
        self.check_not_empty()?;
        if value < 0.0 {
            return Err(Size3DError::NegativeDimension);
        }
        Ok(value)
    }

    /// Parses the text form: `"Empty"`, or `"x,y,z,sizeX,sizeY,sizeZ"` (commas, white space or
    /// both between the numbers) in the invariant culture, with no negative size.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        if is_keyword(text, "Empty") {
            return Ok(Rect3D::EMPTY);
        }
        let [x, y, z, size_x, size_y, size_z] = parse_numbers::<6>(text, "Invalid Rect3D.")?;
        Rect3D::new(x, y, z, size_x, size_y, size_z).map_err(|error| FormatError::new(error.message()))
    }
}

impl FromStr for Rect3D {
    type Err = FormatError;
    #[inline]
    fn from_str(text: &str) -> Result<Self, FormatError> {
        Rect3D::parse(text)
    }
}

/// The text form in the invariant culture: `"Empty"`, or `"x,y,z,sizeX,sizeY,sizeZ"`.
impl fmt::Display for Rect3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("Empty");
        }
        write_numbers(f, &[self.x, self.y, self.z, self.size_x, self.size_y, self.size_z])
    }
}

#[cfg(test)]
#[path = "rect_3d_tests.rs"]
mod tests;
