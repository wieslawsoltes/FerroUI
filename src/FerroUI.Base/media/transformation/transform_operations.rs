use crate::media::transformation::{
    InterpolationUtilities, OperationType, TransformOperation, TransformParser,
};
use crate::media::ITransform;
use crate::utilities::FormatError;
use crate::{Matrix, MatrixDecomposed};
use std::any::Any;
use std::rc::Rc;

/// Contains a list of [`TransformOperation`] that represent primitive
/// transforms that will be applied in declared order.
#[derive(Debug)]
pub struct TransformOperations {
    operations: Vec<TransformOperation>,
    is_identity: bool,
    value: Matrix,
}


/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for TransformOperations {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TransformOperations {
    /// The shared empty operation list.
    pub fn identity() -> Rc<TransformOperations> {
        thread_local! {
            static IDENTITY: Rc<TransformOperations> = Rc::new(TransformOperations::new(Vec::new()));
        }
        IDENTITY.with(Rc::clone)
    }

    fn new(operations: Vec<TransformOperation>) -> Self {
        let is_identity = operations.iter().all(TransformOperation::is_identity);
        let value = Self::apply_transforms(&operations, 0);
        Self { operations, is_identity, value }
    }

    /// Returns whether all operations combined together produce the identity
    /// matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.is_identity
    }

    /// The operations, in declared order.
    #[inline]
    pub fn operations(&self) -> &[TransformOperation] {
        &self.operations
    }

    /// The combined matrix of all operations.
    #[inline]
    pub fn value(&self) -> Matrix {
        self.value
    }

    /// Parses a transform string such as `scale(1.5) rotate(45deg)`.
    pub fn parse(s: &str) -> Result<Rc<TransformOperations>, FormatError> {
        TransformParser::parse(s)
    }

    /// Creates a builder with the given initial capacity.
    pub fn create_builder(capacity: usize) -> TransformOperationsBuilder {
        TransformOperationsBuilder::new(capacity)
    }

    /// Interpolates between two operation lists.
    pub fn interpolate(
        from: &Rc<TransformOperations>,
        to: &Rc<TransformOperations>,
        progress: f64,
    ) -> Rc<TransformOperations> {
        match Self::try_interpolate(from, to, progress) {
            Some(result) => result,
            // If the matrices cannot be interpolated, fallback to discrete animation logic.
            // See https://drafts.csswg.org/css-transforms/#matrix-interpolation
            None => {
                if progress < 0.5 {
                    from.clone()
                } else {
                    to.clone()
                }
            }
        }
    }

    fn apply_transforms(operations: &[TransformOperation], start_offset: usize) -> Matrix {
        let mut matrix = Matrix::IDENTITY;
        for operation in operations.iter().skip(start_offset) {
            matrix *= operation.matrix;
        }
        matrix
    }

    fn try_interpolate(
        from: &TransformOperations,
        to: &TransformOperations,
        progress: f64,
    ) -> Option<Rc<TransformOperations>> {
        let from_identity = from.is_identity;
        let to_identity = to.is_identity;

        if from_identity && to_identity {
            return Some(Self::identity());
        }

        let matching_prefix_length = Self::compute_matching_prefix_length(from, to);
        let from_size = if from_identity { 0 } else { from.operations.len() };
        let to_size = if to_identity { 0 } else { to.operations.len() };
        let num_operations = from_size.max(to_size);

        let mut builder = TransformOperationsBuilder::new(matching_prefix_length);

        for i in 0..matching_prefix_length {
            let mut interpolated =
                TransformOperation { type_: OperationType::Identity, ..TransformOperation::default() };

            if !TransformOperation::try_interpolate(
                if i >= from_size { None } else { Some(from.operations[i]) },
                if i >= to_size { None } else { Some(to.operations[i]) },
                progress,
                &mut interpolated,
            ) {
                return None;
            }

            builder.append(interpolated);
        }

        if matching_prefix_length < num_operations {
            let from_decomposed = Self::compute_decomposed_transform(from, matching_prefix_length)?;
            let to_decomposed = Self::compute_decomposed_transform(to, matching_prefix_length)?;

            let transform =
                InterpolationUtilities::interpolate_decomposed_transforms(&from_decomposed, &to_decomposed, progress);

            builder.append_matrix(InterpolationUtilities::compose_transform(transform));
        }

        Some(builder.build())
    }

    fn compute_decomposed_transform(operations: &TransformOperations, start_offset: usize) -> Option<MatrixDecomposed> {
        let transform = Self::apply_transforms(&operations.operations, start_offset);
        Matrix::try_decompose_transform(transform)
    }

    fn compute_matching_prefix_length(from: &TransformOperations, to: &TransformOperations) -> usize {
        let num_operations = from.operations.len().min(to.operations.len());

        for i in 0..num_operations {
            if from.operations[i].type_ != to.operations[i].type_ {
                return i;
            }
        }

        // If the operations match to the length of the shorter list, then pad its
        // length with the matching identity operations.
        // https://drafts.csswg.org/css-transforms/#transform-function-lists
        from.operations.len().max(to.operations.len())
    }
}

impl ITransform for TransformOperations {
    #[inline]
    fn value(&self) -> Matrix {
        self.value
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Builds a [`TransformOperations`] list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransformOperationsBuilder {
    operations: Vec<TransformOperation>,
}

impl TransformOperationsBuilder {
    pub fn new(capacity: usize) -> Self {
        Self { operations: Vec::with_capacity(capacity) }
    }

    pub fn append_translate(&mut self, x: f64, y: f64) {
        let mut to_add = TransformOperation { type_: OperationType::Translate, ..TransformOperation::default() };
        to_add.data.set_translate(x, y);
        to_add.bake();
        self.operations.push(to_add);
    }

    pub fn append_rotate(&mut self, angle: f64) {
        let mut to_add = TransformOperation { type_: OperationType::Rotate, ..TransformOperation::default() };
        to_add.data.set_rotate(angle);
        to_add.bake();
        self.operations.push(to_add);
    }

    pub fn append_scale(&mut self, x: f64, y: f64) {
        let mut to_add = TransformOperation { type_: OperationType::Scale, ..TransformOperation::default() };
        to_add.data.set_scale(x, y);
        to_add.bake();
        self.operations.push(to_add);
    }

    pub fn append_skew(&mut self, x: f64, y: f64) {
        let mut to_add = TransformOperation { type_: OperationType::Skew, ..TransformOperation::default() };
        to_add.data.set_skew(x, y);
        to_add.bake();
        self.operations.push(to_add);
    }

    pub fn append_matrix(&mut self, matrix: Matrix) {
        self.operations.push(TransformOperation {
            type_: OperationType::Matrix,
            matrix,
            ..TransformOperation::default()
        });
    }

    /// Appends an identity-typed operation. As in the reference
    /// implementation its matrix is left unbaked (all zeros).
    pub fn append_identity(&mut self) {
        self.operations.push(TransformOperation { type_: OperationType::Identity, ..TransformOperation::default() });
    }

    pub fn append(&mut self, to_add: TransformOperation) {
        self.operations.push(to_add);
    }

    pub fn build(self) -> Rc<TransformOperations> {
        Rc::new(TransformOperations::new(self.operations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{TransformGroup, TranslateTransform};
    use crate::utilities::MathUtilities;

    fn assert_close(expected: f64, actual: f64, precision: i32) {
        let scale = 10f64.powi(precision);
        assert_eq!((expected * scale).round(), (actual * scale).round(), "{expected} != {actual}");
    }

    #[test]
    fn can_parse_translation() {
        for (data, x, y) in [
            ("translate(10px)", 10.0, 0.0),
            ("translate(10px, 10px)", 10.0, 10.0),
            ("translate(0px, 10px)", 0.0, 10.0),
            ("translate(10px, 0px)", 10.0, 0.0),
            ("translateX(10px)", 10.0, 0.0),
            ("translateY(10px)", 0.0, 10.0),
        ] {
            let transform = TransformOperations::parse(data).unwrap();
            let operations = transform.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Translate, operations[0].type_);
            assert_eq!(x, operations[0].data.translate().x);
            assert_eq!(y, operations[0].data.translate().y);
        }
    }

    #[test]
    fn can_parse_rotation() {
        for (data, angle_deg) in [
            ("rotate(90deg)", 90.0),
            ("rotate(0.5turn)", 180.0),
            ("rotate(200grad)", 180.0),
            ("rotate(3.14159265rad)", 180.0),
        ] {
            let transform = TransformOperations::parse(data).unwrap();
            let operations = transform.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Rotate, operations[0].type_);
            assert_close(MathUtilities::deg2rad(angle_deg), operations[0].data.rotate().angle, 4);
        }
    }

    #[test]
    fn can_parse_scale() {
        for (data, x, y) in [
            ("scale(10)", 10.0, 10.0),
            ("scale(10, 10)", 10.0, 10.0),
            ("scale(0, 10)", 0.0, 10.0),
            ("scale(10, 0)", 10.0, 0.0),
            ("scaleX(10)", 10.0, 1.0),
            ("scaleY(10)", 1.0, 10.0),
        ] {
            let transform = TransformOperations::parse(data).unwrap();
            let operations = transform.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Scale, operations[0].type_);
            assert_eq!(x, operations[0].data.scale().x);
            assert_eq!(y, operations[0].data.scale().y);
        }
    }

    #[test]
    fn can_parse_skew() {
        for (data, x, y) in [
            ("skew(90deg)", 90.0, 0.0),
            ("skew(0.5turn)", 180.0, 0.0),
            ("skew(200grad)", 180.0, 0.0),
            ("skew(3.14159265rad)", 180.0, 0.0),
            ("skewX(90deg)", 90.0, 0.0),
            ("skewX(0.5turn)", 180.0, 0.0),
            ("skewX(200grad)", 180.0, 0.0),
            ("skewX(3.14159265rad)", 180.0, 0.0),
            ("skew(0, 90deg)", 0.0, 90.0),
            ("skew(0, 0.5turn)", 0.0, 180.0),
            ("skew(0, 200grad)", 0.0, 180.0),
            ("skew(0, 3.14159265rad)", 0.0, 180.0),
            ("skewY(90deg)", 0.0, 90.0),
            ("skewY(0.5turn)", 0.0, 180.0),
            ("skewY(200grad)", 0.0, 180.0),
            ("skewY(3.14159265rad)", 0.0, 180.0),
            ("skew(90deg, 90deg)", 90.0, 90.0),
            ("skew(0.5turn, 0.5turn)", 180.0, 180.0),
            ("skew(200grad, 200grad)", 180.0, 180.0),
            ("skew(3.14159265rad, 3.14159265rad)", 180.0, 180.0),
        ] {
            let transform = TransformOperations::parse(data).unwrap();
            let operations = transform.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Skew, operations[0].type_);
            assert_close(MathUtilities::deg2rad(x), operations[0].data.skew().x, 4);
            assert_close(MathUtilities::deg2rad(y), operations[0].data.skew().y, 4);
        }
    }

    #[test]
    fn can_parse_compound_operations() {
        let data = "scale(1,2) translate(3px,4px) rotate(5deg) skew(6deg,7deg)";
        let transform = TransformOperations::parse(data).unwrap();
        let operations = transform.operations();

        assert_eq!(OperationType::Scale, operations[0].type_);
        assert_eq!(1.0, operations[0].data.scale().x);
        assert_eq!(2.0, operations[0].data.scale().y);

        assert_eq!(OperationType::Translate, operations[1].type_);
        assert_eq!(3.0, operations[1].data.translate().x);
        assert_eq!(4.0, operations[1].data.translate().y);

        assert_eq!(OperationType::Rotate, operations[2].type_);
        assert_eq!(MathUtilities::deg2rad(5.0), operations[2].data.rotate().angle);

        assert_eq!(OperationType::Skew, operations[3].type_);
        assert_eq!(MathUtilities::deg2rad(6.0), operations[3].data.skew().x);
        assert_eq!(MathUtilities::deg2rad(7.0), operations[3].data.skew().y);
    }

    #[test]
    fn can_parse_matrix_operation() {
        let transform = TransformOperations::parse("matrix(1,2,3,4,5,6)").unwrap();
        let operations = transform.operations();
        assert_eq!(1, operations.len());
        assert_eq!(OperationType::Matrix, operations[0].type_);
        assert_eq!(Matrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), operations[0].matrix);
    }

    #[test]
    fn can_interpolate_translation() {
        for (progress, x, y) in [(0.0, 10.0, 0.0), (0.5, 5.0, 10.0), (1.0, 0.0, 20.0)] {
            let from = TransformOperations::parse("translateX(10px)").unwrap();
            let to = TransformOperations::parse("translateY(20px)").unwrap();
            let interpolated = TransformOperations::interpolate(&from, &to, progress);
            let operations = interpolated.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Translate, operations[0].type_);
            assert_eq!(x, operations[0].data.translate().x);
            assert_eq!(y, operations[0].data.translate().y);
        }
    }

    #[test]
    fn can_interpolate_scale() {
        for (progress, x, y) in [(0.0, 10.0, 1.0), (0.5, 5.5, 10.5), (1.0, 1.0, 20.0)] {
            let from = TransformOperations::parse("scaleX(10)").unwrap();
            let to = TransformOperations::parse("scaleY(20)").unwrap();
            let interpolated = TransformOperations::interpolate(&from, &to, progress);
            let operations = interpolated.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Scale, operations[0].type_);
            assert_eq!(x, operations[0].data.scale().x);
            assert_eq!(y, operations[0].data.scale().y);
        }
    }

    #[test]
    fn can_interpolate_skew() {
        for (progress, x, y) in [(0.0, 10.0, 0.0), (0.5, 5.0, 10.0), (1.0, 0.0, 20.0)] {
            let from = TransformOperations::parse("skewX(10deg)").unwrap();
            let to = TransformOperations::parse("skewY(20deg)").unwrap();
            let interpolated = TransformOperations::interpolate(&from, &to, progress);
            let operations = interpolated.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Skew, operations[0].type_);
            assert_eq!(MathUtilities::deg2rad(x), operations[0].data.skew().x);
            assert_eq!(MathUtilities::deg2rad(y), operations[0].data.skew().y);
        }
    }

    #[test]
    fn can_interpolate_rotation() {
        for (progress, angle) in [(0.0, 10.0), (0.5, 15.0), (1.0, 20.0)] {
            let from = TransformOperations::parse("rotate(10deg)").unwrap();
            let to = TransformOperations::parse("rotate(20deg)").unwrap();
            let interpolated = TransformOperations::interpolate(&from, &to, progress);
            let operations = interpolated.operations();
            assert_eq!(1, operations.len());
            assert_eq!(OperationType::Rotate, operations[0].type_);
            assert_eq!(MathUtilities::deg2rad(angle), operations[0].data.rotate().angle);
        }
    }

    #[test]
    fn interpolation_fallback_to_matrix() {
        let from = TransformOperations::parse("rotate(45deg)").unwrap();
        let to = TransformOperations::parse("translate(100px, 100px) rotate(1215deg)").unwrap();
        let interpolated = TransformOperations::interpolate(&from, &to, 0.5);
        let operations = interpolated.operations();
        assert_eq!(1, operations.len());
        assert_eq!(OperationType::Matrix, operations[0].type_);
    }

    fn assert_matrix(matrix: Matrix, scale_x: f64, scale_y: f64, translate_x: f64, translate_y: f64) {
        let composed = Matrix::try_decompose_transform(matrix).unwrap();
        assert_eq!(scale_x, composed.scale.x);
        assert_eq!(scale_y, composed.scale.y);
        assert_eq!(translate_x, composed.translate.x);
        assert_eq!(translate_y, composed.translate.y);
    }

    #[test]
    fn order_of_operations_is_preserved_no_prefix() {
        let from = TransformOperations::parse("scale(1)").unwrap();
        let to = TransformOperations::parse("translate(50px,50px) scale(0.5,0.5)").unwrap();

        let interpolated_0 = TransformOperations::interpolate(&from, &to, 0.0);
        assert!(interpolated_0.is_identity());

        let interpolated_50 = TransformOperations::interpolate(&from, &to, 0.5);
        assert_matrix(interpolated_50.value(), 0.75, 0.75, 12.5, 12.5);

        let interpolated_100 = TransformOperations::interpolate(&from, &to, 1.0);
        assert_matrix(interpolated_100.value(), 0.5, 0.5, 25.0, 25.0);
    }

    #[test]
    fn order_of_operations_is_preserved_one_prefix() {
        let from = TransformOperations::parse("scale(1)").unwrap();
        let to = TransformOperations::parse("scale(0.5,0.5) translate(50px,50px)").unwrap();

        let interpolated_0 = TransformOperations::interpolate(&from, &to, 0.0);
        assert!(interpolated_0.is_identity());

        let interpolated_50 = TransformOperations::interpolate(&from, &to, 0.5);
        assert_matrix(interpolated_50.value(), 0.75, 0.75, 25.0, 25.0);

        let interpolated_100 = TransformOperations::interpolate(&from, &to, 1.0);
        assert_matrix(interpolated_100.value(), 0.5, 0.5, 50.0, 50.0);
    }

    #[test]
    fn transform_group_invalidates_when_child_collection_changes() {
        let group = TransformGroup::new();
        let transform = TranslateTransform::with_offset(10.0, 0.0);
        assert_eq!(Matrix::IDENTITY, group.value());
        group.children().add(transform.clone().upcast());
        assert_ne!(Matrix::IDENTITY, group.value());
        group.children().clear();
        assert_eq!(Matrix::IDENTITY, group.value());
        group.set_children(crate::media::Transforms::from_items([transform.upcast()]));
        assert_ne!(Matrix::IDENTITY, group.value());
    }

    #[test]
    fn parse_rejects_invalid_input() {
        for data in ["", "foo(1)", "rotate(10)", "rotate(10px)", "translate(10deg)", "scaleX(1,2)", "matrix(1,2,3)",
            "matrix(1,2,3,4,5,6,7)", "scale(1", "scale 1)", "scale(abc)", "rotate(1foo)"]
        {
            assert!(TransformOperations::parse(data).is_err(), "{data}");
        }
        assert!(TransformOperations::parse(" NONE ").unwrap().is_identity());
    }
}
