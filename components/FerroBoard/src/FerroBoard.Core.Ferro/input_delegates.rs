//! Synchronous closure conversion at the actual FerroUI markup boundary.
//!
//! Each read owns a callback snapshot. No registry interns delegate identity or
//! retains callbacks beyond their Rust owners.
use ferroboard_core::editor::{EnterAction, TextBufferTarget};
use ferroui_base::metadata::{
    MarkupDelegate, MarkupInvokeError, from_markup_value, into_markup_value,
};
use std::rc::Rc;

pub(crate) fn get(target: &Rc<TextBufferTarget>) -> Option<MarkupDelegate> {
    let call = target.enter_action_handler()?;
    Some(MarkupDelegate::new(move |args| {
        if args.len() != 1 {
            std::panic::panic_any(MarkupInvokeError::ArgumentCount {
                expected: 1,
                actual: args.len(),
            });
        }
        let action = from_markup_value::<EnterAction>(&args[0]).unwrap_or_else(|| {
            std::panic::panic_any(MarkupInvokeError::Failed(
                "enter callback requires EnterAction".into(),
            ))
        });
        into_markup_value(call(action))
    }))
}

pub(crate) fn set(target: &Rc<TextBufferTarget>, value: Option<MarkupDelegate>) {
    target.set_enter_action_handler(value.map(|call| {
        Rc::new(move |action| {
            let result = call
                .try_invoke(&[into_markup_value(action)])
                .unwrap_or_else(|e| std::panic::panic_any(e));
            from_markup_value::<bool>(&result).unwrap_or_else(|| {
                std::panic::panic_any(MarkupInvokeError::Failed(
                    "enter callback must return Boolean".into(),
                ))
            })
        }) as ferroboard_core::editor::EnterActionHandler
    }));
}
