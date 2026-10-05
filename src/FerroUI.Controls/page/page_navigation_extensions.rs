use super::{INavigation, Page};
use ferroui_base::animation::IPageTransition;
use ferroui_base::threading::DispatcherTask;
use ferroui_base::{BoxedValue, ObjectType, Ref, Upcast};
use std::rc::Rc;

/// Navigation members that create the page to navigate to from its type.
pub trait PageNavigationExtensions {
    /// Pushes a new page of class `T`.
    fn push_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()>;

    /// Replaces the current top page with a new page of class `T`.
    fn replace_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()>;

    /// Pushes a new page of class `T` as a modal.
    fn push_modal_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()>;
}

/// C# `new T()` for a page class.
fn create_page<T: ObjectType + Upcast<Page>>() -> Ref<Page> {
    let Some(constructor) = T::TYPE.default_constructor() else {
        panic!("The page class {} has no parameterless constructor.", T::TYPE.name());
    };
    match constructor().cast::<Page>() {
        Some(page) => page,
        None => panic!("The class {} is not a page.", T::TYPE.name()),
    }
}

impl<N: INavigation + ?Sized> PageNavigationExtensions for N {
    fn push_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        let page = create_page::<T>();
        self.push_async_with_parameter(page, transition, parameter)
    }

    fn replace_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        let page = create_page::<T>();
        self.replace_async_with_parameter(page, transition, parameter)
    }

    fn push_modal_async_of<T: ObjectType + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        let page = create_page::<T>();
        self.push_modal_async_with_parameter(page, transition, parameter)
    }
}
