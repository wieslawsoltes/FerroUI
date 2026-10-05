//! Port of `Emit/XamlEmitContext.cs`.
//!
//! The upstream abstract class `XamlEmitContext<TBackendEmitter, TEmitResult>` is split into
//! [`XamlEmitContextBase`] (its state) and the [`XamlEmitContext`] trait (its virtual and
//! abstract members). The non-virtual public members are inherent methods of
//! `dyn XamlEmitContext<_, _>`.

use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use crate::ast::{IXamlAstNode, IXamlPropertySetter, IXamlWrappedMethod};
use crate::exceptions::{XamlError, XamlResult};
use crate::extensions::{
    query_node_interface, query_property_setter_interface, query_wrapped_method_interface,
};
use crate::transform::{TransformerConfiguration, XamlContextBase};
use crate::type_system::{IFileSource, IXamlLocal, IXamlMethod, IXamlType, IXamlTypeBuilder};

use super::{
    IHasLocalsPool, IXamlAstLocalsNodeEmitter, IXamlWrappedMethodEmitterWithLocals,
    XamlLanguageEmitMappings, XamlRuntimeContext,
};

pub trait IXamlEmitResult {
    fn return_type(&self) -> Option<Rc<dyn IXamlType>>;
    fn valid(&self) -> bool;
}

/// An entry of the `Emitters` list (`List<object>` upstream). An emitter object implements
/// one or more of the emitter interfaces and exposes them through the `as_*` accessors.
pub trait IXamlEmitter<TBackendEmitter, TEmitResult: IXamlEmitResult>: 'static {
    /// `this as IXamlAstNodeEmitter<TBackendEmitter, TEmitResult>`.
    fn as_node_emitter(&self) -> Option<&dyn IXamlAstNodeEmitter<TBackendEmitter, TEmitResult>> {
        None
    }
    /// `this as IXamlPropertySetterEmitter<TBackendEmitter>`.
    fn as_property_setter_emitter(
        &self,
    ) -> Option<&dyn IXamlPropertySetterEmitter<TBackendEmitter>> {
        None
    }
    /// `this as IXamlWrappedMethodEmitter<TBackendEmitter, TEmitResult>`.
    fn as_wrapped_method_emitter(
        &self,
    ) -> Option<&dyn IXamlWrappedMethodEmitter<TBackendEmitter, TEmitResult>> {
        None
    }
    /// `this as IXamlAstLocalsNodeEmitter<TBackendEmitter, TEmitResult>`.
    fn as_locals_node_emitter(
        &self,
    ) -> Option<&dyn IXamlAstLocalsNodeEmitter<TBackendEmitter, TEmitResult>>
    where
        TBackendEmitter: IHasLocalsPool,
    {
        None
    }
    /// `this as IXamlWrappedMethodEmitterWithLocals<TBackendEmitter, TEmitResult>`.
    fn as_wrapped_method_emitter_with_locals(
        &self,
    ) -> Option<&dyn IXamlWrappedMethodEmitterWithLocals<TBackendEmitter, TEmitResult>>
    where
        TBackendEmitter: IHasLocalsPool,
    {
        None
    }
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlAstNodeEmitter<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    /// Returns `Ok(None)` (or an invalid result) when this emitter does not handle the node.
    fn emit(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        context: &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<Option<TEmitResult>>;
}

/// A node that knows how to emit itself for a given backend.
/// Exposed by nodes through `IXamlAstNode::query_interface`.
pub trait IXamlAstEmitableNode<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    fn emit(
        &self,
        context: &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<TEmitResult>;
}

pub trait IXamlCustomEmitMethod<TBackendEmitter>: IXamlMethod {
    fn emit_call(&self, emitter: &TBackendEmitter) -> XamlResult<()>;
}

pub trait IXamlCustomEmitMethodWithContext<TBackendEmitter, TEmitResult: IXamlEmitResult>:
    IXamlMethod
{
    fn emit_call(
        &self,
        context: &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        emitter: &TBackendEmitter,
    ) -> XamlResult<()>;
}

pub trait IXamlPropertySetterEmitter<TBackendEmitter> {
    fn emit_call(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        emitter: &TBackendEmitter,
    ) -> XamlResult<bool>;
}

/// A property setter that knows how to emit itself for a given backend.
/// Exposed by setters through `IXamlPropertySetter::query_interface`.
pub trait IXamlEmitablePropertySetter<TBackendEmitter>: IXamlPropertySetter {
    fn emit(&self, emitter: &TBackendEmitter) -> XamlResult<()>;
}

pub trait IXamlWrappedMethodEmitter<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    fn emit_call(
        &self,
        context: &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        method: &Rc<dyn IXamlWrappedMethod>,
        emitter: &TBackendEmitter,
        swallow_result: bool,
    ) -> XamlResult<bool>;
}

/// A wrapped method that knows how to emit itself for a given backend.
/// Exposed by wrapped methods through `IXamlWrappedMethod::query_interface`.
pub trait IXamlEmitableWrappedMethod<TBackendEmitter, TEmitResult: IXamlEmitResult>:
    IXamlWrappedMethod
{
    fn emit(
        &self,
        context: &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        emitter: &TBackendEmitter,
        swallow_result: bool,
    ) -> XamlResult<()>;
}

type AfterEmitCallback = Box<dyn FnOnce() -> XamlResult<()>>;

/// The state of the upstream abstract `XamlEmitContext` class.
pub struct XamlEmitContextBase<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    base: XamlContextBase,
    context_local: Option<Rc<dyn IXamlLocal>>,
    pub file: Option<Rc<dyn IFileSource>>,
    pub emitters: Vec<Rc<dyn IXamlEmitter<TBackendEmitter, TEmitResult>>>,
    after_emit_callbacks: RefCell<Vec<AfterEmitCallback>>,
    current_node: RefCell<Option<Rc<dyn IXamlAstNode>>>,
    pub configuration: Rc<TransformerConfiguration>,
    pub emit_mappings: Rc<XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>>,
    pub runtime_context: Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>,
    pub declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
    pub emitter: TBackendEmitter,
}

/// Gives access to the `XamlContextBase` members (the C# base class).
impl<TBackendEmitter, TEmitResult: IXamlEmitResult> Deref
    for XamlEmitContextBase<TBackendEmitter, TEmitResult>
{
    type Target = XamlContextBase;

    fn deref(&self) -> &XamlContextBase {
        &self.base
    }
}

impl<TBackendEmitter, TEmitResult: IXamlEmitResult>
    XamlEmitContextBase<TBackendEmitter, TEmitResult>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        emitter: TBackendEmitter,
        configuration: Rc<TransformerConfiguration>,
        emit_mappings: Rc<XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>>,
        runtime_context: Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>,
        context_local: Option<Rc<dyn IXamlLocal>>,
        declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        file: Option<Rc<dyn IFileSource>>,
        emitters: Vec<Rc<dyn IXamlEmitter<TBackendEmitter, TEmitResult>>>,
    ) -> Self {
        Self {
            base: XamlContextBase::new(),
            context_local,
            file,
            emitters,
            after_emit_callbacks: RefCell::new(Vec::new()),
            current_node: RefCell::new(None),
            configuration,
            emit_mappings,
            runtime_context,
            declaring_type,
            emitter,
        }
    }

    pub fn context_local(&self) -> XamlResult<Rc<dyn IXamlLocal>> {
        self.context_local.clone().ok_or_else(|| {
            XamlError::invalid_operation(
                "The current emit context doesn't supply a runtime context",
            )
        })
    }

    pub fn add_after_emit_callbacks(&self, callback: impl FnOnce() -> XamlResult<()> + 'static) {
        self.after_emit_callbacks
            .borrow_mut()
            .push(Box::new(callback));
    }

    pub fn execute_after_emit_callbacks(&self) -> XamlResult<()> {
        let callbacks = std::mem::take(&mut *self.after_emit_callbacks.borrow_mut());
        for callback in callbacks {
            callback()?;
        }
        Ok(())
    }
}

/// The virtual and abstract members of the upstream `XamlEmitContext` class.
///
/// A backend implements this trait on its context type, embedding an [`XamlEmitContextBase`].
/// The provided methods are the upstream base implementations; "overriding" one means
/// implementing it and, where the override calls `base.Method(...)`, delegating to the matching
/// `default_*` function of this module.
pub trait XamlEmitContext<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static>:
    'static
{
    /// The base class state.
    fn base(&self) -> &XamlEmitContextBase<TBackendEmitter, TEmitResult>;

    /// `this`, as the base class type.
    fn as_emit_context(&self) -> &dyn XamlEmitContext<TBackendEmitter, TEmitResult>;

    fn as_any(&self) -> &dyn Any;

    /// `protected abstract void EmitConvert(...)`.
    fn emit_convert(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &TBackendEmitter,
        expected_type: &Rc<dyn IXamlType>,
        returned_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<()>;

    /// `protected virtual bool EmitCore(IXamlPropertySetter setter, TBackendEmitter codeGen)`.
    fn emit_core_property_setter(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<bool> {
        default_emit_core_property_setter(self.as_emit_context(), setter, code_gen)
    }

    /// `protected virtual bool EmitCore(IXamlWrappedMethod wrapped, TBackendEmitter codeGen, bool swallowResult)`.
    fn emit_core_wrapped_method(
        &self,
        wrapped: &Rc<dyn IXamlWrappedMethod>,
        code_gen: &TBackendEmitter,
        swallow_result: bool,
    ) -> XamlResult<bool> {
        default_emit_core_wrapped_method(self.as_emit_context(), wrapped, code_gen, swallow_result)
    }

    /// `protected virtual TEmitResult EmitNode(IXamlAstNode value, TBackendEmitter codeGen)`.
    fn emit_node(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<TEmitResult> {
        default_emit_node(self.as_emit_context(), value, code_gen)
    }

    /// `protected virtual TEmitResult? EmitNodeCore(IXamlAstNode value, TBackendEmitter codeGen, out bool foundEmitter)`.
    /// Returns the result together with `foundEmitter`.
    fn emit_node_core(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &TBackendEmitter,
    ) -> XamlResult<(Option<TEmitResult>, bool)> {
        default_emit_node_core(self.as_emit_context(), value, code_gen)
    }
}

/// Base implementation of `EmitCore(IXamlPropertySetter, TBackendEmitter)`.
pub fn default_emit_core_property_setter<B: 'static, R: IXamlEmitResult + 'static>(
    context: &dyn XamlEmitContext<B, R>,
    setter: &Rc<dyn IXamlPropertySetter>,
    code_gen: &B,
) -> XamlResult<bool> {
    let mut found_emitter = false;
    for e in &context.base().emitters {
        if let Some(pse) = e.as_property_setter_emitter() {
            found_emitter = pse.emit_call(setter, code_gen)?;
            if found_emitter {
                break;
            }
        }
    }

    if !found_emitter {
        if let Some(eps) =
            query_property_setter_interface::<dyn IXamlEmitablePropertySetter<B>>(setter)
        {
            found_emitter = true;
            eps.emit(code_gen)?;
        }
    }

    Ok(found_emitter)
}

/// Base implementation of `EmitCore(IXamlWrappedMethod, TBackendEmitter, bool)`.
pub fn default_emit_core_wrapped_method<B: 'static, R: IXamlEmitResult + 'static>(
    context: &dyn XamlEmitContext<B, R>,
    wrapped: &Rc<dyn IXamlWrappedMethod>,
    code_gen: &B,
    swallow_result: bool,
) -> XamlResult<bool> {
    let mut found_emitter = false;
    for e in &context.base().emitters {
        if let Some(wme) = e.as_wrapped_method_emitter() {
            found_emitter = wme.emit_call(context, wrapped, code_gen, swallow_result)?;
            if found_emitter {
                break;
            }
        }
    }

    if !found_emitter {
        if let Some(ewm) =
            query_wrapped_method_interface::<dyn IXamlEmitableWrappedMethod<B, R>>(wrapped)
        {
            found_emitter = true;
            ewm.emit(context, code_gen, swallow_result)?;
        }
    }

    Ok(found_emitter)
}

/// Base implementation of `EmitNode`.
pub fn default_emit_node<B: 'static, R: IXamlEmitResult + 'static>(
    context: &dyn XamlEmitContext<B, R>,
    value: &Rc<dyn IXamlAstNode>,
    code_gen: &B,
) -> XamlResult<R> {
    let result = context
        .emit_node_core(value, code_gen)
        .and_then(|(res, found_emitter)| {
            if !found_emitter {
                return Err(XamlError::load_exception(
                    format!(
                        "Unable to find emitter for node type: {}",
                        value.type_name()
                    ),
                    Some(&**value),
                ));
            }
            res.ok_or_else(|| {
                XamlError::internal(
                    "NullReferenceException",
                    "The node emitter didn't produce an emit result",
                )
            })
        });

    match result {
        Ok(res) => Ok(res),
        Err(e) if e.is_xml_exception() => Err(e),
        Err(e) => Err(XamlError::load_exception(
            format!(
                "Internal compiler error while emitting node {}:\n{}",
                value.to_node_string(),
                e
            ),
            Some(&**value),
        )),
    }
}

/// Base implementation of `EmitNodeCore`.
pub fn default_emit_node_core<B: 'static, R: IXamlEmitResult + 'static>(
    context: &dyn XamlEmitContext<B, R>,
    value: &Rc<dyn IXamlAstNode>,
    code_gen: &B,
) -> XamlResult<(Option<R>, bool)> {
    let mut res: Option<R> = None;
    for e in &context.base().emitters {
        if let Some(ve) = e.as_node_emitter() {
            res = ve.emit(value, context, code_gen)?;
            if res.as_ref().is_some_and(|r| r.valid()) {
                return Ok((res, true));
            }
        }
    }

    if let Some(en) = query_node_interface::<dyn IXamlAstEmitableNode<B, R>>(value) {
        return Ok((Some(en.emit(context, code_gen)?), true));
    }

    Ok((res, false))
}

/// The non-virtual public members of the upstream `XamlEmitContext` class.
impl<B: 'static, R: IXamlEmitResult + 'static> dyn XamlEmitContext<B, R> {
    /// `TEmitResult Emit(IXamlAstNode value, TBackendEmitter codeGen, IXamlType? expectedType)`.
    pub fn emit(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &B,
        expected_type: Option<&Rc<dyn IXamlType>>,
    ) -> XamlResult<R> {
        let base = self.base();
        let previous = base.current_node.borrow().clone();
        match previous {
            Some(current) => {
                base.push_parent(current);
                *base.current_node.borrow_mut() = Some(value.clone());
                let res = self.emit_core(value, code_gen, expected_type);
                *base.current_node.borrow_mut() = base.pop_parent();
                res
            }
            None => {
                *base.current_node.borrow_mut() = Some(value.clone());
                let res = self.emit_core(value, code_gen, expected_type);
                *base.current_node.borrow_mut() = None;
                res
            }
        }
    }

    /// `void Emit(IXamlPropertySetter setter, TBackendEmitter codeGen)`.
    pub fn emit_property_setter(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        code_gen: &B,
    ) -> XamlResult<()> {
        if !self.emit_core_property_setter(setter, code_gen)? {
            return Err(XamlError::invalid_operation(format!(
                "Unable to find emitter for property setter type: {}",
                setter.type_name()
            )));
        }
        Ok(())
    }

    /// `void Emit(IXamlWrappedMethod wrapped, TBackendEmitter codeGen, bool swallowResult)`.
    pub fn emit_wrapped_method(
        &self,
        wrapped: &Rc<dyn IXamlWrappedMethod>,
        code_gen: &B,
        swallow_result: bool,
    ) -> XamlResult<()> {
        if !self.emit_core_wrapped_method(wrapped, code_gen, swallow_result)? {
            return Err(XamlError::invalid_operation(format!(
                "Unable to find emitter for wrapped method type: {}",
                wrapped.type_name()
            )));
        }
        Ok(())
    }

    fn emit_core(
        &self,
        value: &Rc<dyn IXamlAstNode>,
        code_gen: &B,
        expected_type: Option<&Rc<dyn IXamlType>>,
    ) -> XamlResult<R> {
        let res = self.emit_node(value, code_gen)?;
        let returned_type = res.return_type();

        match (&returned_type, expected_type) {
            (None, None) => {}
            (Some(returned_type), None) => {
                return Err(XamlError::load_exception(
                    format!(
                        "Emit of node {} resulted in {} while caller expected void",
                        value.to_node_string(),
                        returned_type.get_fqn()
                    ),
                    Some(&**value),
                ));
            }
            (None, Some(expected_type)) => {
                return Err(XamlError::load_exception(
                    format!(
                        "Emit of node {} resulted in void while caller expected {}",
                        value.to_node_string(),
                        expected_type.get_fqn()
                    ),
                    Some(&**value),
                ));
            }
            (Some(returned_type), Some(expected_type)) => {
                if !returned_type.equals(&**expected_type) {
                    self.emit_convert(value, code_gen, expected_type, returned_type)?;
                }
            }
        }

        Ok(res)
    }

    pub fn context_local(&self) -> XamlResult<Rc<dyn IXamlLocal>> {
        self.base().context_local()
    }

    pub fn add_after_emit_callbacks(&self, callback: impl FnOnce() -> XamlResult<()> + 'static) {
        self.base().add_after_emit_callbacks(callback)
    }

    pub fn execute_after_emit_callbacks(&self) -> XamlResult<()> {
        self.base().execute_after_emit_callbacks()
    }
}
