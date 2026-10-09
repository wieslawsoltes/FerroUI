use crate::{Border, Control, ControlImpl, Controls};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider};
use ferroui_base::media::{DrawingContext, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, ObjectType, Rect, Ref, StyledElement, StyledElementImpl, StyledProperty, Upcast, Visual, VisualImpl, VisualImplExt,
    WeakRef,
};
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

type ChildIndexChangedHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

/// Base class for controls that can contain multiple children.
///
/// Controls can be added to a panel by adding them to its `children`
/// collection. All children are layed out to fill the panel.
#[repr(C)]
pub struct Panel {
    base: Control,
    children: Controls,
    is_items_host: Cell<bool>,
    child_index_changed: Rc<ChildIndexChangedHandlers>,
    child_index_provider: OnceCell<Rc<PanelChildIndexProvider>>,
}

ferro_class! {
    Panel: Control, virtuals PanelImpl: ControlImpl {
        /// Called when the `children` collection changes.
        fn children_changed(this, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>);
        /// Invalidates the measure of the panel after its children changed.
        fn invalidate_measure_on_children_changed(this);
    }
}
ferroui_base::ferro_class_info!(Panel { new: Panel::new });

ferro_impl_classes!(Panel: LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl VisualImpl for Panel {
    /// Renders the visual to a drawing context.
    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(background) = this.background() {
            let render_size = this.bounds().size();
            context.fill_rectangle(&background, Rect::from_size(render_size), 0.0);
        }

        Self::parent_render(this, context);
    }
}

ferroui_base::ferro_overrides! { impl FerroObjectImpl for Panel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.children.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>| {
                if let Some(this) = weak.upgrade() {
                    this.children_changed(e);

                    // The count of the collection changes after the
                    // collection changed notification has been raised.
                    if matches!(
                        e.action,
                        NotifyCollectionChangedAction::Add
                            | NotifyCollectionChangedAction::Remove
                            | NotifyCollectionChangedAction::Reset
                    ) {
                        this.raise_child_index_changed(&ChildIndexChangedEventArgs::total_count_changed());
                    }
                }
            },
        ));
    }
} }

impl StyledElementImpl for Panel {
    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        let provider = this
            .child_index_provider
            .get_or_init(|| Rc::new(PanelChildIndexProvider { owner: this.to_ref().downgrade() }))
            .clone();
        Some(provider)
    }
}

impl PanelImpl for Panel {
    fn children_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        match e.action {
            NotifyCollectionChangedAction::Add => {
                if !this.is_items_host() {
                    this.logical_children().insert_range(
                        e.new_starting_index as usize,
                        e.new_items.iter().map(|c| c.clone().upcast::<StyledElement>()),
                    );
                }
                this.visual_children().insert_range(
                    e.new_starting_index as usize,
                    e.new_items.iter().map(|c| c.clone().upcast::<Visual>()),
                );
            }
            NotifyCollectionChangedAction::Move => {
                if !this.is_items_host() {
                    this.logical_children().move_range(
                        e.old_starting_index as usize,
                        e.old_items.len(),
                        e.new_starting_index as usize,
                    );
                }
                this.visual_children().move_range(
                    e.old_starting_index as usize,
                    e.old_items.len(),
                    e.new_starting_index as usize,
                );
            }
            NotifyCollectionChangedAction::Remove => {
                if !this.is_items_host() {
                    this.logical_children()
                        .remove_all(e.old_items.iter().map(|c| c.clone().upcast::<StyledElement>()));
                }
                this.visual_children().remove_all(e.old_items.iter().map(|c| c.clone().upcast::<Visual>()));
            }
            NotifyCollectionChangedAction::Replace => {
                for i in 0..e.old_items.len() {
                    let index = i + e.old_starting_index as usize;
                    let child = e.new_items[i].clone();
                    if !this.is_items_host() {
                        this.logical_children().set(index, child.clone().upcast());
                    }
                    this.visual_children().set(index, child.upcast());
                }
            }
            NotifyCollectionChangedAction::Reset => panic!("Specified method is not supported."),
        }

        this.raise_child_index_changed(&ChildIndexChangedEventArgs::child_indexes_reset());
        this.invalidate_measure_on_children_changed();
    }

    fn invalidate_measure_on_children_changed(this: &Self) {
        this.invalidate_measure();
    }
}

ferroui_base::ferro_properties! { impl Panel {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<Panel>()
        }
    );
} }

impl Panel {
    fn static_constructor() {
        Visual::affects_render::<Panel>(&[Self::background_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            children: Controls::new(),
            is_items_host: Cell::new(false),
            child_index_changed: Rc::new(HandlerList::new()),
            child_index_provider: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The children of the panel.
    pub fn children(&self) -> Controls {
        self.children.clone()
    }

    /// A brush with which to paint the background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// Whether this panel hosts the items created by an items presenter.
    pub fn is_items_host(&self) -> bool {
        self.is_items_host.get()
    }

    /// Marks the panel as the host of the items created by an items
    /// presenter: its children are then not its logical children.
    pub fn set_is_items_host(&self, value: bool) {
        self.is_items_host.set(value)
    }

    /// Marks a property on a child as affecting the parent panel's
    /// arrangement: after a change to any of the properties on a control,
    /// `invalidate_arrange` is called on its visual parent if that is a
    /// `TPanel`.
    pub fn affects_parent_arrange<TPanel: ObjectType + Upcast<Panel>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                let control = e.sender().downcast_ref::<Control>();
                let parent = control.and_then(|c| c.visual_parent());
                if let Some(panel) = parent.as_ref().and_then(|p| p.downcast_ref::<TPanel>()) {
                    let panel: &Panel = panel.upcast();
                    panel.invalidate_arrange();
                }
            });
        }
    }

    /// Marks a property on a child as affecting the parent panel's
    /// measurement: after a change to any of the properties on a control,
    /// `invalidate_measure` is called on its visual parent if that is a
    /// `TPanel`.
    pub fn affects_parent_measure<TPanel: ObjectType + Upcast<Panel>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                let control = e.sender().downcast_ref::<Control>();
                let parent = control.and_then(|c| c.visual_parent());
                if let Some(panel) = parent.as_ref().and_then(|p| p.downcast_ref::<TPanel>()) {
                    let panel: &Panel = panel.upcast();
                    panel.invalidate_measure();
                }
            });
        }
    }

    fn raise_child_index_changed(&self, e: &ChildIndexChangedEventArgs) {
        if !self.child_index_changed.is_empty() {
            for (_, handler) in self.child_index_changed.snapshot().iter() {
                handler(e);
            }
        }
    }
}

/// The child index provider of a panel.
struct PanelChildIndexProvider {
    owner: WeakRef<Panel>,
}

impl IChildIndexProvider for PanelChildIndexProvider {
    fn get_child_index(&self, child: &StyledElement) -> i32 {
        let Some(owner) = self.owner.upgrade() else { return -1 };
        match child.downcast_ref::<Control>() {
            Some(control) => owner.children.index_of(&control.to_ref()).map_or(-1, |index| index as i32),
            None => -1,
        }
    }

    fn try_get_total_count(&self) -> Option<i32> {
        let owner = self.owner.upgrade()?;
        Some(owner.children.count() as i32)
    }

    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else { return Disposable::empty() };
        let token = owner.child_index_changed.add(handler);
        let handlers = Rc::downgrade(&owner.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
}
