//! The plan of a property assignment and the reading of a numeric constant: what the
//! interpreter follows when it evaluates the node and the emitter of Rust source unrolls
//! into statements, decided from the transformed AST alone.

use std::rc::Rc;

use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::{IXamlAstNode, IXamlPropertySetter, XamlPropertyAssignmentNode};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlType, XamlValue};

/// What is decided once about a property assignment: the setters that can
/// take its values, and the static types of the values.
pub struct AssignmentPlan {
    _node: Rc<dyn IXamlAstNode>,
    pub(crate) setters: Vec<Rc<dyn IXamlPropertySetter>>,
    pub(crate) value_types: Vec<Rc<dyn IXamlType>>,
}

pub(crate) fn last_parameter(setter: &Rc<dyn IXamlPropertySetter>) -> XamlResult<Rc<dyn IXamlType>> {
    setter.parameters().last().cloned().ok_or_else(|| {
        XamlError::internal("ArgumentOutOfRangeException", "A property setter doesn't have a value parameter")
    })
}

pub(crate) fn assignment_plan(node: &Rc<dyn IXamlAstNode>, assignment: &XamlPropertyAssignmentNode) -> XamlResult<AssignmentPlan> {
    let values = assignment.values.borrow().clone();
    let possible = assignment.possible_setters.borrow().clone();
    let mut value_types = Vec::with_capacity(values.len());
    for value in &values {
        value_types.push(value.type_().get_clr_type()?);
    }
    let dynamic_type = value_types
        .last()
        .cloned()
        .ok_or_else(|| XamlError::invalid_operation("Sequence contains no elements"))?;

    // The setters that take as many values as the assignment has.
    let mut setters: Vec<Rc<dyn IXamlPropertySetter>> =
        possible.iter().filter(|s| s.parameters().len() == values.len()).cloned().collect();
    if values.len() > 1 && setters.len() > 1 {
        for c in 0..values.len().saturating_sub(2) {
            let failed = possible
                .iter()
                .find(|s| s.parameters().get(c).is_none_or(|p| !p.is_directly_assignable_from(&*value_types[c])));
            if let Some(failed) = failed {
                return Err(XamlError::load_exception(
                    format!(
                        "Can not statically cast {} to {} and runtime type checking is only supported for the last setter argument",
                        value_types[c].get_fqn(),
                        failed.parameters().get(c).map(|p| p.get_fqn()).unwrap_or_default()
                    ),
                    Some(&**node),
                ));
            }
        }
    }
    if setters.is_empty() {
        return Err(XamlError::load_exception("No setters found for property assignment", Some(&**node)));
    }

    // Removes the setters that can never be chosen.
    if setters.len() > 1 {
        if dynamic_type.is_value_type() {
            // A value of a value type always uses the first one.
            setters.truncate(1);
        } else {
            let mut index = 0;
            while index < setters.len() {
                let setter = setters[index].clone();
                let type_ = last_parameter(&setter)?;
                // The value is assignable and the setter allows null: it always matches.
                if type_.is_assignable_from(&*dynamic_type) && setter.binder_parameters().allow_runtime_null.get() {
                    setters.truncate(index + 1);
                    break;
                }
                // A previous setter already matches the type of this one or a base type of it.
                let mut redundant = false;
                for previous in &setters[..index] {
                    if last_parameter(previous)?.is_assignable_from(&*type_)
                        && (previous.binder_parameters().allow_runtime_null.get()
                            || !setter.binder_parameters().allow_runtime_null.get())
                    {
                        redundant = true;
                        break;
                    }
                }
                if redundant {
                    setters.remove(index);
                    continue;
                }
                index += 1;
            }
        }
    }
    Ok(AssignmentPlan { _node: node.clone(), setters, value_types })
}

/// The setters a property assignment chooses from, in order, and the types
/// of its values: the plan the interpreter follows (one setter: it is
/// always used; several: the first that takes the run-time value of the last
/// value). The emitter of Rust source unrolls the choice from it.
pub fn plan_setters(
    node: &Rc<dyn IXamlAstNode>,
    assignment: &XamlPropertyAssignmentNode,
) -> XamlResult<(Vec<Rc<dyn IXamlPropertySetter>>, Vec<Rc<dyn IXamlType>>)> {
    let plan = assignment_plan(node, assignment)?;
    Ok((plan.setters, plan.value_types))
}

/// The integer and the floating-point reading of a numeric constant; the emitter of Rust
/// source reads constants the same way.
pub fn numeric_constant(constant: &XamlValue) -> Option<(i128, f64)> {
    Some(match constant {
        XamlValue::Boolean(v) => (i128::from(*v), f64::from(u8::from(*v))),
        XamlValue::Char(v) => (i128::from(u32::from(*v)), f64::from(u32::from(*v))),
        XamlValue::SByte(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Byte(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int16(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::UInt16(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int32(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::UInt32(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int64(v) => (i128::from(*v), *v as f64),
        XamlValue::UInt64(v) => (i128::from(*v), *v as f64),
        XamlValue::Single(v) => (*v as i128, f64::from(*v)),
        XamlValue::Double(v) => (*v as i128, *v),
        _ => return None,
    })
}
