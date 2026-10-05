//! Tests of the backend-agnostic emit contracts with a minimal text emitting backend.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::*;
use crate::emit::*;
use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::*;
use crate::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use super::test_xaml_language::TestHost;

/// The backend "code generator": a shared log of emitted instructions.
#[derive(Clone)]
struct TextEmitter {
    log: Rc<RefCell<Vec<String>>>,
    pool: Rc<XamlLocalsPool>,
}

impl TextEmitter {
    fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(Vec::new())),
            pool: Rc::new(XamlLocalsPool::new(|_| Rc::new(TextLocal))),
        }
    }

    fn emit(&self, instruction: impl Into<String>) {
        self.log.borrow_mut().push(instruction.into());
    }
}

struct TextLocal;

impl IXamlLocal for TextLocal {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IHasLocalsPool for TextEmitter {
    fn locals_pool(&self) -> &XamlLocalsPool {
        &self.pool
    }
    fn define_local(&self, type_: &Rc<dyn IXamlType>) -> XamlResult<Rc<dyn IXamlLocal>> {
        self.emit(format!("local {}", type_.name()));
        Ok(Rc::new(TextLocal))
    }
}

struct TextResult {
    return_type: Option<Rc<dyn IXamlType>>,
}

impl IXamlEmitResult for TextResult {
    fn return_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.return_type.clone()
    }
    fn valid(&self) -> bool {
        true
    }
}

/// A type builder that only serves as the declaring type of the emit context.
struct TextTypeBuilder {
    inner: Rc<dyn IXamlType>,
}

macro_rules! delegate_type {
    ($($name:ident -> $ret:ty;)*) => {
        $( fn $name(&self) -> $ret { self.inner.$name() } )*
    };
}

impl IXamlType for TextTypeBuilder {
    delegate_type! {
        id -> XamlTypeId;
        name -> String;
        namespace -> Option<String>;
        full_name -> String;
        is_public -> bool;
        is_nested_private -> bool;
        assembly -> Option<Rc<dyn IXamlAssembly>>;
        properties -> Vec<Rc<dyn IXamlProperty>>;
        events -> Vec<Rc<dyn IXamlEventInfo>>;
        fields -> Vec<Rc<dyn IXamlField>>;
        methods -> Vec<Rc<dyn IXamlMethod>>;
        constructors -> Vec<Rc<dyn IXamlConstructor>>;
        custom_attributes -> Vec<Rc<dyn IXamlCustomAttribute>>;
        generic_arguments -> Vec<Rc<dyn IXamlType>>;
        generic_type_definition -> Option<Rc<dyn IXamlType>>;
        is_array -> bool;
        array_element_type -> Option<Rc<dyn IXamlType>>;
        base_type -> Option<Rc<dyn IXamlType>>;
        declaring_type -> Option<Rc<dyn IXamlType>>;
        is_value_type -> bool;
        is_enum -> bool;
        interfaces -> Vec<Rc<dyn IXamlType>>;
        is_interface -> bool;
        get_enum_underlying_type -> XamlResult<Rc<dyn IXamlType>>;
        generic_parameters -> Vec<Rc<dyn IXamlType>>;
        is_function_pointer -> bool;
        get_hash_code -> u64;
    }
    fn is_assignable_from(&self, type_: &dyn IXamlType) -> bool {
        self.inner.is_assignable_from(type_)
    }
    fn make_generic_type(
        &self,
        type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>> {
        self.inner.make_generic_type(type_arguments)
    }
    fn make_array_type(&self, dimensions: i32) -> XamlResult<Rc<dyn IXamlType>> {
        self.inner.make_array_type(dimensions)
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        self.inner.equals(other)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn unsupported<T>() -> XamlResult<T> {
    Err(XamlError::not_supported(
        "The test type builder doesn't define members",
    ))
}

impl IXamlTypeBuilder<TextEmitter> for TextTypeBuilder {
    fn define_field(
        &self,
        _: &Rc<dyn IXamlType>,
        _: &str,
        _: XamlVisibility,
        _: bool,
    ) -> XamlResult<Rc<dyn IXamlField>> {
        unsupported()
    }
    fn add_interface_implementation(&self, _: &Rc<dyn IXamlType>) -> XamlResult<()> {
        unsupported()
    }
    fn define_method(
        &self,
        _: &Rc<dyn IXamlType>,
        _: &[Rc<dyn IXamlType>],
        _: &str,
        _: XamlVisibility,
        _: bool,
        _: bool,
        _: Option<&Rc<dyn IXamlMethod>>,
    ) -> XamlResult<Rc<dyn IXamlMethodBuilder<TextEmitter>>> {
        unsupported()
    }
    fn define_property(
        &self,
        _: &Rc<dyn IXamlType>,
        _: &str,
        _: Option<&Rc<dyn IXamlMethod>>,
        _: Option<&Rc<dyn IXamlMethod>>,
    ) -> XamlResult<Rc<dyn IXamlProperty>> {
        unsupported()
    }
    fn define_constructor(
        &self,
        _: bool,
        _: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlConstructorBuilder<TextEmitter>>> {
        unsupported()
    }
    fn create_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        Ok(self.inner.clone())
    }
    fn define_sub_type(
        &self,
        _: &Rc<dyn IXamlType>,
        _: &str,
        _: XamlVisibility,
    ) -> XamlResult<Rc<dyn IXamlTypeBuilder<TextEmitter>>> {
        unsupported()
    }
    fn define_delegate_sub_type(
        &self,
        _: &str,
        _: XamlVisibility,
        _: &Rc<dyn IXamlType>,
        _: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlTypeBuilder<TextEmitter>>> {
        unsupported()
    }
    fn define_generic_parameters(
        &self,
        _: &[(String, XamlGenericParameterConstraint)],
    ) -> XamlResult<()> {
        unsupported()
    }
}

/// The backend's emit context ("class TextEmitContext : XamlEmitContextWithLocals<...>").
struct TextEmitContext {
    base: XamlEmitContextWithLocalsBase<TextEmitter, TextResult>,
}

impl XamlEmitContext<TextEmitter, TextResult> for TextEmitContext {
    fn base(&self) -> &XamlEmitContextBase<TextEmitter, TextResult> {
        &self.base.base
    }
    fn as_emit_context(&self) -> &dyn XamlEmitContext<TextEmitter, TextResult> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn emit_convert(
        &self,
        _value: &Rc<dyn IXamlAstNode>,
        code_gen: &TextEmitter,
        expected_type: &Rc<dyn IXamlType>,
        returned_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<()> {
        code_gen.emit(format!(
            "convert {} -> {}",
            returned_type.name(),
            expected_type.name()
        ));
        Ok(())
    }
    fn emit_node_core(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &TextEmitter,
    ) -> XamlResult<(Option<TextResult>, bool)> {
        emit_node_core_with_locals(self, value, code_gen)
    }
    fn emit_core_wrapped_method(
        &self,
        wrapped: &Rc<dyn IXamlWrappedMethod>,
        code_gen: &TextEmitter,
        swallow_result: bool,
    ) -> XamlResult<bool> {
        emit_core_wrapped_method_with_locals(self, wrapped, code_gen, swallow_result)
    }
}

impl XamlEmitContextWithLocals<TextEmitter, TextResult> for TextEmitContext {
    fn locals_base(&self) -> &XamlEmitContextWithLocalsBase<TextEmitter, TextResult> {
        &self.base
    }
    fn as_emit_context_with_locals(
        &self,
    ) -> &dyn XamlEmitContextWithLocals<TextEmitter, TextResult> {
        self
    }
    fn load_local_value(
        &self,
        node: &Rc<XamlAstCompilerLocalNode>,
        code_gen: &TextEmitter,
    ) -> XamlResult<()> {
        self.base.get_local_for_node(node, code_gen, true)?;
        code_gen.emit("ldloc");
        Ok(())
    }
}

/// The backend's emitters for built-in nodes, property setters and wrapped methods.
struct TextEmitters;

impl IXamlEmitter<TextEmitter, TextResult> for TextEmitters {
    fn as_node_emitter(&self) -> Option<&dyn IXamlAstNodeEmitter<TextEmitter, TextResult>> {
        Some(self)
    }
    fn as_property_setter_emitter(&self) -> Option<&dyn IXamlPropertySetterEmitter<TextEmitter>> {
        Some(self)
    }
    fn as_wrapped_method_emitter(
        &self,
    ) -> Option<&dyn IXamlWrappedMethodEmitter<TextEmitter, TextResult>> {
        Some(self)
    }
    fn as_locals_node_emitter(
        &self,
    ) -> Option<&dyn IXamlAstLocalsNodeEmitter<TextEmitter, TextResult>> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IXamlAstNodeEmitter<TextEmitter, TextResult> for TextEmitters {
    fn emit(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        context: &dyn XamlEmitContext<TextEmitter, TextResult>,
        code_gen: &TextEmitter,
    ) -> XamlResult<Option<TextResult>> {
        if let Some(constant) = node.cast::<XamlConstantNode>() {
            code_gen.emit(format!("const {:?}", constant.constant));
            return Ok(Some(TextResult {
                return_type: Some(constant.type_().get_clr_type()?),
            }));
        }
        if let Some(text) = node.cast::<XamlAstTextNode>() {
            code_gen.emit(format!(
                "ldstr {:?} (parents: {})",
                text.text(),
                context.base().parent_nodes().len()
            ));
            return Ok(Some(TextResult {
                return_type: Some(text.type_().get_clr_type()?),
            }));
        }
        if let Some(assignment) = node.cast::<XamlPropertyAssignmentNode>() {
            let setter = assignment.possible_setters.borrow()[0].clone();
            let values = assignment.values.borrow().clone();
            for (value, parameter) in values.iter().zip(setter.parameters()) {
                context.emit(&value.as_node(), code_gen, Some(&parameter))?;
            }
            context.emit_property_setter(&setter, code_gen)?;
            return Ok(Some(TextResult { return_type: None }));
        }
        if let Some(call) = node.cast::<XamlNoReturnMethodCallNode>() {
            context.emit_wrapped_method(&call.base.method(), code_gen, true)?;
            return Ok(Some(TextResult { return_type: None }));
        }
        Ok(None)
    }
}

impl IXamlAstLocalsNodeEmitter<TextEmitter, TextResult> for TextEmitters {
    fn emit(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        context: &dyn XamlEmitContextWithLocals<TextEmitter, TextResult>,
        code_gen: &TextEmitter,
    ) -> XamlResult<Option<TextResult>> {
        if let Some(local) = node.cast::<XamlAstCompilerLocalNode>() {
            context.load_local_value(&local, code_gen)?;
            return Ok(Some(TextResult {
                return_type: Some(local.type_.clone()),
            }));
        }
        Ok(None)
    }
}

impl IXamlPropertySetterEmitter<TextEmitter> for TextEmitters {
    fn emit_call(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        emitter: &TextEmitter,
    ) -> XamlResult<bool> {
        match setter
            .as_any()
            .downcast_ref::<XamlDirectCallPropertySetter>()
        {
            Some(direct) => {
                emitter.emit(format!("call {}", direct.method().name()));
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl IXamlWrappedMethodEmitter<TextEmitter, TextResult> for TextEmitters {
    fn emit_call(
        &self,
        _context: &dyn XamlEmitContext<TextEmitter, TextResult>,
        method: &Rc<dyn IXamlWrappedMethod>,
        emitter: &TextEmitter,
        swallow_result: bool,
    ) -> XamlResult<bool> {
        match method.as_any().downcast_ref::<XamlWrappedMethod>() {
            Some(wrapped) => {
                emitter.emit(format!(
                    "call {}{}",
                    wrapped.method().name(),
                    if swallow_result { " pop" } else { "" }
                ));
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

/// A node defined outside of the compiler core that emits itself for the text backend.
struct SelfEmittingNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

xaml_line_info_impl!(SelfEmittingNode, base);

impl IXamlAstNode for SelfEmittingNode {
    xaml_ast_node_members!("SelfEmittingNode", value);

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(
            self,
            slot,
            dyn IXamlAstEmitableNode<TextEmitter, TextResult>
        )
    }
}

impl IXamlAstValueNode for SelfEmittingNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlAstEmitableNode<TextEmitter, TextResult> for SelfEmittingNode {
    fn emit(
        &self,
        _context: &dyn XamlEmitContext<TextEmitter, TextResult>,
        code_gen: &TextEmitter,
    ) -> XamlResult<TextResult> {
        code_gen.emit("self-emitted");
        Ok(TextResult {
            return_type: Some(self.type_.get_clr_type()?),
        })
    }
}

fn create_context(host: &TestHost) -> (TextEmitContext, TextEmitter) {
    let context_definition =
        host.asm
            .define_generic_class("XamlParserTests", "TestContext`1", &["T"]);
    context_definition.add_constructor(vec![]);
    context_definition.add_field(
        XamlRuntimeContextDefintion::ROOT_OBJECT_FIELD_NAME,
        context_definition.generic_parameter(0),
        false,
        None,
    );
    let definition: Rc<dyn IXamlType> = context_definition;

    let mappings = Rc::new(XamlLanguageEmitMappings::<TextEmitter, TextResult> {
        context_factory_callback: Some(Rc::new(|_, il: &TextEmitter| {
            il.emit("context callback");
            Ok(())
        })),
        ..XamlLanguageEmitMappings::default()
    });
    let runtime_context = XamlRuntimeContext::new(
        &definition,
        &host.t("Control"),
        Some("http://example.com/".to_string()),
        &mappings,
        Rc::new(
            |context: &XamlRuntimeContext<TextEmitter, TextResult>, il: &TextEmitter| {
                il.emit(format!("newobj {}", context.context_type.full_name()));
                Ok(())
            },
        ),
    )
    .expect("runtime context");
    assert!(runtime_context
        .root_object_field
        .as_ref()
        .is_some_and(|f| f.field_type().equals(&*host.t("Control"))));
    assert!(runtime_context.parent_list_field.is_none());

    let emitter = TextEmitter::new();
    let emitters: Vec<Rc<dyn IXamlEmitter<TextEmitter, TextResult>>> = vec![Rc::new(TextEmitters)];
    let base = XamlEmitContextBase::new(
        emitter.clone(),
        host.configuration.clone(),
        mappings,
        Rc::new(runtime_context),
        None,
        Rc::new(TextTypeBuilder {
            inner: host.t("Control"),
        }),
        None,
        emitters,
    );
    (
        TextEmitContext {
            base: XamlEmitContextWithLocalsBase::new(base),
        },
        emitter,
    )
}

#[test]
fn emit_context_dispatches_to_emitters_and_emitable_nodes() {
    let host = TestHost::new();
    let (context, emitter) = create_context(&host);
    let context: &dyn XamlEmitContext<TextEmitter, TextResult> = &context;
    let string = host.ts.get("System.String");
    let object = host.ts.get("System.Object");
    let int32 = host.ts.get("System.Int32");
    let li = XamlLineInfo::new(4, 2);

    context
        .base()
        .runtime_context
        .factory(&emitter)
        .expect("factory");

    // A property assignment emitted through node, setter and nested emit calls
    let root = host.transform_root("<Control xmlns='test' StrProp='abc'/>");
    let assignment = super::whitespace_tests::assignments(&root).remove(0);
    let result = context
        .emit(&assignment.as_node(), &emitter, None)
        .expect("emit");
    assert!(result.return_type().is_none());

    // Expected type handling
    let constant: Rc<dyn IXamlAstNode> =
        XamlConstantNode::new(&li, int32.clone(), XamlValue::Int32(1)).expect("constant");
    context
        .emit(&constant, &emitter, Some(&int32))
        .expect("emit");
    context
        .emit(&constant, &emitter, Some(&object))
        .expect("emit");
    match context.emit(&constant, &emitter, None) {
        Err(XamlError::Load(e)) => {
            assert_eq!(
                e.title,
                "Emit of node XamlConstantNode resulted in System.Runtime:System.Int32 while caller expected void"
            );
            assert_eq!((e.line_number, e.line_position), (4, 2));
        }
        other => panic!("Expected a load exception, got {:?}", other.map(|_| ())),
    }
    match context.emit(&assignment.as_node(), &emitter, Some(&string)) {
        Err(XamlError::Load(e)) => assert!(e
            .title
            .ends_with("resulted in void while caller expected System.Runtime:System.String")),
        other => panic!("Expected a load exception, got {:?}", other.map(|_| ())),
    }

    // A node that emits itself through the queried interface
    let self_emitting: Rc<dyn IXamlAstNode> = Rc::new(SelfEmittingNode {
        base: XamlAstNode::new(&li),
        type_: XamlAstClrTypeReference::new(&li, string.clone(), false),
    });
    context
        .emit(&self_emitting, &emitter, Some(&string))
        .expect("emit");

    // No emitter
    let unknown: Rc<dyn IXamlAstNode> = XamlNullExtensionNode::new(&li);
    match context.emit(&unknown, &emitter, None) {
        Err(XamlError::Load(e)) => assert_eq!(
            e.title,
            "Unable to find emitter for node type: XamlNullExtensionNode"
        ),
        other => panic!("Expected a load exception, got {:?}", other.map(|_| ())),
    }

    // Wrapped methods and setters without an emitter
    let add = host
        .t("InlineCollection")
        .get_method(|m| m.name() == "Add")
        .expect("Add");
    let call: Rc<dyn IXamlAstNode> = XamlNoReturnMethodCallNode::new(&li, add.clone(), vec![]);
    context.emit(&call, &emitter, None).expect("emit");
    let casts: Rc<dyn IXamlWrappedMethod> = XamlWrappedMethodWithCasts::new(
        XamlWrappedMethod::new(add.clone()),
        vec![object.clone(), object.clone()],
    )
    .expect("casts");
    match context.emit_wrapped_method(&casts, &emitter, false) {
        Err(XamlError::Internal(e)) => {
            assert_eq!(
                e.message,
                "Unable to find emitter for wrapped method type: XamlWrappedMethodWithCasts"
            )
        }
        other => panic!("Expected an error, got {other:?}"),
    }
    assert!(XamlWrappedMethodWithCasts::new(XamlWrappedMethod::new(add), vec![object]).is_err());

    assert_eq!(
        *emitter.log.borrow(),
        vec![
            "newobj XamlParserTests.TestContext`1[XamlParserTests.Control]",
            "context callback",
            "ldstr \"abc\" (parents: 1)",
            "call set_StrProp",
            "const 1i32",
            "const 1i32",
            "convert Int32 -> Object",
            "const 1i32",
            "ldstr \"abc\" (parents: 1)",
            "call set_StrProp",
            "self-emitted",
            "call Add pop",
        ]
    );
}

#[test]
fn emit_context_with_locals_tracks_compiler_locals() {
    let host = TestHost::new();
    let (context, emitter) = create_context(&host);
    let li = XamlLineInfo::new(1, 1);
    let control = host.t("Control");
    let local = XamlAstCompilerLocalNode::new(
        &li,
        XamlAstClrTypeReference::new(&li, control.clone(), false),
    );
    let with_locals: &dyn XamlEmitContextWithLocals<TextEmitter, TextResult> = &context;

    // Reading an uninitialized local fails
    let as_context: &dyn XamlEmitContext<TextEmitter, TextResult> = &context;
    match as_context.emit(&local.as_node(), &emitter, Some(&control)) {
        Err(XamlError::Load(e)) => {
            assert_eq!(e.title, "Attempt to read uninitialized local variable")
        }
        other => panic!("Expected a load exception, got {:?}", other.map(|_| ())),
    }

    let first = with_locals
        .get_local_for_node(&local, &emitter, false)
        .expect("defined");
    let second = with_locals
        .get_local_for_node(&local, &emitter, true)
        .expect("reused");
    assert!(Rc::ptr_eq(&first, &second));
    as_context
        .emit(&local.as_node(), &emitter, Some(&control))
        .expect("emit");
    assert_eq!(*emitter.log.borrow(), vec!["local Control", "ldloc"]);

    let pooled = with_locals.get_local_of_type(&control);
    assert!(pooled.local().is_ok());

    let ran = Rc::new(RefCell::new(0));
    let counter = ran.clone();
    as_context.add_after_emit_callbacks(move || {
        *counter.borrow_mut() += 1;
        Ok(())
    });
    as_context
        .execute_after_emit_callbacks()
        .expect("callbacks");
    as_context
        .execute_after_emit_callbacks()
        .expect("callbacks");
    assert_eq!(*ran.borrow(), 1);
    assert!(as_context.context_local().is_err());
}
