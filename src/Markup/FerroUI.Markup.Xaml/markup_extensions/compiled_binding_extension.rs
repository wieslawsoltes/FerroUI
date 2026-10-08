//! Port of `MarkupExtensions/CompiledBindingExtension.cs`.

use crate::ServiceProviderExtensions;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingBase, BindingExpressionBase, CompiledBinding, CompiledBindingPath};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, FerroObject, FerroProperty, Ref};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// `{CompiledBinding Path}`: a binding with a compiled path, as markup
/// declares it. Providing its value creates the binding with the context of
/// the place it is declared in (the default anchor).
///
/// The managed class derives from the compiled binding class; here it holds
/// one ([`base`](Self::base)) and dereferences to it, so the inherited
/// properties are read and set on the extension itself.
pub struct CompiledBindingExtension {
    base: Rc<CompiledBinding>,
    data_type: RefCell<Option<ValueType>>,
}

impl CompiledBindingExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { base: CompiledBinding::empty(), data_type: RefCell::new(None) })
    }

    /// Creates the extension with a path.
    pub fn with_path(path: CompiledBindingPath) -> Rc<Self> {
        let this = Self::new();
        this.set_path(Some(path));
        this
    }

    /// The compiled binding this extension is (the base class part).
    pub fn base(&self) -> &Rc<CompiledBinding> {
        &self.base
    }

    /// Creates the binding the extension describes.
    pub fn provide_value(&self, provider: Option<&Rc<dyn IServiceProvider>>) -> Rc<CompiledBinding> {
        let binding = CompiledBinding::empty();
        binding.set_path(self.path());
        binding.set_delay(self.delay());
        binding.set_converter(self.converter());
        binding.set_converter_culture(self.converter_culture());
        binding.set_converter_parameter(self.converter_parameter());
        binding.set_target_null_value(self.target_null_value());
        binding.set_fallback_value(self.fallback_value());
        binding.set_mode(self.mode());
        binding.set_priority(self.priority());
        binding.set_string_format(self.string_format());
        binding.set_source(self.source());
        binding.set_default_anchor(provider.and_then(|p| p.get_default_anchor()).map(|anchor| anchor.downgrade()));
        binding.set_update_source_trigger(self.update_source_trigger());
        binding
    }

    /// The type of the data context the path was compiled against.
    pub fn data_type(&self) -> Option<ValueType> {
        *self.data_type.borrow()
    }

    pub fn set_data_type(&self, value: Option<ValueType>) {
        *self.data_type.borrow_mut() = value;
    }
}

impl Deref for CompiledBindingExtension {
    type Target = CompiledBinding;

    fn deref(&self) -> &CompiledBinding {
        &self.base
    }
}

impl BindingBase for CompiledBindingExtension {
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        self.base.create_instance(target, target_property, anchor)
    }

    /// The extension itself; the binding class it derives from is its `base()`.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

crate::identity_eq!(CompiledBindingExtension);

type This = Rc<CompiledBindingExtension>;

// The inherited properties are the ones the compiled binding class declares:
// they are reached through `base`.
ferro_markup_type!(class CompiledBindingExtension {
    this: Rc<CompiledBindingExtension>,
    handles: [Rc<CompiledBindingExtension>, Option<Rc<CompiledBindingExtension>>],
    base: Rc<CompiledBinding>,
    constructors: [
        () => CompiledBindingExtension::new,
        (CompiledBindingPath) => CompiledBindingExtension::with_path,
    ],
    properties: [
        DataType: Option<ValueType> {
            get: |this: &This| this.data_type(),
            set: |this: &This, value: Option<ValueType>| this.set_data_type(value)
        },
    ],
    methods: [
        fn ProvideValue(Option<Rc<dyn IServiceProvider>>) -> Rc<CompiledBinding> =>
            |this: &This, provider: Option<Rc<dyn IServiceProvider>>| this.provide_value(provider.as_ref()),
    ],
});
