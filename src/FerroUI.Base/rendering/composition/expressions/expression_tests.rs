//! Tests of expression parsing and evaluation: the reference parser test
//! (`evaluates_expression_correctly`), the expression parts of the reference
//! animation tests (target and parameter access through the object seams),
//! and tests specific to this port.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::{Matrix, RelativePoint, RelativeScalar, RelativeUnit, Vector, Vector3D};

use ExpressionVariant as V;

fn context<'a>() -> ExpressionEvaluationContext<'a> {
    ExpressionEvaluationContext {
        foreign_function_interface: Some(BuiltInExpressionFfi::instance()),
        ..Default::default()
    }
}

#[track_caller]
fn eval(expression: &str) -> ExpressionVariant {
    let expr = ExpressionParser::parse(expression).unwrap();
    expr.evaluate(&context())
}

#[track_caller]
fn eval_double(expression: &str) -> f64 {
    match eval(expression) {
        V::Double(value) => value,
        other => panic!("Invalid result type: {:?}", other.variant_type()),
    }
}

#[track_caller]
fn parse_error(expression: &str) -> (String, i32) {
    let error = ExpressionParser::parse(expression).unwrap_err();
    (error.message().to_owned(), error.position())
}

fn d(value: f64) -> ExpressionVariant {
    V::Double(value)
}

/// A property bag standing in for a server object.
#[derive(Default)]
struct TestObject {
    properties: HashMap<&'static str, ExpressionVariant>,
}

impl TestObject {
    fn with(mut self, name: &'static str, value: impl Into<ExpressionVariant>) -> Self {
        self.properties.insert(name, value.into());
        self
    }
}

impl IExpressionObject for TestObject {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        self.properties.get(name).copied().unwrap_or_default()
    }
}

/// Parameters standing in for a property set snapshot.
#[derive(Default)]
struct TestParameters {
    values: HashMap<&'static str, ExpressionVariant>,
    objects: HashMap<&'static str, TestObject>,
}

impl IExpressionParameterCollection for TestParameters {
    fn get_parameter(&self, name: &str) -> ExpressionVariant {
        self.values.get(name).copied().unwrap_or_default()
    }

    fn get_object_parameter(&self, name: &str) -> Option<&dyn IExpressionObject> {
        self.objects.get(name).map(|o| o as &dyn IExpressionObject)
    }
}

// ---------------------------------------------------------------------------
// Reference parser test
// ---------------------------------------------------------------------------

fn evaluates_expression_correctly_case(expression: &str, value: f64) {
    let expr = ExpressionParser::parse(expression).unwrap();
    let ctx = ExpressionEvaluationContext {
        foreign_function_interface: Some(BuiltInExpressionFfi::instance()),
        ..Default::default()
    };
    let res = expr.evaluate(&ctx);
    let double_res = match res {
        V::Double(v) => v,
        _ => panic!("Invalid result type: {:?}", res.variant_type()),
    };
    assert_eq!(value, double_res);
}

#[test]
fn evaluates_expression_correctly() {
    evaluates_expression_correctly_case("Vector3(0.5+(4.0-0.5)* -2, 1, 0).X", -6.5);
}

// ---------------------------------------------------------------------------
// Expression parts of the reference animation tests
// ---------------------------------------------------------------------------

#[test]
fn expression_operations_work_on_the_target() {
    let target = TestObject::default().with("Offset", Vector3D::new(20.0, 0.0, 0.0));
    let expr = Expression::parse("this.Target.Offset.X * 0.5 + 10").unwrap();
    let ctx = ExpressionEvaluationContext {
        target: Some(&target),
        ..context()
    };
    assert_eq!(expr.evaluate(&ctx), d(20.0));
}

#[test]
fn expression_reads_reference_parameters() {
    let mut parameters = TestParameters::default();
    parameters.objects.insert(
        "obj",
        TestObject::default().with("Offset", Vector3D::new(20.0, 0.0, 0.0)),
    );
    let expr = Expression::parse("obj.Offset.X * 0.5 + 10").unwrap();
    {
        let ctx = ExpressionEvaluationContext {
            parameters: Some(&parameters),
            ..context()
        };
        assert_eq!(expr.evaluate(&ctx), d(20.0));
    }

    // The expression sees the new value of the object on the next evaluation.
    parameters.objects.insert(
        "obj",
        TestObject::default().with("Offset", Vector3D::new(40.0, 0.0, 0.0)),
    );
    let ctx = ExpressionEvaluationContext {
        parameters: Some(&parameters),
        ..context()
    };
    assert_eq!(expr.evaluate(&ctx), d(30.0));

    let mut references = HashSet::new();
    expr.collect_references(&mut references);
    assert_eq!(references, HashSet::from([("obj".to_owned(), "Offset".to_owned())]));
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

#[test]
fn prints_the_parsed_tree() {
    for (expression, printed) in [
        ("a.B + 2 * this.Target.X", "(({a}).B+(2*([Target]).X))"),
        ("a > 1 ? 2 : 3", "(({a}>1)) ? (2) : (3)"),
        ("Min(1, x)", "Min( (1), ({x}) )"),
        ("Foo()", "Foo( () )"),
        ("Matrix3x2.CreateScale ( v )", "Matrix3x2.CreateScale( ({v}) )"),
        ("!a", "!{a}"),
        ("-a", "-{a}"),
        ("-2", "-2"),
        ("- - 2", "2"),
        ("!!a", "{a}"),
        ("!-a", "-!{a}"),
        ("0.5", "0.5"),
        ("this.StartingValue", "[StartingValue]"),
        ("THIS.CurrentValue", "[CurrentValue]"),
        ("this.finalvalue", "[FinalValue]"),
        ("Pi", "[Pi]"),
        ("true || FALSE", "([True]||[False])"),
        ("RelativeUnit.Relative", "RelativeUnit.Relative( () )"),
        ("relativeunit.absolute", "RelativeUnit.Absolute( () )"),
        ("a >= b && c <= d || e != f == g", "((({a}>={b})&&({c}<={d}))||({e}!=({f}Equals{g})))"),
        ("a % b", "({a}%{b})"),
        ("(a)", "{a}"),
        ("pixel.pi", "({pixel}).pi"),
        ("this.X", "({this}).X"),
        ("  1  ", "1"),
        // A trailing comma in an argument list is accepted.
        ("Max(1,)", "Max( (1) )"),
    ] {
        assert_eq!(Expression::parse(expression).unwrap().to_string(), printed, "{expression}");
    }
}

#[test]
fn operator_names() {
    assert_eq!(Expression::operator_name(ExpressionType::Add), "+");
    assert_eq!(Expression::operator_name(ExpressionType::UnaryMinus), "-");
    assert_eq!(Expression::operator_name(ExpressionType::NotEquals), "!=");
    // The reference implementation resolves the name of this operator to
    // the name of the enumeration member.
    assert_eq!(Expression::operator_name(ExpressionType::Equals), "Equals");
    assert_eq!(Expression::operator_name(ExpressionType::MemberAccess), "MemberAccess");
    assert_eq!(
        Expression::operator_name(ExpressionType::ConditionalExpression),
        "ConditionalExpression"
    );
}

#[test]
fn expression_types() {
    for (expression, expression_type) in [
        ("a ? b : c", ExpressionType::ConditionalExpression),
        ("1", ExpressionType::Constant),
        ("F(1)", ExpressionType::FunctionCall),
        ("a.b", ExpressionType::MemberAccess),
        ("a", ExpressionType::Parameter),
        ("pi", ExpressionType::Keyword),
        ("!a", ExpressionType::Not),
        ("-a", ExpressionType::UnaryMinus),
        ("a % b", ExpressionType::Remainder),
        ("a >= b", ExpressionType::MoreThanOrEqual),
    ] {
        assert_eq!(
            Expression::parse(expression).unwrap().expression_type(),
            expression_type,
            "{expression}"
        );
    }
}

#[test]
fn builds_the_expected_tree() {
    let p = |name: &str| Expression::from(ParameterExpression::new(name));
    assert_eq!(
        Expression::parse("a * b + c").unwrap(),
        BinaryExpression::new(
            BinaryExpression::new(p("a"), p("b"), ExpressionType::Multiply).into(),
            p("c"),
            ExpressionType::Add
        )
        .into()
    );
    assert_eq!(
        Expression::parse("a + b * c").unwrap(),
        BinaryExpression::new(
            p("a"),
            BinaryExpression::new(p("b"), p("c"), ExpressionType::Multiply).into(),
            ExpressionType::Add
        )
        .into()
    );
    // Operators of the same precedence group to the right, as in the
    // reference implementation.
    assert_eq!(
        Expression::parse("a - b - c").unwrap(),
        BinaryExpression::new(
            p("a"),
            BinaryExpression::new(p("b"), p("c"), ExpressionType::Subtract).into(),
            ExpressionType::Subtract
        )
        .into()
    );
    assert_eq!(
        Expression::parse("-F(a.B, 1).C").unwrap(),
        UnaryExpression::new(
            MemberAccessExpression::new(
                FunctionCallExpression::new(
                    "F",
                    vec![
                        MemberAccessExpression::new(p("a"), "B").into(),
                        ConstantExpression::new(1.0).into()
                    ]
                )
                .into(),
                "C"
            )
            .into(),
            ExpressionType::UnaryMinus
        )
        .into()
    );
    assert_eq!(
        Expression::parse("this.Target.X").unwrap(),
        MemberAccessExpression::new(KeywordExpression::new(ExpressionKeyword::Target).into(), "X").into()
    );
}

#[test]
fn reports_parse_errors() {
    for (expression, message, position) in [
        ("", "Unexpected end of  expression", 0),
        ("   ", "Unexpected end of  expression", 3),
        // After an operator the parser expects an operand, not the end.
        ("1 +", "Unexpected token", 3),
        ("1.X", "Unexpected token", 2),
        ("(1", "Unexpected end of  expression", 2),
        ("Max(1", "Unexpected end of  expression", 5),
        ("a ? 1", "Unexpected end of  expression", 5),
        ("true ? 1 : ", "Unexpected end of  expression", 11),
        ("1 2", "Unexpected token", 2),
        ("1)", "Unexpected token", 1),
        (")", "Unexpected token", 0),
        ("@", "Unexpected token", 0),
        ("1 $", "Unexpected token", 2),
        ("1 + *", "Unexpected token", 4),
        ("a.", "Unexpected token", 2),
        ("a.1", "Unexpected token", 2),
        ("1e10", "Unexpected token", 1),
        ("a = b", "Unexpected token", 2),
        ("a & b", "Unexpected token", 2),
        ("Max(,1)", "Unexpected ''", 4),
        ("()", "Unexpected ''", 1),
        ("a ? : 1", "Unexpected ''", 4),
        ("\u{e9}", "Unexpected token", 0),
        ("1 + \u{1f600}", "Unexpected token", 4),
    ] {
        assert_eq!(parse_error(expression), (message.to_owned(), position), "{expression:?}");
    }

    let error = Expression::parse("1 2").unwrap_err();
    assert_eq!(error.to_string(), "Unexpected token");
    assert_eq!(error, ExpressionParseException::new("Unexpected token", 2));
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

#[test]
fn evaluates_arithmetic_with_precedence() {
    assert_eq!(eval_double("1+2*3"), 7.0);
    assert_eq!(eval_double("2*3+1"), 7.0);
    assert_eq!(eval_double("(1+2)*3"), 9.0);
    assert_eq!(eval_double("7 % 4"), 3.0);
    assert_eq!(eval_double("-7 % 4"), -3.0);
    assert_eq!(eval_double("1 / 4"), 0.25);
    assert_eq!(eval_double("-(1+2)"), -3.0);
    assert_eq!(eval_double("- -1"), 1.0);
    assert_eq!(eval_double("2 * -3"), -6.0);
    assert_eq!(eval_double("1 / 0"), f64::INFINITY);
    // Constants are single-precision.
    assert_eq!(eval_double("0.1"), 0.1f32 as f64);
    // Same-precedence operators group to the right, as in the reference
    // implementation: 10-(5-2) and 2*(3%4).
    assert_eq!(eval_double("10-5-2"), 7.0);
    assert_eq!(eval_double("2*3%4"), 6.0);
    assert_eq!(eval_double("8/4/2"), 4.0);
}

#[test]
fn evaluates_comparisons_and_logic() {
    assert_eq!(eval("1 < 2"), V::Boolean(true));
    assert_eq!(eval("1 > 2"), V::Boolean(false));
    assert_eq!(eval("1+2 > 2 && true"), V::Boolean(true));
    assert_eq!(eval("1 < 2 == true"), V::Boolean(true));
    assert_eq!(eval("true || false && false"), V::Boolean(true));
    assert_eq!(eval("false && true || true"), V::Boolean(true));
    assert_eq!(eval("1 == 1"), V::Boolean(true));
    assert_eq!(eval("1 != 1"), V::Boolean(false));
    assert_eq!(eval("!true"), V::Boolean(false));
    assert_eq!(eval("!!true"), V::Boolean(true));
    assert_eq!(eval("!(1 > 2)"), V::Boolean(true));
    // `>=` and `<=` evaluate as `>` and `<`, as in the reference implementation.
    assert_eq!(eval("2 >= 2"), V::Boolean(false));
    assert_eq!(eval("2 <= 2"), V::Boolean(false));
    assert_eq!(eval("3 >= 2"), V::Boolean(true));
    assert_eq!(eval("1 <= 2"), V::Boolean(true));
    // Mismatched operand types are invalid, not errors.
    assert_eq!(eval("1 == true"), V::Invalid);
    assert_eq!(eval("1 != true"), V::Invalid);
    assert_eq!(eval("!1"), V::Invalid);
    assert_eq!(eval("1 && true"), V::Invalid);
    assert_eq!(eval("true + true"), V::Invalid);
    assert_eq!(eval("true < false"), V::Invalid);
    assert_eq!(eval("-true"), V::Invalid);
}

#[test]
fn evaluates_conditionals() {
    assert_eq!(eval_double("true ? 1 : 2"), 1.0);
    assert_eq!(eval_double("false ? 1 : 2"), 2.0);
    assert_eq!(eval_double("1 > 2 ? 10 : 20"), 20.0);
    // A condition that is not a boolean selects the false part.
    assert_eq!(eval_double("1 ? 1 : 2"), 2.0);
    assert_eq!(eval_double("unknown ? 1 : 2"), 2.0);
    assert_eq!(eval_double("false ? 1 : true ? 2 : 3"), 2.0);
    assert_eq!(eval_double("true ? false ? 1 : 2 : 3"), 2.0);
    assert_eq!(eval_double("(true ? 1 : 2) + 1"), 2.0);
    assert_eq!(eval_double("true ? 1 : 2 + 1"), 1.0);
    assert_eq!(eval_double("Max(true ? 1 : 2, 0)"), 1.0);
    assert_eq!(eval_double("Max(0, false ? 1 : 2)"), 2.0);
}

#[test]
fn evaluates_keywords() {
    let ctx = ExpressionEvaluationContext {
        starting_value: d(1.0),
        current_value: d(2.0),
        final_value: Vector2::new(3.0, 4.0).into(),
        ..context()
    };
    let eval = |s: &str| Expression::parse(s).unwrap().evaluate(&ctx);
    assert_eq!(eval("this.StartingValue"), d(1.0));
    assert_eq!(eval("THIS.CURRENTVALUE"), d(2.0));
    assert_eq!(eval("this.FinalValue"), Vector2::new(3.0, 4.0).into());
    assert_eq!(eval("this.FinalValue.Y"), d(4.0));
    assert_eq!(eval("this.StartingValue + this.CurrentValue * 2"), d(5.0));
    assert_eq!(eval("Pi"), d(std::f32::consts::PI as f64));
    assert_eq!(eval("true"), V::Boolean(true));
    assert_eq!(eval("False"), V::Boolean(false));
    // The target itself is not a value.
    assert_eq!(eval("this.Target"), V::Invalid);
    // Without a context value the keywords are invalid.
    assert_eq!(super::Expression::parse("this.StartingValue").unwrap().evaluate(&context()), V::Invalid);
}

#[test]
fn evaluates_parameters() {
    let mut parameters = TestParameters::default();
    parameters.values.insert("x", d(4.0));
    parameters.values.insert("v", Vector3::new(1.0, 2.0, 3.0).into());
    parameters
        .objects
        .insert("obj", TestObject::default().with("Opacity", 0.5f32).with("Size", Vector::new(10.0, 20.0)));
    // An object parameter may also have a value; member access prefers the object.
    parameters.values.insert("obj", d(1.0));
    let target = TestObject::default().with("Scale", Vector3D::new(1.0, 2.0, 3.0));

    let ctx = ExpressionEvaluationContext {
        target: Some(&target),
        parameters: Some(&parameters),
        ..context()
    };
    let eval = |s: &str| Expression::parse(s).unwrap().evaluate(&ctx);

    assert_eq!(eval("x * 2"), d(8.0));
    assert_eq!(eval("v.Z"), d(3.0));
    assert_eq!(eval("v.ZX"), Vector2::new(3.0, 1.0).into());
    assert_eq!(eval("obj.Opacity"), d(0.5));
    assert_eq!(eval("obj.Size.Y + x"), d(24.0));
    assert_eq!(eval("obj"), d(1.0));
    assert_eq!(eval("this.Target.Scale.YZ"), Vector::new(2.0, 3.0).into());
    assert_eq!(eval("THIS.TARGET.Scale.X"), d(1.0));
    // Unknown names are invalid values.
    assert_eq!(eval("unknown"), V::Invalid);
    assert_eq!(eval("unknown + 1"), V::Invalid);
    assert_eq!(eval("obj.Unknown"), V::Invalid);
    assert_eq!(eval("this.Target.Unknown"), V::Invalid);
    assert_eq!(eval("x.Y"), V::Invalid);
    // Property and parameter names are case-sensitive.
    assert_eq!(eval("X"), V::Invalid);
    assert_eq!(eval("obj.opacity"), V::Invalid);

    // Without a parameter collection every parameter is invalid.
    assert_eq!(Expression::parse("x").unwrap().evaluate(&context()), V::Invalid);
    assert_eq!(Expression::parse("obj.Opacity").unwrap().evaluate(&context()), V::Invalid);
}

#[test]
#[should_panic(expected = "no target")]
fn target_member_without_a_target_is_a_programmer_error() {
    Expression::parse("this.Target.X").unwrap().evaluate(&context());
}

#[test]
#[should_panic(expected = "no foreign function interface")]
fn function_call_without_an_interface_is_a_programmer_error() {
    Expression::parse("Abs(1)")
        .unwrap()
        .evaluate(&ExpressionEvaluationContext::default());
}

#[test]
fn collects_references() {
    let expr = Expression::parse(
        "obj.Offset.X + this.Target.Size.Y + Max(a.B, c) + x + (!d.E ? -f.G : This.Target.H) + this.StartingValue.I + F(1).J",
    )
    .unwrap();
    let mut references = HashSet::new();
    expr.collect_references(&mut references);
    let expected: HashSet<(String, String)> = [
        ("obj", "Offset"),
        ("this.target", "Size"),
        ("a", "B"),
        ("d", "E"),
        ("f", "G"),
        ("this.target", "H"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .collect();
    assert_eq!(references, expected);
}

#[test]
fn expressions_and_variants_are_plain_shareable_data() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<Expression>();
    assert_send_sync::<ExpressionVariant>();
    assert_send_sync::<ExpressionParseException>();
    assert_send_sync::<BuiltInExpressionFfi>();
    assert_send_sync::<ExpressionTrackedObjects>();

    let expr = Expression::parse("Max(1, 2) + 3").unwrap();
    let result = std::thread::spawn(move || expr.evaluate(&context())).join().unwrap();
    assert_eq!(result, d(5.0));
}

// ---------------------------------------------------------------------------
// Built-in functions
// ---------------------------------------------------------------------------

#[test]
fn vector_constructors_create_double_precision_vectors() {
    assert_eq!(eval("Vector2(1, 2)"), Vector::new(1.0, 2.0).into());
    assert_eq!(eval("Vector3(1, 2, 3)"), Vector3D::new(1.0, 2.0, 3.0).into());
    assert_eq!(eval("Vector3(Vector2(1, 2), 3)"), Vector3D::new(1.0, 2.0, 3.0).into());
    assert_eq!(eval("Vector4(1, 2, 3, 4)"), Vector4::new(1.0, 2.0, 3.0, 4.0).into());
    // The single-precision overloads are reached by narrowing the arguments.
    assert_eq!(
        eval("Vector4(Vector2(1, 2), 3, 4)"),
        Vector4::new(1.0, 2.0, 3.0, 4.0).into()
    );
    assert_eq!(
        eval("Vector4(Vector3(1, 2, 3), 4)"),
        Vector4::new(1.0, 2.0, 3.0, 4.0).into()
    );
    // The arguments pass through single precision.
    assert_eq!(eval("Vector2(0.1, 0)"), Vector::new(0.1f32 as f64, 0.0).into());
}

#[test]
fn member_access_and_swizzles() {
    assert_eq!(eval("Vector3(1, 2, 3).XY"), Vector::new(1.0, 2.0).into());
    assert_eq!(eval("Vector3(1, 2, 3).ZY"), Vector::new(3.0, 2.0).into());
    assert_eq!(eval("Vector3(1, 2, 3).ZY.X"), d(3.0));
    assert_eq!(eval("Vector4(1, 2, 3, 4).W"), d(4.0));
    assert_eq!(eval("Vector4(1, 2, 3, 4).Q"), V::Invalid);
    assert_eq!(eval("Vector2(1, 2).x"), V::Invalid);
    assert_eq!(eval("Vector2(1, 2).Z"), V::Invalid);
    assert_eq!(eval_double("-Vector2(1, 2).X"), -1.0);
    assert_eq!(eval("Quaternion(1, 2, 3, 4).W"), d(4.0));
    assert_eq!(eval("ColorRGB(255, 1, 2, 3).G"), d(2.0));
}

#[test]
fn vector_arithmetic_in_expressions() {
    assert_eq!(
        eval("Vector3(1, 2, 3) + Vector3(1, 1, 1)"),
        Vector3D::new(2.0, 3.0, 4.0).into()
    );
    assert_eq!(eval("Vector2(1, 2) * 2"), Vector::new(2.0, 4.0).into());
    assert_eq!(eval("Vector2(4, 6) / Vector2(2, 3)"), Vector::new(2.0, 2.0).into());
    assert_eq!(eval("-Vector2(1, 2)"), Vector::new(-1.0, -2.0).into());
    assert_eq!(eval("Vector2(1, 2) == Vector2(1, 2)"), V::Boolean(true));
    assert_eq!(eval("Vector2(1, 2) != Vector2(1, 3)"), V::Boolean(true));
    // A scalar on the left of a vector is not defined.
    assert_eq!(eval("2 * Vector2(1, 2)"), V::Invalid);
    assert_eq!(eval("Vector2(1, 2) + 1"), V::Invalid);
    assert_eq!(eval("Vector2(1, 2) + Vector3(1, 2, 3)"), V::Invalid);
}

#[test]
fn scalar_functions() {
    assert_eq!(eval_double("Abs(-2)"), 2.0);
    assert_eq!(eval_double("ACos(1)"), 0.0);
    assert_eq!(eval_double("ASin(0)"), 0.0);
    assert_eq!(eval_double("ATan(0)"), 0.0);
    assert_eq!(eval_double("Ceil(1.2)"), 2.0);
    assert_eq!(eval_double("Floor(1.8)"), 1.0);
    assert_eq!(eval_double("Floor(-1.2)"), -2.0);
    // Rounds half to even.
    assert_eq!(eval_double("Round(2.5)"), 2.0);
    assert_eq!(eval_double("Round(3.5)"), 4.0);
    assert_eq!(eval_double("Round(-0.5)"), 0.0);
    assert_eq!(eval_double("Clamp(5, 0, 2)"), 2.0);
    assert_eq!(eval_double("Clamp(-5, 0, 2)"), 0.0);
    // The bounds may be given in either order.
    assert_eq!(eval_double("Clamp(5, 2, 0)"), 2.0);
    assert_eq!(eval_double("Cos(0)"), 1.0);
    assert_eq!(eval_double("Sin(0)"), 0.0);
    assert_eq!(eval_double("Tan(0)"), 0.0);
    assert_eq!(eval_double("Sqrt(16)"), 4.0);
    assert_eq!(eval_double("Square(3)"), 9.0);
    assert_eq!(eval_double("Pow(2, 10)"), 1024.0);
    assert_eq!(eval_double("Mod(7, 3)"), 1.0);
    assert_eq!(eval_double("Min(1, 2)"), 1.0);
    assert_eq!(eval_double("Max(1, 2)"), 2.0);
    assert_eq!(eval_double("Ln(1)"), 0.0);
    assert_eq!(eval_double("Log10(1000)"), 3.0);
    assert_eq!(eval_double("ToRadians(180)"), std::f32::consts::PI as f64);
    assert_eq!(eval_double("ToDegrees(Pi)"), 180.0);
    assert_eq!(eval_double("Lerp(0, 10, 0.25)"), 2.5);
    assert_eq!(eval_double("Slerp(0, 10, 0.5)"), 5.0);
    assert_eq!(eval_double("SmoothStep(0, 1, 0.5)"), 0.5);
    assert_eq!(eval_double("SmoothStep(0, 1, 2)"), 1.0);
    assert_eq!(eval_double("SmoothStep(0, 10, 2.5)"), 0.15625);
    // Single-precision results are widened.
    assert_eq!(eval_double("Sqrt(2)"), 2f64.sqrt() as f32 as f64);
    assert_eq!(eval_double("Sin(1)"), 1f64.sin() as f32 as f64);
}

#[test]
fn unknown_functions_and_signatures_are_invalid() {
    assert_eq!(eval("Unknown(1)"), V::Invalid);
    assert_eq!(eval("Abs()"), V::Invalid);
    assert_eq!(eval("Abs(1, 2)"), V::Invalid);
    assert_eq!(eval("Abs(true)"), V::Invalid);
    assert_eq!(eval("Abs(unknown)"), V::Invalid);
    // Function names are case-sensitive.
    assert_eq!(eval("abs(1)"), V::Invalid);
    assert_eq!(eval("vector2(1, 2)"), V::Invalid);
}

#[test]
fn vector_functions() {
    assert_eq!(eval("Abs(Vector2(-1, 2))"), Vector::new(1.0, 2.0).into());
    assert_eq!(eval("Abs(Vector3(-1, 2, -3))"), Vector3D::new(1.0, 2.0, 3.0).into());
    assert_eq!(eval("Abs(Vector4(-1, 2, -3, 4))"), Vector4::new(1.0, 2.0, 3.0, 4.0).into());
    assert_eq!(
        eval("Clamp(Vector2(5, -5), Vector2(0, 0), Vector2(1, 1))"),
        Vector::new(1.0, 0.0).into()
    );
    assert_eq!(eval_double("Distance(Vector2(0, 0), Vector2(3, 4))"), 5.0);
    assert_eq!(eval_double("DistanceSquared(Vector2(0, 0), Vector2(3, 4))"), 25.0);
    assert_eq!(eval_double("Distance(Vector3(0, 0, 0), Vector3(2, 3, 6))"), 7.0);
    assert_eq!(
        eval_double("Distance(Vector4(0, 0, 0, 0), Vector4(2, 4, 5, 6))"),
        9.0
    );
    assert_eq!(eval_double("Length(Vector2(3, 4))"), 5.0);
    assert_eq!(eval_double("Length(Vector3(2, 3, 6))"), 7.0);
    assert_eq!(eval_double("Length(Vector4(2, 4, 5, 6))"), 9.0);
    assert_eq!(eval_double("LengthSquared(Vector2(3, 4))"), 25.0);
    assert_eq!(eval_double("LengthSquared(Vector3(1, 2, 2))"), 9.0);
    assert_eq!(eval_double("LengthSquared(Vector4(1, 2, 2, 4))"), 25.0);
    assert_eq!(
        eval("Lerp(Vector2(0, 2), Vector2(4, 4), 0.5)"),
        Vector::new(2.0, 3.0).into()
    );
    assert_eq!(
        eval("Lerp(Vector3(0, 2, 4), Vector3(4, 4, 4), 0.5)"),
        Vector3D::new(2.0, 3.0, 4.0).into()
    );
    assert_eq!(
        eval("Lerp(Vector4(0, 2, 4, 0), Vector4(4, 4, 4, 1), 0.5)"),
        Vector4::new(2.0, 3.0, 4.0, 0.5).into()
    );
    assert_eq!(eval("Max(Vector2(1, 5), Vector2(3, 2))"), Vector::new(3.0, 5.0).into());
    assert_eq!(eval("Min(Vector2(1, 5), Vector2(3, 2))"), Vector::new(1.0, 2.0).into());
    assert_eq!(
        eval("Max(Vector3(1, 5, 0), Vector3(3, 2, 0))"),
        Vector3D::new(3.0, 5.0, 0.0).into()
    );
    assert_eq!(
        eval("Min(Vector4(1, 5, 0, 1), Vector4(3, 2, 0, 0))"),
        Vector4::new(1.0, 2.0, 0.0, 0.0).into()
    );
    assert_eq!(
        eval("Normalize(Vector2(3, 4))"),
        Vector::normalize_vector(Vector::new(3.0, 4.0)).into()
    );
    assert_eq!(
        eval("Normalize(Vector3(0, 3, 4))"),
        Vector3D::normalize(Vector3D::new(0.0, 3.0, 4.0)).into()
    );
    assert_eq!(
        eval("Normalize(Vector4(0, 0, 0, 2))"),
        Vector4::new(0.0, 0.0, 0.0, 1.0).into()
    );
    assert_eq!(eval("Scale(Vector2(1, 2), 3)"), Vector::new(3.0, 6.0).into());
    assert_eq!(eval("Scale(Vector3(1, 2, 3), 2)"), Vector3D::new(2.0, 4.0, 6.0).into());
    assert_eq!(
        eval("Scale(Vector4(1, 2, 3, 4), 2)"),
        Vector4::new(2.0, 4.0, 6.0, 8.0).into()
    );
    assert_eq!(
        eval("SmoothStep(Vector2(0, 0), Vector2(1, 10), Vector2(0.5, 2.5))"),
        Vector::new(0.5, 0.15625).into()
    );
    assert_eq!(
        eval("SmoothStep(Vector3(0, 0, 0), Vector3(1, 10, 1), Vector3(0.5, 2.5, 2))"),
        Vector3D::new(0.5, 0.15625, 1.0).into()
    );
    assert_eq!(
        eval("SmoothStep(Vector4(0, 0, 0, 0), Vector4(1, 10, 1, 1), Vector4(0.5, 2.5, 2, -1))"),
        Vector4::new(0.5, 0.15625, 1.0, 0.0).into()
    );
}

#[test]
fn color_functions() {
    assert_eq!(
        eval("ColorRGB(255, 300, -5, 16.9)"),
        Color::from_argb(255, 255, 0, 16).into()
    );
    let lerped: ExpressionVariant = Color::from_argb(255, 127, 127, 127).into();
    assert_eq!(
        eval("ColorLerp(ColorRGB(255, 0, 0, 0), ColorRGB(255, 255, 255, 255), 0.5)"),
        lerped
    );
    assert_eq!(
        eval("ColorLerpRGB(ColorRGB(255, 0, 0, 0), ColorRGB(255, 255, 255, 255), 0.5)"),
        lerped
    );
    assert_eq!(
        eval_double("ColorLerp(ColorRGB(255, 0, 0, 0), ColorRGB(0, 100, 0, 0), 0.25).R"),
        25.0
    );
    assert_eq!(
        eval_double("ColorLerp(ColorRGB(255, 0, 0, 0), ColorRGB(0, 100, 0, 0), 0.25).A"),
        191.0
    );
}

#[test]
fn matrix_functions() {
    assert_eq!(
        eval("Matrix3x2.CreateScale(Vector2(2, 3))"),
        Matrix3x2::create_scale(Vector2::new(2.0, 3.0)).into()
    );
    assert_eq!(
        eval("Matrix3x2.CreateFromScale(Vector2(2, 3))"),
        Matrix3x2::create_scale(Vector2::new(2.0, 3.0)).into()
    );
    assert_eq!(
        eval("Matrix3x2.CreateFromTranslation(Vector2(2, 3))"),
        Matrix3x2::create_translation(Vector2::new(2.0, 3.0)).into()
    );
    // As in the reference implementation, `CreateTranslation` creates a scale.
    assert_eq!(
        eval("Matrix3x2.CreateTranslation(Vector2(2, 3))"),
        Matrix3x2::create_scale(Vector2::new(2.0, 3.0)).into()
    );
    assert_eq!(
        eval("Matrix4x4.CreateTranslation(Vector3(2, 3, 4))"),
        Matrix4x4::create_scale(Vector3::new(2.0, 3.0, 4.0)).into()
    );
    assert_eq!(
        eval("Matrix3x2.CreateRotation(1)"),
        Matrix3x2::create_rotation(1.0).into()
    );
    assert_eq!(
        eval("Matrix3x2.CreateSkew(0.5, 0.25, Vector2(3, 4))"),
        Matrix3x2::create_skew_at(0.5, 0.25, Vector2::new(3.0, 4.0)).into()
    );
    // As in the reference implementation, the sixth component repeats the fifth.
    assert_eq!(
        eval("Matrix3x2(1, 2, 3, 4, 5, 6)"),
        Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 5.0).into()
    );
    assert_eq!(
        eval("Matrix4x4(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)"),
        Matrix4x4::new(1.0, 2.0, 3.0, 4.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0).into()
    );
    assert_eq!(
        eval("Matrix4x4(Matrix3x2.CreateFromTranslation(Vector2(2, 3)))"),
        Matrix4x4::create_translation(Vector3::new(2.0, 3.0, 0.0)).into()
    );
    assert_eq!(
        eval("Matrix4x4.CreateFromAxisAngle(Vector3(0, 0, 1), 1)"),
        Matrix4x4::create_from_axis_angle(Vector3::UNIT_Z, 1.0).into()
    );
    assert_eq!(
        eval("Matrix4x4.CreateFromScale(Vector3(2, 3, 4))"),
        Matrix4x4::create_scale(Vector3::new(2.0, 3.0, 4.0)).into()
    );
    assert_eq!(
        eval("Matrix4x4.CreateScale(Vector3(2, 3, 4))"),
        Matrix4x4::create_scale(Vector3::new(2.0, 3.0, 4.0)).into()
    );
    assert_eq!(
        eval("Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4))"),
        Matrix4x4::create_translation(Vector3::new(2.0, 3.0, 4.0)).into()
    );
    assert_eq!(eval_double("Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4)).M43"), 4.0);
    assert_eq!(eval_double("Inverse(Matrix3x2.CreateScale(Vector2(2, 4))).M22"), 0.25);
    assert_eq!(eval_double("Inverse(Matrix4x4.CreateScale(Vector3(2, 4, 8))).M33"), 0.125);
    // A singular matrix inverts to NaN components.
    assert!(eval_double("Inverse(Scale(Matrix3x2.CreateRotation(1), 0)).M11").is_nan());
    assert!(eval_double("Inverse(Scale(Matrix4x4.CreateScale(Vector3(1, 1, 1)), 0)).M44").is_nan());
    assert_eq!(
        eval("Scale(Matrix3x2.CreateScale(Vector2(2, 3)), 2)"),
        (Matrix3x2::create_scale(Vector2::new(2.0, 3.0)) * 2.0).into()
    );
    assert_eq!(
        eval("Transform(Vector2(1, 1), Matrix3x2.CreateScale(Vector2(2, 3)))"),
        Vector2::new(2.0, 3.0).into()
    );
    assert_eq!(
        eval("Transform(Vector3(1, 1, 1), Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4)))"),
        Vector3::new(3.0, 4.0, 5.0).into()
    );
    assert_eq!(
        eval("Matrix3x2.CreateScale(Vector2(2, 3)) * Matrix3x2.CreateFromTranslation(Vector2(1, 1))"),
        Matrix3x2::new(2.0, 0.0, 0.0, 3.0, 1.0, 1.0).into()
    );
}

#[test]
fn quaternion_functions() {
    assert_eq!(eval("Quaternion(1, 2, 3, 4)"), Quaternion::new(1.0, 2.0, 3.0, 4.0).into());
    let expected = Quaternion::create_from_axis_angle(Vector3::UNIT_Z, 1.0);
    assert_eq!(eval("Quaternion.CreateFromAxisAngle(Vector3(0, 0, 1), 1)"), expected.into());
    assert_eq!(eval_double("Length(Quaternion(0, 0, 3, 4))"), 5.0);
    assert_eq!(eval_double("LengthSquared(Quaternion(0, 0, 3, 4))"), 25.0);
    assert_eq!(
        eval("Normalize(Quaternion(0, 0, 0, 2))"),
        Quaternion::IDENTITY.into()
    );
    assert_eq!(
        eval("Concatenate(Quaternion(1, 0, 0, 0), Quaternion(0, 1, 0, 0))"),
        Quaternion::concatenate(Quaternion::new(1.0, 0.0, 0.0, 0.0), Quaternion::new(0.0, 1.0, 0.0, 0.0)).into()
    );
    assert_eq!(
        eval("Slerp(Quaternion(0, 0, 0, 1), Quaternion.CreateFromAxisAngle(Vector3(0, 0, 1), 1), 0.5)"),
        Quaternion::slerp(Quaternion::IDENTITY, expected, 0.5).into()
    );
}

#[test]
fn relative_functions() {
    assert_eq!(eval("RelativeUnit.Relative"), RelativeUnit::Relative.into());
    assert_eq!(eval("RelativeUnit.Absolute"), RelativeUnit::Absolute.into());
    assert_eq!(eval("RelativeUnit.Absolute()"), RelativeUnit::Absolute.into());
    assert_eq!(
        eval("RelativePoint(0.5, 0.25, RelativeUnit.Relative)"),
        RelativePoint::new(0.5, 0.25, RelativeUnit::Relative).into()
    );
    // The arguments pass through single precision.
    assert_eq!(
        eval("RelativePoint(0.1, 0.5, RelativeUnit.Absolute)"),
        RelativePoint::new(0.1f32 as f64, 0.5, RelativeUnit::Absolute).into()
    );
    assert_eq!(
        eval("RelativeScalar(0.5, RelativeUnit.Relative)"),
        RelativeScalar::new(0.5, RelativeUnit::Relative).into()
    );
    assert_eq!(
        eval("RelativePoint(0.5, 0.25, RelativeUnit.Relative).Unit"),
        RelativeUnit::Relative.into()
    );
    assert_eq!(eval_double("RelativePoint(0.5, 0.25, RelativeUnit.Relative).Y"), 0.25);
    assert_eq!(eval_double("RelativeScalar(0.5, RelativeUnit.Relative).Scalar"), 0.5);
    assert_eq!(
        eval("RelativeScalar(0.5, RelativeUnit.Relative) * 2"),
        RelativeScalar::new(1.0, RelativeUnit::Relative).into()
    );
    assert_eq!(
        eval("RelativeScalar(0.5, RelativeUnit.Relative) + RelativeScalar(0.25, RelativeUnit.Relative)"),
        RelativeScalar::new(0.75, RelativeUnit::Relative).into()
    );
    assert_eq!(
        eval("RelativeScalar(0.5, RelativeUnit.Relative) + RelativeScalar(0.25, RelativeUnit.Absolute)"),
        V::Invalid
    );
    assert_eq!(
        eval("RelativeUnit.Relative == RelativeUnit.Relative"),
        V::Boolean(true)
    );
    assert_eq!(
        eval("RelativeUnit.Relative != RelativeUnit.Absolute"),
        V::Boolean(true)
    );
}

// ---------------------------------------------------------------------------
// Differential test against the reference implementation
// ---------------------------------------------------------------------------

/// `expression => printed tree => result`, or `expression => ERR message
/// @position => `, as produced by the reference implementation for each
/// expression (evaluated with only the built-in functions in the context).
const REFERENCE_OUTPUT: &str = r#"
Vector3(0.5+(4.0-0.5)* -2, 1, 0).X => (Vector3( ((0.5+((4-0.5)*-2))), (1), (0) )).X => -6.5
a > 1 ? 2 : 3 => (({a}>1)) ? (2) : (3) => 3
Min(1, x) => Min( (1), ({x}) ) => Invalid
Matrix3x2.CreateScale ( v ) => Matrix3x2.CreateScale( ({v}) ) => Invalid
!-a => -!{a} => Invalid
- - 2 => 2 => 2
!!a => {a} => Invalid
a >= b && c <= d || e != f == g => ((({a}>={b})&&({c}<={d}))||({e}!=({f}Equals{g}))) => Invalid
pixel.pi => ({pixel}).pi => Invalid
this.X => ({this}).X => Invalid
Max(,1) => ERR Unexpected '' @4 => 
() => ERR Unexpected '' @1 => 
a ? : 1 => ERR Unexpected '' @4 => 
1 + => ERR Unexpected token @3 => 
1.X => ERR Unexpected token @2 => 
1 2 => ERR Unexpected token @2 => 
1) => ERR Unexpected token @1 => 
) => ERR Unexpected token @0 => 
@ => ERR Unexpected token @0 => 
1 $ => ERR Unexpected token @2 => 
1 + * => ERR Unexpected token @4 => 
a. => ERR Unexpected token @2 => 
a.1 => ERR Unexpected token @2 => 
1e10 => ERR Unexpected token @1 => 
a = b => ERR Unexpected token @2 => 
a & b => ERR Unexpected token @2 => 
(1 => ERR Unexpected end of  expression @2 => 
Max(1 => ERR Unexpected end of  expression @5 => 
a ? 1 => ERR Unexpected end of  expression @5 => 
true ? 1 :  => ERR Unexpected end of  expression @11 => 
10-5-2 => (10-(5-2)) => 7
2*3%4 => (2*(3%4)) => 6
8/4/2 => (8/(4/2)) => 4
1+2*3 => (1+(2*3)) => 7
2 >= 2 => (2>=2) => False
3 >= 2 => (3>=2) => True
1 < 2 == true => ((1<2)Equals[True]) => True
true || false && false => ([True]||([False]&&[False])) => True
1 == true => (1Equals[True]) => Invalid
-true => -[True] => Invalid
1 ? 1 : 2 => (1) ? (1) : (2) => 2
false ? 1 : true ? 2 : 3 => ([False]) ? (1) : (([True]) ? (2) : (3)) => 2
true ? false ? 1 : 2 : 3 => ([True]) ? (([False]) ? (1) : (2)) : (3) => 2
true ? 1 : 2 + 1 => ([True]) ? (1) : ((2+1)) => 1
Max(true ? 1 : 2, 0) => Max( (([True]) ? (1) : (2)), (0) ) => 1
Max(0, false ? 1 : 2) => Max( (0), (([False]) ? (1) : (2)) ) => 2
Pi => [Pi] => 3.1415927410125732
0.1 => 0.1 => 0.10000000149011612
1 / 0 => (1/0) => Infinity
-7 % 4 => (-7%4) => -3
Vector2(1, 2) => Vector2( (1), (2) ) => 1, 2
Vector2(0.1, 0) => Vector2( (0.1), (0) ) => 0.10000000149011612, 0
Vector3(1, 2, 3) => Vector3( (1), (2), (3) ) => Vector3D { X = 1, Y = 2, Z = 3, Length = 3.7416573867739413 }
Vector3(Vector2(1, 2), 3) => Vector3( (Vector2( (1), (2) )), (3) ) => Vector3D { X = 1, Y = 2, Z = 3, Length = 3.7416573867739413 }
Vector4(1, 2, 3, 4) => Vector4( (1), (2), (3), (4) ) => <1, 2, 3, 4>
Vector4(Vector2(1, 2), 3, 4) => Vector4( (Vector2( (1), (2) )), (3), (4) ) => <1, 2, 3, 4>
Vector4(Vector3(1, 2, 3), 4) => Vector4( (Vector3( (1), (2), (3) )), (4) ) => <1, 2, 3, 4>
Vector3(1, 2, 3).ZY => (Vector3( (1), (2), (3) )).ZY => 3, 2
Vector3(1, 2, 3).ZY.X => ((Vector3( (1), (2), (3) )).ZY).X => 3
Vector4(1, 2, 3, 4).Q => (Vector4( (1), (2), (3), (4) )).Q => Invalid
-Vector2(1, 2).X => -(Vector2( (1), (2) )).X => -1
ColorRGB(255, 1, 2, 3).G => (ColorRGB( (255), (1), (2), (3) )).G => 2
ColorRGB(255, 300, -5, 16.9) => ColorRGB( (255), (300), (-5), (16.9) ) => #ffff0010
ColorLerp(ColorRGB(255, 0, 0, 0), ColorRGB(255, 255, 255, 255), 0.5) => ColorLerp( (ColorRGB( (255), (0), (0), (0) )), (ColorRGB( (255), (255), (255), (255) )), (0.5) ) => #ff7f7f7f
ColorLerp(ColorRGB(255, 0, 0, 0), ColorRGB(0, 100, 0, 0), 0.25) => ColorLerp( (ColorRGB( (255), (0), (0), (0) )), (ColorRGB( (0), (100), (0), (0) )), (0.25) ) => #bf190000
2 * Vector2(1, 2) => (2*Vector2( (1), (2) )) => Invalid
Vector2(1, 2) * 2 => (Vector2( (1), (2) )*2) => 2, 4
Vector2(4, 6) / Vector2(2, 3) => (Vector2( (4), (6) )/Vector2( (2), (3) )) => 2, 2
Vector2(1, 2) == Vector2(1, 2) => (Vector2( (1), (2) )EqualsVector2( (1), (2) )) => True
Round(2.5) => Round( (2.5) ) => 2
Round(-0.5) => Round( (-0.5) ) => -0
Clamp(5, 2, 0) => Clamp( (5), (2), (0) ) => 2
Log10(1000) => Log10( (1000) ) => 3
ToRadians(180) => ToRadians( (180) ) => 3.1415927410125732
ToDegrees(Pi) => ToDegrees( ([Pi]) ) => 180
Sqrt(2) => Sqrt( (2) ) => 1.4142135381698608
Sin(1) => Sin( (1) ) => 0.8414709568023682
Cos(1) => Cos( (1) ) => 0.5403022766113281
Tan(1) => Tan( (1) ) => 1.5574077367782593
ACos(0.5) => ACos( (0.5) ) => 1.0471975803375244
ASin(0.5) => ASin( (0.5) ) => 0.5235987901687622
ATan(0.5) => ATan( (0.5) ) => 0.46364760398864746
Ln(10) => Ln( (10) ) => 2.3025851249694824
Pow(2, 0.5) => Pow( (2), (0.5) ) => 1.4142135381698608
Mod(7.5, 2) => Mod( (7.5), (2) ) => 1.5
Lerp(1.5, 4, 0.3) => Lerp( (1.5), (4), (0.3) ) => 2.25
SmoothStep(0, 10, 2.5) => SmoothStep( (0), (10), (2.5) ) => 0.15625
SmoothStep(Vector2(0, 0), Vector2(1, 10), Vector2(0.5, 2.5)) => SmoothStep( (Vector2( (0), (0) )), (Vector2( (1), (10) )), (Vector2( (0.5), (2.5) )) ) => 0.5, 0.15625
SmoothStep(Vector4(0, 0, 0, 0), Vector4(1, 10, 1, 1), Vector4(0.5, 2.5, 2, -1)) => SmoothStep( (Vector4( (0), (0), (0), (0) )), (Vector4( (1), (10), (1), (1) )), (Vector4( (0.5), (2.5), (2), (-1) )) ) => <0.5, 0.15625, 1, 0>
abs(1) => abs( (1) ) => Invalid
Abs(true) => Abs( ([True]) ) => Invalid
Abs(Vector2(-1, 2)) => Abs( (Vector2( (-1), (2) )) ) => 1, 2
Abs(Vector4(-1, 2, -3, 4)) => Abs( (Vector4( (-1), (2), (-3), (4) )) ) => <1, 2, 3, 4>
Clamp(Vector2(5, -5), Vector2(0, 0), Vector2(1, 1)) => Clamp( (Vector2( (5), (-5) )), (Vector2( (0), (0) )), (Vector2( (1), (1) )) ) => 1, 0
Distance(Vector3(0, 0, 0), Vector3(2, 3, 6)) => Distance( (Vector3( (0), (0), (0) )), (Vector3( (2), (3), (6) )) ) => 7
Distance(Vector4(0, 0, 0, 0), Vector4(2, 4, 5, 6)) => Distance( (Vector4( (0), (0), (0), (0) )), (Vector4( (2), (4), (5), (6) )) ) => 9
Length(Vector2(3, 4)) => Length( (Vector2( (3), (4) )) ) => 5
LengthSquared(Vector3(1, 2, 2)) => LengthSquared( (Vector3( (1), (2), (2) )) ) => 9
Lerp(Vector2(0, 2), Vector2(4, 4), 0.3) => Lerp( (Vector2( (0), (2) )), (Vector2( (4), (4) )), (0.3) ) => 1.2000000476837158, 2.600000023841858
Lerp(Vector3(0, 2, 4), Vector3(4, 4, 4), 0.3) => Lerp( (Vector3( (0), (2), (4) )), (Vector3( (4), (4), (4) )), (0.3) ) => Vector3D { X = 1.2000000476837158, Y = 2.600000023841858, Z = 4, Length = 4.919349574732272 }
Lerp(Vector4(0, 2, 4, 0), Vector4(4, 4, 4, 1), 0.3) => Lerp( (Vector4( (0), (2), (4), (0) )), (Vector4( (4), (4), (4), (1) )), (0.3) ) => <1.2, 2.6, 4, 0.3>
Normalize(Vector2(3, 4)) => Normalize( (Vector2( (3), (4) )) ) => 0.6, 0.8
Normalize(Vector3(1, 2, 3)) => Normalize( (Vector3( (1), (2), (3) )) ) => Vector3D { X = 0.2672612419124244, Y = 0.5345224838248488, Z = 0.8017837257372732, Length = 1 }
Normalize(Vector4(1, 2, 3, 4)) => Normalize( (Vector4( (1), (2), (3), (4) )) ) => <0.18257418, 0.36514837, 0.5477225, 0.73029673>
Normalize(Quaternion(1, 2, 3, 4)) => Normalize( (Quaternion( (1), (2), (3), (4) )) ) => {X:0.18257418 Y:0.36514837 Z:0.5477225 W:0.73029673}
Scale(Vector3(1, 2, 3), 2) => Scale( (Vector3( (1), (2), (3) )), (2) ) => Vector3D { X = 2, Y = 4, Z = 6, Length = 7.483314773547883 }
Matrix3x2.CreateTranslation(Vector2(2, 3)) => Matrix3x2.CreateTranslation( (Vector2( (2), (3) )) ) => { {M11:2 M12:0} {M21:0 M22:3} {M31:0 M32:0} }
Matrix3x2.CreateFromTranslation(Vector2(2, 3)) => Matrix3x2.CreateFromTranslation( (Vector2( (2), (3) )) ) => { {M11:1 M12:0} {M21:0 M22:1} {M31:2 M32:3} }
Matrix3x2.CreateRotation(1) => Matrix3x2.CreateRotation( (1) ) => { {M11:0.5403023 M12:0.84147096} {M21:-0.84147096 M22:0.5403023} {M31:0 M32:0} }
Matrix3x2.CreateRotation(ToRadians(90)) => Matrix3x2.CreateRotation( (ToRadians( (90) )) ) => { {M11:0 M12:1} {M21:-1 M22:0} {M31:0 M32:0} }
Matrix3x2.CreateSkew(0.5, 0.25, Vector2(3, 4)) => Matrix3x2.CreateSkew( (0.5), (0.25), (Vector2( (3), (4) )) ) => { {M11:1 M12:0.25534192} {M21:0.5463025 M22:1} {M31:-2.18521 M32:-0.7660258} }
Matrix3x2(1, 2, 3, 4, 5, 6) => Matrix3x2( (1), (2), (3), (4), (5), (6) ) => { {M11:1 M12:2} {M21:3 M22:4} {M31:5 M32:5} }
Matrix4x4(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16) => Matrix4x4( (1), (2), (3), (4), (5), (6), (7), (8), (9), (10), (11), (12), (13), (14), (15), (16) ) => { {M11:1 M12:2 M13:3 M14:4} {M21:5 M22:5 M23:5 M24:5} {M31:5 M32:5 M33:5 M34:5} {M41:5 M42:5 M43:5 M44:5} }
Matrix4x4(Matrix3x2.CreateFromTranslation(Vector2(2, 3))) => Matrix4x4( (Matrix3x2.CreateFromTranslation( (Vector2( (2), (3) )) )) ) => { {M11:1 M12:0 M13:0 M14:0} {M21:0 M22:1 M23:0 M24:0} {M31:0 M32:0 M33:1 M34:0} {M41:2 M42:3 M43:0 M44:1} }
Matrix4x4.CreateFromAxisAngle(Vector3(0, 0, 1), 1) => Matrix4x4.CreateFromAxisAngle( (Vector3( (0), (0), (1) )), (1) ) => { {M11:0.5403023 M12:0.841471 M13:0 M14:0} {M21:-0.841471 M22:0.5403023 M23:0 M24:0} {M31:0 M32:0 M33:1 M34:0} {M41:0 M42:0 M43:0 M44:1} }
Matrix4x4.CreateTranslation(Vector3(2, 3, 4)) => Matrix4x4.CreateTranslation( (Vector3( (2), (3), (4) )) ) => { {M11:2 M12:0 M13:0 M14:0} {M21:0 M22:3 M23:0 M24:0} {M31:0 M32:0 M33:4 M34:0} {M41:0 M42:0 M43:0 M44:1} }
Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4)) => Matrix4x4.CreateFromTranslation( (Vector3( (2), (3), (4) )) ) => { {M11:1 M12:0 M13:0 M14:0} {M21:0 M22:1 M23:0 M24:0} {M31:0 M32:0 M33:1 M34:0} {M41:2 M42:3 M43:4 M44:1} }
Inverse(Matrix3x2.CreateScale(Vector2(2, 4))) => Inverse( (Matrix3x2.CreateScale( (Vector2( (2), (4) )) )) ) => { {M11:0.5 M12:-0} {M21:-0 M22:0.25} {M31:0 M32:0} }
Inverse(Scale(Matrix3x2.CreateRotation(1), 0)) => Inverse( (Scale( (Matrix3x2.CreateRotation( (1) )), (0) )) ) => { {M11:NaN M12:NaN} {M21:NaN M22:NaN} {M31:NaN M32:NaN} }
Inverse(Matrix4x4.CreateFromAxisAngle(Vector3(0, 0, 1), 1) * Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4))) => Inverse( ((Matrix4x4.CreateFromAxisAngle( (Vector3( (0), (0), (1) )), (1) )*Matrix4x4.CreateFromTranslation( (Vector3( (2), (3), (4) )) ))) ) => { {M11:0.5403023 M12:-0.841471 M13:0 M14:-0} {M21:0.841471 M22:0.5403023 M23:-0 M24:0} {M31:0 M32:-0 M33:1 M34:-0} {M41:-3.6050177 M42:0.062035203 M43:-4 M44:1} }
Inverse(Scale(Matrix4x4.CreateScale(Vector3(1, 1, 1)), 0)) => Inverse( (Scale( (Matrix4x4.CreateScale( (Vector3( (1), (1), (1) )) )), (0) )) ) => { {M11:NaN M12:NaN M13:NaN M14:NaN} {M21:NaN M22:NaN M23:NaN M24:NaN} {M31:NaN M32:NaN M33:NaN M34:NaN} {M41:NaN M42:NaN M43:NaN M44:NaN} }
Transform(Vector2(1, 1), Matrix3x2.CreateRotation(1)) => Transform( (Vector2( (1), (1) )), (Matrix3x2.CreateRotation( (1) )) ) => <-0.30116868, 1.3817732>
Transform(Vector3(1, 1, 1), Matrix4x4.CreateFromTranslation(Vector3(2, 3, 4))) => Transform( (Vector3( (1), (1), (1) )), (Matrix4x4.CreateFromTranslation( (Vector3( (2), (3), (4) )) )) ) => <3, 4, 5>
Matrix3x2.CreateScale(Vector2(2, 3)) * Matrix3x2.CreateFromTranslation(Vector2(1, 1)) => (Matrix3x2.CreateScale( (Vector2( (2), (3) )) )*Matrix3x2.CreateFromTranslation( (Vector2( (1), (1) )) )) => { {M11:2 M12:0} {M21:0 M22:3} {M31:1 M32:1} }
Quaternion.CreateFromAxisAngle(Vector3(0, 0, 1), 1) => Quaternion.CreateFromAxisAngle( (Vector3( (0), (0), (1) )), (1) ) => {X:0 Y:0 Z:0.47942555 W:0.87758255}
Concatenate(Quaternion.CreateFromAxisAngle(Vector3(0, 0, 1), 1), Quaternion.CreateFromAxisAngle(Vector3(1, 0, 0), 0.5)) => Concatenate( (Quaternion.CreateFromAxisAngle( (Vector3( (0), (0), (1) )), (1) )), (Quaternion.CreateFromAxisAngle( (Vector3( (1), (0), (0) )), (0.5) )) ) => {X:0.2171174 Y:-0.11861178 Z:0.46452138 W:0.8503006}
Slerp(Quaternion(0, 0, 0, 1), Quaternion.CreateFromAxisAngle(Vector3(0, 0, 1), 1), 0.3) => Slerp( (Quaternion( (0), (0), (0), (1) )), (Quaternion.CreateFromAxisAngle( (Vector3( (0), (0), (1) )), (1) )), (0.3) ) => {X:0 Y:0 Z:0.14943814 W:0.988771}
Quaternion(1, 2, 3, 4) / Quaternion(0, 1, 0, 1) => (Quaternion( (1), (2), (3), (4) )/Quaternion( (0), (1), (0), (1) )) => {X:2 Y:-1 Z:1 W:3}
Quaternion(1, 2, 3, 4) * Quaternion(0, 1, 0, 1) => (Quaternion( (1), (2), (3), (4) )*Quaternion( (0), (1), (0), (1) )) => {X:-2 Y:6 Z:4 W:2}
Length(Quaternion(0, 0, 3, 4)) => Length( (Quaternion( (0), (0), (3), (4) )) ) => 5
-Matrix3x2.CreateScale(Vector2(2, 3)) => -Matrix3x2.CreateScale( (Vector2( (2), (3) )) ) => { {M11:-2 M12:-0} {M21:-0 M22:-3} {M31:-0 M32:-0} }
Vector3(1, 2, 3) - Vector3(3, 2, 1) => (Vector3( (1), (2), (3) )-Vector3( (3), (2), (1) )) => Vector3D { X = -2, Y = 0, Z = 2, Length = 2.8284271247461903 }
unknown => {unknown} => Invalid
this.Target => [Target] => Invalid
this.StartingValue => [StartingValue] => Invalid
true => [True] => True
Max(Vector2(1, 5), Vector2(3, 2)) => Max( (Vector2( (1), (5) )), (Vector2( (3), (2) )) ) => 3, 5
Min(1, 2) => Min( (1), (2) ) => 1
1000000000 => 1E+09 => 1000000000
0.00001 => 1E-05 => 9.999999747378752E-06
123456789 => 123456790 => 123456792
"#;

#[test]
fn matches_the_output_of_the_reference_implementation() {
    let mut count = 0;
    for line in REFERENCE_OUTPUT.lines().filter(|l| !l.is_empty()) {
        let mut parts = line.splitn(3, " => ");
        let expression = parts.next().unwrap();
        let printed = parts.next().unwrap();
        let result = parts.next().unwrap();

        let (actual_printed, actual_result) = match Expression::parse(expression) {
            Ok(expr) => (expr.to_string(), expr.evaluate(&context()).to_string()),
            Err(error) => (format!("ERR {} @{}", error.message(), error.position()), String::new()),
        };
        assert_eq!(actual_printed, printed, "{expression}");

        // The reference text of a double-precision 3D vector also has its
        // length, which the `Display` of `Vector3D` in this crate omits.
        let result = match result.find(", Length = ") {
            Some(index) if result.starts_with("Vector3D {") => format!("{} }}", &result[..index]),
            _ => result.to_owned(),
        };
        assert_eq!(actual_result, result, "{expression}");
        count += 1;
    }
    assert_eq!(count, 141);
}

// ---------------------------------------------------------------------------
// ExpressionVariant
// ---------------------------------------------------------------------------

const REL: RelativeUnit = RelativeUnit::Relative;
const ABS: RelativeUnit = RelativeUnit::Absolute;

fn v2(x: f32, y: f32) -> ExpressionVariant {
    Vector2::new(x, y).into()
}

fn vd(x: f64, y: f64) -> ExpressionVariant {
    Vector::new(x, y).into()
}

fn v3(x: f32, y: f32, z: f32) -> ExpressionVariant {
    Vector3::new(x, y, z).into()
}

fn v3d(x: f64, y: f64, z: f64) -> ExpressionVariant {
    Vector3D::new(x, y, z).into()
}

fn v4(x: f32, y: f32, z: f32, w: f32) -> ExpressionVariant {
    Vector4::new(x, y, z, w).into()
}

fn rp(x: f64, y: f64, unit: RelativeUnit) -> ExpressionVariant {
    RelativePoint::new(x, y, unit).into()
}

fn rs(scalar: f64, unit: RelativeUnit) -> ExpressionVariant {
    RelativeScalar::new(scalar, unit).into()
}

/// One value of every variant type.
fn samples() -> Vec<ExpressionVariant> {
    vec![
        V::Invalid,
        true.into(),
        d(2.0),
        v2(1.0, 2.0),
        v3(1.0, 2.0, 3.0),
        v4(1.0, 2.0, 3.0, 4.0),
        vd(1.0, 2.0),
        v3d(1.0, 2.0, 3.0),
        Matrix::create_scale(2.0, 4.0).into(),
        Matrix3x2::create_scale_xy(2.0, 4.0).into(),
        Matrix4x4::create_scale_xyz(2.0, 4.0, 8.0).into(),
        Quaternion::new(1.0, 2.0, 3.0, 4.0).into(),
        Color::from_argb(1, 2, 3, 4).into(),
        rp(1.0, 2.0, REL),
        rs(2.0, REL),
        REL.into(),
    ]
}

#[test]
fn variant_types_and_default() {
    let types: Vec<VariantType> = samples().iter().map(|v| v.variant_type()).collect();
    assert_eq!(
        types,
        vec![
            VariantType::Invalid,
            VariantType::Boolean,
            VariantType::Double,
            VariantType::Vector2,
            VariantType::Vector3,
            VariantType::Vector4,
            VariantType::Vector,
            VariantType::Vector3D,
            VariantType::FerroMatrix,
            VariantType::Matrix3x2,
            VariantType::Matrix4x4,
            VariantType::Quaternion,
            VariantType::Color,
            VariantType::RelativePoint,
            VariantType::RelativeScalar,
            VariantType::RelativeUnit,
        ]
    );
    assert_eq!(ExpressionVariant::default(), V::Invalid);
    assert_eq!(VariantType::default(), VariantType::Invalid);
}

#[test]
fn variant_addition_and_subtraction() {
    assert_eq!(d(1.0) + d(2.0), d(3.0));
    assert_eq!(d(1.0) - d(2.0), d(-1.0));
    assert_eq!(v2(1.0, 2.0) + v2(3.0, 5.0), v2(4.0, 7.0));
    assert_eq!(v2(1.0, 2.0) - v2(3.0, 5.0), v2(-2.0, -3.0));
    assert_eq!(vd(1.0, 2.0) + vd(3.0, 5.0), vd(4.0, 7.0));
    assert_eq!(vd(1.0, 2.0) - vd(3.0, 5.0), vd(-2.0, -3.0));
    assert_eq!(v3(1.0, 2.0, 3.0) + v3(1.0, 1.0, 1.0), v3(2.0, 3.0, 4.0));
    assert_eq!(v3(1.0, 2.0, 3.0) - v3(1.0, 1.0, 1.0), v3(0.0, 1.0, 2.0));
    assert_eq!(v3d(1.0, 2.0, 3.0) + v3d(1.0, 1.0, 1.0), v3d(2.0, 3.0, 4.0));
    assert_eq!(v3d(1.0, 2.0, 3.0) - v3d(1.0, 1.0, 1.0), v3d(0.0, 1.0, 2.0));
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0) + v4(1.0, 1.0, 1.0, 1.0), v4(2.0, 3.0, 4.0, 5.0));
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0) - v4(1.0, 1.0, 1.0, 1.0), v4(0.0, 1.0, 2.0, 3.0));

    let m = Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(V::from(m) + m.into(), (m + m).into());
    assert_eq!(V::from(m) - m.into(), Matrix3x2::default().into());
    let m = Matrix4x4::create_scale_xyz(1.0, 2.0, 3.0);
    assert_eq!(V::from(m) + m.into(), (m + m).into());
    assert_eq!(V::from(m) - m.into(), Matrix4x4::default().into());
    let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(V::from(q) + q.into(), Quaternion::new(2.0, 4.0, 6.0, 8.0).into());
    assert_eq!(V::from(q) - q.into(), Quaternion::default().into());

    assert_eq!(rp(1.0, 2.0, REL) + rp(3.0, 4.0, REL), rp(4.0, 6.0, REL));
    assert_eq!(rp(1.0, 2.0, ABS) - rp(3.0, 5.0, ABS), rp(-2.0, -3.0, ABS));
    assert_eq!(rp(1.0, 2.0, REL) + rp(3.0, 4.0, ABS), V::Invalid);
    assert_eq!(rp(1.0, 2.0, REL) - rp(3.0, 4.0, ABS), V::Invalid);
    assert_eq!(rs(1.0, REL) + rs(3.0, REL), rs(4.0, REL));
    assert_eq!(rs(1.0, ABS) - rs(3.0, ABS), rs(-2.0, ABS));
    assert_eq!(rs(1.0, REL) + rs(3.0, ABS), V::Invalid);
    assert_eq!(rs(1.0, REL) - rs(3.0, ABS), V::Invalid);

    // Not defined: booleans, colors, units, the 3x3 matrix, mixed types.
    let m: ExpressionVariant = Matrix::IDENTITY.into();
    assert_eq!(m + m, V::Invalid);
    assert_eq!(m - m, V::Invalid);
    for v in [V::Invalid, true.into(), Color::default().into(), REL.into()] {
        assert_eq!(v + v, V::Invalid);
        assert_eq!(v - v, V::Invalid);
    }
    assert_eq!(d(1.0) + v2(1.0, 2.0), V::Invalid);
    assert_eq!(v2(1.0, 2.0) + vd(1.0, 2.0), V::Invalid);
    assert_eq!(v3(1.0, 2.0, 3.0) - v3d(1.0, 2.0, 3.0), V::Invalid);
}

#[test]
fn variant_negation() {
    assert_eq!(-d(1.0), d(-1.0));
    assert_eq!(-v2(1.0, -2.0), v2(-1.0, 2.0));
    assert_eq!(-vd(1.0, -2.0), vd(-1.0, 2.0));
    assert_eq!(-v3(1.0, -2.0, 3.0), v3(-1.0, 2.0, -3.0));
    assert_eq!(-v3d(1.0, -2.0, 3.0), v3d(-1.0, 2.0, -3.0));
    assert_eq!(-v4(1.0, -2.0, 3.0, 4.0), v4(-1.0, 2.0, -3.0, -4.0));
    let m = Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(-V::from(m), (-m).into());
    let m = Matrix4x4::create_scale_xyz(1.0, 2.0, 3.0);
    assert_eq!(-V::from(m), (-m).into());
    // The 3x3 matrix defines its own negation operator.
    let m = Matrix::create_scale(2.0, 4.0);
    assert_eq!(-V::from(m), (-m).into());
    assert_eq!(
        -V::from(Quaternion::new(1.0, 2.0, 3.0, 4.0)),
        Quaternion::new(-1.0, -2.0, -3.0, -4.0).into()
    );
    assert_eq!(-rp(1.0, -2.0, ABS), rp(-1.0, 2.0, ABS));
    assert_eq!(-rs(1.0, REL), rs(-1.0, REL));
    for v in [V::Invalid, true.into(), Color::default().into(), REL.into()] {
        assert_eq!(-v, V::Invalid);
    }
}

#[test]
fn variant_multiplication() {
    assert_eq!(d(3.0) * d(2.0), d(6.0));
    assert_eq!(v2(1.0, 2.0) * v2(3.0, 5.0), v2(3.0, 10.0));
    assert_eq!(v2(1.0, 2.0) * d(2.0), v2(2.0, 4.0));
    assert_eq!(vd(1.0, 2.0) * vd(3.0, 5.0), vd(3.0, 10.0));
    assert_eq!(vd(1.0, 2.0) * d(2.0), vd(2.0, 4.0));
    assert_eq!(v3(1.0, 2.0, 3.0) * v3(2.0, 2.0, 3.0), v3(2.0, 4.0, 9.0));
    assert_eq!(v3(1.0, 2.0, 3.0) * d(2.0), v3(2.0, 4.0, 6.0));
    assert_eq!(v3d(1.0, 2.0, 3.0) * v3d(2.0, 2.0, 3.0), v3d(2.0, 4.0, 9.0));
    assert_eq!(v3d(1.0, 2.0, 3.0) * d(2.0), v3d(2.0, 4.0, 6.0));
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0) * v4(2.0, 2.0, 3.0, 0.5), v4(2.0, 4.0, 9.0, 2.0));
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0) * d(2.0), v4(2.0, 4.0, 6.0, 8.0));

    let a = Matrix3x2::create_scale_xy(2.0, 3.0);
    let b = Matrix3x2::create_translation_xy(1.0, 1.0);
    assert_eq!(V::from(a) * b.into(), (a * b).into());
    assert_eq!(V::from(a) * d(2.0), (a * 2.0).into());
    let a = Matrix::create_scale(2.0, 3.0);
    let b = Matrix::create_translation(1.0, 1.0);
    assert_eq!(V::from(a) * b.into(), (a * b).into());
    assert_eq!(V::from(a) * d(2.0), V::Invalid);
    let a = Matrix4x4::create_scale_xyz(2.0, 3.0, 4.0);
    let b = Matrix4x4::create_translation_xyz(1.0, 1.0, 1.0);
    assert_eq!(V::from(a) * b.into(), (a * b).into());
    assert_eq!(V::from(a) * d(2.0), (a * 2.0).into());
    let a = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let b = Quaternion::new(0.0, 1.0, 0.0, 1.0);
    assert_eq!(V::from(a) * b.into(), (a * b).into());
    assert_eq!(V::from(a) * d(2.0), (a * 2.0).into());

    assert_eq!(rp(1.0, 2.0, REL) * d(2.0), rp(2.0, 4.0, REL));
    assert_eq!(d(2.0) * rp(1.0, 2.0, ABS), rp(2.0, 4.0, ABS));
    assert_eq!(rs(1.5, REL) * d(2.0), rs(3.0, REL));
    assert_eq!(d(2.0) * rs(1.5, ABS), rs(3.0, ABS));
    assert_eq!(rp(1.0, 2.0, REL) * rp(3.0, 4.0, REL), rp(3.0, 8.0, REL));
    assert_eq!(rp(1.0, 2.0, REL) * rp(3.0, 4.0, ABS), V::Invalid);
    assert_eq!(rs(2.0, REL) * rs(3.0, REL), rs(6.0, REL));
    assert_eq!(rs(2.0, REL) * rs(3.0, ABS), V::Invalid);

    // A scalar on the left of a vector, matrix or quaternion is not defined.
    assert_eq!(d(2.0) * v2(1.0, 2.0), V::Invalid);
    assert_eq!(d(2.0) * vd(1.0, 2.0), V::Invalid);
    assert_eq!(d(2.0) * v3(1.0, 2.0, 3.0), V::Invalid);
    assert_eq!(d(2.0) * V::from(Quaternion::IDENTITY), V::Invalid);
    assert_eq!(v2(1.0, 2.0) * vd(1.0, 2.0), V::Invalid);
    assert_eq!(V::Invalid * d(1.0), V::Invalid);
    assert_eq!(d(1.0) * V::Invalid, V::Invalid);
    assert_eq!(V::Boolean(true) * V::Boolean(true), V::Invalid);
}

#[test]
fn variant_division_and_remainder() {
    assert_eq!(d(3.0) / d(2.0), d(1.5));
    assert_eq!(v2(3.0, 10.0) / v2(3.0, 5.0), v2(1.0, 2.0));
    assert_eq!(v2(2.0, 4.0) / d(2.0), v2(1.0, 2.0));
    assert_eq!(vd(3.0, 10.0) / vd(3.0, 5.0), vd(1.0, 2.0));
    assert_eq!(vd(2.0, 4.0) / d(2.0), vd(1.0, 2.0));
    assert_eq!(v3(2.0, 4.0, 9.0) / v3(2.0, 2.0, 3.0), v3(1.0, 2.0, 3.0));
    assert_eq!(v3(2.0, 4.0, 6.0) / d(2.0), v3(1.0, 2.0, 3.0));
    assert_eq!(v3d(2.0, 4.0, 9.0) / v3d(2.0, 2.0, 3.0), v3d(1.0, 2.0, 3.0));
    assert_eq!(v3d(2.0, 4.0, 6.0) / d(2.0), v3d(1.0, 2.0, 3.0));
    assert_eq!(v4(2.0, 4.0, 9.0, 2.0) / v4(2.0, 2.0, 3.0, 0.5), v4(1.0, 2.0, 3.0, 4.0));
    assert_eq!(v4(2.0, 4.0, 6.0, 8.0) / d(2.0), v4(1.0, 2.0, 3.0, 4.0));
    let a = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let b = Quaternion::new(0.0, 1.0, 0.0, 1.0);
    assert_eq!(V::from(a) / b.into(), (a / b).into());
    assert_eq!(V::from(a) / d(2.0), V::Invalid);
    assert_eq!(rp(2.0, 4.0, REL) / d(2.0), rp(1.0, 2.0, REL));
    assert_eq!(rs(3.0, ABS) / d(2.0), rs(1.5, ABS));
    assert_eq!(rp(3.0, 8.0, REL) / rp(3.0, 4.0, REL), rp(1.0, 2.0, REL));
    assert_eq!(rp(3.0, 8.0, REL) / rp(3.0, 4.0, ABS), V::Invalid);
    assert_eq!(rs(6.0, REL) / rs(3.0, REL), rs(2.0, REL));
    assert_eq!(rs(6.0, REL) / rs(3.0, ABS), V::Invalid);
    // Not defined: matrices, a scalar divided by a relative value.
    let m: ExpressionVariant = Matrix3x2::IDENTITY.into();
    assert_eq!(m / m, V::Invalid);
    assert_eq!(m / d(2.0), V::Invalid);
    let m: ExpressionVariant = Matrix4x4::IDENTITY.into();
    assert_eq!(m / m, V::Invalid);
    let m: ExpressionVariant = Matrix::IDENTITY.into();
    assert_eq!(m / m, V::Invalid);
    assert_eq!(d(2.0) / rs(1.0, REL), V::Invalid);
    assert_eq!(d(2.0) / rp(1.0, 1.0, REL), V::Invalid);
    assert_eq!(V::Invalid / d(1.0), V::Invalid);

    assert_eq!(d(7.0) % d(4.0), d(3.0));
    assert_eq!(d(-7.0) % d(4.0), d(-3.0));
    assert_eq!(rs(7.0, REL) % rs(4.0, REL), rs(3.0, REL));
    assert_eq!(rs(7.0, REL) % rs(4.0, ABS), V::Invalid);
    assert_eq!(v2(1.0, 2.0) % v2(1.0, 2.0), V::Invalid);
    assert_eq!(d(7.0) % V::Invalid, V::Invalid);
}

#[test]
fn variant_comparisons() {
    // Every type but Invalid and Color is comparable with itself.
    for v in samples() {
        let expected = match v {
            V::Invalid | V::Color(_) => V::Invalid,
            _ => V::Boolean(true),
        };
        assert_eq!(v.equals_to(v), expected, "{v:?}");
        let expected_not = match expected {
            V::Boolean(b) => V::Boolean(!b),
            other => other,
        };
        assert_eq!(v.not_equals_to(v), expected_not, "{v:?}");
    }
    // Values of different types are not comparable.
    let all = samples();
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_eq!(a.equals_to(*b), V::Invalid);
                assert_eq!(a.not_equals_to(*b), V::Invalid);
            }
        }
    }
    assert_eq!(d(1.0).equals_to(d(2.0)), V::Boolean(false));
    assert_eq!(d(1.0).not_equals_to(d(2.0)), V::Boolean(true));
    assert_eq!(d(f64::NAN).equals_to(d(f64::NAN)), V::Boolean(false));
    assert_eq!(v2(1.0, 2.0).equals_to(v2(1.0, 3.0)), V::Boolean(false));
    assert_eq!(V::Boolean(true).equals_to(V::Boolean(false)), V::Boolean(false));
    assert_eq!(rs(1.0, REL).equals_to(rs(1.0, ABS)), V::Boolean(false));
    assert_eq!(V::from(REL).equals_to(ABS.into()), V::Boolean(false));

    assert_eq!(d(1.0).less_than(d(2.0)), V::Boolean(true));
    assert_eq!(d(2.0).less_than(d(2.0)), V::Boolean(false));
    assert_eq!(d(3.0).more_than(d(2.0)), V::Boolean(true));
    assert_eq!(d(2.0).more_than(d(2.0)), V::Boolean(false));
    assert_eq!(rs(1.0, REL).less_than(rs(2.0, REL)), V::Boolean(true));
    assert_eq!(rs(1.0, REL).more_than(rs(2.0, REL)), V::Boolean(false));
    assert_eq!(rs(1.0, REL).less_than(rs(2.0, ABS)), V::Invalid);
    assert_eq!(rs(1.0, REL).more_than(rs(2.0, ABS)), V::Invalid);
    assert_eq!(v2(1.0, 2.0).less_than(v2(1.0, 2.0)), V::Invalid);
    assert_eq!(d(1.0).more_than(V::Boolean(true)), V::Invalid);
}

#[test]
fn variant_logic() {
    let (t, f) = (V::Boolean(true), V::Boolean(false));
    assert_eq!(t.and(t), t);
    assert_eq!(t.and(f), f);
    assert_eq!(f.or(t), t);
    assert_eq!(f.or(f), f);
    assert_eq!(!t, f);
    assert_eq!(!f, t);
    assert_eq!(t.and(d(1.0)), V::Invalid);
    assert_eq!(d(1.0).or(t), V::Invalid);
    assert_eq!(!d(1.0), V::Invalid);
    assert_eq!(!V::Invalid, V::Invalid);
}

#[test]
fn variant_members() {
    let m = Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    let v = V::from(m);
    let members: Vec<_> = ["M11", "M12", "M21", "M22", "M31", "M32", "M13"]
        .iter()
        .map(|n| v.get_property(n))
        .collect();
    assert_eq!(members, vec![d(1.0), d(2.0), d(3.0), d(4.0), d(5.0), d(6.0), V::Invalid]);

    let v = V::from(Matrix::new_3x3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0));
    let members: Vec<_> = ["M11", "M12", "M13", "M21", "M22", "M23", "M31", "M32", "M33", "M44"]
        .iter()
        .map(|n| v.get_property(n))
        .collect();
    assert_eq!(
        members,
        vec![d(1.0), d(2.0), d(3.0), d(4.0), d(5.0), d(6.0), d(7.0), d(8.0), d(9.0), V::Invalid]
    );

    let v = V::from(Matrix4x4::new(
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ));
    let names = [
        "M11", "M12", "M13", "M14", "M21", "M22", "M23", "M24", "M31", "M32", "M33", "M34", "M41", "M42", "M43", "M44",
    ];
    for (i, name) in names.iter().enumerate() {
        assert_eq!(v.get_property(name), d(i as f64 + 1.0));
    }
    assert_eq!(v.get_property("M45"), V::Invalid);

    assert_eq!(v2(1.0, 2.0).get_property("X"), d(1.0));
    assert_eq!(v2(1.0, 2.0).get_property("Y"), d(2.0));
    assert_eq!(v2(1.0, 2.0).get_property("XY"), V::Invalid);
    assert_eq!(vd(1.0, 2.0).get_property("Y"), d(2.0));
    for (name, expected) in [
        ("XY", (1.0, 2.0)),
        ("YX", (2.0, 1.0)),
        ("XZ", (1.0, 3.0)),
        ("ZX", (3.0, 1.0)),
        ("YZ", (2.0, 3.0)),
        ("ZY", (3.0, 2.0)),
    ] {
        assert_eq!(v3(1.0, 2.0, 3.0).get_property(name), v2(expected.0, expected.1));
        assert_eq!(
            v3d(1.0, 2.0, 3.0).get_property(name),
            vd(expected.0 as f64, expected.1 as f64)
        );
    }
    assert_eq!(v3(1.0, 2.0, 3.0).get_property("Z"), d(3.0));
    assert_eq!(v3d(1.0, 2.0, 3.0).get_property("Z"), d(3.0));
    assert_eq!(v3(1.0, 2.0, 3.0).get_property("XYZ"), V::Invalid);
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0).get_property("W"), d(4.0));
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0).get_property("XY"), V::Invalid);

    let q = V::from(Quaternion::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(
        ["X", "Y", "Z", "W"].map(|n| q.get_property(n)),
        [d(1.0), d(2.0), d(3.0), d(4.0)]
    );
    let c = V::from(Color::from_argb(1, 2, 3, 4));
    assert_eq!(
        ["A", "R", "G", "B", "X"].map(|n| c.get_property(n)),
        [d(1.0), d(2.0), d(3.0), d(4.0), V::Invalid]
    );
    // The point coordinates are narrowed to single precision.
    let p = rp(0.1, 2.0, ABS);
    assert_eq!(p.get_property("X"), d(0.1f32 as f64));
    assert_eq!(p.get_property("Y"), d(2.0));
    assert_eq!(p.get_property("Unit"), ABS.into());
    assert_eq!(p.get_property("Point"), V::Invalid);
    let s = rs(0.1, REL);
    assert_eq!(s.get_property("Scalar"), d(0.1));
    assert_eq!(s.get_property("Unit"), REL.into());
    assert_eq!(s.get_property("X"), V::Invalid);

    for v in [V::Invalid, true.into(), d(1.0), REL.into()] {
        assert_eq!(v.get_property("X"), V::Invalid);
    }
}

#[test]
fn variant_casts() {
    assert_eq!(V::Boolean(true).try_cast::<bool>(), Some(true));
    assert_eq!(d(0.1).try_cast::<f32>(), Some(0.1f32));
    assert_eq!(d(0.1).try_cast::<f64>(), Some(0.1));
    assert_eq!(d(0.1).try_cast::<bool>(), None);
    assert_eq!(V::Boolean(true).try_cast::<f64>(), None);

    // The two vector families convert into each other.
    assert_eq!(v2(1.0, 2.0).try_cast::<Vector2>(), Some(Vector2::new(1.0, 2.0)));
    assert_eq!(vd(1.0, 2.0).try_cast::<Vector2>(), Some(Vector2::new(1.0, 2.0)));
    assert_eq!(vd(1.0, 2.0).try_cast::<Vector>(), Some(Vector::new(1.0, 2.0)));
    assert_eq!(v2(1.0, 2.0).try_cast::<Vector>(), Some(Vector::new(1.0, 2.0)));
    assert_eq!(v3(1.0, 2.0, 3.0).try_cast::<Vector3>(), Some(Vector3::new(1.0, 2.0, 3.0)));
    assert_eq!(v3d(1.0, 2.0, 3.0).try_cast::<Vector3>(), Some(Vector3::new(1.0, 2.0, 3.0)));
    assert_eq!(
        v3d(1.0, 2.0, 3.0).try_cast::<Vector3D>(),
        Some(Vector3D::new(1.0, 2.0, 3.0))
    );
    assert_eq!(
        v3(1.0, 2.0, 3.0).try_cast::<Vector3D>(),
        Some(Vector3D::new(1.0, 2.0, 3.0))
    );
    assert_eq!(v3(1.0, 2.0, 3.0).try_cast::<Vector2>(), None);
    assert_eq!(v2(1.0, 2.0).try_cast::<Vector3D>(), None);
    assert_eq!(v4(1.0, 2.0, 3.0, 4.0).try_cast::<Vector3>(), None);

    assert_eq!(
        v4(1.0, 2.0, 3.0, 4.0).try_cast::<Vector4>(),
        Some(Vector4::new(1.0, 2.0, 3.0, 4.0))
    );
    let m = Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(V::from(m).try_cast::<Matrix3x2>(), Some(m));
    assert_eq!(V::from(m).try_cast::<Matrix>(), None);
    assert_eq!(V::from(Matrix::IDENTITY).try_cast::<Matrix>(), Some(Matrix::IDENTITY));
    assert_eq!(
        V::from(Matrix4x4::IDENTITY).try_cast::<Matrix4x4>(),
        Some(Matrix4x4::IDENTITY)
    );
    assert_eq!(
        V::from(Quaternion::IDENTITY).try_cast::<Quaternion>(),
        Some(Quaternion::IDENTITY)
    );
    let c = Color::from_argb(1, 2, 3, 4);
    assert_eq!(V::from(c).try_cast::<Color>(), Some(c));
    assert_eq!(
        rp(1.0, 2.0, ABS).try_cast::<RelativePoint>(),
        Some(RelativePoint::new(1.0, 2.0, ABS))
    );
    assert_eq!(
        rs(1.0, ABS).try_cast::<RelativeScalar>(),
        Some(RelativeScalar::new(1.0, ABS))
    );
    assert_eq!(V::from(ABS).try_cast::<RelativeUnit>(), Some(ABS));
    assert_eq!(V::Invalid.try_cast::<f64>(), None);

    assert_eq!(V::Boolean(true).cast_or_default::<f64>(), 0.0);
    assert_eq!(d(2.0).cast_or_default::<Vector2>(), Vector2::default());
    assert_eq!(d(2.0).cast_or_default::<f32>(), 2.0);
    assert_eq!(V::Invalid.cast_or_default::<RelativeUnit>(), RelativeUnit::default());
}

#[test]
fn variant_create() {
    assert_eq!(V::create(true), V::Boolean(true));
    assert_eq!(V::create(0.1f32), d(0.1f32 as f64));
    assert_eq!(V::create(0.1f64), d(0.1));
    assert_eq!(V::create(Vector2::new(1.0, 2.0)), v2(1.0, 2.0));
    assert_eq!(V::create(Vector::new(1.0, 2.0)), vd(1.0, 2.0));
    assert_eq!(V::create(Vector3::new(1.0, 2.0, 3.0)), v3(1.0, 2.0, 3.0));
    assert_eq!(V::create(Vector3D::new(1.0, 2.0, 3.0)), v3d(1.0, 2.0, 3.0));
    assert_eq!(V::create(Vector4::new(1.0, 2.0, 3.0, 4.0)), v4(1.0, 2.0, 3.0, 4.0));
    assert_eq!(V::create(Matrix3x2::IDENTITY).variant_type(), VariantType::Matrix3x2);
    assert_eq!(V::create(Matrix::IDENTITY).variant_type(), VariantType::FerroMatrix);
    assert_eq!(V::create(Matrix4x4::IDENTITY).variant_type(), VariantType::Matrix4x4);
    assert_eq!(V::create(Quaternion::IDENTITY).variant_type(), VariantType::Quaternion);
    assert_eq!(V::create(Color::default()).variant_type(), VariantType::Color);
    assert_eq!(V::create(RelativePoint::default()).variant_type(), VariantType::RelativePoint);
    assert_eq!(
        V::create(RelativeScalar::default()).variant_type(),
        VariantType::RelativeScalar
    );
    assert_eq!(V::create(ABS), V::RelativeUnit(ABS));
}

#[test]
fn variant_to_string() {
    assert_eq!(V::Boolean(true).to_string(), "True");
    assert_eq!(V::Boolean(false).to_string(), "False");
    assert_eq!(d(1.5).to_string(), "1.5");
    assert_eq!(d(1e20).to_string(), "1E+20");
    assert_eq!(v2(1.0, 2.5).to_string(), "<1, 2.5>");
    assert_eq!(vd(1.0, 2.5).to_string(), Vector::new(1.0, 2.5).to_string());
    assert_eq!(v3(1.0, 2.5, 3.0).to_string(), "<1, 2.5, 3>");
    assert_eq!(v3d(1.0, 2.5, 3.0).to_string(), Vector3D::new(1.0, 2.5, 3.0).to_string());
    assert_eq!(v4(1.0, 2.5, 3.0, 4.0).to_string(), "<1, 2.5, 3, 4>");
    assert_eq!(
        V::from(Quaternion::new(1.0, 2.0, 3.0, 4.0)).to_string(),
        "{X:1 Y:2 Z:3 W:4}"
    );
    assert_eq!(
        V::from(Matrix3x2::IDENTITY).to_string(),
        "{ {M11:1 M12:0} {M21:0 M22:1} {M31:0 M32:0} }"
    );
    assert_eq!(V::from(Matrix::IDENTITY).to_string(), Matrix::IDENTITY.to_string());
    assert_eq!(
        V::from(Matrix4x4::IDENTITY).to_string(),
        "{ {M11:1 M12:0 M13:0 M14:0} {M21:0 M22:1 M23:0 M24:0} {M31:0 M32:0 M33:1 M34:0} {M41:0 M42:0 M43:0 M44:1} }"
    );
    let c = Color::from_argb(1, 2, 3, 4);
    assert_eq!(V::from(c).to_string(), c.to_string());
    assert_eq!(
        rp(1.0, 2.0, ABS).to_string(),
        RelativePoint::new(1.0, 2.0, ABS).to_string()
    );
    assert_eq!(rs(0.5, REL).to_string(), RelativeScalar::new(0.5, REL).to_string());
    assert_eq!(V::from(REL).to_string(), "Relative");
    assert_eq!(V::from(ABS).to_string(), "Absolute");
    assert_eq!(V::Invalid.to_string(), "Invalid");
}
