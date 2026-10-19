//! Port of `Views/CommandLabView.xaml.cs`: the class of the document `CommandLabView.xaml`, the
//! routed command lab view.

use super::bindable_view::{element_control, element_property, BindableView};
use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::CommandLabViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, TextBlock, TextBox};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactiveui::{settable_prop, BindCommandMixins, IInteractionContext, IViewForOf, PropertyBindingMixins, ViewForMixins};
use ferroui_reactiveui_controls::{ferro_prop, ReactiveUserControl, ReactiveUserControlBase};
use ferroui_system::{prop, IDisposable};
use std::rc::Rc;

/// The view the bindings of the command lab take.
type View = BindableView<ReactiveUserControlBase, CommandLabViewModel>;

/// The routed command lab view (`CommandLabView : ReactiveUserControl<CommandLabViewModel>`).
#[repr(C)]
pub struct CommandLabView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(CommandLabView);
ferro_class_info!(CommandLabView { new: CommandLabView::new });
xaml_class!(CommandLabView, "/Views/CommandLabView.xaml");

impl CommandLabView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`CommandLabView()`): the bindings and the handler of the interaction are
    /// created on each activation and disposed on deactivation.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        let control: Ref<ReactiveUserControlBase> = this.clone().upcast();
        let view = View::new(&control);
        let bound = view.clone();
        let _ = view.when_activated_disposables(move |disposables| Self::bind_view(&bound, disposables));
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<CommandLabViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<CommandLabViewModel> {
        ReactiveUserControl::<CommandLabViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<CommandLabViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<CommandLabViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<CommandLabViewModel>> {
        Self::typed(this).view_model()
    }

    /// The input of the work item (the element `WorkItemTextBox`).
    pub fn work_item_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("WorkItemTextBox")
    }

    /// The button that runs the work (the element `RunWorkButton`).
    pub fn run_work_button(&self) -> Ref<Button> {
        self.get_control::<Button>("RunWorkButton")
    }

    /// The button that fails (the element `FailWorkButton`).
    pub fn fail_work_button(&self) -> Ref<Button> {
        self.get_control::<Button>("FailWorkButton")
    }

    /// The text of the last result (the element `LastResultText`).
    pub fn last_result_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastResultText")
    }

    /// The text of the last error (the element `LastErrorText`).
    pub fn last_error_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastErrorText")
    }

    /// The text of the interaction status (the element `InteractionStatusText`).
    pub fn interaction_status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("InteractionStatusText")
    }

    /// Creates the bindings owned by this activation (`BindView`).
    fn bind_view(view: &Rc<View>, disposables: &Rc<CompositeDisposable>) {
        let Some(this) = view.control().and_then(|control| control.cast::<CommandLabView>()) else {
            return;
        };
        let add = |binding: Option<Rc<dyn IDisposable>>| {
            if let Some(binding) = binding {
                disposables.add(binding);
            }
        };
        let text_block = |name: &str, element: &Ref<TextBlock>| {
            element_property(name, element, ferro_prop!(TextBlock => text), |text: &Ref<TextBlock>, value: Option<String>| {
                text.set_value(TextBlock::text_property(), value)
            })
        };
        let none = None::<Rc<CommandLabViewModel>>;

        let input = this.work_item_text_box();
        add(view
            .bind_select(
                none.clone(),
                settable_prop!(CommandLabViewModel => work_item_text),
                element_property("WorkItemTextBox", &input, ferro_prop!(TextBox => text), |text: &Ref<TextBox>, value: Option<String>| {
                    text.set_value(TextBox::text_property(), value)
                }),
                Some,
                |text: Option<String>| text.unwrap_or_default(),
            )
            .map(|binding| binding as Rc<dyn IDisposable>));

        add(view
            .one_way_bind_select(none.clone(), prop!(CommandLabViewModel => last_result), text_block("LastResultText", &this.last_result_text()), Some)
            .map(|binding| binding as Rc<dyn IDisposable>));

        add(view
            .one_way_bind_select(none.clone(), prop!(CommandLabViewModel => last_error), text_block("LastErrorText", &this.last_error_text()), Some)
            .map(|binding| binding as Rc<dyn IDisposable>));

        add(view
            .one_way_bind_select(
                none,
                prop!(CommandLabViewModel => interaction_status),
                text_block("InteractionStatusText", &this.interaction_status_text()),
                Some,
            )
            .map(|binding| binding as Rc<dyn IDisposable>));

        disposables.add(view.bind_command(prop!(CommandLabViewModel => run_work), element_control("RunWorkButton", &this.run_work_button()), None));

        disposables.add(view.bind_command(prop!(CommandLabViewModel => fail_work), element_control("FailWorkButton", &this.fail_work_button()), None));

        let Some(view_model) = view.view_model() else {
            return;
        };

        let acknowledging = Rc::downgrade(&view_model);
        disposables.add(view_model.report_error().register_handler(move |context| {
            if let Some(view_model) = acknowledging.upgrade() {
                view_model.acknowledge_error(context.input());
            }
            context.set_output(());
        }));
    }
}
