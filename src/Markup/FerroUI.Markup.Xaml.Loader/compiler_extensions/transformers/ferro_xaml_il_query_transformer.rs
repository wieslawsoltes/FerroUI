//! Port of `CompilerExtensions/Transformers/FerroXamlIlQueryTransformer.cs`.
//!
//! The transformer turns the text of a `ContainerQuery.Query` property into typed query nodes
//! and wraps the container query in a target-type scope. The query nodes generate IL upstream;
//! here they carry their data and document what a back end has to do with it.
//!
//! # What a back end has to do with a query node (upstream IL)
//!
//! Every query node is a value node typed `StyleQuery` ([`IXamlAstValueNode::type_`]). Emitting
//! a node pushes exactly one value of that type:
//!
//! 1. when the node has a [`XamlIlQueryNode::previous`] node, emit it first, converted to the
//!    query type (it becomes the first argument, `previous`, of the builder call below);
//! 2. perform the node's own step, documented on each node type. Except for the initial node
//!    (which pushes `null`) the step pushes the node's further arguments and calls one static
//!    method of the `StyleQueries` builder class (`FerroXamlIlWellKnownTypes::style_queries`),
//!    which pops `previous` and the arguments and pushes the resulting query.
//!
//! The builder method is looked up by shape: the first method of `StyleQueries` (declared
//! methods first, then base types, then interfaces) that is static, has at least one parameter
//! and satisfies the predicate of the node. Each node exposes that lookup as `builder_method`;
//! when there is no such method it fails with the upstream
//! `XamlTypeSystemException("Unable to find <TargetType> in FerroUI.Styling.StyleQueries")`.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::styling::StyleQueryComparisonOperator;
use ferroui_markup::markup::parsers::{ContainerQueryGrammar, ContainerQuerySyntax};
use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo, XamlAstCast,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::extensions::query_node_interface;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlConstructor, IXamlMethod, IXamlType, XamlTypeWellKnownTypes};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use crate::compiler_extensions::transformers::{
    create_nullable_clr_type_reference, FerroXamlIlTargetTypeMetadataNode,
    FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};

pub struct FerroXamlIlQueryTransformer;

impl IXamlAstTransformer for FerroXamlIlQueryTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let types = context.get_ferro_types();
        if !types.container.is_assignable_from(&*on.type_().get_clr_type()?) {
            return Ok(node);
        }

        let mut pn: Option<Rc<XamlAstXamlPropertyValueNode>> = None;
        let children = on.children.borrow().clone();
        for child in children {
            if let Some(p) = child.cast::<XamlAstXamlPropertyValueNode>() {
                if p.property().get_clr_property()?.name() == "Query" {
                    pn = Some(p);
                    break;
                }
            }
        }

        let Some(pn) = pn else {
            return Ok(node);
        };
        let Some(getter) = pn.property().get_clr_property()?.getter() else {
            return Ok(node);
        };

        if pn.values.borrow().len() != 1 {
            return Err(XamlError::parse_exception(
                "Query property should should have exactly one value",
                Some(&*node),
            ));
        }

        let value = pn.values.borrow()[0].clone();
        if value.is::<dyn XamlIlQueryNode>() {
            // Deja vu. I've just been in this place before
            return Ok(node);
        }

        let Some(tn) = value.cast::<XamlAstTextNode>() else {
            return Err(XamlError::parse_exception(
                "Query property should be a text node",
                Some(&*node),
            ));
        };

        let query_type = getter.return_type();
        let initial_node: Rc<dyn XamlIlQueryNode> =
            XamlIlQueryInitialNode::new(&*node, query_type.clone());

        let parsed = match ContainerQueryGrammar::parse(&tn.text()) {
            Ok(parsed) => parsed,
            Err(e) => {
                // Upstream appends `Exception.ToString()`: the full type name, the message and
                // the stack trace. There is no stack trace here.
                return Err(XamlError::parse_exception(
                    format!(
                        "Unable to parse query: FerroUI.Data.Core.ExpressionParseException: {}",
                        e.message()
                    ),
                    Some(&*node),
                ));
            }
        };

        let query = create(&node, &query_type, &initial_node, &parsed)?;
        {
            let query_value: Rc<dyn IXamlAstValueNode> = query.clone();
            pn.values.borrow_mut()[0] = query_value;
        }

        let container: Rc<dyn IXamlAstValueNode> = on;
        Ok(FerroXamlIlTargetTypeMetadataNode::new(
            container,
            create_nullable_clr_type_reference(&*query, query.target_type()),
            ScopeTypes::Container,
        ))
    }
}

/// The local function `Create` of the upstream `Transform`.
///
/// Ported as written, including what it does with `and`: the features joined by `and` are
/// collected in an and-node, but that node only becomes part of the result when a comma follows
/// it (`a and b, c` is `Or[And[a, b], c]`). Without a following comma the function returns the
/// last feature alone (`a and b` is `b`; `c, a and b` is `Or[c, b]`), and a feature between two
/// `and`s is added to the and-node twice (`a and b and c` collects `[a, b, b, c]`).
///
/// This looks like an upstream defect (the and-node is dropped unless a comma follows). It is
/// preserved deliberately: the port is exact, the tests pin this behaviour, and the fix belongs
/// upstream first.
pub(super) fn create(
    node: &Rc<dyn IXamlAstNode>,
    query_type: &Rc<dyn IXamlType>,
    initial_node: &Rc<dyn XamlIlQueryNode>,
    syntax: &[ContainerQuerySyntax],
) -> XamlResult<Rc<dyn XamlIlQueryNode>> {
    let line_info = &**node;
    let mut result: Rc<dyn XamlIlQueryNode> = initial_node.clone();
    let mut results: Option<Rc<XamlIlOrQueryNode>> = None;
    let mut and_node: Option<Rc<XamlIlAndQueryNode>> = None;
    for i in syntax {
        match i {
            ContainerQuerySyntax::Width { value, operator } => {
                result = XamlIlWidthQuery::new(result, *operator, *value);
            }
            ContainerQuerySyntax::Height { value, operator } => {
                result = XamlIlHeightQuery::new(result, *operator, *value);
            }
            ContainerQuerySyntax::Or => {
                let results = results
                    .get_or_insert_with(|| XamlIlOrQueryNode::new(line_info, query_type.clone()));
                if and_node.is_some() && result.same_node(initial_node) {
                    return Err(XamlError::parse_exception(
                        "Previously opened And node is not closed.",
                        Some(line_info),
                    ));
                }
                match and_node.take() {
                    Some(and_node) => results.add(and_node),
                    None => results.add(result),
                }
                result = initial_node.clone();
            }
            ContainerQuerySyntax::And => {
                let and_node = and_node
                    .get_or_insert_with(|| XamlIlAndQueryNode::new(line_info, query_type.clone()));
                and_node.add(result);
                result = initial_node.clone();
            } // The upstream `default` branch ("Unsupported query grammar") is unreachable:
              // the syntax is a closed enum.
        }

        if let Some(and_node) = &and_node {
            if !result.same_node(initial_node) {
                and_node.add(result.clone());
            }
        }
    }

    match results {
        Some(results) => {
            results.add(result);
            Ok(results)
        }
        None => Ok(result),
    }
}

/// The upstream abstract class `XamlIlQueryNode`: a step of a container query.
///
/// `node is XamlIlQueryNode` is `node.cast::<dyn XamlIlQueryNode>()`; the concrete node type is
/// reached with `as_any().downcast_ref::<T>()`. As upstream, query nodes do not visit any
/// children. See the module documentation for what a back end has to do with a node.
pub trait XamlIlQueryNode: IXamlAstValueNode {
    /// `Previous`: the node this step is applied to, `None` for the first node of a chain.
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>>;

    /// `TargetType`: the control type the query is known to apply to, if any.
    fn target_type(&self) -> Option<Rc<dyn IXamlType>>;
}

impl XamlAstCast for dyn XamlIlQueryNode {
    fn kind_name() -> &'static str {
        "XamlIlQueryNode"
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        query_node_interface::<dyn XamlIlQueryNode>(&node)
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

/// The state of the upstream abstract class `XamlIlQueryNode`.
pub struct XamlIlQueryNodeBase {
    node: XamlAstNode,
    pub previous: Option<Rc<dyn XamlIlQueryNode>>,
    /// `Type`: the query type; the previous node's type reference unless the node was created
    /// with an explicit query type.
    pub type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlIlQueryNodeBase {
    /// `XamlIlQueryNode(previous)`: line info and type are taken from `previous`.
    pub fn with_previous(previous: Rc<dyn XamlIlQueryNode>) -> Self {
        Self {
            node: XamlAstNode::new(&*previous),
            type_: previous.type_(),
            previous: Some(previous),
        }
    }

    /// `XamlIlQueryNode(null, info, queryType)`.
    pub fn root(info: &dyn IXamlLineInfo, query_type: Rc<dyn IXamlType>) -> Self {
        Self {
            node: XamlAstNode::new(info),
            previous: None,
            type_: XamlAstClrTypeReference::new(info, query_type, false),
        }
    }

    /// The lookup of `EmitCall`: the first method of `StyleQueries` that is static, has at
    /// least one parameter and satisfies `method`. When there is none, the upstream
    /// `XamlTypeSystemException`, whose message names `target_type` (the `TargetType` of the
    /// node that looks the method up; empty when there is none).
    pub fn get_style_queries_method(
        types: &FerroXamlIlWellKnownTypes,
        target_type: Option<Rc<dyn IXamlType>>,
        method: impl Fn(&dyn IXamlMethod) -> bool,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        types
            .style_queries
            .find_method(|m| m.is_static() && !m.parameters().is_empty() && method(m))
            .ok_or_else(|| {
                XamlError::type_system_exception(format!(
                    "Unable to find {} in FerroUI.Styling.StyleQueries",
                    target_type.map(|t| t.to_type_string()).unwrap_or_default()
                ))
            })
    }
}

macro_rules! xaml_il_query_node {
    ($ty:ident, $name:literal) => {
        xaml_line_info_impl!($ty, base.node);

        impl IXamlAstNode for $ty {
            xaml_ast_node_members!($name, value);

            fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
                xaml_query_interface!(self, slot, dyn XamlIlQueryNode)
            }
        }

        impl IXamlAstValueNode for $ty {
            fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
                self.base.type_.clone()
            }
        }
    };
}

/// The start of every query chain: no query yet.
///
/// # What a back end has to do (upstream IL)
///
/// Push `null` (the `previous` argument of the first builder call). No builder method is called.
pub struct XamlIlQueryInitialNode {
    pub base: XamlIlQueryNodeBase,
}

impl XamlIlQueryInitialNode {
    pub fn new(info: &dyn IXamlLineInfo, query_type: Rc<dyn IXamlType>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::root(info, query_type),
        })
    }
}

xaml_il_query_node!(XamlIlQueryInitialNode, "XamlIlQueryInitialNode");

impl XamlIlQueryNode for XamlIlQueryInitialNode {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
}

/// A query on a control type. Declared upstream but never created by the transformer.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the runtime type object of `target_type` and call
/// [`XamlIlTypeQuery::builder_method`]: the method named `OfType` (when `concrete`) or `Is` with
/// two parameters whose second parameter is `System.Type`.
pub struct XamlIlTypeQuery {
    pub base: XamlIlQueryNodeBase,
    pub target_type: Rc<dyn IXamlType>,
    pub concrete: bool,
}

impl XamlIlTypeQuery {
    pub fn new(previous: Rc<dyn XamlIlQueryNode>, type_: Rc<dyn IXamlType>, concrete: bool) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::with_previous(previous),
            target_type: type_,
            concrete,
        })
    }

    pub fn builder_method(
        &self,
        types: &FerroXamlIlWellKnownTypes,
        well_known_types: &XamlTypeWellKnownTypes,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = if self.concrete { "OfType" } else { "Is" };
        XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
            let parameters = m.parameters();
            m.name() == name
                && parameters.len() == 2
                && parameters[1].equals(&*well_known_types.type_)
        })
    }
}

xaml_il_query_node!(XamlIlTypeQuery, "XamlIlTypeQuery");

impl XamlIlQueryNode for XamlIlTypeQuery {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        Some(self.target_type.clone())
    }
}

/// `XamlIlStringQuery.QueryType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XamlIlStringQueryType {
    Class,
    Name,
}

impl XamlIlStringQueryType {
    /// `Enum.ToString()`: the name of the builder method.
    pub fn name(self) -> &'static str {
        match self {
            XamlIlStringQueryType::Class => "Class",
            XamlIlStringQueryType::Name => "Name",
        }
    }
}

/// A query on a class or a name. Declared upstream but never created by the transformer.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the string `string` and call [`XamlIlStringQuery::builder_method`]:
/// the method named after the query type with two parameters whose second parameter is
/// `System.String`.
pub struct XamlIlStringQuery {
    pub base: XamlIlQueryNodeBase,
    pub string: RefCell<String>,
    /// `_type` (private upstream).
    pub query_type: XamlIlStringQueryType,
}

impl XamlIlStringQuery {
    pub fn new(previous: Rc<dyn XamlIlQueryNode>, type_: XamlIlStringQueryType, s: &str) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::with_previous(previous),
            string: RefCell::new(s.to_string()),
            query_type: type_,
        })
    }

    pub fn string(&self) -> String {
        self.string.borrow().clone()
    }

    pub fn builder_method(
        &self,
        types: &FerroXamlIlWellKnownTypes,
        well_known_types: &XamlTypeWellKnownTypes,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = self.query_type.name();
        XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
            let parameters = m.parameters();
            m.name() == name
                && parameters.len() == 2
                && parameters[1].equals(&*well_known_types.string)
        })
    }
}

xaml_il_query_node!(XamlIlStringQuery, "XamlIlStringQuery");

impl XamlIlQueryNode for XamlIlStringQuery {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// `XamlIlCombinatorQuery.CombinatorQueryType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombinatorQueryType {
    Child,
    Descendant,
    Template,
}

impl CombinatorQueryType {
    /// `Enum.ToString()`: the name of the builder method.
    pub fn name(self) -> &'static str {
        match self {
            CombinatorQueryType::Child => "Child",
            CombinatorQueryType::Descendant => "Descendant",
            CombinatorQueryType::Template => "Template",
        }
    }
}

/// A combinator between queries. Declared upstream but never created by the transformer.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: call [`XamlIlCombinatorQuery::builder_method`] without further arguments:
/// the method named after the combinator type with exactly one parameter.
pub struct XamlIlCombinatorQuery {
    pub base: XamlIlQueryNodeBase,
    /// `QueryType` (`_type` upstream).
    pub query_type: CombinatorQueryType,
}

impl XamlIlCombinatorQuery {
    pub fn new(previous: Rc<dyn XamlIlQueryNode>, type_: CombinatorQueryType) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::with_previous(previous),
            query_type: type_,
        })
    }

    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = self.query_type.name();
        XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
            m.name() == name && m.parameters().len() == 1
        })
    }
}

xaml_il_query_node!(XamlIlCombinatorQuery, "XamlIlCombinatorQuery");

impl XamlIlQueryNode for XamlIlCombinatorQuery {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
}

/// A width feature (`width:100`, `min-width:100`, `max-width:100`).
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the comparison operator as a 32-bit integer
/// ([`XamlIlWidthQuery::operator_value`]: `None` 0, `Equals` 1, `LessThan` 2, `GreaterThan` 3,
/// `LessThanOrEquals` 4, `GreaterThanOrEquals` 5), push the 64-bit float `value` and call
/// [`XamlIlWidthQuery::builder_method`]:
/// `StyleQueries.Width(StyleQuery previous, StyleQueryComparisonOperator operator, double value)`.
pub struct XamlIlWidthQuery {
    pub base: XamlIlQueryNodeBase,
    /// `_argument.Operator` (private upstream).
    pub operator: StyleQueryComparisonOperator,
    /// `_argument.Value` (private upstream).
    pub value: f64,
}

impl XamlIlWidthQuery {
    /// `XamlIlWidthQuery(previous, argument)`; the width syntax element is passed as its two
    /// members.
    pub fn new(
        previous: Rc<dyn XamlIlQueryNode>,
        operator: StyleQueryComparisonOperator,
        value: f64,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::with_previous(previous),
            operator,
            value,
        })
    }

    /// `(int)_argument.Operator`.
    pub fn operator_value(&self) -> i32 {
        self.operator as i32
    }

    /// The method named `Width` with exactly three parameters.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
            m.name() == "Width" && m.parameters().len() == 3
        })
    }
}

xaml_il_query_node!(XamlIlWidthQuery, "XamlIlWidthQuery");

impl XamlIlQueryNode for XamlIlWidthQuery {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// A height feature (`height:100`, `min-height:100`, `max-height:100`).
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the comparison operator as a 32-bit integer
/// ([`XamlIlHeightQuery::operator_value`], same values as for [`XamlIlWidthQuery`]), push the
/// 64-bit float `value` and call [`XamlIlHeightQuery::builder_method`]:
/// `StyleQueries.Height(StyleQuery previous, StyleQueryComparisonOperator operator, double value)`.
pub struct XamlIlHeightQuery {
    pub base: XamlIlQueryNodeBase,
    /// `_argument.Operator` (private upstream).
    pub operator: StyleQueryComparisonOperator,
    /// `_argument.Value` (private upstream).
    pub value: f64,
}

impl XamlIlHeightQuery {
    /// `XamlIlHeightQuery(previous, argument)`; the height syntax element is passed as its two
    /// members.
    pub fn new(
        previous: Rc<dyn XamlIlQueryNode>,
        operator: StyleQueryComparisonOperator,
        value: f64,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlQueryNodeBase::with_previous(previous),
            operator,
            value,
        })
    }

    /// `(int)_argument.Operator`.
    pub fn operator_value(&self) -> i32 {
        self.operator as i32
    }

    /// The method named `Height` with exactly three parameters.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
            m.name() == "Height" && m.parameters().len() == 3
        })
    }
}

xaml_il_query_node!(XamlIlHeightQuery, "XamlIlHeightQuery");

impl XamlIlQueryNode for XamlIlHeightQuery {
    fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// `TargetType` of the or- and and-nodes: the most derived type all members' target types are
/// assignable to, or none when any member has no target type.
fn common_target_type(queries: &[Rc<dyn XamlIlQueryNode>]) -> Option<Rc<dyn IXamlType>> {
    let mut result: Option<Rc<dyn IXamlType>> = None;

    for query in queries {
        let target_type = query.target_type()?;
        match result {
            None => result = Some(target_type),
            Some(_) => {
                while let Some(current) = result.clone() {
                    if current.is_assignable_from(&*target_type) {
                        break;
                    }
                    result = current.base_type();
                }
            }
        }
    }

    result
}

/// What the list-building branch of an or- or and-node needs: the list type, its `Add` method
/// and its parameterless constructor.
pub struct XamlIlQueryListMembers {
    /// `System.Collections.Generic.List<StyleQuery>`.
    pub list_type: Rc<dyn IXamlType>,
    /// `List<StyleQuery>.Add(StyleQuery)`.
    pub add: Rc<dyn IXamlMethod>,
    /// `new List<StyleQuery>()`.
    pub constructor: Rc<dyn IXamlConstructor>,
}

/// `ListOfT.MakeGenericType(type)`, `FindMethod("Add", Void, false, type)` and
/// `FindConstructor()`; `None` when the method or the constructor is missing.
fn query_list_members(
    type_argument: &Rc<dyn IXamlType>,
    well_known_types: &XamlTypeWellKnownTypes,
) -> XamlResult<Option<XamlIlQueryListMembers>> {
    let list_type = well_known_types
        .list_of_t
        .make_generic_type(std::slice::from_ref(type_argument))?;
    let Some(add) = list_type.find_method_by_name(
        "Add",
        &*well_known_types.void,
        false,
        std::slice::from_ref(type_argument),
    ) else {
        return Ok(None);
    };
    let Some(constructor) = list_type.find_constructor(None) else {
        return Ok(None);
    };
    Ok(Some(XamlIlQueryListMembers {
        list_type,
        add,
        constructor,
    }))
}

macro_rules! xaml_il_query_group_node {
    ($ty:ident, $name:literal, $method:literal) => {
        impl $ty {
            pub fn new(info: &dyn IXamlLineInfo, query_type: Rc<dyn IXamlType>) -> Rc<Self> {
                Rc::new(Self {
                    base: XamlIlQueryNodeBase::root(info, query_type),
                    queries: RefCell::new(Vec::new()),
                })
            }

            pub fn add(&self, node: Rc<dyn XamlIlQueryNode>) {
                self.queries.borrow_mut().push(node);
            }

            pub fn queries(&self) -> Vec<Rc<dyn XamlIlQueryNode>> {
                self.queries.borrow().clone()
            }

            /// The list type, its `Add` method and its constructor for the list-building
            /// branch; `None` when the back end has to emit nothing at all (see the type
            /// documentation).
            pub fn list_members(
                &self,
                well_known_types: &XamlTypeWellKnownTypes,
            ) -> XamlResult<Option<XamlIlQueryListMembers>> {
                query_list_members(&self.base.type_.get_clr_type()?, well_known_types)
            }

            /// The method with one parameter whose type name starts with `IReadOnlyList` (not
            /// the overload taking an array).
            pub fn builder_method(
                &self,
                types: &FerroXamlIlWellKnownTypes,
            ) -> XamlResult<Rc<dyn IXamlMethod>> {
                XamlIlQueryNodeBase::get_style_queries_method(types, self.target_type(), |m| {
                    let parameters = m.parameters();
                    m.name() == $method
                        && parameters.len() == 1
                        && parameters[0].name().starts_with("IReadOnlyList")
                })
            }
        }

        xaml_il_query_node!($ty, $name);

        impl XamlIlQueryNode for $ty {
            fn previous(&self) -> Option<Rc<dyn XamlIlQueryNode>> {
                self.base.previous.clone()
            }

            fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
                common_target_type(&self.queries.borrow())
            }
        }
    };
}

/// Alternatives separated by commas (`width:100, height:200`). Each member is a query chain of
/// its own (or an and-node); the node itself has no previous node.
///
/// # What a back end has to do (upstream IL)
///
/// * no members: fail with `XamlLoadException("Invalid query count")` positioned at this node;
/// * one member: emit that query node and nothing else (no builder call);
/// * otherwise: take [`XamlIlOrQueryNode::list_members`]; when it is `None` (the list type has
///   no `Add(StyleQuery)` method or no parameterless constructor) emit nothing at all, as
///   upstream. Otherwise create a new `List<StyleQuery>` with the constructor; for each member,
///   in order, duplicate the list reference, emit the member converted to the query type and
///   call `Add`; finally call [`XamlIlOrQueryNode::builder_method`]:
///   `StyleQueries.Or(IReadOnlyList<StyleQuery> query)` with the list as its only argument.
pub struct XamlIlOrQueryNode {
    pub base: XamlIlQueryNodeBase,
    /// `_queries` (private upstream).
    pub queries: RefCell<Vec<Rc<dyn XamlIlQueryNode>>>,
}

xaml_il_query_group_node!(XamlIlOrQueryNode, "XamlIlOrQueryNode", "Or");

/// Features joined by `and` (`min-width:100 and max-width:200`). Each member is a query chain of
/// its own; the node itself has no previous node.
///
/// # What a back end has to do (upstream IL)
///
/// Exactly as [`XamlIlOrQueryNode`], with the final call going to
/// [`XamlIlAndQueryNode::builder_method`]: `StyleQueries.And(IReadOnlyList<StyleQuery> query)`.
pub struct XamlIlAndQueryNode {
    pub base: XamlIlQueryNodeBase,
    /// `_queries` (private upstream).
    pub queries: RefCell<Vec<Rc<dyn XamlIlQueryNode>>>,
}

xaml_il_query_group_node!(XamlIlAndQueryNode, "XamlIlAndQueryNode", "And");
