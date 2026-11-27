//! Ordered overload selection when a markup extension supplies the value at runtime.
use super::compiled::{is_instance, no_setter};
use crate::XamlLoadError;
use ferroui_base::{
    data::{core::ValueType, BindingPriority},
    metadata::MarkupValue,
};
use ferroui_markup::{compiled_operands::read_unsigned, construction_opcodes::choice};
use std::any::Any;

pub type SetterAction = fn(&dyn Any, &MarkupValue, BindingPriority) -> Result<(), XamlLoadError>;

/// Predicates and target methods have both been resolved by the build-time type system.
pub struct TypedSetterChoice {
    pub code: &'static [u8],
    pub types: &'static [fn() -> ValueType],
    pub actions: &'static [SetterAction],
    pub property: &'static str,
}

pub fn choice_receiver<T: 'static>(receiver: &dyn Any) -> Result<&T, XamlLoadError> {
    receiver.downcast_ref().ok_or_else(invalid)
}

fn invalid() -> XamlLoadError {
    XamlLoadError::with_message("Invalid compiled XAML setter choice or import")
}

impl TypedSetterChoice {
    #[inline(never)]
    pub fn apply(
        &self,
        target: &dyn Any,
        value: &MarkupValue,
        priority: BindingPriority,
        line: i32,
        column: i32,
    ) -> Result<(), XamlLoadError> {
        let mut code = self.code;
        while let Some((&test, rest)) = code.split_first() {
            code = rest;
            let matches = match test {
                choice::ALWAYS => true,
                choice::NON_NULL => value.is_some(),
                choice::NULL => value.is_none(),
                choice::INSTANCE => {
                    let index = read_unsigned(&mut code).map_err(|_| invalid())? as usize;
                    is_instance(value, self.types.get(index).ok_or_else(invalid)?())
                }
                _ => return Err(invalid()),
            };
            let index = read_unsigned(&mut code).map_err(|_| invalid())? as usize;
            let action = self.actions.get(index).ok_or_else(invalid)?;
            if matches {
                return action(target, value, priority);
            }
        }
        Err(no_setter(self.property, value, line, column))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xaml_il::runtime::compiled::to_object;
    use std::cell::Cell;

    #[test]
    fn instance_null_and_fallback_follow_the_compilers_order() {
        let choice = TypedSetterChoice {
            code: &[choice::INSTANCE, 0, 0, choice::NULL, 1, choice::ALWAYS, 2],
            types: &[ValueType::of::<i32>],
            actions: &[
                |r, _, _| {
                    choice_receiver::<Cell<i32>>(r)?.set(1);
                    Ok(())
                },
                |r, _, _| {
                    choice_receiver::<Cell<i32>>(r)?.set(2);
                    Ok(())
                },
                |r, _, _| {
                    choice_receiver::<Cell<i32>>(r)?.set(3);
                    Ok(())
                },
            ],
            property: "Test",
        };
        let receiver = Cell::new(0_i32);
        for (value, expected) in [(to_object(4_i32), 1), (None, 2), (to_object(true), 3)] {
            choice
                .apply(&receiver, &value, BindingPriority::Template, 19, 2)
                .unwrap();
            assert_eq!(receiver.get(), expected);
        }
        let partial = TypedSetterChoice {
            code: &[choice::NON_NULL, 0],
            ..choice
        };
        let error = partial
            .apply(&receiver, &None, BindingPriority::LocalValue, 17, 8)
            .unwrap_err();
        assert_eq!(error.message(), no_setter("Test", &None, 17, 8).message());
        let invalid = TypedSetterChoice {
            code: &[choice::INSTANCE, 11, 0],
            ..partial
        };
        assert!(invalid
            .apply(&receiver, &None, BindingPriority::LocalValue, 17, 8)
            .unwrap_err()
            .message()
            .contains("Invalid compiled XAML"));
    }
}
