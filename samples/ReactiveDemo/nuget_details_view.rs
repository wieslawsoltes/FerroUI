//! Port of `NugetDetailsView.xaml.cs`: the class of the document `NugetDetailsView.xaml`.
//!
//! Second level derived class off ReactiveUserControl which contains the ViewModel property. In
//! our MainWindow when we register the ListBox with the collection of NugetDetailsViewModels if no
//! ItemTemplate has been declared it will search for a class derived off
//! IViewFor<NugetDetailsViewModel> and show that for the item.

use crate::code_behind_view::{dispose_with, CodeBehindView};
use crate::markup::xaml_class;
use crate::NugetDetailsViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IImage;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::documents::Run;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, HyperlinkButton, Image};
use ferroui_reactiveui::{ActivationDisposables, BindCommandMixins, PropertyBindingMixins, SettableProperty, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, IReactiveViewClass, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::prop;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct NugetDetailsView {
    base: ReactiveUserControlBase,
    /// The control as the view ReactiveUI binds.
    view: OnceCell<Rc<NugetDetailsViewFor>>,
    /// The icon the image shows or downloads.
    icon_source: RefCell<Option<RemoteBitmap>>,
}

ferro_class!(NugetDetailsView: ReactiveUserControlBase);
ferro_impl_classes!(
    NugetDetailsView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(NugetDetailsView { new: NugetDetailsView::new });
xaml_class!(NugetDetailsView, "/NugetDetailsView.xaml");

/// The details of a package as the view ReactiveUI binds and locates (`IViewFor<NugetDetailsViewModel>`
/// of the control upstream).
pub type NugetDetailsViewFor = CodeBehindView<NugetDetailsView, NugetDetailsViewModel>;

impl NugetDetailsView {
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct(), view: OnceCell::new(), icon_source: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The control takes only a NugetDetailsViewModel (`ReactiveUserControl<NugetDetailsViewModel>`).
        let _ = ReactiveUserControl::<NugetDetailsViewModel>::from_control(this.clone());
        let view = NugetDetailsViewFor::new(&this, <ReactiveUserControlBase as IReactiveViewClass>::view_model_property());
        let _ = this.view.set(view.clone());

        let weak = Rc::downgrade(&view);
        let _ = view.when_activated_disposables(move |disposable_registration: &Rc<ActivationDisposables>| {
            let Some(view) = weak.upgrade() else {
                return;
            };

            // Our 4th parameter we convert from Url into a bitmap. This is an easy way of doing
            // value conversion using ReactiveUI binding.
            dispose_with(
                view.one_way_bind_select(
                    view.view_model(),
                    prop!(NugetDetailsViewModel => icon_url),
                    NugetDetailsViewFor::icon_image_source(),
                    |url: String| Some(RemoteBitmap::new(url)),
                ),
                disposable_registration,
            );

            dispose_with(
                view.one_way_bind(view.view_model(), prop!(NugetDetailsViewModel => title), NugetDetailsViewFor::title_run_text()),
                disposable_registration,
            );

            dispose_with(
                view.one_way_bind(view.view_model(), prop!(NugetDetailsViewModel => description), NugetDetailsViewFor::description_run_text()),
                disposable_registration,
            );

            disposable_registration.add(view.bind_command(prop!(NugetDetailsViewModel => open_page), prop!(NugetDetailsViewFor => open_button), None));
        });
        this
    }

    /// The control as the view ReactiveUI binds and locates.
    ///
    /// # Panics
    /// Panics for a control that was not made by [`new`](Self::new).
    pub fn view(&self) -> Rc<NugetDetailsViewFor> {
        self.view.get().expect("the control is made by NugetDetailsView::new").clone()
    }

    /// The view model of the control (`ViewModel`).
    pub fn view_model(&self) -> Option<Rc<NugetDetailsViewModel>> {
        self.view.get().and_then(|view| view.view_model())
    }

    /// `iconImage`.
    pub fn icon_image(&self) -> Ref<Image> {
        self.get_control::<Image>("iconImage")
    }

    /// `titleRun`.
    pub fn title_run(&self) -> Ref<Run> {
        self.get_control::<Run>("titleRun")
    }

    /// `descriptionRun`.
    pub fn description_run(&self) -> Ref<Run> {
        self.get_control::<Run>("descriptionRun")
    }

    /// `openButton`.
    pub fn open_button(&self) -> Ref<HyperlinkButton> {
        self.get_control::<HyperlinkButton>("openButton")
    }

    /// The icon the image shows (the `Source` the reference binds).
    pub fn icon_source(&self) -> Option<RemoteBitmap> {
        self.icon_source.borrow().clone()
    }

    /// Shows the icon `source`: the image is empty until the bitmap arrives, and a bitmap that
    /// arrives after another icon was asked for is dropped.
    pub fn set_icon_source(&self, source: Option<RemoteBitmap>) {
        *self.icon_source.borrow_mut() = source.clone();
        self.icon_image().set_source(None);
        let Some(source) = source else {
            return;
        };
        let weak = self.to_ref().downgrade();
        let expected = source.clone();
        source.load(move |bitmap: Rc<dyn IImage>| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if this.icon_source.borrow().as_ref() == Some(&expected) {
                this.icon_image().set_source(Some(bitmap));
            }
        });
    }
}

/// The named elements of the control as the properties of the view that the bindings write
/// (`view => view.titleRun.Text` upstream).
impl NugetDetailsViewFor {
    /// `view.iconImage`'s source, as the icon the image shows.
    pub fn icon_source(&self) -> Option<RemoteBitmap> {
        self.control().icon_source()
    }

    /// Shows the icon `source` in `view.iconImage`.
    pub fn set_icon_source(&self, source: Option<RemoteBitmap>) {
        self.control().set_icon_source(source);
    }

    /// `view.titleRun`.
    pub fn title_run(&self) -> Ref<Run> {
        self.control().title_run()
    }

    /// `view.descriptionRun`.
    pub fn description_run(&self) -> Ref<Run> {
        self.control().description_run()
    }

    /// `view.openButton`, as the command binding takes a control.
    pub fn open_button(&self) -> Rc<Ref<HyperlinkButton>> {
        Rc::new(self.control().open_button())
    }

    /// `view => view.iconImage.Source`.
    ///
    /// Deviation (DEVIATIONS.md, samples): the source of an `Image` of FerroUI is a decoded image,
    /// not a `BitmapImage` that downloads itself; the property the binding writes is the icon to
    /// show ([`RemoteBitmap`]), which the control downloads and decodes.
    pub fn icon_image_source() -> SettableProperty<Rc<NugetDetailsViewFor>, Option<RemoteBitmap>> {
        SettableProperty::new(prop!(NugetDetailsViewFor => icon_source), |view: &Rc<NugetDetailsViewFor>, source: Option<RemoteBitmap>| {
            view.set_icon_source(source)
        })
    }

    /// `view => view.titleRun.Text`.
    pub fn title_run_text() -> SettableProperty<Rc<NugetDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(NugetDetailsViewFor => title_run).then(ferro_prop!(Run => text)),
            |view: &Rc<NugetDetailsViewFor>, text: Option<String>| view.title_run().set_text(text.as_deref()),
        )
    }

    /// `view => view.descriptionRun.Text`.
    pub fn description_run_text() -> SettableProperty<Rc<NugetDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(NugetDetailsViewFor => description_run).then(ferro_prop!(Run => text)),
            |view: &Rc<NugetDetailsViewFor>, text: Option<String>| view.description_run().set_text(text.as_deref()),
        )
    }
}

/// An image at an address, downloaded when it is shown (`new BitmapImage(url)` of the reference,
/// which downloads itself).
///
/// With the feature `online` the bytes are downloaded on a worker thread and decoded on the
/// thread that asked; without it (and in the tests) nothing is downloaded and the image stays
/// empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteBitmap {
    url: String,
}

impl RemoteBitmap {
    /// The image at `url`.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// The address of the image.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Downloads and decodes the image, and hands it to `loaded` on this thread; an image that
    /// cannot be downloaded or decoded is not handed over (`BitmapImage` reports its failure
    /// through an event the reference does not handle).
    pub fn load(&self, loaded: impl FnOnce(Rc<dyn IImage>) + 'static) {
        load_image(&self.url, loaded);
    }
}

/// Downloads the image at `url` on a worker thread, decodes it on this thread and hands it to
/// `loaded`.
#[cfg(all(feature = "online", not(target_arch = "wasm32"), not(test)))]
fn load_image(url: &str, loaded: impl FnOnce(Rc<dyn IImage>) + 'static) {
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_system::threading::tasks::{self, Task};

    let url = url.to_owned();
    let download: Task<Vec<u8>> = tasks::run(move || crate::nuget_search::download(&url));
    let _ = download.continue_with(move |download: &Task<Vec<u8>>| {
        let Ok(bytes) = download.result() else {
            return;
        };
        let Ok(bitmap) = Bitmap::from_stream(&mut std::io::Cursor::new(bytes)) else {
            return;
        };
        loaded(Rc::new(bitmap) as Rc<dyn IImage>);
    });
}

/// Without the feature `online` (and in the tests) no image is downloaded.
#[cfg(not(all(feature = "online", not(target_arch = "wasm32"), not(test))))]
fn load_image(_url: &str, _loaded: impl FnOnce(Rc<dyn IImage>) + 'static) {}
