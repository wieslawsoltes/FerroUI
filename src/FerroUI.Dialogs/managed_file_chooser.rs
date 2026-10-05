use crate::internal::{ManagedFileChooserItemType, ManagedFileChooserItemViewModel, ManagedFileChooserViewModel};
use ferroui_base::input::{InputElement, InputElementImpl, PointerPressedEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref,
    StyledElement, StyledElementImpl, StyledElementImplExt, VisualImpl,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{Control, ControlImpl, ListBox};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The managed file chooser: the quick links, the entries of the current
/// folder and the controls of the file name, the filters and the buttons,
/// over a [`ManagedFileChooserViewModel`] as its data context.
#[repr(C)]
pub struct ManagedFileChooser {
    base: TemplatedControl,
    quick_links_root: RefCell<Option<Ref<Control>>>,
    files_view: RefCell<Option<Ref<ListBox>>>,
}

ferro_class!(ManagedFileChooser: TemplatedControl);
ferro_impl_classes!(ManagedFileChooser: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(ManagedFileChooser {
    new: ManagedFileChooser::new,
    markup: {
        attributes: [
            TemplatePart("PART_QuickLinks", type(Ref<Control>), IsRequired = true),
            TemplatePart("PART_Files", type(Ref<ListBox>), IsRequired = true),
        ],
    },
});

/// Whether `logical` is a (strict) logical ancestor of `target`
/// (`LogicalExtensions.IsLogicalAncestorOf`).
fn is_logical_ancestor_of(logical: &StyledElement, target: Option<&StyledElement>) -> bool {
    let mut current = target.and_then(|target| target.parent());

    while let Some(parent) = current {
        if std::ptr::eq::<StyledElement>(&*parent, logical) {
            return true;
        }
        current = parent.parent();
    }

    false
}

impl FerroObjectImpl for ManagedFileChooser {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e: &PointerPressedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_pressed(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }
}

impl StyledElementImpl for ManagedFileChooser {
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        let Some(model) = this.model() else {
            return;
        };

        let Some(preselected) = model.selected_items().try_get(0) else {
            return;
        };

        // Let everything to settle down and scroll to selected item
        let weak = this.to_ref().downgrade();
        DispatcherTimer::run_once(
            move || {
                let Some(this) = weak.upgrade() else { return };

                if Some(&preselected) != model.selected_items().try_get(0).as_ref() {
                    return;
                }

                // Workaround for ListBox bug, scroll to the previous file
                let index_of_preselected =
                    model.items().to_vec().iter().position(|item| Some(item.clone()) == from_markup_value(&preselected));

                let files_view = this.files_view.borrow().clone();
                if let (Some(files_view), Some(index_of_preselected)) = (files_view, index_of_preselected) {
                    if index_of_preselected > 1 {
                        files_view.scroll_into_view(index_of_preselected as i32 - 1);
                    }
                }
            },
            Duration::from_millis(100),
            DispatcherPriority::DEFAULT,
        );
    }
}

impl TemplatedControlImpl for ManagedFileChooser {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        *this.quick_links_root.borrow_mut() = Some(e.name_scope().get_as::<Control>("PART_QuickLinks"));
        *this.files_view.borrow_mut() = Some(e.name_scope().get_as::<ListBox>("PART_Files"));
    }
}

impl ManagedFileChooser {
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), quick_links_root: RefCell::new(None), files_view: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn model(&self) -> Option<Rc<ManagedFileChooserViewModel>> {
        from_markup_value::<Rc<ManagedFileChooserViewModel>>(&self.data_context())
    }

    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let source = e.source();
        let source_element = source.as_ref().and_then(|source| source.cast::<StyledElement>());
        let Some(model) = source_element
            .as_ref()
            .and_then(|element| from_markup_value::<Rc<ManagedFileChooserItemViewModel>>(&element.data_context()))
        else {
            return;
        };

        let quick_links_root = self.quick_links_root.borrow().clone();
        if let Some(quick_links_root) = quick_links_root {
            let source_control = source.as_ref().and_then(|source| source.cast::<Control>());
            let is_quick_link = is_logical_ancestor_of(&quick_links_root, source_control.as_deref().map(|c| -> &StyledElement { c }));

            if e.click_count() == 2 || is_quick_link {
                if model.item_type() == ManagedFileChooserItemType::File {
                    if let Some(chooser) = self.model() {
                        chooser.select_single_file(&model);
                    }
                } else if let Some(chooser) = self.model() {
                    chooser.navigate(model.path().as_deref(), None);
                }

                e.set_handled(true);
            }
        }
    }
}
