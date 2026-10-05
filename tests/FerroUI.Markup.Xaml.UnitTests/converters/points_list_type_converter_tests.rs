//! Port of `Converters/PointsListTypeConverterTests.cs`.
//!
//! Each theory is one table-driven test that reports the failing row. (The
//! static constructor of the original only forces the markup assembly to
//! load; here the test base registers the types.)

use ferroui_base::{Point, Ref};
use ferroui_controls::shapes::Polygon;
use ferroui_markup_xaml::converters::{PointsListTypeConverter, TypeConverter};

use crate::support::app::xaml_test_base;
use crate::support::helpers::{boxed, value_of};
use crate::support::loader::load_as;

const INPUTS: [&str; 4] = ["1,2 3,4", "1 2 3 4", "1 2,3 4", "1,2,3,4"];

#[test]
fn type_converter_should_parse() {
    for input in INPUTS {
        let _base = xaml_test_base();
        let conv = PointsListTypeConverter::new();

        let converted = conv
            .convert_from(None, None, Some(&boxed(input.to_string())))
            .unwrap_or_else(|error| panic!("row {input:?}: {}", error.message()));
        let points = value_of::<Vec<Point>>(&converted).unwrap_or_else(|| panic!("row {input:?}: not a list of points"));

        assert_eq!(2, points.len(), "row {input:?}");
        assert_eq!(Point::new(1.0, 2.0), points[0], "row {input:?}");
        assert_eq!(Point::new(3.0, 4.0), points[1], "row {input:?}");
    }
}

#[test]
fn should_parse_points_in_xaml() {
    for input in INPUTS {
        let _base = xaml_test_base();
        let xaml = format!("<Polygon xmlns='https://github.com/ferroui' Points='{input}' />");
        let polygon = load_as::<Ref<Polygon>>(&xaml);

        let points = polygon.points().unwrap_or_else(|| panic!("row {input:?}: the points are null"));

        assert_eq!(2, points.len(), "row {input:?}");
        assert_eq!(Point::new(1.0, 2.0), points.get(0), "row {input:?}");
        assert_eq!(Point::new(3.0, 4.0), points.get(1), "row {input:?}");
    }
}
