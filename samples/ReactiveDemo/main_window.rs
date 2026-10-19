//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.
//!
//! MainWindow class derives off ReactiveWindow which implements the IViewFor<TViewModel>
//! interface using a dependency property. We need this to use WhenActivated extension method
//! that helps us handling View and ViewModel activation and deactivation.

use crate::code_behind_view::{dispose_with, CodeBehindView};
use crate::markup::xaml_class;
use crate::{AppViewModel, LauncherSource, SearchResults};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::platform::storage::{ILauncher, NoopLauncher};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, ItemsSource, ListBox, TextBox, TopLevelImpl, WindowBaseImpl, WindowImpl};
use ferroui_reactiveui::{settable_prop, ActivationDisposables, PropertyBindingMixins, SettableProperty, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, IReactiveViewClass, ReactiveWindow, ReactiveWindowBase};
use ferroui_system::prop;
use std::cell::OnceCell;
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: ReactiveWindowBase,
    /// The window as the view ReactiveUI binds.
    view: OnceCell<Rc<MainWindowView>>,
}

ferro_class!(MainWindow: ReactiveWindowBase);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow { new: MainWindow::new });
xaml_class!(MainWindow, "/MainWindow.xaml");

/// The main window as the view ReactiveUI binds (`IViewFor<AppViewModel>` of the window
/// upstream).
pub type MainWindowView = CodeBehindView<MainWindow, AppViewModel>;

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: ReactiveWindowBase::construct(), view: OnceCell::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // ViewModel = new AppViewModel(): the window takes only an AppViewModel
        // (`ReactiveWindow<AppViewModel>`).
        let window = ReactiveWindow::<AppViewModel>::from_control(this.clone());
        window.set_view_model(Some(AppViewModel::new(launcher_of(&this))));

        let view = MainWindowView::new(&this, <ReactiveWindowBase as IReactiveViewClass>::view_model_property());
        let _ = this.view.set(view.clone());

        // We create our bindings here. These are the code behind bindings which allow type
        // safety. The bindings will only become active when the Window is being shown. We register
        // our subscription in our disposableRegistration, this will cause the binding
        // subscription to become inactive when the Window is closed. The disposableRegistration
        // is a CompositeDisposable which is a container of other Disposables. We use the
        // dispose_with() function which simply adds the subscription disposable to the
        // CompositeDisposable.
        let weak = Rc::downgrade(&view);
        let _ = view.when_activated_disposables(move |disposable_registration: &Rc<ActivationDisposables>| {
            let Some(view) = weak.upgrade() else {
                return;
            };

            // Deviation (DEVIATIONS.md, samples): WPF converts the boolean to a `Visibility`
            // through a global converter; a control of FerroUI is shown or hidden by the boolean
            // `IsVisible`, which needs no converter.
            dispose_with(
                view.one_way_bind(view.view_model(), prop!(AppViewModel => is_available), MainWindowView::search_results_list_box_is_visible()),
                disposable_registration,
            );

            // Deviation (DEVIATIONS.md, samples): the `ItemsSource` of a list of FerroUI is a list
            // of boxed items, not any `IEnumerable`; the results are converted to one.
            dispose_with(
                view.one_way_bind_select(
                    view.view_model(),
                    prop!(AppViewModel => search_results),
                    MainWindowView::search_results_list_box_items_source(),
                    |search_results: SearchResults| {
                        search_results.map(|search_results| {
                            ItemsSource::from_items(search_results.iter().map(|view_model| Some(view_model.clone() as BoxedValue)))
                        })
                    },
                ),
                disposable_registration,
            );

            dispose_with(
                view.bind(view.view_model(), settable_prop!(AppViewModel => search_term), MainWindowView::search_text_box_text()),
                disposable_registration,
            );
        });
        this
    }

    /// The window as the view ReactiveUI binds.
    ///
    /// # Panics
    /// Panics for a window that was not made by [`new`](Self::new).
    pub fn view(&self) -> Rc<MainWindowView> {
        self.view.get().expect("the window is made by MainWindow::new").clone()
    }

    /// The view model of the window (`ViewModel`).
    pub fn view_model(&self) -> Option<Rc<AppViewModel>> {
        self.view.get().and_then(|view| view.view_model())
    }

    /// `searchTextBox`.
    pub fn search_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("searchTextBox")
    }

    /// `searchResultsListBox`.
    pub fn search_results_list_box(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("searchResultsListBox")
    }
}

/// The launcher of `window`, while it lives (`Process.Start` of the reference opens pages without
/// a window).
fn launcher_of(window: &Ref<MainWindow>) -> LauncherSource {
    let window = window.downgrade();
    Rc::new(move || -> Rc<dyn ILauncher> {
        match window.upgrade() {
            Some(window) => window.launcher(),
            None => Rc::new(NoopLauncher),
        }
    })
}

/// The named elements of the window as the properties of the view that the bindings write
/// (`view => view.searchTextBox.Text` upstream).
impl MainWindowView {
    /// `view.searchTextBox`.
    pub fn search_text_box(&self) -> Ref<TextBox> {
        self.control().search_text_box()
    }

    /// `view.searchResultsListBox`.
    pub fn search_results_list_box(&self) -> Ref<ListBox> {
        self.control().search_results_list_box()
    }

    /// `view => view.searchTextBox.Text`.
    pub fn search_text_box_text() -> SettableProperty<Rc<MainWindowView>, Option<String>> {
        SettableProperty::new(
            prop!(MainWindowView => search_text_box).then(ferro_prop!(TextBox => text)),
            |view: &Rc<MainWindowView>, text: Option<String>| view.search_text_box().set_text(text.as_deref()),
        )
    }

    /// `view => view.searchResultsListBox.ItemsSource`.
    pub fn search_results_list_box_items_source() -> SettableProperty<Rc<MainWindowView>, Option<ItemsSource>> {
        SettableProperty::new(
            prop!(MainWindowView => search_results_list_box).then(ferro_prop!(ListBox => items_source)),
            |view: &Rc<MainWindowView>, items: Option<ItemsSource>| view.search_results_list_box().set_items_source(items),
        )
    }

    /// `view => view.searchResultsListBox.Visibility`, as `IsVisible`.
    pub fn search_results_list_box_is_visible() -> SettableProperty<Rc<MainWindowView>, bool> {
        SettableProperty::new(
            prop!(MainWindowView => search_results_list_box).then(ferro_prop!(ListBox => is_visible)),
            |view: &Rc<MainWindowView>, visible: bool| view.search_results_list_box().set_is_visible(visible),
        )
    }
}
