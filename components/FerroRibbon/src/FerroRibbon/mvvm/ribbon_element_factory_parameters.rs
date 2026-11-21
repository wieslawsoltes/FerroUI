//! ParameterConverter/ParameterCommand from RibbonElementFactory.cs, 9755e281.
//! Copyright Wiesław Šoltés, MIT. Native commands use ordinary Rust functions.
use super::RibbonModelBindings;
use crate::{
    controls::{RibbonColorPicker, RibbonControlBase, RibbonGallery},
    metadata::model_objects,
};
use ferroribbon_core::model::*;
use ferroui_base::{
    BoxedValue, Ref, data::core::ValueTypes, input::ICommand, media::Color, reactive::IDisposable,
};
use ferroui_controls::Control;
use std::{
    any::Any,
    cell::RefCell,
    rc::{Rc, Weak},
};

type Converter = fn(Option<&BoxedValue>) -> Option<BoxedValue>;

// The source recognizes its private wrapper type when catalog assignment changes
// Command. These weak markers recognize only our own concrete wrappers; they do
// not retain commands, targets, models or subscriptions.
ferroui_base::ferro_thread_local! {
    static WRAPPED: RefCell<Vec<Weak<dyn ICommand>>> = RefCell::default();
}
fn is_wrapped(command: &Rc<dyn ICommand>) -> bool {
    WRAPPED.with_borrow_mut(|commands| {
        commands.retain(|value| value.strong_count() > 0);
        commands.iter().any(|value| {
            value
                .upgrade()
                .is_some_and(|value| Rc::ptr_eq(&value, command))
        })
    })
}

fn gallery_item_value(source: &Rc<dyn Any>) -> Option<RibbonObject> {
    if is_registered_model_type(source.as_ref().type_id())
        && let Some(model) = try_cast_model::<dyn RibbonGalleryItemModelImpl>(Some(source.clone()))
            .expect("Gallery parameter model hierarchy")
    {
        model.value()
    } else {
        None
    }
}

fn gallery_parameter(value: Option<&BoxedValue>) -> Option<BoxedValue> {
    let source = crate::commands::command_parameter(value)?;
    model_objects::to_native(gallery_item_value(&source))
        .expect("Gallery model Value")
        .or_else(|| value.cloned())
}

fn gallery_core_parameter(value: Option<&BoxedValue>) -> Option<BoxedValue> {
    let source = crate::commands::command_parameter(value)?;
    // The Core command adapter unwraps RibbonObject to its original payload.
    gallery_item_value(&source)
        .map(|value| Rc::new(value) as BoxedValue)
        .or_else(|| value.cloned())
}

pub(super) fn gallery_preview_parameter(value: Option<&BoxedValue>) -> Option<Rc<dyn Any>> {
    let source = crate::commands::command_parameter(value)?;
    Some(
        gallery_item_value(&source)
            .map(|value| value.payload())
            .unwrap_or(source),
    )
}

fn color_parameter(value: Option<&BoxedValue>) -> Option<BoxedValue> {
    let normalized = ValueTypes::normalize(value?.clone())?;
    if Rc::ptr_eq(&normalized, &RibbonColorPicker::no_color_parameter()) {
        None
    } else if let Some(color) = normalized.downcast_ref::<Color>() {
        Some(Rc::new(crate::theming::to_ribbon_color(*color)))
    } else {
        value.cloned()
    }
}

#[derive(Clone, Copy)]
enum ParameterKind {
    Gallery,
    Color,
}

fn parameter_kind(element: &Ref<Control>) -> Option<ParameterKind> {
    if element.cast::<RibbonGallery>().is_some() {
        Some(ParameterKind::Gallery)
    } else if element.cast::<RibbonColorPicker>().is_some() {
        Some(ParameterKind::Color)
    } else {
        None
    }
}

struct ParameterCommand {
    inner: Rc<dyn ICommand>,
    convert: Converter,
}
impl ICommand for ParameterCommand {
    fn can_execute(&self, value: Option<&BoxedValue>) -> bool {
        self.inner.can_execute((self.convert)(value).as_ref())
    }
    fn execute(&self, value: Option<&BoxedValue>) {
        self.inner.execute((self.convert)(value).as_ref())
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.inner.can_execute_changed(handler)
    }
}
fn wrap(inner: Rc<dyn ICommand>, kind: ParameterKind) -> Rc<dyn ICommand> {
    let convert: Converter = match kind {
        ParameterKind::Gallery if crate::commands::source_core_command(&inner).is_some() => {
            gallery_core_parameter
        }
        ParameterKind::Gallery => gallery_parameter,
        ParameterKind::Color => color_parameter,
    };
    let command: Rc<dyn ICommand> = Rc::new(ParameterCommand { inner, convert });
    WRAPPED.with_borrow_mut(|commands| {
        commands.retain(|command| command.strong_count() > 0);
        commands.push(Rc::downgrade(&command));
    });
    command
}

pub(super) fn item_command(
    element: &Ref<Control>,
    command: Option<Rc<dyn ICommand>>,
) -> Option<Rc<dyn ICommand>> {
    match parameter_kind(element) {
        Some(kind) => command.map(|command| wrap(command, kind)),
        None => command,
    }
}

pub(super) fn watch_command(element: &Ref<Control>, bindings: &Rc<RibbonModelBindings>) {
    let Some(kind) = parameter_kind(element) else {
        return;
    };
    let Some(target) = element.cast::<RibbonControlBase>() else {
        return;
    };
    let weak = target.downgrade();
    let subscription = target.property_changed(move |event| {
        if event.property() == RibbonControlBase::command_property().as_property()
            && let Some(target) = weak.upgrade()
            && let Some(command) = target.command()
            && !is_wrapped(&command)
        {
            target.set_command(Some(wrap(command, kind)));
        }
    });
    bindings.add(Some(Rc::new(move || subscription.dispose())));
}
