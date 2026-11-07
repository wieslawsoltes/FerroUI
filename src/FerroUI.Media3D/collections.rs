//! The collections of the scene model: WPF's `Point3DCollection`, `Vector3DCollection` and
//! `Int32Collection` (with their text forms) and the collections of the scene objects
//! (`Model3DCollection`, `Transform3DCollection`, `MaterialCollection`, `Visual3DCollection`).
//!
//! Each is a shared handle to an observable list ([`MediaCollection`]), the form the collections
//! of the 2D media classes have: cloning the handle shares the list, equality is identity, and
//! the owner of a collection follows its changes. The texture coordinates of a mesh are the
//! base library's [`Points`](ferroui_base::media::Points), WPF's `PointCollection`.

use crate::text::{parse_integer_list, parse_number_list};
use crate::{Material, Model3D, Point3D, Transform3D, Vector3D, Visual3D};
use ferroui_base::media::MediaCollection;
use ferroui_base::utilities::FormatError;
use ferroui_base::Ref;
use std::fmt;
use std::ops::Deref;

/// Declares a named collection type over a [`MediaCollection`].
macro_rules! collection_type {
    ($(#[$meta:meta])* $name:ident, $item:ty) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq)]
        pub struct $name(MediaCollection<$item>);

        impl Deref for $name {
            type Target = MediaCollection<$item>;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl FromIterator<$item> for $name {
            fn from_iter<I: IntoIterator<Item = $item>>(iter: I) -> Self {
                Self::from_items(iter)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }

        impl $name {
            /// An empty collection.
            pub fn new() -> Self {
                Self(MediaCollection::new())
            }

            /// A collection holding `items`.
            pub fn from_items(items: impl IntoIterator<Item = $item>) -> Self {
                Self(MediaCollection::from_items(items))
            }
        }
    };
}

collection_type!(
    /// WPF's `Point3DCollection`: the positions of a mesh. Its text form is the points separated
    /// by white space, each `x,y,z` (`"0,0,0 1,0,0 0,1,0"`).
    Point3DCollection, Point3D
);

collection_type!(
    /// WPF's `Vector3DCollection`: the normals of a mesh, in the text form of
    /// [`Point3DCollection`].
    Vector3DCollection, Vector3D
);

collection_type!(
    /// WPF's `Int32Collection`: the triangle indices of a mesh. Its text form is the numbers
    /// separated by white space or commas (`"0 1 2"`, `"0,1,2"`).
    Int32Collection, i32
);

collection_type!(
    /// WPF's `Model3DCollection`: the children of a `Model3DGroup`.
    Model3DCollection, Ref<Model3D>
);

collection_type!(
    /// WPF's `Transform3DCollection`: the children of a `Transform3DGroup`.
    Transform3DCollection, Ref<Transform3D>
);

collection_type!(
    /// WPF's `MaterialCollection`: the children of a `MaterialGroup`.
    MaterialCollection, Ref<Material>
);

collection_type!(
    /// WPF's `Visual3DCollection`: the children of a `ModelVisual3D`, a `Viewport3DVisual` and a
    /// `Viewport3D`.
    Visual3DCollection, Ref<Visual3D>
);

const INVALID_POINT_3D_COLLECTION: &str = "Invalid Point3DCollection.";
const INVALID_VECTOR_3D_COLLECTION: &str = "Invalid Vector3DCollection.";
const INVALID_INT32_COLLECTION: &str = "Invalid Int32Collection.";

/// The numbers of a list of 3D values, three at a time.
fn triples(text: &str, message: &'static str) -> Result<Vec<[f64; 3]>, FormatError> {
    let numbers = parse_number_list(text, message)?;
    if numbers.len() % 3 != 0 {
        return Err(FormatError::new(message));
    }
    Ok(numbers.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

/// Writes the items separated by a space.
fn write_items<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: impl Iterator<Item = T>) -> fmt::Result {
    for (index, item) in items.enumerate() {
        if index > 0 {
            f.write_str(" ")?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

impl Point3DCollection {
    /// WPF's `Point3DCollection.Parse` in the invariant culture: three numbers per point,
    /// separated by commas, white space or both; an empty text is an empty collection.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let points = triples(text, INVALID_POINT_3D_COLLECTION)?;
        Ok(Self::from_items(points.into_iter().map(|[x, y, z]| Point3D::new(x, y, z))))
    }
}

impl fmt::Display for Point3DCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_items(f, self.iter())
    }
}

impl std::str::FromStr for Point3DCollection {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Vector3DCollection {
    /// WPF's `Vector3DCollection.Parse` in the invariant culture, as [`Point3DCollection::parse`].
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        let vectors = triples(text, INVALID_VECTOR_3D_COLLECTION)?;
        Ok(Self::from_items(vectors.into_iter().map(|[x, y, z]| Vector3D::new(x, y, z))))
    }
}

impl fmt::Display for Vector3DCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_items(f, self.iter())
    }
}

impl std::str::FromStr for Vector3DCollection {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Int32Collection {
    /// WPF's `Int32Collection.Parse` in the invariant culture: integers separated by commas,
    /// white space or both; an empty text is an empty collection.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        Ok(Self::from_items(parse_integer_list(text, INVALID_INT32_COLLECTION)?))
    }
}

impl fmt::Display for Int32Collection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_items(f, self.iter())
    }
}

impl std::str::FromStr for Int32Collection {
    type Err = FormatError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

#[cfg(test)]
#[path = "collections_tests.rs"]
mod tests;
