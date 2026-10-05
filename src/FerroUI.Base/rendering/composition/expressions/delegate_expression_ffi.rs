//! Foreign function interface for composition animations based on calling closures.

use std::collections::HashMap;
use std::sync::Arc;

use super::expression_evaluation_context::IExpressionForeignFunctionInterface;
use super::expression_variant::{ExpressionVariant, ExpressionVariantValue, VariantType};

/// A registered function taking its arguments as variants.
type FfiDelegate = Arc<dyn Fn(&[ExpressionVariant]) -> ExpressionVariant + Send + Sync>;

struct FfiRecord {
    types: Vec<VariantType>,
    delegate: FfiDelegate,
}

/// Foreign function interface for composition animations based on calling closures.
///
/// Functions are overloaded by name, argument count and argument types. A
/// call first looks for an overload whose parameter types are exactly the
/// argument types, in registration order; then for one reachable by widening
/// single-precision vector arguments to the double-precision vector types;
/// then for one reachable by converting between the two vector families in
/// either direction.
#[derive(Default)]
pub struct DelegateExpressionFfi {
    registry: HashMap<String, HashMap<usize, Vec<FfiRecord>>>,
}

/// The variant type a parameter of type `T` is matched against.
///
/// # Panics
/// Panics when `T` cannot be a foreign function parameter (a programmer error).
fn type_of<T: ExpressionVariantValue>() -> VariantType {
    match T::FFI_VARIANT_TYPE {
        Some(t) => t,
        None => panic!(
            "{} cannot be a parameter of an expression foreign function",
            std::any::type_name::<T>()
        ),
    }
}

/// Generates a typed `add` overload. Each parameter is given with the index
/// of the argument it is read from.
macro_rules! add_overload {
    ($(#[$meta:meta])* $name:ident; $($t:ident => $index:expr),+) => {
        $(#[$meta])*
        pub fn $name<$($t,)+ R, F>(&mut self, name: &str, cb: F)
        where
            $($t: ExpressionVariantValue,)+
            R: Into<ExpressionVariant>,
            F: Fn($($t),+) -> R + Send + Sync + 'static,
        {
            self.add(
                name,
                Arc::new(move |args: &[ExpressionVariant]| {
                    cb($(args[$index].cast_or_default::<$t>()),+).into()
                }),
                vec![$(type_of::<$t>()),+],
            );
        }
    };
}

impl DelegateExpressionFfi {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    fn call_with_cast(
        count_group: &[FfiRecord],
        arguments: &[ExpressionVariant],
        any_cast: bool,
    ) -> Option<ExpressionVariant> {
        for record in count_group {
            let mut matches = true;
            for (c, argument) in arguments.iter().enumerate() {
                let parameter = record.types[c];
                let arg = argument.variant_type();
                if parameter != arg {
                    let can_cast = (parameter == VariantType::Vector3D && arg == VariantType::Vector3)
                        || (parameter == VariantType::Vector && arg == VariantType::Vector2)
                        || (any_cast
                            && ((arg == VariantType::Vector3D && parameter == VariantType::Vector3)
                                || (arg == VariantType::Vector && parameter == VariantType::Vector2)));
                    if !can_cast {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                return Some((record.delegate)(arguments));
            }
        }

        if !any_cast {
            return Self::call_with_cast(count_group, arguments, true);
        }
        None
    }

    fn add(&mut self, name: &str, cb: FfiDelegate, types: Vec<VariantType>) {
        self.registry
            .entry(name.to_owned())
            .or_default()
            .entry(types.len())
            .or_default()
            .push(FfiRecord { types, delegate: cb });
    }

    /// Registers a function without parameters.
    pub fn add0<R, F>(&mut self, name: &str, cb: F)
    where
        R: Into<ExpressionVariant>,
        F: Fn() -> R + Send + Sync + 'static,
    {
        self.add(name, Arc::new(move |_: &[ExpressionVariant]| cb().into()), Vec::new());
    }

    add_overload!(
        /// Registers a function with one parameter.
        add1; T1 => 0
    );
    add_overload!(
        /// Registers a function with two parameters.
        add2; T1 => 0, T2 => 1
    );
    add_overload!(
        /// Registers a function with three parameters.
        add3; T1 => 0, T2 => 1, T3 => 2
    );
    add_overload!(
        /// Registers a function with four parameters.
        add4; T1 => 0, T2 => 1, T3 => 2, T4 => 3
    );
    add_overload!(
        /// Registers a function with five parameters.
        add5; T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4
    );
    add_overload!(
        /// Registers a function with six parameters.
        ///
        /// As in the reference implementation, the sixth parameter is read
        /// from the fifth argument.
        add6; T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 4
    );
    add_overload!(
        /// Registers a function with sixteen parameters.
        ///
        /// As in the reference implementation, the sixth to sixteenth
        /// parameters are read from the fifth argument.
        add16;
        T1 => 0, T2 => 1, T3 => 2, T4 => 3,
        T5 => 4, T6 => 4, T7 => 4, T8 => 4,
        T9 => 4, T10 => 4, T11 => 4, T12 => 4,
        T13 => 4, T14 => 4, T15 => 4, T16 => 4
    );
}

impl IExpressionForeignFunctionInterface for DelegateExpressionFfi {
    fn call(&self, name: &str, arguments: &[ExpressionVariant]) -> Option<ExpressionVariant> {
        let name_group = self.registry.get(name)?;
        let count_group = name_group.get(&arguments.len())?;
        for record in count_group {
            let mut matches = true;
            for (c, argument) in arguments.iter().enumerate() {
                if record.types[c] != argument.variant_type() {
                    matches = false;
                    break;
                }
            }

            if matches {
                return Some((record.delegate)(arguments));
            }
        }

        Self::call_with_cast(count_group, arguments, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerics::{Vector2, Vector3};
    use crate::{Matrix, Vector, Vector3D};

    fn call(ffi: &DelegateExpressionFfi, name: &str, args: &[ExpressionVariant]) -> Option<ExpressionVariant> {
        ffi.call(name, args)
    }

    #[test]
    fn selects_the_overload_by_name_count_and_types() {
        let mut ffi = DelegateExpressionFfi::new();
        ffi.add0("F", || 0.0f32);
        ffi.add1("F", |a: f32| a + 1.0);
        ffi.add1("F", |a: Vector2| a.x + a.y);
        ffi.add2("F", |a: f32, b: bool| if b { a } else { -a });

        assert_eq!(call(&ffi, "F", &[]), Some(0.0.into()));
        assert_eq!(call(&ffi, "F", &[2.0.into()]), Some(3.0.into()));
        assert_eq!(call(&ffi, "F", &[Vector2::new(1.0, 2.0).into()]), Some(3.0.into()));
        assert_eq!(call(&ffi, "F", &[2.0.into(), false.into()]), Some((-2.0).into()));
        assert_eq!(call(&ffi, "F", &[true.into()]), None);
        assert_eq!(call(&ffi, "F", &[2.0.into(), 2.0.into()]), None);
        assert_eq!(call(&ffi, "F", &[2.0.into(), false.into(), 1.0.into()]), None);
        assert_eq!(call(&ffi, "G", &[]), None);
        assert_eq!(call(&ffi, "f", &[]), None);
    }

    #[test]
    fn the_first_registered_of_equal_signatures_wins() {
        let mut ffi = DelegateExpressionFfi::new();
        ffi.add1("F", |a: f32| a);
        ffi.add1("F", |a: f64| a * 2.0);
        // Both are matched against a Double argument; the single-precision
        // overload narrows the argument.
        assert_eq!(call(&ffi, "F", &[0.1.into()]), Some((0.1f32 as f64).into()));
    }

    #[test]
    fn widens_single_precision_vectors_before_narrowing() {
        let mut ffi = DelegateExpressionFfi::new();
        ffi.add1("Wide", |a: Vector| a.x);
        ffi.add1("Wide3", |a: Vector3D| a.z);
        ffi.add1("Narrow", |a: Vector2| a.x);
        ffi.add1("Narrow3", |a: Vector3| a.z);
        ffi.add2("Both", |a: Vector2, _b: Vector2| a.x);
        ffi.add2("Both", |a: Vector, _b: Vector| a.x + 100.0);

        assert_eq!(call(&ffi, "Wide", &[Vector2::new(1.5, 0.0).into()]), Some(1.5.into()));
        assert_eq!(
            call(&ffi, "Wide3", &[Vector3::new(0.0, 0.0, 2.5).into()]),
            Some(2.5.into())
        );
        assert_eq!(call(&ffi, "Narrow", &[Vector::new(1.5, 0.0).into()]), Some(1.5.into()));
        assert_eq!(
            call(&ffi, "Narrow3", &[Vector3D::new(0.0, 0.0, 2.5).into()]),
            Some(2.5.into())
        );
        // Mixed arguments: only the widening overload is reachable in the first pass.
        assert_eq!(
            call(
                &ffi,
                "Both",
                &[Vector2::new(1.0, 0.0).into(), Vector::new(0.0, 0.0).into()]
            ),
            Some(101.0.into())
        );
        assert_eq!(call(&ffi, "Wide", &[Vector3::new(1.0, 2.0, 3.0).into()]), None);
    }

    #[test]
    fn parameters_past_the_fifth_read_the_fifth_argument() {
        let mut ffi = DelegateExpressionFfi::new();
        ffi.add5("F", |a: f32, b: f32, c: f32, d: f32, e: f32| {
            a + 10.0 * b + 100.0 * c + 1000.0 * d + 10000.0 * e
        });
        ffi.add6("F", |_a: f32, _b: f32, _c: f32, _d: f32, e: f32, f: f32| e * 10.0 + f);
        let args: Vec<ExpressionVariant> = (1..=6).map(|v| (v as f64).into()).collect();
        assert_eq!(call(&ffi, "F", &args[..5]), Some(54321.0.into()));
        assert_eq!(call(&ffi, "F", &args), Some(55.0.into()));
    }

    #[test]
    #[should_panic(expected = "cannot be a parameter")]
    fn the_3x3_matrix_cannot_be_a_parameter() {
        let mut ffi = DelegateExpressionFfi::new();
        ffi.add1("F", |m: Matrix| m);
    }

    #[test]
    fn is_shareable_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DelegateExpressionFfi>();
    }
}
