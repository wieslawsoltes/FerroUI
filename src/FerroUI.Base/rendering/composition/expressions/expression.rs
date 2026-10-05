//! The syntax tree of a parsed composition expression.
//!
//! The reference implementation is a class hierarchy; here the tree is the
//! [`Expression`] enum, one variant (and one struct) per node class. A tree
//! is plain data (`Send + Sync`): it is built on the UI thread and evaluated
//! by the server compositor.

use std::collections::HashSet;
use std::fmt;

use super::expression_evaluation_context::ExpressionEvaluationContext;
use super::expression_parse_exception::ExpressionParseException;
use super::expression_parser::ExpressionParser;
use super::expression_variant::ExpressionVariant;
use crate::numerics::InvariantF32;

/// A parsed composition expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Conditional(ConditionalExpression),
    Constant(ConstantExpression),
    FunctionCall(FunctionCallExpression),
    MemberAccess(MemberAccessExpression),
    Parameter(ParameterExpression),
    Keyword(KeywordExpression),
    Unary(UnaryExpression),
    Binary(BinaryExpression),
}

impl Expression {
    /// The kind of the node.
    pub fn expression_type(&self) -> ExpressionType {
        match self {
            Expression::Conditional(_) => ExpressionType::ConditionalExpression,
            Expression::Constant(_) => ExpressionType::Constant,
            Expression::FunctionCall(_) => ExpressionType::FunctionCall,
            Expression::MemberAccess(_) => ExpressionType::MemberAccess,
            Expression::Parameter(_) => ExpressionType::Parameter,
            Expression::Keyword(_) => ExpressionType::Keyword,
            Expression::Unary(e) => e.expression_type,
            Expression::Binary(e) => e.expression_type,
        }
    }

    /// Parses an expression.
    pub fn parse(expression: &str) -> Result<Expression, ExpressionParseException> {
        ExpressionParser::parse(expression)
    }

    /// Evaluates the expression. Anything that cannot be evaluated (an
    /// unknown parameter, member or function, mismatched operand types)
    /// gives an invalid variant.
    ///
    /// # Panics
    /// Panics when the expression reads a member of `this.Target` and the
    /// context has no target, or calls a function and the context has no
    /// foreign function interface (programmer errors).
    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        match self {
            Expression::Conditional(e) => e.evaluate(context),
            Expression::Constant(e) => e.evaluate(context),
            Expression::FunctionCall(e) => e.evaluate(context),
            Expression::MemberAccess(e) => e.evaluate(context),
            Expression::Parameter(e) => e.evaluate(context),
            Expression::Keyword(e) => e.evaluate(context),
            Expression::Unary(e) => e.evaluate(context),
            Expression::Binary(e) => e.evaluate(context),
        }
    }

    /// Collects the `(parameter, property)` pairs the expression reads.
    /// Members of `this.Target` are collected with the parameter name
    /// [`ExpressionKeywords::TARGET`].
    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        match self {
            Expression::Conditional(e) => e.collect_references(references),
            Expression::FunctionCall(e) => e.collect_references(references),
            Expression::MemberAccess(e) => e.collect_references(references),
            Expression::Unary(e) => e.collect_references(references),
            Expression::Binary(e) => e.collect_references(references),
            Expression::Constant(_) | Expression::Parameter(_) | Expression::Keyword(_) => {}
        }
    }

    /// The text of an operator, or the name of the expression type for the
    /// types that are not operators.
    pub fn operator_name(t: ExpressionType) -> &'static str {
        match t {
            ExpressionType::Add => "+",
            ExpressionType::Subtract => "-",
            ExpressionType::Divide => "/",
            ExpressionType::Multiply => "*",
            ExpressionType::MoreThan => ">",
            ExpressionType::LessThan => "<",
            ExpressionType::MoreThanOrEqual => ">=",
            ExpressionType::LessThanOrEqual => "<=",
            ExpressionType::LogicalAnd => "&&",
            ExpressionType::LogicalOr => "||",
            ExpressionType::Remainder => "%",
            // The reference implementation looks the operator text up by the
            // name of the enumeration member, and for this member the lookup
            // finds the inherited `Equals` method first, so the name of the
            // member is used instead of `==`.
            ExpressionType::Equals => "Equals",
            ExpressionType::NotEquals => "!=",
            ExpressionType::Not => "!",
            ExpressionType::UnaryMinus => "-",
            ExpressionType::MemberAccess => "MemberAccess",
            ExpressionType::Parameter => "Parameter",
            ExpressionType::FunctionCall => "FunctionCall",
            ExpressionType::Keyword => "Keyword",
            ExpressionType::Constant => "Constant",
            ExpressionType::ConditionalExpression => "ConditionalExpression",
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Conditional(e) => e.fmt(f),
            Expression::Constant(e) => e.fmt(f),
            Expression::FunctionCall(e) => e.fmt(f),
            Expression::MemberAccess(e) => e.fmt(f),
            Expression::Parameter(e) => e.fmt(f),
            Expression::Keyword(e) => e.fmt(f),
            Expression::Unary(e) => e.fmt(f),
            Expression::Binary(e) => e.fmt(f),
        }
    }
}

macro_rules! into_expression {
    ($($variant:ident($ty:ident)),+ $(,)?) => {
        $(impl From<$ty> for Expression {
            #[inline]
            fn from(value: $ty) -> Expression {
                Expression::$variant(value)
            }
        })+
    };
}

into_expression!(
    Conditional(ConditionalExpression),
    Constant(ConstantExpression),
    FunctionCall(FunctionCallExpression),
    MemberAccess(MemberAccessExpression),
    Parameter(ParameterExpression),
    Keyword(KeywordExpression),
    Unary(UnaryExpression),
    Binary(BinaryExpression),
);

/// The kind of an expression node (for unary and binary nodes, the operator).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpressionType {
    // Binary operators
    Add,
    Subtract,
    Divide,
    Multiply,
    MoreThan,
    LessThan,
    MoreThanOrEqual,
    LessThanOrEqual,
    LogicalAnd,
    LogicalOr,
    Remainder,
    Equals,
    NotEquals,
    // Unary operators
    Not,
    UnaryMinus,
    // The rest
    MemberAccess,
    Parameter,
    FunctionCall,
    Keyword,
    Constant,
    ConditionalExpression,
}

/// The keywords of the expression language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpressionKeyword {
    StartingValue,
    CurrentValue,
    FinalValue,
    Target,
    Pi,
    True,
    False,
}

/// The (lower-case) text of the keywords of the expression language.
pub struct ExpressionKeywords;

impl ExpressionKeywords {
    pub const STARTING_VALUE: &'static str = "this.startingvalue";
    pub const CURRENT_VALUE: &'static str = "this.currentvalue";
    pub const FINAL_VALUE: &'static str = "this.finalvalue";
    pub const PI: &'static str = "pi";
    pub const TRUE: &'static str = "true";
    pub const FALSE: &'static str = "false";
    pub const TARGET: &'static str = "this.target";
}

/// `condition ? true_part : false_part`.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalExpression {
    pub condition: Box<Expression>,
    pub true_part: Box<Expression>,
    pub false_part: Box<Expression>,
}

impl ConditionalExpression {
    pub fn new(condition: Expression, true_part: Expression, false_part: Expression) -> Self {
        Self {
            condition: Box::new(condition),
            true_part: Box::new(true_part),
            false_part: Box::new(false_part),
        }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        let cond = self.condition.evaluate(context);
        if cond == ExpressionVariant::Boolean(true) {
            return self.true_part.evaluate(context);
        }
        self.false_part.evaluate(context)
    }

    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        self.condition.collect_references(references);
        self.true_part.collect_references(references);
        self.false_part.collect_references(references);
    }
}

impl fmt::Display for ConditionalExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}) ? ({}) : ({})", self.condition, self.true_part, self.false_part)
    }
}

/// A numeric literal.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstantExpression {
    pub constant: f32,
}

impl ConstantExpression {
    pub fn new(constant: f32) -> Self {
        Self { constant }
    }

    pub fn evaluate(&self, _context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        self.constant.into()
    }
}

impl fmt::Display for ConstantExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        InvariantF32(self.constant).fmt(f)
    }
}

/// A call of a foreign function.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCallExpression {
    pub name: String,
    pub parameters: Vec<Expression>,
}

impl FunctionCallExpression {
    pub fn new(name: impl Into<String>, parameters: Vec<Expression>) -> Self {
        Self {
            name: name.into(),
            parameters,
        }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        let mut args = Vec::with_capacity(self.parameters.len());
        for expr in &self.parameters {
            args.push(expr.evaluate(context));
        }
        context
            .foreign_function_interface
            .expect("the expression evaluation context has no foreign function interface")
            .call(&self.name, &args)
            .unwrap_or_default()
    }

    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        for arg in &self.parameters {
            arg.collect_references(references);
        }
    }
}

impl fmt::Display for FunctionCallExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}( (", self.name)?;
        for (i, parameter) in self.parameters.iter().enumerate() {
            if i != 0 {
                f.write_str("), (")?;
            }
            parameter.fmt(f)?;
        }
        f.write_str(") )")
    }
}

/// `target.member`.
#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessExpression {
    pub target: Box<Expression>,
    pub member: String,
}

impl MemberAccessExpression {
    pub fn new(target: Expression, member: impl Into<String>) -> Self {
        Self {
            target: Box::new(target),
            member: member.into(),
        }
    }

    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        self.target.collect_references(references);
        match &*self.target {
            Expression::Parameter(pe) => {
                references.insert((pe.name.clone(), self.member.clone()));
            }
            Expression::Keyword(KeywordExpression {
                keyword: ExpressionKeyword::Target,
            }) => {
                references.insert((ExpressionKeywords::TARGET.to_owned(), self.member.clone()));
            }
            _ => {}
        }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        match &*self.target {
            Expression::Keyword(ke) if ke.keyword == ExpressionKeyword::Target => {
                return context
                    .target
                    .expect("the expression evaluation context has no target")
                    .get_property(&self.member);
            }
            Expression::Parameter(pe) => {
                if let Some(obj) = context.parameters.and_then(|p| p.get_object_parameter(&pe.name)) {
                    return obj.get_property(&self.member);
                }
            }
            _ => {}
        }
        // Those are considered immutable
        self.target.evaluate(context).get_property(&self.member)
    }
}

impl fmt::Display for MemberAccessExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}).{}", self.target, self.member)
    }
}

/// A reference to a parameter of the animation.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterExpression {
    pub name: String,
}

impl ParameterExpression {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        context
            .parameters
            .map(|p| p.get_parameter(&self.name))
            .unwrap_or_default()
    }
}

impl fmt::Display for ParameterExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", self.name)
    }
}

/// A keyword of the expression language.
#[derive(Clone, Debug, PartialEq)]
pub struct KeywordExpression {
    pub keyword: ExpressionKeyword,
}

impl KeywordExpression {
    pub fn new(keyword: ExpressionKeyword) -> Self {
        Self { keyword }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        match self.keyword {
            ExpressionKeyword::StartingValue => context.starting_value,
            ExpressionKeyword::CurrentValue => context.current_value,
            ExpressionKeyword::FinalValue => context.final_value,
            // should be handled by MemberAccess
            ExpressionKeyword::Target => ExpressionVariant::Invalid,
            ExpressionKeyword::True => true.into(),
            ExpressionKeyword::False => false.into(),
            ExpressionKeyword::Pi => std::f32::consts::PI.into(),
        }
    }
}

impl fmt::Display for KeywordExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}]", self.keyword)
    }
}

/// A unary operator applied to an expression.
#[derive(Clone, Debug, PartialEq)]
pub struct UnaryExpression {
    pub parameter: Box<Expression>,
    pub expression_type: ExpressionType,
}

impl UnaryExpression {
    pub fn new(parameter: Expression, expression_type: ExpressionType) -> Self {
        Self {
            parameter: Box::new(parameter),
            expression_type,
        }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        match self.expression_type {
            ExpressionType::Not => !self.parameter.evaluate(context),
            ExpressionType::UnaryMinus => -self.parameter.evaluate(context),
            _ => ExpressionVariant::Invalid,
        }
    }

    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        self.parameter.collect_references(references);
    }
}

impl fmt::Display for UnaryExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Expression::operator_name(self.expression_type), self.parameter)
    }
}

/// A binary operator applied to two expressions.
#[derive(Clone, Debug, PartialEq)]
pub struct BinaryExpression {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub expression_type: ExpressionType,
}

impl BinaryExpression {
    pub fn new(left: Expression, right: Expression, expression_type: ExpressionType) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            expression_type,
        }
    }

    pub fn evaluate(&self, context: &ExpressionEvaluationContext<'_>) -> ExpressionVariant {
        let left = self.left.evaluate(context);
        let right = self.right.evaluate(context);
        match self.expression_type {
            ExpressionType::Add => left + right,
            ExpressionType::Subtract => left - right,
            ExpressionType::Multiply => left * right,
            ExpressionType::Divide => left / right,
            ExpressionType::Remainder => left % right,
            ExpressionType::MoreThan => left.more_than(right),
            ExpressionType::LessThan => left.less_than(right),
            // The reference implementation evaluates `>=` as `>` and `<=` as `<`.
            ExpressionType::MoreThanOrEqual => left.more_than(right),
            ExpressionType::LessThanOrEqual => left.less_than(right),
            ExpressionType::LogicalAnd => left.and(right),
            ExpressionType::LogicalOr => left.or(right),
            ExpressionType::Equals => left.equals_to(right),
            ExpressionType::NotEquals => left.not_equals_to(right),
            _ => ExpressionVariant::Invalid,
        }
    }

    pub fn collect_references(&self, references: &mut HashSet<(String, String)>) {
        self.left.collect_references(references);
        self.right.collect_references(references);
    }
}

impl fmt::Display for BinaryExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}{}{})",
            self.left,
            Expression::operator_name(self.expression_type),
            self.right
        )
    }
}
