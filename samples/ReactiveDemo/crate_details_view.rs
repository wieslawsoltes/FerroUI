//! Port of `NugetDetailsView.xaml.cs`: the class of the document `CrateDetailsView.xaml`.
//!
//! Second level derived class off ReactiveUserControl which contains the ViewModel property. In
//! our MainWindow when we register the ListBox with the collection of CrateDetailsViewModels if no
//! ItemTemplate has been declared it will search for a class derived off
//! IViewFor<CrateDetailsViewModel> and show that for the item.

use crate::code_behind_view::{dispose_with, CodeBehindView};
use crate::crate_details_view_model::format_thousands;
use crate::markup::xaml_class;
use crate::CrateDetailsViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::documents::Run;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, HyperlinkButton, TextBlock};
use ferroui_reactiveui::{ActivationDisposables, BindCommandMixins, PropertyBindingMixins, SettableProperty, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, IReactiveViewClass, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::prop;
use std::cell::OnceCell;
use std::rc::Rc;

#[repr(C)]
pub struct CrateDetailsView {
    base: ReactiveUserControlBase,
    /// The control as the view ReactiveUI binds.
    view: OnceCell<Rc<CrateDetailsViewFor>>,
}

ferro_class!(CrateDetailsView: ReactiveUserControlBase);
ferro_impl_classes!(
    CrateDetailsView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CrateDetailsView { new: CrateDetailsView::new });
xaml_class!(CrateDetailsView, "/CrateDetailsView.xaml");

/// The details of a crate as the view ReactiveUI binds and locates
/// (`IViewFor<CrateDetailsViewModel>` of the control upstream).
pub type CrateDetailsViewFor = CodeBehindView<CrateDetailsView, CrateDetailsViewModel>;

impl CrateDetailsView {
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct(), view: OnceCell::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The control takes only a CrateDetailsViewModel (`ReactiveUserControl<CrateDetailsViewModel>`).
        let _ = ReactiveUserControl::<CrateDetailsViewModel>::from_control(this.clone());
        let view = CrateDetailsViewFor::new(&this, <ReactiveUserControlBase as IReactiveViewClass>::view_model_property());
        let _ = this.view.set(view.clone());

        let weak = Rc::downgrade(&view);
        let _ = view.when_activated_disposables(move |disposable_registration: &Rc<ActivationDisposables>| {
            let Some(view) = weak.upgrade() else {
                return;
            };

            // Deviation (DEVIATIONS.md, samples): the reference binds the icon of the package to
            // an image; crates.io has no icons, so the badge shows the first letter of the name
            // of the crate.
            dispose_with(
                view.one_way_bind(view.view_model(), prop!(CrateDetailsViewModel => initial), CrateDetailsViewFor::badge_text_block_text()),
                disposable_registration,
            );

            dispose_with(
                view.one_way_bind(view.view_model(), prop!(CrateDetailsViewModel => title), CrateDetailsViewFor::title_run_text()),
                disposable_registration,
            );

            // The version follows the name on the same line, separated by a space.
            dispose_with(
                view.one_way_bind_select(
                    view.view_model(),
                    prop!(CrateDetailsViewModel => version),
                    CrateDetailsViewFor::version_run_text(),
                    |version: String| Some(format!(" {version}")),
                ),
                disposable_registration,
            );

            dispose_with(
                view.one_way_bind(view.view_model(), prop!(CrateDetailsViewModel => description), CrateDetailsViewFor::description_run_text()),
                disposable_registration,
            );

            // We convert the number of downloads into text. This is an easy way of doing value
            // conversion using ReactiveUI binding.
            dispose_with(
                view.one_way_bind_select(
                    view.view_model(),
                    prop!(CrateDetailsViewModel => downloads),
                    CrateDetailsViewFor::downloads_text_block_text(),
                    |downloads: u64| Some(format!("{} downloads", format_thousands(downloads))),
                ),
                disposable_registration,
            );

            disposable_registration.add(view.bind_command(prop!(CrateDetailsViewModel => open_page), prop!(CrateDetailsViewFor => open_button), None));
        });
        this
    }

    /// The control as the view ReactiveUI binds and locates.
    ///
    /// # Panics
    /// Panics for a control that was not made by [`new`](Self::new).
    pub fn view(&self) -> Rc<CrateDetailsViewFor> {
        self.view.get().expect("the control is made by CrateDetailsView::new").clone()
    }

    /// The view model of the control (`ViewModel`).
    pub fn view_model(&self) -> Option<Rc<CrateDetailsViewModel>> {
        self.view.get().and_then(|view| view.view_model())
    }

    /// `badgeTextBlock`: the letter of the badge of the crate.
    pub fn badge_text_block(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("badgeTextBlock")
    }

    /// `titleRun`.
    pub fn title_run(&self) -> Ref<Run> {
        self.get_control::<Run>("titleRun")
    }

    /// `versionRun`.
    pub fn version_run(&self) -> Ref<Run> {
        self.get_control::<Run>("versionRun")
    }

    /// `descriptionRun`.
    pub fn description_run(&self) -> Ref<Run> {
        self.get_control::<Run>("descriptionRun")
    }

    /// `downloadsTextBlock`.
    pub fn downloads_text_block(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("downloadsTextBlock")
    }

    /// `openButton`.
    pub fn open_button(&self) -> Ref<HyperlinkButton> {
        self.get_control::<HyperlinkButton>("openButton")
    }
}

/// The named elements of the control as the properties of the view that the bindings write
/// (`view => view.titleRun.Text` upstream).
impl CrateDetailsViewFor {
    /// `view.badgeTextBlock`.
    pub fn badge_text_block(&self) -> Ref<TextBlock> {
        self.control().badge_text_block()
    }

    /// `view.titleRun`.
    pub fn title_run(&self) -> Ref<Run> {
        self.control().title_run()
    }

    /// `view.versionRun`.
    pub fn version_run(&self) -> Ref<Run> {
        self.control().version_run()
    }

    /// `view.descriptionRun`.
    pub fn description_run(&self) -> Ref<Run> {
        self.control().description_run()
    }

    /// `view.downloadsTextBlock`.
    pub fn downloads_text_block(&self) -> Ref<TextBlock> {
        self.control().downloads_text_block()
    }

    /// `view.openButton`, as the command binding takes a control.
    pub fn open_button(&self) -> Rc<Ref<HyperlinkButton>> {
        Rc::new(self.control().open_button())
    }

    /// `view => view.badgeTextBlock.Text`.
    pub fn badge_text_block_text() -> SettableProperty<Rc<CrateDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(CrateDetailsViewFor => badge_text_block).then(ferro_prop!(TextBlock => text)),
            |view: &Rc<CrateDetailsViewFor>, text: Option<String>| view.badge_text_block().set_text(text.as_deref()),
        )
    }

    /// `view => view.titleRun.Text`.
    pub fn title_run_text() -> SettableProperty<Rc<CrateDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(CrateDetailsViewFor => title_run).then(ferro_prop!(Run => text)),
            |view: &Rc<CrateDetailsViewFor>, text: Option<String>| view.title_run().set_text(text.as_deref()),
        )
    }

    /// `view => view.versionRun.Text`.
    pub fn version_run_text() -> SettableProperty<Rc<CrateDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(CrateDetailsViewFor => version_run).then(ferro_prop!(Run => text)),
            |view: &Rc<CrateDetailsViewFor>, text: Option<String>| view.version_run().set_text(text.as_deref()),
        )
    }

    /// `view => view.descriptionRun.Text`.
    pub fn description_run_text() -> SettableProperty<Rc<CrateDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(CrateDetailsViewFor => description_run).then(ferro_prop!(Run => text)),
            |view: &Rc<CrateDetailsViewFor>, text: Option<String>| view.description_run().set_text(text.as_deref()),
        )
    }

    /// `view => view.downloadsTextBlock.Text`.
    pub fn downloads_text_block_text() -> SettableProperty<Rc<CrateDetailsViewFor>, Option<String>> {
        SettableProperty::new(
            prop!(CrateDetailsViewFor => downloads_text_block).then(ferro_prop!(TextBlock => text)),
            |view: &Rc<CrateDetailsViewFor>, text: Option<String>| view.downloads_text_block().set_text(text.as_deref()),
        )
    }
}
