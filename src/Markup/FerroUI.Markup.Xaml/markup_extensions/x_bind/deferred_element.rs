//! `x:Load` and `x:DeferLoadStrategy` (step X5 of docs/porting/x-bind.md): the placeholder of an
//! element whose creation is deferred.
//!
//! Not a port of an upstream file. The behaviour is WinUI's (NOTICE.md of the crate): an element
//! with `x:Load="False"` (or `x:DeferLoadStrategy="Lazy"`) is not created with its document but
//! when it is found by name (`FindName`), and an element with a bound `x:Load` is created when the
//! value becomes `true` and released when it becomes `false` (the `FindName` and `UnloadObject`
//! calls the XAML compiler generates for a `BoundLoadAssignment`, `CSharpPagePass2.tt`).

use crate::templates::TemplateContent;
use crate::XamlLoadError;
use ferroui_base::controls::{INameScope, NameScope};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::FerroObjectImplExt;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, AttachedProperty,
    BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{ContentControl, Control, ControlImpl, Decorator, Panel};
use std::cell::RefCell;

/// The placeholder of an element with `x:Load` (or `x:DeferLoadStrategy="Lazy"`).
///
/// The compiler replaces the element by a placeholder that holds it as deferred content
/// ([`content`](Self::content), built as a template is) with its name
/// ([`element_name`](Self::element_name)) and the value of `x:Load` ([`load`](Self::load_property),
/// a literal or an `{x:Bind}`). The placeholder takes no room and is not shown. When `Load`
/// becomes `true` (or [`find_name`](Self::find_name) asks for the element), the element is built
/// and takes the place of the placeholder in its parent (a panel, a decorator or a content
/// control); when `Load` becomes `false` the placeholder takes its place again and the element is
/// released, so that loading it again builds a new one (WinUI's `UnloadObject`).
///
/// The element is built with the root of its document and a name scope of its own, a child of
/// the one of the document: its `x:Bind` bindings read the root of the document (or the item of
/// the template instance it is in, or its templated parent), and its named elements are found in
/// that scope, from it and through [`find_name`](Self::find_name). Inside a control template the
/// element is given the templated parent of the template instance, so that a bound `x:Load` in a
/// control template works as in a page (WinUI's fix of bound `x:Load` in control templates).
#[repr(C)]
pub struct DeferredElement {
    base: Control,
    element_name: RefCell<Option<String>>,
    content: RefCell<Option<BoxedValue>>,
    /// The element while it is loaded; it holds the placeholder
    /// ([`placeholder_property`](Self::placeholder_property)).
    loaded: RefCell<Option<WeakRef<Control>>>,
    /// Why the last change of `Load` could not be applied.
    error: RefCell<Option<String>>,
}

ferro_class!(DeferredElement: Control);
ferro_class_info!(DeferredElement {
    new: DeferredElement::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.MarkupExtensions",
        properties: [
            ElementName: Option<String> {
                get: |this: &Ref<DeferredElement>| this.element_name(),
                set: |this: &Ref<DeferredElement>, value: Option<String>| this.set_element_name(value)
            },
            Content: Option<BoxedValue> {
                get: |this: &Ref<DeferredElement>| this.content(),
                set: |this: &Ref<DeferredElement>, value: Option<BoxedValue>| this.set_content(value)
            } [TemplateContent],
        ],
    },
});
ferro_impl_classes!(DeferredElement: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferro_properties! { impl DeferredElement {
    ferro_property!(
        /// `x:Load`: whether the element is loaded.
        pub fn load_property() -> StyledProperty<bool> {
            FerroProperty::register::<DeferredElement, _>("Load", false)
        }
    );

    ferro_property!(
        /// The placeholder a loaded element replaces, which the element holds while it is loaded.
        pub fn placeholder_property() -> AttachedProperty<Option<Ref<DeferredElement>>> {
            FerroProperty::register_attached::<DeferredElement, Control, _>("Placeholder", None)
        }
    );
} }

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DeferredElement {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);
            if change.property() == Self::load_property().as_property() {
                this.load_changed();
            }
        }
    }
}

impl DeferredElement {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            element_name: RefCell::new(None),
            content: RefCell::new(None),
            loaded: RefCell::new(None),
            error: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let result = instantiate(Self::construct());
        // The placeholder takes no room and is not shown: WinUI keeps no element there at all.
        result.set_value(Visual::is_visible_property(), false);
        result
    }

    /// The `x:Name` of the deferred element.
    pub fn element_name(&self) -> Option<String> {
        self.element_name.borrow().clone()
    }

    pub fn set_element_name(&self, value: Option<String>) {
        *self.element_name.borrow_mut() = value;
    }

    /// The deferred element: deferred content, built as the content of a template is.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// The value of `x:Load`.
    pub fn is_load_requested(&self) -> bool {
        self.get_value(Self::load_property())
    }

    pub fn set_load(&self, value: bool) {
        self.set_value(Self::load_property(), value);
    }

    /// The element, while it is loaded.
    pub fn loaded_element(&self) -> Option<Ref<Control>> {
        self.loaded.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Why the last change of `Load` could not be applied (a placeholder whose parent does not
    /// hold it as a child, content that is not a control), if it could not.
    pub fn error(&self) -> Option<String> {
        self.error.borrow().clone()
    }

    fn load_changed(&self) {
        let result = match self.is_load_requested() {
            true => self.load().map(|_| ()),
            false => {
                self.unload();
                Ok(())
            }
        };
        *self.error.borrow_mut() = result.err().map(|error| error.to_string());
    }

    /// Builds the element and puts it in the place of the placeholder (`FindName` of WinUI on a
    /// deferred element); the element when it is already loaded. `None` while the placeholder
    /// has no parent, or when the content builds nothing.
    pub fn load(&self) -> Result<Option<Ref<Control>>, XamlLoadError> {
        if let Some(element) = self.loaded_element() {
            return Ok(Some(element));
        }
        let Some(parent) = self.parent() else {
            return Ok(None);
        };
        let content = self.content();
        let Some(built) = TemplateContent::try_load_as::<Ref<Control>>(content.as_ref())? else {
            return Ok(None);
        };
        let (element, name_scope) = built.deconstruct();
        // The named elements of the deferred element are found from it.
        if NameScope::get_name_scope(&element).is_none() {
            NameScope::set_name_scope(&element, Some(name_scope));
        }
        // In a control template the element belongs to the template instance of its place.
        if element.templated_parent().is_none() {
            if let Some(templated_parent) = self.templated_parent().or_else(|| parent.templated_parent()) {
                TemplatedControl::apply_templated_parent(&element, Some(&templated_parent));
            }
        }
        element.set_value(Self::placeholder_property(), Some(self.to_ref()));
        *self.loaded.borrow_mut() = Some(element.downgrade());
        let placeholder: Ref<Control> = self.to_ref().upcast();
        if let Err(error) = replace_child(&parent, &placeholder, &element) {
            self.loaded.borrow_mut().take();
            element.clear_value(Self::placeholder_property());
            return Err(error);
        }
        Ok(Some(element))
    }

    /// Puts the placeholder back in the place of the loaded element and releases the element
    /// (`UnloadObject`).
    pub fn unload(&self) {
        // The element holds the placeholder until its place is given back.
        let _placeholder = self.to_ref();
        let loaded = self.loaded.borrow_mut().take();
        let Some(element) = loaded.and_then(|element| element.upgrade()) else {
            return;
        };
        if let Some(parent) = element.parent() {
            let placeholder: Ref<Control> = self.to_ref().upcast();
            if let Err(error) = replace_child(&parent, &element, &placeholder) {
                *self.error.borrow_mut() = Some(error.to_string());
            }
        }
        element.clear_value(Self::placeholder_property());
    }

    /// WinUI's `FindName` with the realization of `x:Load`: the element named `name` in the name
    /// scope of `scope`, else a deferred element of that name among the logical descendants of
    /// `scope`, loaded if it is not, else a named element of a loaded deferred element.
    pub fn find_name(scope: &StyledElement, name: &str) -> Result<Option<Ref<FerroObject>>, XamlLoadError> {
        if let Some(found) = NameScope::get_name_scope(scope).and_then(|names| names.find(name)) {
            return Ok(Some(found));
        }
        for element in scope.get_logical_descendants() {
            if let Some(placeholder) = element.clone().cast::<DeferredElement>() {
                if placeholder.element_name().as_deref() == Some(name) {
                    return Ok(placeholder.load()?.map(|loaded| loaded.upcast::<FerroObject>()));
                }
                continue;
            }
            if let Some(placeholder) = element.get_value(Self::placeholder_property()) {
                if placeholder.element_name().as_deref() == Some(name) {
                    return Ok(Some(element.upcast::<FerroObject>()));
                }
                if let Some(found) = NameScope::get_name_scope(&element).and_then(|names| names.find(name)) {
                    return Ok(Some(found));
                }
            }
        }
        Ok(None)
    }
}

/// Puts `new` in the place of `old` among the children of `parent`: an element of the children
/// of a panel, the child of a decorator or the content of a content control.
fn replace_child(parent: &StyledElement, old: &Ref<Control>, new: &Ref<Control>) -> Result<(), XamlLoadError> {
    let parent = parent.to_ref();
    if let Some(panel) = parent.clone().cast::<Panel>() {
        let children = panel.children();
        if let Some(index) = children.index_of(old) {
            children.set(index, new.clone());
            return Ok(());
        }
    } else if let Some(decorator) = parent.clone().cast::<Decorator>() {
        if decorator.child().as_ref() == Some(old) {
            decorator.set_child(new.clone());
            return Ok(());
        }
    } else if let Some(content_control) = parent.clone().cast::<ContentControl>() {
        let old_object: Ref<FerroObject> = old.clone().upcast();
        let holds = content_control
            .content()
            .and_then(|content| ValueTypes::as_object(&*content))
            .is_some_and(|content| content == old_object);
        if holds {
            content_control.set_content(Some(Control::boxed(new.clone())));
            return Ok(());
        }
    }
    Err(XamlLoadError::with_message(format!(
        "x:Load: the element of '{}' can only take the place of its placeholder in a panel, a decorator or a content control",
        parent.get_type().name()
    )))
}
