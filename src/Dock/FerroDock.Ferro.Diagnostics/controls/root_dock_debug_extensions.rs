// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::RootDockDebug;
use ferroui_base::{
    input::{InputElement, Key, KeyGesture, KeyModifiers},
    interactivity::RoutingStrategies,
    metadata::MarkupDelegate,
    BoxedValue, Ref,
};
use ferroui_controls::{Control, TopLevel, Window};
use ferroui_system::IDisposable;
use std::{cell::RefCell, rc::Rc};
/// Resolves the current source layout lazily each time a debug window opens.
pub type DockDebugLayoutProvider = Rc<dyn Fn() -> Option<BoxedValue>>;
/// Creates a custom source debug window for the attached top level.
pub type DockDebugWindowFactory = Rc<dyn Fn(Ref<TopLevel>) -> Ref<Window>>;
type NullableWindowFactory = Rc<dyn Fn(Ref<TopLevel>) -> Option<Ref<Window>>>;
/// Original extension overloads for the F11 docking inspector.
pub struct RootDockDebugExtensions;
ferroui_base::ferro_static_type!(RootDockDebugExtensions);
ferroui_base::ferro_markup_type!(static RootDockDebugExtensions {
 namespace:"FerroDock.Ferro.Diagnostics.Controls",type_info:RootDockDebugExtensions,
 methods:[
  static fn AttachDockDebug(Ref<TopLevel>,Rc<MarkupDelegate>,Option<KeyGesture>)->Rc<dyn IDisposable> => |top:Ref<TopLevel>,provider:Rc<MarkupDelegate>,gesture|RootDockDebugExtensions::attach_dock_debug(top,Rc::new(move||provider.try_invoke(&[]).unwrap_or_else(|error|panic!("{error}"))),gesture),
  static fn AttachDockDebug(Ref<TopLevel>,Rc<MarkupDelegate>,Option<KeyGesture>,Option<Rc<MarkupDelegate>>)->Rc<dyn IDisposable> => |top:Ref<TopLevel>,provider:Rc<MarkupDelegate>,gesture,factory:Option<Rc<MarkupDelegate>>|RootDockDebugExtensions::attach_delegates(top,provider,gesture,factory),
  static fn AttachDockDebug(Ref<TopLevel>,Rc<MarkupDelegate>,Option<Rc<MarkupDelegate>>,Option<KeyGesture>)->Rc<dyn IDisposable> => |top:Ref<TopLevel>,provider:Rc<MarkupDelegate>,factory:Option<Rc<MarkupDelegate>>,gesture|RootDockDebugExtensions::attach_delegates(top,provider,gesture,factory),
 ],
});
impl RootDockDebugExtensions {
    /// Attaches the source inspector with the F11/default or supplied gesture.
    pub fn attach_dock_debug(
        top_level: Ref<TopLevel>,
        layout_provider: DockDebugLayoutProvider,
        gesture: Option<KeyGesture>,
    ) -> Rc<dyn IDisposable> {
        Self::attach_dock_debug_with_factory(top_level, layout_provider, gesture, None)
    }
    /// Attaches the source inspector with the original gesture/factory argument order.
    pub fn attach_dock_debug_with_factory(
        top_level: Ref<TopLevel>,
        layout_provider: DockDebugLayoutProvider,
        gesture: Option<KeyGesture>,
        window_factory: Option<DockDebugWindowFactory>,
    ) -> Rc<dyn IDisposable> {
        Self::attach_core(
            top_level,
            layout_provider,
            gesture,
            window_factory
                .map(|factory| Rc::new(move |top| Some(factory(top))) as NullableWindowFactory),
        )
    }
    /// Attaches the source inspector with the original factory/gesture argument order.
    pub fn attach_dock_debug_with_window_factory(
        top_level: Ref<TopLevel>,
        layout_provider: DockDebugLayoutProvider,
        window_factory: Option<DockDebugWindowFactory>,
        gesture: Option<KeyGesture>,
    ) -> Rc<dyn IDisposable> {
        Self::attach_dock_debug_with_factory(top_level, layout_provider, gesture, window_factory)
    }
    fn attach_delegates(
        top: Ref<TopLevel>,
        provider: Rc<MarkupDelegate>,
        gesture: Option<KeyGesture>,
        factory: Option<Rc<MarkupDelegate>>,
    ) -> Rc<dyn IDisposable> {
        Self::attach_core(
            top,
            Rc::new(move || {
                provider
                    .try_invoke(&[])
                    .unwrap_or_else(|error| panic!("{error}"))
            }),
            gesture,
            factory.map(|factory| {
                Rc::new(move |top| {
                    factory
                        .try_invoke(&[Some(Rc::new(top))])
                        .unwrap_or_else(|error| panic!("{error}"))
                        .map(|value| {
                            ferroui_base::data::core::ValueTypes::as_object(&*value)
                                .and_then(|object| object.cast::<Window>())
                                .expect("Source debug window factory must return a Window")
                        })
                }) as NullableWindowFactory
            }),
        )
    }
    fn attach_core(
        top_level: Ref<TopLevel>,
        layout_provider: DockDebugLayoutProvider,
        gesture: Option<KeyGesture>,
        window_factory: Option<NullableWindowFactory>,
    ) -> Rc<dyn IDisposable> {
        let gesture = gesture.unwrap_or_else(|| KeyGesture::new(Key::F11, KeyModifiers::empty()));
        let window: Rc<RefCell<Option<Ref<Window>>>> = Rc::new(RefCell::new(None));
        let state = window.clone();
        let owner = top_level.clone();
        let key = top_level.add_disposable_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if !gesture.matches(Some(event)) {
                    return;
                }
                let current = state.borrow().clone();
                if let Some(current) = current {
                    current.close();
                } else {
                    let Some(layout) = layout_provider() else {
                        return;
                    };
                    let created = window_factory
                        .as_ref()
                        .and_then(|factory| factory(owner.clone()))
                        .unwrap_or_else(|| {
                            let window = Window::new();
                            window.set_width(400.0);
                            window.set_height(680.0);
                            window
                        });
                    state.replace(Some(created.clone()));
                    let inspector = RootDockDebug::new();
                    inspector.set_data_context(Some(layout));
                    created.set_content(Some(Control::boxed(inspector)));
                    let closed = state.clone();
                    created.closed(move || {
                        closed.replace(None);
                    });
                    created.show();
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let closing = top_level.cast::<Window>().map(|main| {
            let state = window.clone();
            main.closing(move |_| {
                let current = state.borrow().clone();
                if let Some(current) = current {
                    current.close();
                    state.replace(None);
                }
            })
        });
        Rc::new(ActionDisposable(Box::new(move || {
            let _owner = &top_level;
            key.dispose();
            if let Some(closing) = &closing {
                closing.dispose();
            }
            let current = window.borrow().clone();
            if let Some(current) = current {
                current.close();
                window.replace(None);
            }
        })))
    }
}
struct ActionDisposable(Box<dyn Fn()>);
impl IDisposable for ActionDisposable {
    fn dispose(&self) {
        (self.0)();
    }
}
