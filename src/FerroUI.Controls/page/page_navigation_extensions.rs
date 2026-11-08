use super::{INavigation, Page};
use ferroui_base::animation::IPageTransition;
#[allow(deprecated)]
use ferroui_base::threading::DispatcherTask;
use ferroui_system::task::JoinHandle;
use ferroui_base::{BoxedValue, DefaultConstructible, Ref, Upcast};
use std::rc::Rc;

/// Navigation members that create the page to navigate to from its type.
pub trait PageNavigationExtensions {
    /// Pushes a new page of class `T`, created with its default constructor (`new` of
    /// `ferro_class_info!`).
    fn push_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()>;

    /// Replaces the current top page with a new page of class `T`.
    fn replace_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()>;

    /// Pushes a new page of class `T` as a modal.
    fn push_modal_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()>;

    /// The deprecated form of [`push_async_of`](Self::push_async_of): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_async_of`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_async_of_task<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.push_async_of::<T>(transition, parameter).into()
    }

    /// The deprecated form of [`replace_async_of`](Self::replace_async_of): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `replace_async_of`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn replace_async_of_task<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.replace_async_of::<T>(transition, parameter).into()
    }

    /// The deprecated form of [`push_modal_async_of`](Self::push_modal_async_of): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_modal_async_of`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_modal_async_of_task<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.push_modal_async_of::<T>(transition, parameter).into()
    }
}

/// C# `new T()` for a page class: the constructor is reached through the class, so the compiler
/// checks that it has one and only the pages an application navigates to are linked (#612, B3).
fn create_page<T: DefaultConstructible + Upcast<Page>>() -> Ref<Page> {
    T::new_instance().upcast::<Page>()
}

impl<N: INavigation + ?Sized> PageNavigationExtensions for N {
    fn push_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        let page = create_page::<T>();
        self.push_async_with_parameter(page, transition, parameter)
    }

    fn replace_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        let page = create_page::<T>();
        self.replace_async_with_parameter(page, transition, parameter)
    }

    fn push_modal_async_of<T: DefaultConstructible + Upcast<Page>>(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        let page = create_page::<T>();
        self.push_modal_async_with_parameter(page, transition, parameter)
    }
}
