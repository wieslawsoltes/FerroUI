use crate::items_source::ItemsSource;
use crate::metadata::TemplatePartAttribute;
use crate::native_menu_bar_presenter::NativeMenuBarPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{ControlImpl, ItemsControl, MenuBase, NativeMenu, TopLevel};
use ferroui_base::data::{BindingPriority, TemplateBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectExtensions, FerroObjectImpl, Ref,
    StaticType, StyledElementImpl, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Shows the native menu of the top-level it is in when the platform does
/// not export the menu.
#[repr(C)]
pub struct NativeMenuBar {
    base: TemplatedControl,
    menu: RefCell<Option<Ref<MenuBase>>>,
    subscriptions: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(NativeMenuBar: TemplatedControl);
ferro_class_info!(NativeMenuBar { new: NativeMenuBar::new });

ferro_impl_classes!(
    NativeMenuBar: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl ControlImpl for NativeMenuBar {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NativeMenuBarAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for NativeMenuBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let menu = e
            .name_scope()
            .find_as::<MenuBase>("PART_NativeMenuPresenter")
            .or_else(|| this.find_descendant_of_type::<MenuBase>(false))
            .unwrap_or_else(|| panic!("NativeMenuBar requires a MenuBase#PART_NativeMenuPresenter template part."));
        *this.menu.borrow_mut() = Some(menu.clone());

        if let Some(top_level) = TopLevel::get_top_level(Some(this)) {
            this.subscribe_to_toplevel(&top_level, &menu);
        }
    }
}

impl VisualImpl for NativeMenuBar {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let Some(menu) = this.menu.borrow().clone() else { return };

        if let Some(top_level) = TopLevel::get_top_level(Some(this)) {
            this.subscribe_to_toplevel(&top_level, &menu);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let subscriptions = this.subscriptions.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }
    }
}

impl NativeMenuBar {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_NativeMenuPresenter", <MenuBase as StaticType>::TYPE)];

    fn static_constructor() {
        // TODO12 Ideally we should make NativeMenuBar inherit MenuBase directly, but it would be a breaking change for 11.x.
        // Changing default template while keeping old StyleKeyOverride => Menu isn't a breaking change.
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
            let presenter = NativeMenuBarPresenter::new();
            presenter.set_name(Some("PART_NativeMenuPresenter".to_string()));
            presenter.bind_binding(
                TemplatedControl::background_property().as_property(),
                &TemplateBinding::new(TemplatedControl::background_property().as_property()),
            );
            presenter.bind_binding(
                TemplatedControl::border_brush_property().as_property(),
                &TemplateBinding::new(TemplatedControl::border_brush_property().as_property()),
            );
            presenter.register_in_name_scope(&**ns).upcast()
        });
        TemplatedControl::template_property().override_default_value::<NativeMenuBar>(Some(template));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), menu: RefCell::new(None), subscriptions: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn subscribe_to_toplevel(&self, top_level: &Ref<TopLevel>, menu: &Ref<MenuBase>) {
        let old = self.subscriptions.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }

        let subscriptions: Rc<dyn IDisposable> = Rc::new(CompositeDisposable::from_disposables([
            menu.bind(
                Visual::is_visible_property(),
                top_level
                    .get_binding_observable(NativeMenu::is_native_menu_exported_property())
                    .select(|v| !v.get_value_or_default(false)),
                BindingPriority::LocalValue,
            ),
            menu.bind(
                ItemsControl::items_source_property(),
                top_level
                    .get_binding_observable(NativeMenu::menu_property())
                    .select(|v| v.get_value_or_default(None).map(|menu| ItemsSource::from(Rc::new(menu.items())))),
                BindingPriority::LocalValue,
            ),
        ]));
        *self.subscriptions.borrow_mut() = Some(subscriptions);
    }
}
