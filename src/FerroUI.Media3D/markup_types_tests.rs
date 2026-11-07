//! Tests of the markup metadata: each value parses its text form, constructs and reads its
//! components through the metadata alone, as the markup loader uses it.

use super::*;
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupValue};

fn text(value: &str) -> MarkupValue {
    into_markup_value(value.to_string())
}

fn unbox<T: Clone + 'static>(value: &MarkupValue) -> T {
    from_markup_value::<T>(value).unwrap_or_else(|| panic!("a value of type {}", std::any::type_name::<T>()))
}

fn parse<T: Clone + 'static>(markup: &MarkupType, value: &str) -> T {
    let parse = markup.parse.unwrap_or_else(|| panic!("{} parses", markup.full_name()));
    unbox::<T>(&parse(&[text(value)]).unwrap_or_else(|error| panic!("{}: {error}", markup.full_name())))
}

fn get<T: Clone + 'static>(markup: &MarkupType, value: &MarkupValue, property: &str) -> T {
    let property = markup.find_property(property).unwrap_or_else(|| panic!("{property}"));
    assert!(property.set.is_none(), "{}: the components are read-only", property.name);
    unbox::<T>(&(property.get.unwrap())(std::slice::from_ref(value)).unwrap())
}

fn get_static<T: Clone + 'static>(markup: &MarkupType, property: &str) -> T {
    let property = markup.find_static_property(property).unwrap_or_else(|| panic!("{property}"));
    unbox::<T>(&(property.get.unwrap())(&[]).unwrap())
}

#[test]
fn every_value_is_in_the_media3d_namespace_and_parses_its_text_form() {
    register_types();
    let samples = [
        ("FerroUI.Media.Media3D.Point3D", "0,0,5"),
        ("FerroUI.Media.Media3D.Vector3D", "0 0 -1"),
        ("FerroUI.Media.Media3D.Size3D", "Empty"),
        ("FerroUI.Media.Media3D.Rect3D", "0,0,0,1,1,1"),
        ("FerroUI.Media.Media3D.Point4D", "1,2,3,1"),
        ("FerroUI.Media.Media3D.Quaternion", "Identity"),
        ("FerroUI.Media.Media3D.Matrix3D", "Identity"),
    ];
    assert_eq!(TYPES.len(), samples.len());
    for (markup, (name, sample)) in TYPES.iter().zip(samples) {
        assert_eq!(markup.full_name(), name);
        let parse = markup.parse.unwrap();
        assert!(parse(&[text(sample)]).is_ok(), "{name}: {sample}");
        assert!(parse(&[text("not a value")]).is_err(), "{name}");
        assert!(parse(&[into_markup_value(1i32)]).is_err(), "{name}: only text is parsed");
    }
}

#[test]
fn points_and_vectors_parse_and_read_their_components() {
    let point = <Point3D as MarkupTyped>::MARKUP;
    assert_eq!(parse::<Point3D>(point, "0,0,5"), Point3D::new(0.0, 0.0, 5.0));
    let value = into_markup_value(Point3D::new(1.0, 2.0, 3.0));
    assert_eq!(get::<f64>(point, &value, "Z"), 3.0);
    let constructor = point.constructors.iter().find(|c| c.parameters.len() == 3).unwrap();
    let constructed = (constructor.invoke)(&[into_markup_value(1.0f64), into_markup_value(2.0f64), into_markup_value(3.0f64)]);
    assert_eq!(unbox::<Point3D>(&constructed.unwrap()), Point3D::new(1.0, 2.0, 3.0));
    // The nullable form is recorded (a property of type `Option<Point3D>` takes the text too).
    assert!(point.nullable.is_some());

    let vector = <Vector3D as MarkupTyped>::MARKUP;
    let value = into_markup_value(parse::<Vector3D>(vector, "0, 3, -4"));
    assert_eq!(get::<f64>(vector, &value, "Length"), 5.0);
    assert_eq!(get::<f64>(vector, &value, "LengthSquared"), 25.0);

    let point4 = <Point4D as MarkupTyped>::MARKUP;
    assert_eq!(parse::<Point4D>(point4, "1 2 3 4"), Point4D::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn sizes_and_boxes_have_empty_and_guarded_constructors() {
    let size = <Size3D as MarkupTyped>::MARKUP;
    assert_eq!(parse::<Size3D>(size, "Empty"), Size3D::EMPTY);
    assert_eq!(get_static::<Size3D>(size, "Empty"), Size3D::EMPTY);
    let constructor = size.constructors.iter().find(|c| c.parameters.len() == 3).unwrap();
    let negative = [into_markup_value(1.0f64), into_markup_value(-1.0f64), into_markup_value(1.0f64)];
    assert!((constructor.invoke)(&negative).is_err(), "a negative dimension is refused");
    assert!((size.parse.unwrap())(&[text("1,-1,1")]).is_err());

    let rect = <Rect3D as MarkupTyped>::MARKUP;
    let value = into_markup_value(parse::<Rect3D>(rect, "1,2,3,4,5,6"));
    assert_eq!(get::<f64>(rect, &value, "SizeZ"), 6.0);
    assert_eq!(get::<Point3D>(rect, &value, "Location"), Point3D::new(1.0, 2.0, 3.0));
    assert!(!get::<bool>(rect, &value, "IsEmpty"));
    assert_eq!(get_static::<Rect3D>(rect, "Empty"), Rect3D::EMPTY);
    assert_eq!(parse::<Rect3D>(rect, "Empty"), Rect3D::EMPTY);
}

#[test]
fn matrices_and_quaternions_have_identity() {
    let matrix = <Matrix3D as MarkupTyped>::MARKUP;
    assert_eq!(parse::<Matrix3D>(matrix, "Identity"), Matrix3D::IDENTITY);
    assert_eq!(get_static::<Matrix3D>(matrix, "Identity"), Matrix3D::IDENTITY);
    let value = into_markup_value(parse::<Matrix3D>(matrix, "2,0,0,0 0,2,0,0 0,0,2,0 1,2,3,1"));
    assert_eq!(get::<f64>(matrix, &value, "OffsetY"), 2.0);
    assert_eq!(get::<f64>(matrix, &value, "Determinant"), 8.0);
    assert!(get::<bool>(matrix, &value, "IsAffine"));
    assert!(get::<bool>(matrix, &value, "HasInverse"));
    assert!(!get::<bool>(matrix, &value, "IsIdentity"));

    let quaternion = <Quaternion as MarkupTyped>::MARKUP;
    assert_eq!(parse::<Quaternion>(quaternion, "Identity"), Quaternion::IDENTITY);
    assert_eq!(get_static::<Quaternion>(quaternion, "Identity"), Quaternion::IDENTITY);
    let axis_angle = quaternion
        .constructors
        .iter()
        .find(|c| c.parameters.len() == 2)
        .unwrap();
    let quarter = (axis_angle.invoke)(&[into_markup_value(Vector3D::new(0.0, 0.0, 1.0)), into_markup_value(90.0f64)]).unwrap();
    assert!((get::<f64>(quaternion, &quarter, "Angle") - 90.0).abs() < 1e-12);
    let zero_axis = [into_markup_value(Vector3D::default()), into_markup_value(90.0f64)];
    assert!((axis_angle.invoke)(&zero_axis).is_err(), "the zero axis is refused");
}
