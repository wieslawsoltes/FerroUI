//! Port of `Emit/XamlEmitContextWithLocals.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{IXamlAstNode, IXamlWrappedMethod, XamlAstCompilerLocalNode};
use crate::exceptions::{XamlError, XamlResult};
use crate::extensions::{query_node_interface, query_wrapped_method_interface};
use crate::type_system::{IXamlLocal, IXamlType, PooledLocal, XamlLocalsPool};

use super::{
    default_emit_core_wrapped_method, default_emit_node_core, IXamlEmitResult, XamlEmitContext,
    XamlEmitContextBase,
};

pub trait IHasLocalsPool {
    fn locals_pool(&self) -> &XamlLocalsPool;
    fn define_local(&self, type_: &Rc<dyn IXamlType>) -> XamlResult<Rc<dyn IXamlLocal>>;
}

/// The state of the upstream abstract `XamlEmitContextWithLocals` class: the base context state
/// plus the locals assigned to compiler local nodes (keyed by node identity).
pub struct XamlEmitContextWithLocalsBase<
    TBackendEmitter: IHasLocalsPool,
    TEmitResult: IXamlEmitResult,
> {
    pub base: XamlEmitContextBase<TBackendEmitter, TEmitResult>,
    locals: RefCell<Vec<(Rc<XamlAstCompilerLocalNode>, Rc<dyn IXamlLocal>)>>,
}

impl<TBackendEmitter: IHasLocalsPool, TEmitResult: IXamlEmitResult>
    XamlEmitContextWithLocalsBase<TBackendEmitter, TEmitResult>
{
    pub fn new(base: XamlEmitContextBase<TBackendEmitter, TEmitResult>) -> Self {
        Self {
            base,
            locals: RefCell::new(Vec::new()),
        }
    }

    pub fn get_local_for_node(
        &self,
        node: &Rc<XamlAstCompilerLocalNode>,
        code_gen: &TBackendEmitter,
        throw_on_uninitialized: bool,
    ) -> XamlResult<Rc<dyn IXamlLocal>> {
        let existing = self
            .locals
            .borrow()
            .iter()
            .find(|(n, _)| Rc::ptr_eq(n, node))
            .map(|(_, l)| l.clone());
        if let Some(local) = existing {
            return Ok(local);
        }
        if throw_on_uninitialized {
            return Err(XamlError::load_exception(
                "Attempt to read uninitialized local variable",
                Some(&**node),
            ));
        }
        let local = code_gen.define_local(&node.type_)?;
        self.locals.borrow_mut().push((node.clone(), local.clone()));
        Ok(local)
    }

    pub fn get_local_of_type(&self, type_: &Rc<dyn IXamlType>) -> PooledLocal {
        self.base.emitter.locals_pool().get_local(type_)
    }
}

/// The members `XamlEmitContextWithLocals` adds to `XamlEmitContext`.
///
/// An implementation must route the two virtual methods the upstream class overrides to
/// [`emit_node_core_with_locals`] and [`emit_core_wrapped_method_with_locals`]:
///
/// ```text
/// fn emit_node_core(&self, value: &Rc<dyn IXamlAstNode>, code_gen: &B) -> XamlResult<(Option<R>, bool)> {
///     emit_node_core_with_locals(self, value, code_gen)
/// }
/// fn emit_core_wrapped_method(&self, wrapped: &Rc<dyn IXamlWrappedMethod>, code_gen: &B, swallow: bool) -> XamlResult<bool> {
///     emit_core_wrapped_method_with_locals(self, wrapped, code_gen, swallow)
/// }
/// ```
pub trait XamlEmitContextWithLocals<
    TBackendEmitter: IHasLocalsPool + 'static,
    TEmitResult: IXamlEmitResult + 'static,
>: XamlEmitContext<TBackendEmitter, TEmitResult>
{
    /// The base class state.
    fn locals_base(&self) -> &XamlEmitContextWithLocalsBase<TBackendEmitter, TEmitResult>;

    /// `this`, as the `XamlEmitContextWithLocals` class type.
    fn as_emit_context_with_locals(
        &self,
    ) -> &dyn XamlEmitContextWithLocals<TBackendEmitter, TEmitResult>;

    /// `public abstract void LoadLocalValue(XamlAstCompilerLocalNode node, TBackendEmitter codeGen)`.
    fn load_local_value(
        &self,
        node: &Rc<XamlAstCompilerLocalNode>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<()>;
}

impl<B: IHasLocalsPool + 'static, R: IXamlEmitResult + 'static>
    dyn XamlEmitContextWithLocals<B, R>
{
    pub fn get_local_for_node(
        &self,
        node: &Rc<XamlAstCompilerLocalNode>,
        code_gen: &B,
        throw_on_uninitialized: bool,
    ) -> XamlResult<Rc<dyn IXamlLocal>> {
        self.locals_base()
            .get_local_for_node(node, code_gen, throw_on_uninitialized)
    }

    pub fn get_local_of_type(&self, type_: &Rc<dyn IXamlType>) -> PooledLocal {
        self.locals_base().get_local_of_type(type_)
    }
}

/// `XamlEmitContextWithLocals.EmitNodeCore` override.
pub fn emit_node_core_with_locals<B: IHasLocalsPool + 'static, R: IXamlEmitResult + 'static>(
    context: &dyn XamlEmitContextWithLocals<B, R>,
    value: &Rc<dyn IXamlAstNode>,
    code_gen: &B,
) -> XamlResult<(Option<R>, bool)> {
    let (mut result, mut found_emitter) =
        default_emit_node_core(context.as_emit_context(), value, code_gen)?;
    if result.as_ref().is_some_and(|r| r.valid()) {
        return Ok((result, found_emitter));
    }

    for e in &context.base().emitters {
        if let Some(ve) = e.as_locals_node_emitter() {
            result = ve.emit(value, context, code_gen)?;
            if result.as_ref().is_some_and(|r| r.valid()) {
                return Ok((result, true));
            }
        }
    }

    if !found_emitter {
        if let Some(emittable) = query_node_interface::<dyn IXamlAstLocalsEmitableNode<B, R>>(value)
        {
            found_emitter = true;
            return Ok((Some(emittable.emit(context, code_gen)?), found_emitter));
        }
    }

    Ok((result, found_emitter))
}

/// `XamlEmitContextWithLocals.EmitCore(IXamlWrappedMethod, ...)` override.
///
/// As upstream, a method that emits itself through `IXamlEmitableWrappedMethodWithLocals` is
/// emitted but still reported as "no emitter found".
pub fn emit_core_wrapped_method_with_locals<
    B: IHasLocalsPool + 'static,
    R: IXamlEmitResult + 'static,
>(
    context: &dyn XamlEmitContextWithLocals<B, R>,
    wrapped: &Rc<dyn IXamlWrappedMethod>,
    code_gen: &B,
    swallow_result: bool,
) -> XamlResult<bool> {
    let mut found_emitter = default_emit_core_wrapped_method(
        context.as_emit_context(),
        wrapped,
        code_gen,
        swallow_result,
    )?;
    if found_emitter {
        return Ok(true);
    }

    for e in &context.base().emitters {
        if let Some(wme) = e.as_wrapped_method_emitter_with_locals() {
            found_emitter = wme.emit_call(context, wrapped, code_gen, swallow_result)?;
            if found_emitter {
                break;
            }
        }
    }

    if !found_emitter {
        if let Some(ewm) = query_wrapped_method_interface::<
            dyn IXamlEmitableWrappedMethodWithLocals<B, R>,
        >(wrapped)
        {
            ewm.emit(context, code_gen, swallow_result)?;
        }
    }

    Ok(found_emitter)
}

pub trait IXamlAstLocalsNodeEmitter<
    TBackendEmitter: IHasLocalsPool + 'static,
    TEmitResult: IXamlEmitResult + 'static,
>
{
    /// Returns `Ok(None)` (or an invalid result) when this emitter does not handle the node.
    fn emit(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        context: &dyn XamlEmitContextWithLocals<TBackendEmitter, TEmitResult>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<Option<TEmitResult>>;
}

/// A node that knows how to emit itself for a given backend that supports locals.
/// Exposed by nodes through `IXamlAstNode::query_interface`.
pub trait IXamlAstLocalsEmitableNode<
    TBackendEmitter: IHasLocalsPool + 'static,
    TEmitResult: IXamlEmitResult + 'static,
>
{
    fn emit(
        &self,
        context: &dyn XamlEmitContextWithLocals<TBackendEmitter, TEmitResult>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<TEmitResult>;
}

pub trait IXamlWrappedMethodEmitterWithLocals<
    TBackendEmitter: IHasLocalsPool + 'static,
    TEmitResult: IXamlEmitResult + 'static,
>
{
    fn emit_call(
        &self,
        context: &dyn XamlEmitContextWithLocals<TBackendEmitter, TEmitResult>,
        method: &Rc<dyn IXamlWrappedMethod>,
        emitter: &TBackendEmitter,
        swallow_result: bool,
    ) -> XamlResult<bool>;
}

/// A wrapped method that knows how to emit itself for a given backend that supports locals.
/// Exposed by wrapped methods through `IXamlWrappedMethod::query_interface`.
pub trait IXamlEmitableWrappedMethodWithLocals<
    TBackendEmitter: IHasLocalsPool + 'static,
    TEmitResult: IXamlEmitResult + 'static,
>: IXamlWrappedMethod
{
    fn emit(
        &self,
        context: &dyn XamlEmitContextWithLocals<TBackendEmitter, TEmitResult>,
        emitter: &TBackendEmitter,
        swallow_result: bool,
    ) -> XamlResult<()>;
}
