use super::{FuncTemplateWithParam, IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use crate::primitives::AccessText;
use crate::{Control, TextBlock};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::{AnyValue, BoxedValue, FerroObject, FerroObjectExtensions, Ref, StyledElement};
use std::rc::Rc;

/// Builds a control for a piece of data.
///
/// The typed constructors ([`for_type`](Self::for_type) and friends) take
/// the place of the generic class of the same name: they match data whose
/// boxed value holds exactly `T`, and null data when `T` accepts null.
pub struct FuncDataTemplate {
    base: FuncTemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>>,
    match_: Box<dyn Fn(Option<&BoxedValue>) -> bool>,
    supports_recycling: bool,
}

thread_local! {
    static DEFAULT: Rc<FuncDataTemplate> = FuncDataTemplate::new(
        |_| true,
        |data, _| {
            data.as_ref().map(|_| {
                let result = TextBlock::new();
                bind_text_to_data_context(&result);
                result.upcast()
            })
        },
        true,
    );
    static ACCESS: Rc<FuncDataTemplate> = FuncDataTemplate::new(
        |data| data.is_some(),
        |data, _| {
            data.as_ref().map(|_| {
                let result = AccessText::new();
                bind_text_to_data_context(&result);
                result.upcast()
            })
        },
        true,
    );
}

/// Binds the text of a text block to the string form of its data context.
fn bind_text_to_data_context(text_block: &TextBlock) {
    let object: &FerroObject = text_block;
    text_block.bind(
        TextBlock::text_property(),
        FerroObjectExtensions::get_observable_with(object, StyledElement::data_context_property(), |x: Option<BoxedValue>| {
            x.map(|x| ValueTypes::to_display_string(Some(&x)))
        }),
        BindingPriority::LocalValue,
    );
}

impl FuncDataTemplate {
    /// The default data template used in the case where no matching data
    /// template is found.
    pub fn default_template() -> Rc<FuncDataTemplate> {
        DEFAULT.with(Rc::clone)
    }

    /// The implementation of [`default_template`](Self::default_template)
    /// for data that displays an access key.
    pub fn access() -> Rc<FuncDataTemplate> {
        ACCESS.with(Rc::clone)
    }

    /// Creates a data template.
    ///
    /// `match_` determines whether the data template matches the specified
    /// data and `build` returns a control when supplied with data matching
    /// the template and the name scope of the built content.
    pub fn new(
        match_: impl Fn(Option<&BoxedValue>) -> bool + 'static,
        build: impl Fn(&Option<BoxedValue>, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        supports_recycling: bool,
    ) -> Rc<Self> {
        Rc::new(Self { base: FuncTemplateWithParam::new(build), match_: Box::new(match_), supports_recycling })
    }

    /// Creates a data template that matches data of type `T`.
    pub fn for_type<T: 'static>(
        build: impl Fn(&T, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        supports_recycling: bool,
    ) -> Rc<Self> {
        Self::new(|data| Self::with_cast::<T, _>(data, |_| ()).is_some(), Self::cast_build(build), supports_recycling)
    }

    /// Creates a data template that matches data of type `T` accepted by
    /// `match_`.
    pub fn for_type_with_match<T: 'static>(
        match_: impl Fn(&T) -> bool + 'static,
        build: impl Fn(&T, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        supports_recycling: bool,
    ) -> Rc<Self> {
        Self::new(
            move |data| Self::with_cast::<T, _>(data, &match_).unwrap_or(false),
            Self::cast_build(build),
            supports_recycling,
        )
    }

    // Deviation (DEVIATIONS.md, Templates): upstream `TypeUtilities.CanCast<T>` also accepts null
    // for a reference type `T` and a non-null `T` for `T?`; here null is a `T` only for a
    // registered nullable form, and a value is a `T` only when it holds exactly `T`.
    /// Runs `f` with the data as a `T`, if it is one. Null data is a `T`
    /// when `T` accepts null (a registered nullable form such as
    /// `Option<i32>`): `f` then receives the null of `T`.
    fn with_cast<T: 'static, R>(data: Option<&BoxedValue>, f: impl FnOnce(&T) -> R) -> Option<R> {
        let null;
        let data = match data {
            Some(data) => data,
            None => {
                null = ValueTypes::null_value(ValueType::of::<T>())?;
                &null
            }
        };
        let value: &dyn AnyValue = &**data;
        value.downcast_ref::<T>().map(f)
    }

    fn cast_build<T: 'static>(
        build: impl Fn(&T, &NameScopeRef) -> Option<Ref<Control>> + 'static,
    ) -> impl Fn(&Option<BoxedValue>, &NameScopeRef) -> Option<Ref<Control>> + 'static {
        move |data, scope| match Self::with_cast::<T, _>(data.as_ref(), |data| build(data, scope)) {
            Some(result) => result,
            None => panic!("The data passed to the data template is not of type {}.", std::any::type_name::<T>()),
        }
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for FuncDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.base.build(param)
    }
}

impl IDataTemplate for FuncDataTemplate {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        (self.match_)(data)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for FuncDataTemplate {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        match existing {
            Some(existing) if self.supports_recycling => Some(existing),
            _ => self.base.build(&data.cloned()),
        }
    }
}
