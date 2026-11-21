//! Narrow bridge for the selected SDK's actual markup task/stream service.
//! The Rust dialog API and picker use standard futures, not this transport.
use crate::controls::RibbonColorDialog;
use ferroui_base::{
    Ref,
    data::{BindingError, core::plugins::TaskValue},
    media::Color,
    metadata::into_markup_value,
    styling::ThemeVariant,
};
use ferroui_controls::TopLevel;
use std::task::{Context, Poll, Waker};

pub(crate) fn show_async(
    root: Option<Ref<TopLevel>>,
    initial: Color,
    theme: Option<ThemeVariant>,
) -> TaskValue {
    let mut future = RibbonColorDialog::show_async(root, initial, theme);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(Ok(color)) => TaskValue::from_untyped_result(into_markup_value(color)),
        Poll::Ready(Err(error)) => TaskValue::from_exception(BindingError::new(error)),
        Poll::Pending => TaskValue::run(async move {
            future
                .await
                .map(into_markup_value)
                .map_err(BindingError::new)
        }),
    }
}
