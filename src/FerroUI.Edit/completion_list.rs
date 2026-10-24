use crate::completion_data::{CompletionDataRef, ICompletionData};
use crate::completion_list_box::CompletionListBox;
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs, MouseButton};
use ferroui_base::interactivity::{
    IRoutedEventArgs, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_routed_event, ferro_routed_event_args,
    instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::{FuncDataTemplate, IControlTemplate};
use ferroui_controls::{
    ContentControl, ContentControlImpl, Control, ControlImpl, Image, StackPanel,
};
use ferroui_edit_core::completion::{CompletionItem, CompletionList as CompletionModel};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompletionAcceptAction {
    #[default]
    PointerPressed,
    PointerReleased,
    DoubleTapped,
}
ferroui_base::ferro_markup_enum!(CompletionAcceptAction { PointerPressed, PointerReleased, DoubleTapped }, { namespace: "FerroUI.Edit" });

/// Insertion request carrying the original key/pointer event to custom items.
#[derive(Clone)]
pub struct CompletionInsertionRequestedEventArgs {
    base: RoutedEventArgs,
    source_event: Option<Rc<dyn IRoutedEventArgs>>,
}
ferro_routed_event_args!(CompletionInsertionRequestedEventArgs: RoutedEventArgs);
impl CompletionInsertionRequestedEventArgs {
    pub fn source_event(&self) -> Option<&dyn IRoutedEventArgs> {
        self.source_event.as_deref()
    }
}

/// Rich completion presentation over the platform-independent filter model.
/// List entries retain their original identities through ranking and filtering.
#[repr(C)]
pub struct CompletionList {
    base: ContentControl,
    list: OnceCell<Ref<CompletionListBox>>,
    empty_presenter: OnceCell<Ref<ContentControl>>,
    source: FerroList<CompletionDataRef>,
    source_updating: Cell<bool>,
    items: RefCell<Vec<Rc<dyn ICompletionData>>>,
    model: RefCell<CompletionModel>,
    query: RefCell<Option<String>>,
    refreshing: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(CompletionList: ContentControl);
ferroui_base::ferro_class_info!(CompletionList {
    new: CompletionList::new
});
ferro_impl_classes!(CompletionList: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
ferro_properties! { impl CompletionList {
    pub fn is_filtering_property() -> StyledProperty<bool> { FerroProperty::register::<CompletionList, _>("IsFiltering", true) }
    pub fn completion_accept_action_property() -> StyledProperty<CompletionAcceptAction> { FerroProperty::register::<CompletionList, _>("CompletionAcceptAction", CompletionAcceptAction::PointerPressed) }
    pub fn completion_accept_keys_property() -> StyledProperty<Vec<Key>> { FerroProperty::register::<CompletionList, _>("CompletionAcceptKeys", vec![Key::Return, Key::Tab]) }
    pub fn empty_template_property() -> StyledProperty<Option<Rc<dyn IControlTemplate>>> { FerroProperty::register::<CompletionList, _>("EmptyTemplate", None) }
} }
impl FerroObjectImpl for CompletionList {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.build_content();
        let weak = this.to_ref().downgrade();
        let source = this.source.clone();
        let token = source.add_collection_changed(Rc::new(move |_| {
            if let Some(control) = weak.upgrade() {
                if !control.source_updating.get() {
                    control.refresh_source_items();
                }
            }
        }));
        this.subscriptions
            .borrow_mut()
            .push(Disposable::create(move || {
                source.remove_collection_changed(token);
            }));
        this.rebuild_items();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_filtering_property().as_property() {
            let query = this.query.borrow().clone().unwrap_or_default();
            this.query.borrow_mut().take();
            this.select_item(&query);
        } else if change.property() == Self::empty_template_property().as_property() {
            this.update_empty_presenter();
        }
    }
}
impl CompletionList {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            list: OnceCell::new(),
            empty_presenter: OnceCell::new(),
            source: FerroList::new(),
            source_updating: Cell::new(false),
            items: RefCell::new(Vec::new()),
            model: RefCell::new(CompletionModel::default()),
            query: RefCell::new(None),
            refreshing: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    ferro_routed_event!(
        pub fn insertion_requested_event() -> RoutedEvent<CompletionInsertionRequestedEventArgs> {
            RoutedEvent::register::<CompletionList, _>(
                "InsertionRequested",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<CompletionList, _>(
                "SelectionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    pub fn list_box(&self) -> Ref<CompletionListBox> {
        self.list
            .get()
            .expect("constructed completion list")
            .clone()
    }
    pub fn is_filtering(&self) -> bool {
        self.get_value(Self::is_filtering_property())
    }
    pub fn set_is_filtering(&self, value: bool) {
        self.set_value(Self::is_filtering_property(), value);
    }
    pub fn completion_accept_action(&self) -> CompletionAcceptAction {
        self.get_value(Self::completion_accept_action_property())
    }
    pub fn set_completion_accept_action(&self, value: CompletionAcceptAction) {
        self.set_value(Self::completion_accept_action_property(), value);
    }
    pub fn completion_accept_keys(&self) -> Vec<Key> {
        self.get_value(Self::completion_accept_keys_property())
    }
    pub fn set_completion_accept_keys(&self, value: Vec<Key>) {
        self.set_value(Self::completion_accept_keys_property(), value);
    }
    pub fn empty_template(&self) -> Option<Rc<dyn IControlTemplate>> {
        self.get_value(Self::empty_template_property())
    }
    pub fn set_empty_template(&self, value: Option<Rc<dyn IControlTemplate>>) {
        self.set_value(Self::empty_template_property(), value);
    }
    /// The observable collection is a shared handle; adding/removing entries
    /// updates filtering and selection immediately, including after attachment.
    pub fn completion_data(&self) -> FerroList<CompletionDataRef> {
        self.source.clone()
    }
    pub fn set_completion_data(&self, items: Vec<Rc<dyn ICompletionData>>) {
        self.source_updating.set(true);
        self.source.clear();
        self.source
            .add_range(items.into_iter().map(CompletionDataRef));
        self.source_updating.set(false);
        self.refresh_source_items();
    }
    fn refresh_source_items(&self) {
        let previous = self.selected_item();
        let items: Vec<_> = self
            .source
            .snapshot()
            .iter()
            .map(|item| item.0.clone())
            .collect();
        let metadata = items
            .iter()
            .map(|item| CompletionItem {
                text: item.text().to_owned(),
                priority: item.priority(),
                ..CompletionItem::default()
            })
            .collect();
        *self.items.borrow_mut() = items;
        *self.model.borrow_mut() = CompletionModel::new(metadata);
        let query = self.query.borrow_mut().take().unwrap_or_default();
        self.select_item(&query);
        if let Some(previous) = previous {
            if let Some(index) = self
                .current_list()
                .iter()
                .position(|item| Rc::ptr_eq(item, &previous))
            {
                self.set_selected_index(Some(index));
            }
        }
    }
    pub fn current_list(&self) -> Vec<Rc<dyn ICompletionData>> {
        let indices: Vec<_> = self.model.borrow().current_indices().collect();
        let items = self.items.borrow();
        indices
            .into_iter()
            .filter_map(|index| items.get(index).cloned())
            .collect()
    }
    pub fn selected_item(&self) -> Option<Rc<dyn ICompletionData>> {
        let index = self.model.borrow().selected_data_index()?;
        self.items.borrow().get(index).cloned()
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.model.borrow().selected_index()
    }
    pub fn set_selected_index(&self, index: Option<usize>) {
        if let Some(index) = index {
            self.model.borrow_mut().select_index(index);
        } else {
            self.model.borrow_mut().clear_selection();
        }
        self.synchronize_selection();
    }
    pub fn select_item(&self, query: &str) {
        if self.query.borrow().as_deref() == Some(query) {
            return;
        }
        *self.query.borrow_mut() = Some(query.to_owned());
        {
            let mut model = self.model.borrow_mut();
            model.is_filtering = self.is_filtering();
            model.select_item(query);
        }
        self.rebuild_items();
    }
    fn rebuild_items(&self) {
        let Some(list) = self.list.get() else {
            return;
        };
        let items = self.current_list();
        self.refreshing.set(true);
        list.items().clear();
        for item in items {
            list.items().add(Some(Rc::new(CompletionDataRef(item))));
        }
        self.refreshing.set(false);
        self.synchronize_selection();
        self.update_empty_presenter();
    }
    fn synchronize_selection(&self) {
        let Some(list) = self.list.get() else {
            return;
        };
        self.refreshing.set(true);
        if let Some(index) = self.selected_index() {
            list.select_index(index);
        } else {
            list.clear_selection();
        }
        self.refreshing.set(false);
        self.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
    }
    fn update_empty_presenter(&self) {
        let Some(presenter) = self.empty_presenter.get() else {
            return;
        };
        let empty = self.model.borrow().current_list().len() == 0;
        self.list_box().set_is_visible(!empty);
        presenter.set_is_visible(empty);
        let content = if empty {
            self.empty_template()
                .and_then(|template| template.build(&self.to_ref().upcast::<TemplatedControl>()))
                .map(|result| Rc::new(result.result().clone()) as BoxedValue)
        } else {
            None
        };
        presenter.set_content(content);
    }
    pub fn request_insertion(&self, source_event: Option<&dyn IRoutedEventArgs>) -> bool {
        if self.selected_item().is_none() {
            return false;
        }
        let event = CompletionInsertionRequestedEventArgs {
            base: RoutedEventArgs::with_event(Self::insertion_requested_event()),
            source_event: source_event.map(IRoutedEventArgs::share),
        };
        self.raise_event(&event);
        event.handled()
    }
    pub fn handle_key(&self, event: &KeyEventArgs) {
        if event.handled() {
            return;
        }
        if self.completion_accept_keys().contains(&event.key) {
            if self.request_insertion(Some(event)) {
                event.set_handled(true);
            }
            return;
        }
        if self.model.borrow().current_list().len() == 0 {
            return;
        }
        let page = self
            .list_box()
            .visible_item_count()
            .min(isize::MAX as usize) as isize;
        match event.key {
            Key::Up | Key::Down | Key::PageUp | Key::PageDown => {
                self.model.borrow_mut().move_selection(
                    match event.key {
                        Key::Up => -1,
                        Key::Down => 1,
                        Key::PageUp => -page,
                        _ => page,
                    },
                    matches!(event.key, Key::Up | Key::Down),
                )
            }
            Key::Home => self.model.borrow_mut().select_index(0),
            Key::End => self.model.borrow_mut().select_index(usize::MAX),
            _ => return,
        }
        self.synchronize_selection();
        event.set_handled(true);
    }
    fn event_item_index(&self, event: &RoutedEventArgs) -> Option<usize> {
        let mut visual = event.source()?.cast::<ferroui_base::Visual>()?;
        let list = self.list_box();
        loop {
            if visual == list.clone().upcast::<ferroui_base::Visual>() {
                return None;
            }
            if let Some(control) = visual.cast::<Control>() {
                let index = list.index_from_container(&control);
                if index >= 0 {
                    return Some(index as usize);
                }
            }
            visual = visual.visual_parent()?;
        }
    }
    fn accept_pointer(&self, event: &dyn IRoutedEventArgs) {
        if let Some(index) = self.event_item_index(event.as_routed_event_args()) {
            self.set_selected_index(Some(index));
            if self.request_insertion(Some(event)) {
                event.as_routed_event_args().set_handled(true);
            }
        }
    }
    fn build_content(&self) {
        let panel = StackPanel::new();
        let list = CompletionListBox::new();
        list.set_min_width(280.0);
        list.set_max_height(250.0);
        list.set_focusable(false);
        list.set_item_template(Some(FuncDataTemplate::for_type::<CompletionDataRef>(
            |item, _| {
                let row = StackPanel::new();
                row.set_orientation(Orientation::Horizontal);
                row.set_spacing(4.0);
                if let Some(source) = item.0.image() {
                    let image = Image::new();
                    image.set_source(Some(source));
                    image.set_width(16.0);
                    image.set_height(16.0);
                    row.children().add(image.upcast::<Control>());
                }
                let content = ContentControl::new();
                content.set_content(item.0.content());
                row.children().add(content.upcast::<Control>());
                Some(row.upcast::<Control>())
            },
            false,
        )));
        let empty = ContentControl::new();
        panel.children().add(list.clone().upcast::<Control>());
        panel.children().add(empty.clone().upcast::<Control>());
        self.set_content(Some(Rc::new(panel.upcast::<Control>())));
        self.list.set(list.clone()).ok();
        self.empty_presenter.set(empty).ok();
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler(
                SelectingItemsControl::selection_changed_event(),
                move |_, _| {
                    if let Some(control) = weak.upgrade() {
                        if control.refreshing.get() {
                            return;
                        }
                        let index = control.list_box().selected_index();
                        if index >= 0 {
                            control.model.borrow_mut().select_index(index as usize);
                        } else {
                            control.model.borrow_mut().clear_selection();
                        }
                        control.raise_event(&RoutedEventArgs::with_event(
                            Self::selection_changed_event(),
                        ));
                    }
                },
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, event| {
                    if let Some(control) = weak.upgrade() {
                        if control.completion_accept_action()
                            == CompletionAcceptAction::PointerPressed
                            && event
                                .get_current_point(None)
                                .properties
                                .is_left_button_pressed
                        {
                            control.accept_pointer(event);
                        }
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler_with(
                InputElement::pointer_released_event(),
                move |_, event| {
                    if let Some(control) = weak.upgrade() {
                        if control.completion_accept_action()
                            == CompletionAcceptAction::PointerReleased
                            && event.initial_press_mouse_button() == MouseButton::Left
                        {
                            if let Some(index) = control.event_item_index(event) {
                                if let Some(container) =
                                    control.list_box().container_from_index(index as i32)
                                {
                                    let point = event.get_position(Some(&container));
                                    let size = container.bounds().size();
                                    if point.x >= 0.0
                                        && point.y >= 0.0
                                        && point.x < size.width
                                        && point.y < size.height
                                    {
                                        control.accept_pointer(event);
                                    }
                                }
                            }
                        }
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler_with(
                InputElement::double_tapped_event(),
                move |_, event| {
                    if let Some(control) = weak.upgrade() {
                        if control.completion_accept_action()
                            == CompletionAcceptAction::DoubleTapped
                        {
                            control.accept_pointer(event);
                        }
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
    }
}
impl Drop for CompletionList {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filtering_preserves_rich_identity_and_navigation_until_prefix_changes() {
        let _scope = crate::tests::test_scope();
        let control = CompletionList::new();
        let items: Vec<Rc<dyn ICompletionData>> = ["Read", "ReadLine", "Write"]
            .into_iter()
            .map(|text| Rc::new(CompletionItem::new(text)) as Rc<dyn ICompletionData>)
            .collect();
        control.set_completion_data(items.clone());
        control.select_item("Re");
        assert_eq!(control.current_list().len(), 2);
        let mut key = KeyEventArgs::new();
        key.key = Key::Down;
        control.handle_key(&key);
        assert!(key.handled());
        assert!(Rc::ptr_eq(&control.selected_item().unwrap(), &items[1]));
        control.select_item("Re");
        assert_eq!(control.selected_index(), Some(1));
        control.select_item("missing");
        assert!(control.selected_item().is_none());
        control.set_is_filtering(false);
        assert_eq!(control.current_list().len(), 3);
    }
    #[test]
    fn acceptance_keys_are_configurable_and_original_event_is_delivered() {
        let _scope = crate::tests::test_scope();
        let control = CompletionList::new();
        control.set_completion_data(vec![Rc::new(CompletionItem::new("Read"))]);
        control.set_completion_accept_keys(vec![Key::Space]);
        let received = Rc::new(Cell::new(false));
        let received_handler = received.clone();
        let subscription = control.add_disposable_handler(
            CompletionList::insertion_requested_event(),
            move |_, event| {
                assert_eq!(
                    event
                        .source_event()
                        .unwrap()
                        .downcast_ref::<KeyEventArgs>()
                        .unwrap()
                        .key,
                    Key::Space
                );
                received_handler.set(true);
                event.set_handled(true);
            },
        );
        let mut tab = KeyEventArgs::new();
        tab.key = Key::Tab;
        control.handle_key(&tab);
        assert!(!tab.handled());
        let mut space = KeyEventArgs::new();
        space.key = Key::Space;
        control.handle_key(&space);
        assert!(space.handled());
        assert!(received.get());
        subscription.dispose();
    }

    #[test]
    fn observable_collection_updates_preserve_selected_item_identity() {
        let _scope = crate::tests::test_scope();
        let control = CompletionList::new();
        let collection = control.completion_data();
        let read: Rc<dyn ICompletionData> = Rc::new(CompletionItem::new("Read"));
        let line: Rc<dyn ICompletionData> = Rc::new(CompletionItem::new("ReadLine"));
        collection.add(CompletionDataRef(read));
        collection.add(CompletionDataRef(line.clone()));
        control.select_item("Re");
        control.set_selected_index(Some(1));
        collection.insert(0, CompletionDataRef(Rc::new(CompletionItem::new("Really"))));
        assert!(Rc::ptr_eq(&control.selected_item().unwrap(), &line));
        assert_eq!(control.current_list().len(), 3);
        collection.clear();
        assert!(control.selected_item().is_none());
        assert_eq!(control.list_box().selected_index(), -1);
        drop(control);
        assert!(!collection.has_collection_changed_subscribers());
    }

    #[test]
    fn runtime_xaml_constructs_completion_and_overload_native_properties() {
        use ferroui_base::metadata::from_markup_value;
        let _scope = crate::tests::test_scope();
        crate::register_types();
        ferroui_markup_xaml::register_types();
        let list = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
            r#"<edit:CompletionList xmlns="https://github.com/ferroui" xmlns:edit="clr-namespace:FerroUI.Edit;assembly=FerroUI.Edit" IsFiltering="False" MinWidth="123" CompletionAcceptAction="PointerReleased"/>"#,
            Some(&crate::ASSEMBLY), None, None, false).unwrap();
        let list = from_markup_value::<Ref<CompletionList>>(&Some(list)).unwrap();
        assert!(!list.is_filtering());
        assert_eq!(list.min_width(), 123.0);
        assert_eq!(
            list.completion_accept_action(),
            CompletionAcceptAction::PointerReleased
        );
        let insight = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
            r#"<edit:OverloadInsightWindow xmlns="https://github.com/ferroui" xmlns:edit="clr-namespace:FerroUI.Edit;assembly=FerroUI.Edit" CloseAutomatically="False"/>"#,
            Some(&crate::ASSEMBLY), None, None, false).unwrap();
        let insight =
            from_markup_value::<Ref<crate::OverloadInsightWindow>>(&Some(insight)).unwrap();
        assert!(!insight.close_automatically());
    }

    #[test]
    fn empty_template_builds_native_content_and_clears_when_items_arrive() {
        let _scope = crate::tests::test_scope();
        let control = CompletionList::new();
        control.set_empty_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, _| {
                let text = ferroui_controls::TextBlock::new();
                text.set_text(Some("No suggestions"));
                text.upcast::<Control>()
            },
        )));
        let presenter = control.empty_presenter.get().unwrap();
        assert!(presenter.is_visible());
        assert!(!control.list_box().is_visible());
        assert!(presenter
            .content()
            .unwrap()
            .downcast_ref::<Ref<Control>>()
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .is_some());
        control
            .completion_data()
            .add(CompletionDataRef(Rc::new(CompletionItem::new("Read"))));
        assert!(!presenter.is_visible());
        assert!(presenter.content().is_none());
        assert!(control.list_box().is_visible());
    }
}
