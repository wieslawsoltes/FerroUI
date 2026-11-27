//! Source RibbonElementFactory.cs. Supported creation paths are native virtuals;
//! missing known model kinds fail explicitly rather than masquerading as unsupported.
use super::{RibbonAction, RibbonModelBindings, model_values::box_command_parameter};
use crate::compat::{DataTemplateSelectorExt, DataTemplateSelectorImpl};
use crate::{commands::adapt_core_command, controls::*};
use ferroribbon_core::{RibbonString, commands::ICommand, model::*};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Ref,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, instantiate,
};
use ferroui_controls::{Control, automation::AutomationProperties, primitives::ToggleButton};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

pub type RibbonItemModelReference = Rc<dyn RibbonItemModelImpl>;
enum FactoryRibbonContext {
    Owned(Ref<Ribbon>),
    Parent(ferroui_base::WeakRef<Ribbon>),
}
enum ModelAssociation {
    Owned(Rc<dyn RibbonNodeModelImpl>),
    // Direct custom content already owns this control through the model. The
    // binding scope owns the model; this backlink must not close a strong cycle.
    Bound(std::rc::Weak<dyn RibbonNodeModelImpl>),
}
#[derive(Default)]
struct ChangeToken(Event<str>);
impl INotifyPropertyChanged for ChangeToken {
    fn property_changed(&self) -> &Event<str> {
        &self.0
    }
}

#[repr(C)]
pub struct RibbonElementFactory {
    base: FerroObject,
    ribbon: RefCell<Option<FactoryRibbonContext>>,
    ribbon_owners: RefCell<Vec<ferroui_base::WeakRef<Ribbon>>>,
    commands: Rc<ChangeToken>,
    content_template_selector: RefCell<Option<Rc<dyn DataTemplateSelectorImpl>>>,
}
ferro_class!(RibbonElementFactory:FerroObject, virtuals RibbonElementFactoryImpl:FerroObjectImpl {
    fn create_backstage(this,model:Option<Rc<dyn RibbonBackstageModelImpl>>,bindings:Option<Rc<RibbonModelBindings>>)->Ref<RibbonBackstage>;
    fn create_backstage_item(this,model:Option<Rc<dyn RibbonBackstageItemModelImpl>>,bindings:Option<Rc<RibbonModelBindings>>)->Ref<RibbonBackstageItem>;
    fn create_contextual_group(this,model:Option<Rc<dyn RibbonContextualGroupModelImpl>>,bindings:Option<Rc<RibbonModelBindings>>)->Ref<RibbonContextualTabGroup>;
    fn create_tab(this,model:Rc<dyn RibbonTabModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<RibbonTab>;
    fn create_group(this,model:Rc<dyn RibbonGroupModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<RibbonGroup>;
    fn create_container(this,container:Option<Ref<RibbonItemsContainer>>,model:Rc<dyn RibbonButtonGroupModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_item(this,model:Option<RibbonItemModelReference>,bindings:Rc<RibbonModelBindings>)->Option<Ref<Control>>;
    fn bind_common(this,element:Ref<Control>,model:RibbonItemModelReference,bindings:Rc<RibbonModelBindings>);
    fn create_item_command(this,element:Ref<Control>,model:RibbonItemModelReference)->Option<Rc<dyn ferroui_base::input::ICommand>>;
    fn create_button(this,model:Rc<dyn RibbonButtonModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_toggle_button(this,model:Rc<dyn RibbonToggleButtonModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_drop_down_button(this,model:Rc<dyn RibbonDropDownButtonModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_split_button(this,model:Rc<dyn RibbonSplitButtonModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_flyout(this,entries:Option<ferroribbon_core::collections::RibbonReadOnlyList<RibbonNodeReference>>,content:Option<RibbonObject>,bindings:Option<Rc<RibbonModelBindings>>)->Option<Ref<ferroui_controls::primitives::FlyoutBase>>;
    fn create_menu_items(this,entries:Option<RibbonItemsSource<RibbonNodeReference>>,bindings:Option<Rc<RibbonModelBindings>>)->RibbonItemsSource<Ref<Control>>;
    fn create_menu_item(this,item:Option<Rc<dyn RibbonMenuItemModelImpl>>,bindings:Option<Rc<RibbonModelBindings>>)->Ref<crate::compat::MenuFlyoutItem>;
    fn create_menu_element(this,entry:RibbonNodeReference,bindings:Option<Rc<RibbonModelBindings>>)->Option<Ref<Control>>;
    fn create_menu_button(this,entry:RibbonNodeReference,bindings:Option<Rc<RibbonModelBindings>>)->Option<Ref<Control>>;
    fn create_check_box(this,model:Rc<dyn RibbonCheckBoxModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_combo_box(this,model:Rc<dyn RibbonComboBoxModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_spinner(this,model:Rc<dyn RibbonSpinnerModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_text_box(this,model:Rc<dyn RibbonTextBoxModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_gallery(this,model:Rc<dyn RibbonGalleryModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_color_picker(this,model:Rc<dyn RibbonColorPickerModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_grid_picker(this,model:Rc<dyn RibbonGridPickerModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_segmented(this,model:Rc<dyn RibbonSegmentedModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_custom(this,model:Rc<dyn RibbonCustomItemModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
    fn create_slider(this,model:Rc<dyn RibbonSliderModelImpl>,bindings:Rc<RibbonModelBindings>)->Ref<Control>;
});
ferro_class_info!(RibbonElementFactory{new:RibbonElementFactory::new,markup:{namespace:"FerroRibbon.Controls.Mvvm",properties:[ContentTemplateSelector:Option<Rc<dyn DataTemplateSelectorImpl>>{get:RibbonElementFactory::content_template_selector,set:RibbonElementFactory::set_content_template_selector},Ribbon:Option<Ref<Ribbon>>{get:RibbonElementFactory::ribbon}],methods:[fn InvalidateCommands()=>RibbonElementFactory::invalidate_commands]}});
impl FerroObjectImpl for RibbonElementFactory {}
pub(super) fn text(value: Option<RibbonString>) -> Option<String> {
    value.map(|v| {
        v.to_utf8()
            .expect("FerroUI native text cannot represent an unpaired UTF-16 surrogate")
    })
}
pub(super) fn native_payload(value: RibbonObject) -> BoxedValue {
    crate::metadata::model_objects::to_native(Some(value))
        .unwrap_or_else(|error| panic!("{error}"))
        .expect("non-null RibbonObject")
}

fn same_backstage_model(
    a: &Option<Rc<dyn RibbonBackstageItemModelImpl>>,
    b: &Option<Rc<dyn RibbonBackstageItemModelImpl>>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
fn cast<T: ?Sized + ferroribbon_core::model::RibbonModelView>(
    m: &RibbonItemModelReference,
) -> Option<Rc<T>> {
    try_cast_node::<T>(Some(m.clone()))
        .expect("model's compile-time inheritance must be registered")
}
pub(super) fn editable(element: &Control) -> Option<Rc<dyn IRibbonItemEditable>> {
    ValueTypes::try_cast(
        &Control::boxed(element.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItemEditable>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItemEditable>>().cloned())
}
pub(super) fn source(element: &Control) -> Option<Rc<dyn IRibbonCommandSource>> {
    ValueTypes::try_cast(
        &Control::boxed(element.to_ref()),
        ValueType::of::<Rc<dyn IRibbonCommandSource>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonCommandSource>>().cloned())
}
impl RibbonElementFactory {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            ribbon: RefCell::default(),
            ribbon_owners: RefCell::default(),
            commands: Rc::new(ChangeToken::default()),
            content_template_selector: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn create_segment(segment: &Rc<dyn RibbonSegmentModelImpl>) -> Ref<RibbonSegment> {
        let value = RibbonSegment::new();
        value.set_label(text(segment.label()));
        value.set_icon(segment.icon().map(|v| Rc::new(v) as BoxedValue));
        value.set_segment_value(segment.value().map(native_payload));
        value
    }

    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        self.ribbon
            .borrow()
            .as_ref()
            .and_then(|context| match context {
                FactoryRibbonContext::Owned(ribbon) => Some(ribbon.clone()),
                FactoryRibbonContext::Parent(ribbon) => ribbon.upgrade(),
            })
    }
    pub fn content_template_selector(&self) -> Option<Rc<dyn DataTemplateSelectorImpl>> {
        self.content_template_selector.borrow().clone()
    }
    pub fn set_content_template_selector(&self, value: Option<Rc<dyn DataTemplateSelectorImpl>>) {
        let old = self.content_template_selector.replace(value);
        drop(old);
    }
    pub fn set_ribbon(&self, value: Option<Ref<Ribbon>>) {
        let context = value.map(|ribbon| {
            if self.has_ribbon_owner(&ribbon) {
                FactoryRibbonContext::Parent(ribbon.downgrade())
            } else {
                FactoryRibbonContext::Owned(ribbon)
            }
        });
        let old = self.ribbon.replace(context);
        drop(old);
    }
    fn has_ribbon_owner(&self, ribbon: &Ref<Ribbon>) -> bool {
        self.ribbon_owners
            .borrow()
            .iter()
            .any(|owner| owner.upgrade().is_some_and(|owner| owner.ptr_eq(ribbon)))
    }
    /// Ownership registration does not assign a context. In particular, assigning
    /// ItemFactory before Model must preserve a null or unrelated Ribbon context.
    pub(crate) fn add_ribbon_owner(&self, owner: &Ref<Ribbon>) {
        {
            let mut owners = self.ribbon_owners.borrow_mut();
            owners.retain(|owner| owner.upgrade().is_some());
            if !owners.iter().any(|candidate| {
                candidate
                    .upgrade()
                    .is_some_and(|candidate| candidate.ptr_eq(owner))
            }) {
                owners.push(owner.downgrade());
            }
        }
        if let Some(context) = self.ribbon()
            && context.ptr_eq(owner)
        {
            self.set_ribbon(Some(context));
        }
    }
    pub(crate) fn remove_ribbon_owner(&self, owner: &Ref<Ribbon>) {
        self.ribbon_owners.borrow_mut().retain(|candidate| {
            candidate
                .upgrade()
                .is_some_and(|candidate| !candidate.ptr_eq(owner))
        });
        // Once detached, a standalone factory again owns its ordinary context.
        // Other ribbons sharing this factory retain their weak parent links.
        if let Some(context) = self.ribbon() {
            self.set_ribbon(Some(context));
        }
    }
    /// Native parent-owned context uses weak references; standalone context
    /// ownership and context identity remain intact.
    pub(crate) fn set_composed_ribbon(&self, value: Option<Ref<Ribbon>>) {
        if let Some(owner) = &value {
            self.add_ribbon_owner(owner);
        }
        self.set_ribbon(value);
    }
    pub fn get_model(element: &FerroObject) -> Option<Rc<dyn RibbonNodeModelImpl>> {
        element
            .instance_data::<ModelAssociation>()
            .and_then(|v| match &*v {
                ModelAssociation::Owned(model) => Some(model.clone()),
                ModelAssociation::Bound(model) => model.upgrade(),
            })
    }
    /// Source protected static SetModel, also usable by native factory subclasses.
    pub fn set_model(element: &FerroObject, model: Rc<dyn RibbonNodeModelImpl>) {
        element.set_instance_data(Some(Rc::new(ModelAssociation::Owned(model))));
    }
    pub fn invalidate_commands(&self) {
        self.commands.0.raise("");
    }
    pub fn resolve_command(&self, id: Option<RibbonString>) -> Option<Rc<dyn ICommand>> {
        id.and_then(|id| {
            self.ribbon()
                .and_then(|r| r.command_catalog())
                .and_then(|c| c.find(Some(&id)))
                .and_then(|c| c.command())
        })
    }
    /// Executes a command if it accepts the supplied parameter. Available to
    /// downstream factory implementations, like RibbonSpace's protected helper.
    pub fn execute(command: Option<&dyn ICommand>, parameter: Option<&dyn Any>) {
        if let Some(command) = command
            && command.can_execute(parameter)
        {
            command.execute(parameter);
        }
    }
    pub fn bind_commands(
        &self,
        bindings: &Rc<RibbonModelBindings>,
        model: Rc<dyn INotifyPropertyChanged>,
        apply: RibbonAction,
        properties: Vec<Option<RibbonString>>,
    ) {
        bindings.one_way(Some(model), Some(apply.clone()), Some(properties));
        bindings.watch(Some(self.commands.clone()), Some(apply), Some(vec![]));
    }
    /// No initial on_changed call; replacement follows the source child-scope order.
    pub fn watch_inner(
        bindings: &Rc<RibbonModelBindings>,
        model: Rc<dyn INotifyPropertyChanged>,
        inner: Rc<dyn Fn() -> Option<Rc<dyn INotifyPropertyChanged>>>,
        property: Option<RibbonString>,
        on_changed: RibbonAction,
    ) {
        #[derive(Default)]
        struct State {
            scope: Option<Rc<RibbonModelBindings>>,
            watched: Option<Rc<dyn INotifyPropertyChanged>>,
        }
        let state = Rc::new(RefCell::new(State::default()));
        let owner = Rc::downgrade(bindings);
        bindings.one_way(
            Some(model),
            Some(Rc::new(move || {
                let current = inner();
                let same = {
                    let s = state.borrow();
                    s.scope.is_some()
                        && match (&current, &s.watched) {
                            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                            (None, None) => true,
                            _ => false,
                        }
                };
                if same {
                    return;
                }
                let old = state.borrow().scope.clone();
                if let Some(old) = old {
                    old.dispose();
                }
                let bindings = owner
                    .upgrade()
                    .expect("binding owner must remain alive while its callback executes");
                let scope = bindings.child();
                let old = state.borrow_mut().scope.replace(scope.clone());
                drop(old);
                let old = std::mem::replace(&mut state.borrow_mut().watched, current.clone());
                drop(old);
                if let Some(current) = current {
                    scope.watch(Some(current), Some(on_changed.clone()), Some(vec![]));
                }
            })),
            Some(vec![property]),
        );
    }
    pub(super) fn watch_screen_tip(
        bindings: &Rc<RibbonModelBindings>,
        model: Rc<dyn RibbonNodeModelImpl>,
        action: RibbonAction,
    ) {
        let m = model.clone();
        Self::watch_inner(
            bindings,
            model,
            Rc::new(move || m.screen_tip().map(|m| m as Rc<dyn INotifyPropertyChanged>)),
            Some("ScreenTip".into()),
            action,
        );
    }
    fn bind_keywords(
        element: Ref<Control>,
        model: Rc<dyn RibbonNodeModelImpl>,
        bindings: &Rc<RibbonModelBindings>,
    ) {
        let keywords = model.keywords();
        let observed = keywords.clone();
        bindings.collection(
            Some(Rc::new(observed)),
            Some(Rc::new(move || {
                if keywords.count() > 0 {
                    let mut units = Vec::new();
                    for (i, k) in keywords.iter().enumerate() {
                        if i > 0 {
                            units.push(44);
                        }
                        if let Some(k) = k {
                            units.extend_from_slice(k.as_utf16());
                        }
                    }
                    RibbonSearch::set_keywords(
                        &element,
                        text(Some(RibbonString::from_utf16(units))),
                    );
                } else {
                    element.clear_value(RibbonSearch::keywords_property());
                }
            })),
        );
    }
    fn on_structure_changed(&self) {
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_model_elements_changed();
        }
    }
    fn apply_automation_id(element: &Control, id: Option<RibbonString>, explicit: &Cell<bool>) {
        if id.as_ref().is_some_and(|v| !v.is_empty()) {
            AutomationProperties::set_automation_id(element, text(id).as_deref());
            explicit.set(true);
        } else if explicit.get() {
            explicit.set(false);
            element.clear_value(AutomationProperties::automation_id_property());
            RibbonItemHelper::update_automation(element.to_ref()).expect("factory automation");
        }
    }
}
impl RibbonElementFactoryImpl for RibbonElementFactory {
    fn create_backstage(
        this: &Self,
        model: Option<Rc<dyn RibbonBackstageModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<RibbonBackstage> {
        let model = model.expect("ArgumentNullException: model");
        let bindings = bindings.expect("ArgumentNullException: bindings");
        let backstage = RibbonBackstage::new();
        let m = model.clone();
        let b = backstage.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || b.set_title(text(m.title())))),
            Some(vec![Some("Title".into())]),
        );
        let updating_items = Rc::new(Cell::new(false));
        let m = model.clone();
        let b = backstage.clone();
        let apply_selection: RibbonAction = Rc::new(move || {
            let selected = m.selected_item();
            let found = selected.as_ref().and_then(|wanted| {
                b.items()
                    .iter()
                    .find(|i| {
                        let i = i.as_ref().expect("NullReference: backstage item");
                        Self::get_model(i).is_some_and(|node| {
                            std::ptr::addr_eq(Rc::as_ptr(&node), Rc::as_ptr(wanted))
                        })
                    })
                    .flatten()
            });
            if let Some(selected) = found {
                b.set_selected_item(Some(selected));
                return;
            }
            b.ensure_selection();
            let current = b
                .selected_item()
                .and_then(|i| Self::get_model(&i))
                .and_then(|m| {
                    try_cast_node::<dyn RibbonBackstageItemModelImpl>(Some(m))
                        .expect("registered backstage model cast")
                });
            if !same_backstage_model(&m.selected_item(), &current) {
                m.set_selected_item(current);
            }
        });
        let f = this.to_ref();
        let changing = updating_items.clone();
        let changed = updating_items.clone();
        let apply = apply_selection.clone();
        bindings.items(
            model.items(),
            Rc::new(backstage.items()),
            Rc::new(move |m, b| Some(f.create_backstage_item(m, Some(b)))),
            false,
            Some(Rc::new(move || changing.set(true))),
            Some(Rc::new(move || {
                changed.set(false);
                apply();
            })),
        );
        bindings.one_way(
            Some(model.clone()),
            Some(apply_selection),
            Some(vec![Some("SelectedItem".into())]),
        );
        let weak_scope = Rc::downgrade(&bindings);
        let target = backstage.downgrade();
        let m = model;
        let token = backstage.add_property_changed_handler(Rc::new(move |e| {
            if e.property() != RibbonBackstage::selected_item_property().as_property()
                || !e.is_effective_value_change()
            {
                return;
            }
            let Some(scope) = weak_scope.upgrade() else {
                return;
            };
            if updating_items.get() || scope.is_suspended() {
                return;
            }
            let b = target.upgrade().expect("backstage callback target alive");
            if let Some(selected) = b.selected_item()
                && let Some(node) = Self::get_model(&selected)
                && let Some(item) = try_cast_node::<dyn RibbonBackstageItemModelImpl>(Some(node))
                    .expect("registered backstage model cast")
                && !same_backstage_model(&m.selected_item(), &Some(item.clone()))
            {
                m.set_selected_item(Some(item));
            }
        }));
        let b = backstage.clone();
        bindings.add(Some(Rc::new(move || {
            b.remove_property_changed_handler(token);
        })));
        backstage
    }
    fn create_backstage_item(
        this: &Self,
        model: Option<Rc<dyn RibbonBackstageItemModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<RibbonBackstageItem> {
        let model = model.expect("ArgumentNullException: model");
        let bindings = bindings.expect("ArgumentNullException: bindings");
        let item = RibbonBackstageItem::new();
        Self::set_model(&item, model.clone());
        let m = model.clone();
        let e = item.clone();
        let f = this.to_ref();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_id(text(m.id()));
                e.set_header(text(m.label()));
                e.set_icon(m.icon().map(|v| Rc::new(v) as BoxedValue));
                e.set_key_tip(text(m.key_tip()));
                e.set_is_enabled(m.is_enabled());
                e.set_is_visible(m.is_visible());
                e.set_screen_tip(if m.screen_tip().is_none() {
                    text(m.description()).map(|v| Rc::new(v) as BoxedValue)
                } else {
                    RibbonScreenTipService::create(
                        text(m.label()),
                        m.screen_tip().map(|v| Rc::new(v) as BoxedValue),
                        None,
                    )
                    .expect("factory backstage ScreenTip")
                });
                e.set_placement(m.placement());
                e.set_closes_backstage(m.closes_backstage());
                e.set_has_separator_before(m.has_separator_before());
                e.set_command_parameter(m.command_parameter().map(box_command_parameter));
                match m.content().map(native_payload) {
                    Some(content)
                        if ValueTypes::as_object(&*content)
                            .and_then(|v| v.cast::<Control>())
                            .is_some() =>
                    {
                        e.set_content_template(None);
                        e.set_content(Some(content));
                    }
                    Some(content) => {
                        e.set_content(Some(content.clone()));
                        e.set_content_template(
                            f.content_template_selector()
                                .and_then(|selector| selector.select_template(Some(content))),
                        );
                    }
                    None => {
                        e.set_content(None);
                        e.set_content_template(None);
                    }
                }
            })),
            Some(vec![]),
        );
        let m = model.clone();
        let e = item.clone();
        Self::watch_screen_tip(
            &bindings,
            model.clone(),
            Rc::new(move || {
                e.set_screen_tip(
                    RibbonScreenTipService::create(
                        text(m.label()),
                        m.screen_tip().map(|v| Rc::new(v) as BoxedValue),
                        None,
                    )
                    .expect("factory backstage ScreenTip refresh"),
                )
            }),
        );
        let m = model.clone();
        let e = item.clone();
        let f = this.to_ref();
        this.bind_commands(
            &bindings,
            model,
            Rc::new(move || {
                e.set_command(
                    m.command()
                        .or_else(|| f.resolve_command(m.command_id()))
                        .map(adapt_core_command),
                )
            }),
            vec![Some("Command".into()), Some("CommandId".into())],
        );
        item
    }
    fn create_contextual_group(
        _this: &Self,
        model: Option<Rc<dyn RibbonContextualGroupModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<RibbonContextualTabGroup> {
        let model = model.expect("ArgumentNullException: model");
        let bindings = bindings.expect("ArgumentNullException: bindings");
        let group = RibbonContextualTabGroup::new();
        Self::set_model(&group, model.clone());
        let m = model.clone();
        let e = group.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_id(text(m.id()));
                e.set_header(text(m.label()));
                let c = m.color();
                e.set_color(ferroui_base::media::Color::from_argb(c.a, c.r, c.g, c.b));
                e.set_activation(m.activation());
                e.set_key_tip(text(m.key_tip()));
                e.set_is_enabled(m.is_enabled());
            })),
            Some(vec![
                Some("Id".into()),
                Some("Label".into()),
                Some("Color".into()),
                Some("Activation".into()),
                Some("KeyTip".into()),
                Some("IsEnabled".into()),
            ]),
        );
        let m = model.clone();
        let e = group.clone();
        let output = model.clone();
        let control = group.clone();
        bindings.two_way(
            Some(model),
            Some("IsVisible".into()),
            Some(Rc::new(move || e.set_is_visible(m.is_visible()))),
            Some(group.clone().upcast()),
            Some(RibbonContextualTabGroup::is_visible_property().as_property()),
            Some(Rc::new(move || output.set_is_visible(control.is_visible()))),
        );
        group
    }

    fn create_tab(
        this: &Self,
        model: Rc<dyn RibbonTabModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<RibbonTab> {
        let tab = RibbonTab::new();
        Self::set_model(&tab, model.clone());
        let explicit = Cell::new(false);
        let e = tab.clone();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_id(text(m.id()));
                e.set_header(text(m.label()));
                e.set_key_tip(text(m.key_tip()));
                e.set_is_tab_visible(m.is_visible());
                e.set_is_enabled(m.is_enabled());
                e.set_contextual_group_id(text(m.contextual_group_id()));
                e.set_icon(
                    m.icon()
                        .or_else(|| m.large_icon())
                        .map(|v| Rc::new(v) as BoxedValue),
                );
                e.set_screen_tip(if m.screen_tip().is_none() {
                    text(m.description()).map(|v| Rc::new(v) as BoxedValue)
                } else {
                    RibbonScreenTipService::create(
                        text(m.label()),
                        m.screen_tip().map(|v| Rc::new(v) as BoxedValue),
                        None,
                    )
                    .expect("factory tab ScreenTip")
                });
                Self::apply_automation_id(&e, m.automation_id(), &explicit);
            })),
            Some(vec![]),
        );
        let e = tab.clone();
        let m = model.clone();
        Self::watch_screen_tip(
            &bindings,
            model.clone(),
            Rc::new(move || {
                e.set_screen_tip(
                    RibbonScreenTipService::create(
                        text(m.label()),
                        m.screen_tip().map(|v| Rc::new(v) as BoxedValue),
                        None,
                    )
                    .expect("factory tab ScreenTip"),
                )
            }),
        );
        Self::bind_keywords(tab.clone().upcast(), model.clone(), &bindings);
        let f = this.to_ref();
        let changed = this.to_ref();
        bindings.items(
            model.groups(),
            Rc::new(tab.groups()),
            Rc::new(move |m, b| Some(f.create_group(m.expect("ArgumentNullException: model"), b))),
            false,
            None,
            Some(Rc::new(move || changed.on_structure_changed())),
        );
        tab
    }
    fn create_group(
        this: &Self,
        model: Rc<dyn RibbonGroupModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<RibbonGroup> {
        let group = RibbonGroup::new();
        Self::set_model(&group, model.clone());
        let explicit = Cell::new(false);
        let e = group.clone();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_id(text(m.id()));
                e.set_header(text(m.label()));
                e.set_icon(
                    m.large_icon()
                        .or_else(|| m.icon())
                        .map(|v| Rc::new(v) as BoxedValue),
                );
                e.set_key_tip(text(m.key_tip()));
                e.set_is_group_visible(m.is_visible());
                e.set_is_enabled(m.is_enabled());
                e.set_reduction_order(m.reduction_order());
                e.set_can_collapse(m.can_collapse());
                e.set_items_layout(m.items_layout());
                e.set_row_count(m.row_count());
                e.set_simplified_visibility(m.simplified_visibility());
                e.set_dialog_launcher_screen_tip(
                    m.dialog_launcher_screen_tip()
                        .map(|v| Rc::new(v) as BoxedValue),
                );
                Self::apply_automation_id(&e, m.automation_id(), &explicit);
                e.set_screen_tip(
                    m.screen_tip()
                        .map(|v| Rc::new(v) as BoxedValue)
                        .or_else(|| text(m.description()).map(|v| Rc::new(v) as BoxedValue)),
                );
            })),
            Some(vec![]),
        );
        let f = this.to_ref();
        let e = group.clone();
        let m = model.clone();
        this.bind_commands(
            &bindings,
            model.clone(),
            Rc::new(move || {
                e.set_dialog_launcher_command(
                    m.dialog_launcher_command()
                        .or_else(|| f.resolve_command(m.dialog_launcher_command_id()))
                        .map(adapt_core_command),
                );
                e.set_is_dialog_launcher_visible(
                    m.dialog_launcher_command().is_some()
                        || m.dialog_launcher_command_id().is_some(),
                );
            }),
            vec![
                Some("DialogLauncherCommand".into()),
                Some("DialogLauncherCommandId".into()),
            ],
        );
        let e = group.clone();
        let m = model.clone();
        Self::watch_screen_tip(
            &bindings,
            model.clone(),
            Rc::new(move || {
                e.set_screen_tip(None);
                e.set_screen_tip(
                    m.screen_tip()
                        .map(|v| Rc::new(v) as BoxedValue)
                        .or_else(|| text(m.description()).map(|v| Rc::new(v) as BoxedValue)),
                );
            }),
        );
        let e = group.clone();
        let m = model.clone();
        let inner = model.clone();
        Self::watch_inner(
            &bindings,
            model.clone(),
            Rc::new(move || {
                inner
                    .dialog_launcher_screen_tip()
                    .map(|v| v as Rc<dyn INotifyPropertyChanged>)
            }),
            Some("DialogLauncherScreenTip".into()),
            Rc::new(move || {
                e.set_dialog_launcher_screen_tip(None);
                e.set_dialog_launcher_screen_tip(
                    m.dialog_launcher_screen_tip()
                        .map(|v| Rc::new(v) as BoxedValue),
                );
            }),
        );
        Self::bind_keywords(group.clone().upcast(), model.clone(), &bindings);
        let f = this.to_ref();
        let changed = this.to_ref();
        bindings.items(
            model.items(),
            Rc::new(group.items()),
            Rc::new(move |m, b| f.create_item(m, b)),
            false,
            None,
            Some(Rc::new(move || changed.on_structure_changed())),
        );
        let f = this.to_ref();
        let changed = this.to_ref();
        bindings.items(
            model.slide_out_items(),
            Rc::new(group.slide_out_items()),
            Rc::new(move |m, b| f.create_item(m, b)),
            false,
            None,
            Some(Rc::new(move || changed.on_structure_changed())),
        );
        group
    }
    fn create_container(
        this: &Self,
        container: Option<Ref<RibbonItemsContainer>>,
        model: Rc<dyn RibbonButtonGroupModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let container = container.expect("ArgumentNullException: container");
        let f = this.to_ref();
        let changed = this.to_ref();
        bindings.items(
            model.items(),
            Rc::new(container.items()),
            Rc::new(move |m, b| f.create_item(m, b)),
            false,
            None,
            Some(Rc::new(move || changed.on_structure_changed())),
        );
        container.upcast()
    }
    fn create_item(
        this: &Self,
        model: Option<RibbonItemModelReference>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Option<Ref<Control>> {
        let model = model?;
        // Preserve the source most-derived-first model switch.
        let element = if let Some(m) = cast::<dyn RibbonGalleryModelImpl>(&model) {
            this.create_gallery(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonColorPickerModelImpl>(&model) {
            this.create_color_picker(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonSplitButtonModelImpl>(&model) {
            this.create_split_button(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonDropDownButtonModelImpl>(&model) {
            this.create_drop_down_button(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonToggleButtonModelImpl>(&model) {
            this.create_toggle_button(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonButtonModelImpl>(&model) {
            this.create_button(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonCheckBoxModelImpl>(&model) {
            this.create_check_box(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonComboBoxModelImpl>(&model) {
            this.create_combo_box(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonSpinnerModelImpl>(&model) {
            this.create_spinner(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonTextBoxModelImpl>(&model) {
            this.create_text_box(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonSliderModelImpl>(&model) {
            this.create_slider(m, bindings.clone())
        } else if cast::<dyn RibbonLabelModelImpl>(&model).is_some() {
            RibbonLabel::new().upcast()
        } else if cast::<dyn RibbonSeparatorModelImpl>(&model).is_some() {
            RibbonSeparator::new().upcast()
        } else if let Some(m) = cast::<dyn RibbonRowModelImpl>(&model) {
            this.create_container(Some(RibbonStackPanel::new().upcast()), m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonButtonGroupModelImpl>(&model) {
            this.create_container(Some(RibbonButtonGroup::new().upcast()), m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonGridPickerModelImpl>(&model) {
            this.create_grid_picker(m, bindings.clone())
        } else if let Some(m) = cast::<dyn RibbonSegmentedModelImpl>(&model) {
            this.create_segmented(m, bindings.clone())
        } else {
            let m = cast::<dyn RibbonCustomItemModelImpl>(&model)?;
            this.create_custom(m, bindings.clone())
        };
        let is_direct_content = cast::<dyn RibbonCustomItemModelImpl>(&model)
            .and_then(|m| m.content())
            .map(native_payload)
            .as_ref()
            .and_then(Control::from_boxed)
            .is_some_and(|content| content.ptr_eq(&element));
        if is_direct_content {
            let node: Rc<dyn RibbonNodeModelImpl> = model.clone();
            element.set_instance_data(Some(Rc::new(ModelAssociation::Bound(Rc::downgrade(&node)))));
        } else {
            Self::set_model(&element, model.clone());
        }
        this.bind_common(element.clone(), model, bindings);
        Some(element)
    }
    fn bind_common(
        this: &Self,
        element: Ref<Control>,
        model: RibbonItemModelReference,
        bindings: Rc<RibbonModelBindings>,
    ) {
        let m = model.clone();
        let e = element.clone();
        let explicit = Cell::new(false);
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                if let Some(item) = editable(&e) {
                    item.set_id(m.id());
                    item.set_label(m.label());
                    item.set_icon(m.icon().map(|v| Rc::new(v) as BoxedValue));
                    item.set_large_icon(m.large_icon().map(|v| Rc::new(v) as BoxedValue));
                    item.set_size(m.size());
                    item.set_size_definition(m.size_definition().map(|v| v.to_string().into()));
                    item.set_key_tip(m.key_tip());
                    item.set_screen_tip(
                        m.screen_tip()
                            .map(|v| Rc::new(v) as BoxedValue)
                            .or_else(|| text(m.description()).map(|v| Rc::new(v) as BoxedValue)),
                    );
                    item.set_command_id(m.command_id());
                    item.set_shortcut(m.shortcut());
                    item.set_simplified_visibility(m.simplified_visibility());
                    item.set_simplified_label(match m.show_label_in_simplified() {
                        Some(true) => RibbonSimplifiedLabel::Show,
                        Some(false) => RibbonSimplifiedLabel::Hide,
                        None => RibbonSimplifiedLabel::Auto,
                    });
                    item.set_show_label(m.show_label() && !e.is::<RibbonComboBox>());
                    item.set_can_add_to_quick_access(m.can_add_to_quick_access());
                }
                Self::apply_automation_id(&e, m.automation_id(), &explicit);
            })),
            Some(vec![]),
        );
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || e.set_is_visible(m.is_visible()))),
            Some(vec![Some("IsVisible".into())]),
        );
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || e.set_is_enabled(m.is_enabled()))),
            Some(vec![Some("IsEnabled".into())]),
        );
        if source(&element).is_some() && cast::<dyn RibbonGridPickerModelImpl>(&model).is_none() {
            let f = this.to_ref();
            let m = model.clone();
            let e = element.clone();
            this.bind_commands(
                &bindings,
                model.clone(),
                Rc::new(move || {
                    let s = source(&e).expect("native command source registration changed");
                    s.set_command(f.create_item_command(e.clone(), m.clone()));
                    if e.cast::<RibbonColorPicker>().is_none() {
                        s.set_command_parameter(m.command_parameter().map(box_command_parameter));
                    }
                }),
                vec![
                    Some("Command".into()),
                    Some("CommandId".into()),
                    Some("CommandParameter".into()),
                ],
            );
            super::ribbon_element_factory_parameters::watch_command(&element, &bindings);
        }
        Self::bind_keywords(element.clone(), model.clone(), &bindings);
        Self::watch_screen_tip(
            &bindings,
            model,
            Rc::new(move || {
                RibbonItemHelper::update_tool_tip(element.clone()).expect("factory tooltip")
            }),
        );
    }
    fn create_item_command(
        this: &Self,
        element: Ref<Control>,
        model: RibbonItemModelReference,
    ) -> Option<Rc<dyn ferroui_base::input::ICommand>> {
        let command = model
            .command()
            .or_else(|| this.resolve_command(model.command_id()))
            .map(adapt_core_command);
        super::ribbon_element_factory_parameters::item_command(&element, command)
    }
    fn create_button(
        _this: &Self,
        _model: Rc<dyn RibbonButtonModelImpl>,
        _bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        RibbonButton::new().upcast()
    }
    fn create_toggle_button(
        _this: &Self,
        model: Rc<dyn RibbonToggleButtonModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let element = RibbonToggleButton::new();
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_group_name(text(m.group_name()));
            })),
            Some(vec![Some("GroupName".into())]),
        );
        let m = model.clone();
        let e = element.clone();
        let mm = model.clone();
        let ee = element.clone();
        bindings.two_way(
            Some(model),
            Some("IsChecked".into()),
            Some(Rc::new(move || {
                e.set_is_checked(Some(m.is_checked()));
            })),
            Some(element.clone().upcast()),
            Some(ToggleButton::is_checked_property().as_property()),
            Some(Rc::new(move || {
                let m = &mm;
                let e = &ee;
                m.set_is_checked(e.is_checked() == Some(true));
            })),
        );
        element.upcast()
    }
    fn create_drop_down_button(
        this: &Self,
        model: Rc<dyn RibbonDropDownButtonModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let button = RibbonDropDownButton::new();
        let e = button.clone();
        this.bind_flyout(model, bindings, Rc::new(move |flyout| e.set_flyout(flyout)));
        button.upcast()
    }
    fn create_split_button(
        this: &Self,
        model: Rc<dyn RibbonSplitButtonModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let split = RibbonSplitButton::new();
        let e = split.clone();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_is_checkable(m.is_checkable());
                e.set_follow_last_choice(m.follow_last_choice());
            })),
            Some(vec![
                Some("IsCheckable".into()),
                Some("FollowLastChoice".into()),
            ]),
        );
        let e = split.clone();
        let m = model.clone();
        let ee = split.clone();
        let mm = model.clone();
        bindings.two_way(
            Some(model.clone()),
            Some("IsChecked".into()),
            Some(Rc::new(move || e.set_is_checked(Some(m.is_checked())))),
            Some(split.clone().upcast()),
            Some(RibbonSplitButton::is_checked_property().as_property()),
            Some(Rc::new(move || {
                mm.set_is_checked(ee.is_checked() == Some(true))
            })),
        );
        let e = split.clone();
        this.bind_flyout(model, bindings, Rc::new(move |flyout| e.set_flyout(flyout)));
        split.upcast()
    }
    fn create_flyout(
        this: &Self,
        entries: Option<ferroribbon_core::collections::RibbonReadOnlyList<RibbonNodeReference>>,
        content: Option<RibbonObject>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<ferroui_controls::primitives::FlyoutBase>> {
        this.build_flyout(entries, content, bindings)
    }
    fn create_menu_items(
        this: &Self,
        entries: Option<RibbonItemsSource<RibbonNodeReference>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> RibbonItemsSource<Ref<Control>> {
        this.build_menu_items(entries, bindings)
    }
    fn create_menu_item(
        this: &Self,
        item: Option<Rc<dyn RibbonMenuItemModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<crate::compat::MenuFlyoutItem> {
        this.build_menu_item(item, bindings)
    }
    fn create_menu_element(
        this: &Self,
        entry: RibbonNodeReference,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<Control>> {
        this.build_menu_element(entry, bindings)
    }
    fn create_menu_button(
        this: &Self,
        entry: RibbonNodeReference,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<Control>> {
        this.build_menu_button(entry, bindings)
    }
    fn create_check_box(
        _this: &Self,
        model: Rc<dyn RibbonCheckBoxModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let element = RibbonCheckBox::new();
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_is_three_state(m.is_three_state());
            })),
            Some(vec![Some("IsThreeState".into())]),
        );
        let m = model.clone();
        let e = element.clone();
        let mm = model.clone();
        let ee = element.clone();
        bindings.two_way(
            Some(model),
            Some("IsChecked".into()),
            Some(Rc::new(move || {
                e.set_is_checked(m.is_checked());
            })),
            Some(element.clone().upcast()),
            Some(ToggleButton::is_checked_property().as_property()),
            Some(Rc::new(move || {
                let m = &mm;
                let e = &ee;
                m.set_is_checked(e.is_checked());
            })),
        );
        element.upcast()
    }
    fn create_combo_box(
        _this: &Self,
        model: Rc<dyn RibbonComboBoxModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let combo = if model.preview_font_family() {
            RibbonFontComboBox::new().upcast::<RibbonComboBox>()
        } else {
            RibbonComboBox::new()
        };
        combo.items().clear();
        combo.set_items_source(Some(Rc::new(model.items())));
        let m = model.clone();
        let e = combo.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_is_editable(m.is_editable());
                e.set_input_width(m.input_width());
                e.set_placeholder_text(text(m.placeholder()));
                e.set_is_font_preview(m.preview_font_family());
                e.set_max_drop_down_height(m.max_drop_down_height());
                e.set_display_member_path(text(m.display_member_path()));
            })),
            Some(vec![
                Some("IsEditable".into()),
                Some("InputWidth".into()),
                Some("Placeholder".into()),
                Some("PreviewFontFamily".into()),
                Some("MaxDropDownHeight".into()),
                Some("DisplayMemberPath".into()),
            ]),
        );
        let m = model.clone();
        let e = combo.clone();
        let weak = Rc::downgrade(&bindings);
        bindings.watch(
            Some(model.clone()),
            Some(Rc::new(move || {
                if e.is::<RibbonFontComboBox>() != m.preview_font_family()
                    && let Some(b) = weak.upgrade()
                {
                    b.request_recreate();
                }
            })),
            Some(vec![Some("PreviewFontFamily".into())]),
        );
        let m = model.clone();
        let e = combo.clone();
        let mm = model.clone();
        let ee = combo.clone();
        bindings.two_way(
            Some(model.clone()),
            Some("SelectedItem".into()),
            Some(Rc::new(move || {
                e.set_selected_item(
                    crate::metadata::model_objects::to_native(m.selected_item())
                        .expect("Core combo SelectedItem"),
                )
            })),
            Some(combo.clone().upcast()),
            Some(RibbonComboBox::selected_item_property().as_property()),
            Some(Rc::new(move || {
                mm.set_selected_item(crate::metadata::model_objects::from_native_replacing(
                    ee.selected_item(),
                    || mm.selected_item(),
                ))
            })),
        );
        let m = model.clone();
        let e = combo.clone();
        let ee = combo.clone();
        bindings.two_way(
            Some(model.clone()),
            Some("Text".into()),
            Some(Rc::new(move || e.set_text(text(m.text())))),
            Some(combo.clone().upcast()),
            Some(RibbonComboBox::text_property().as_property()),
            Some(Rc::new(move || model.set_text(ee.text().map(Into::into)))),
        );
        combo.upcast()
    }
    fn create_spinner(
        _this: &Self,
        model: Rc<dyn RibbonSpinnerModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let element = RibbonSpinner::new();
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_minimum(m.minimum());
                e.set_maximum(m.maximum());
                e.set_increment(m.increment());
                e.set_format(text(m.format()));
                e.set_unit(text(m.unit()));
                e.set_input_width(m.input_width());
            })),
            Some(vec![
                Some("Minimum".into()),
                Some("Maximum".into()),
                Some("Increment".into()),
                Some("Format".into()),
                Some("Unit".into()),
                Some("InputWidth".into()),
            ]),
        );
        let m = model.clone();
        let e = element.clone();
        let mm = model.clone();
        let ee = element.clone();
        bindings.two_way(
            Some(model),
            Some("Value".into()),
            Some(Rc::new(move || {
                e.set_numeric_value(m.value());
            })),
            Some(element.clone().upcast()),
            Some(RibbonSpinner::value_property().as_property()),
            Some(Rc::new(move || {
                let m = &mm;
                let e = &ee;
                m.set_value(e.value());
            })),
        );
        element.upcast()
    }
    fn create_text_box(
        _this: &Self,
        model: Rc<dyn RibbonTextBoxModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let element = RibbonTextBox::new();
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_placeholder_text(text(m.placeholder()));
                e.set_input_width(m.input_width());
            })),
            Some(vec![Some("Placeholder".into()), Some("InputWidth".into())]),
        );
        let m = model.clone();
        let e = element.clone();
        let mm = model.clone();
        let ee = element.clone();
        bindings.two_way(
            Some(model),
            Some("Text".into()),
            Some(Rc::new(move || {
                e.set_text(Some(text(m.text()).unwrap_or_default()));
            })),
            Some(element.clone().upcast()),
            Some(RibbonTextBox::text_property().as_property()),
            Some(Rc::new(move || {
                let m = &mm;
                let e = &ee;
                m.set_text(e.text().map(Into::into));
            })),
        );
        element.upcast()
    }
    fn create_gallery(
        this: &Self,
        model: Rc<dyn RibbonGalleryModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        this.build_gallery(model, bindings)
    }
    fn create_color_picker(
        this: &Self,
        model: Rc<dyn RibbonColorPickerModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let picker = RibbonColorPicker::new();
        let m = model.clone();
        let p = picker.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                p.set_show_automatic(m.show_automatic());
                p.set_show_no_color(m.show_no_color());
                p.set_show_more_colors(m.show_more_colors());
                p.set_is_split(m.is_split());
                p.set_automatic_color(crate::theming::to_color(m.automatic_color()));
            })),
            Some(vec![
                Some("ShowAutomatic".into()),
                Some("ShowNoColor".into()),
                Some("ShowMoreColors".into()),
                Some("IsSplit".into()),
                Some("AutomaticColor".into()),
            ]),
        );
        this.bind_palette(
            Some(&picker.palette()),
            Some(model.clone()),
            Some(&bindings),
        );
        let m = model.clone();
        let p = picker.clone();
        let mm = model.clone();
        let pp = picker.clone();
        bindings.two_way(
            Some(model),
            Some("SelectedColor".into()),
            Some(Rc::new(move || {
                p.set_selected_color(m.selected_color().map(crate::theming::to_color))
            })),
            Some(picker.clone().upcast()),
            Some(RibbonColorPicker::selected_color_property().as_property()),
            Some(Rc::new(move || {
                mm.set_selected_color(pp.selected_color().map(crate::theming::to_ribbon_color))
            })),
        );
        picker.upcast()
    }
    fn create_custom(
        this: &Self,
        model: Rc<dyn RibbonCustomItemModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        this.build_custom(model, bindings)
    }
    fn create_grid_picker(
        this: &Self,
        model: Rc<dyn RibbonGridPickerModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let picker = RibbonGridPicker::new();
        let flyout = ferroui_controls::Flyout::new();
        flyout.set_content(Some(Control::boxed(picker.clone())));
        Self::apply_flyout_style(&flyout);
        let m = model.clone();
        let p = picker.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                p.set_rows(m.rows());
                p.set_columns(m.columns());
            })),
            Some(vec![Some("Rows".into()), Some("Columns".into())]),
        );
        let m = model.clone();
        let p = picker.clone();
        let f = this.to_ref();
        this.bind_commands(
            &bindings,
            model,
            Rc::new(move || {
                p.set_command(
                    m.command()
                        .or_else(|| f.resolve_command(m.command_id()))
                        .map(adapt_core_command),
                )
            }),
            vec![Some("Command".into()), Some("CommandId".into())],
        );
        let weak = flyout.downgrade();
        let handler = ferroui_base::metadata::MarkupDelegate::new(move |_| {
            if let Some(f) = weak.upgrade() {
                f.hide();
            }
            None
        });
        picker.add_size_picked(Some(handler.clone()));
        let p = picker.clone();
        bindings.add(Some(Rc::new(move || {
            p.remove_size_picked(Some(handler.clone()))
        })));
        let button = RibbonDropDownButton::new();
        button.set_flyout(Some(flyout.upcast()));
        button.upcast()
    }
    fn create_segmented(
        _this: &Self,
        model: Rc<dyn RibbonSegmentedModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let control = RibbonSegmentedControl::new();
        let syncing = Rc::new(Cell::new(false));
        let m = model.clone();
        let c = control.clone();
        let sync = syncing.clone();
        let apply_selection: RibbonAction = Rc::new(move || {
            let index = m
                .selected_segment()
                .and_then(|selected| m.segments().index_of(&Some(selected)))
                .map_or(-1, |i| i32::try_from(i).expect("Int32 collection Count"));
            if c.selected_index() != index {
                sync.set(true);
                let _finally = Reset(&sync);
                c.set_selected_index(index);
            }
        });
        let segment_scope: Rc<RefCell<Option<Rc<RibbonModelBindings>>>> =
            Rc::new(RefCell::new(None));
        let state = segment_scope.clone();
        let parent = Rc::downgrade(&bindings);
        let m = model.clone();
        let c = control.clone();
        let sync = syncing.clone();
        let apply = apply_selection.clone();
        bindings.collection(
            Some(Rc::new(model.segments())),
            Some(Rc::new(move || {
                let previous = state.borrow().clone();
                if let Some(previous) = previous {
                    previous.dispose();
                }
                let parent = parent.upgrade().expect("live segmented binding scope");
                let scope = parent.child();
                state.replace(Some(scope.clone()));
                sync.set(true);
                {
                    let _finally = Reset(&sync);
                    c.segments().clear();
                    for segment in m.segments().iter() {
                        let segment = segment.expect("NullReference: RibbonSegmentModel");
                        c.segments().add(Some(Self::create_segment(&segment)));
                        let captured = segment.clone();
                        let model = m.clone();
                        let control = c.clone();
                        scope.watch(
                            Some(segment),
                            Some(Rc::new(move || {
                                if let Some(index) =
                                    model.segments().index_of(&Some(captured.clone()))
                                    && index < control.segments().count()
                                {
                                    control
                                        .segments()
                                        .set(index, Some(Self::create_segment(&captured)));
                                }
                            })),
                            Some(vec![
                                Some("Label".into()),
                                Some("Icon".into()),
                                Some("Value".into()),
                            ]),
                        );
                    }
                }
                apply();
            })),
        );
        bindings.one_way(
            Some(model.clone()),
            Some(apply_selection),
            Some(vec![Some("SelectedSegment".into())]),
        );
        let m = model;
        let weak_control = control.downgrade();
        let weak_bindings = Rc::downgrade(&bindings);
        let token = control.add_property_changed_handler(Rc::new(move |e| {
            if e.property() != RibbonSegmentedControl::selected_index_property().as_property()
                || !e.is_effective_value_change()
            {
                return;
            }
            let Some(bindings) = weak_bindings.upgrade() else {
                return;
            };
            if syncing.get() || bindings.is_suspended() {
                return;
            }
            let Some(c) = weak_control.upgrade() else {
                return;
            };
            let index = c.selected_index();
            m.set_selected_segment(if index >= 0 && (index as usize) < m.segments().count() {
                m.segments().get(index as usize)
            } else {
                None
            });
        }));
        let c = control.clone();
        bindings.add(Some(Rc::new(move || {
            c.remove_property_changed_handler(token);
        })));
        control.upcast()
    }
    fn create_slider(
        _this: &Self,
        model: Rc<dyn RibbonSliderModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let element = RibbonSlider::new();
        let m = model.clone();
        let e = element.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                e.set_minimum(m.minimum());
                e.set_maximum(m.maximum());
                e.set_step_frequency(m.step_frequency());
                e.set_input_width(m.slider_width());
            })),
            Some(vec![
                Some("Minimum".into()),
                Some("Maximum".into()),
                Some("StepFrequency".into()),
                Some("SliderWidth".into()),
            ]),
        );
        let m = model.clone();
        let e = element.clone();
        let mm = model.clone();
        let ee = element.clone();
        bindings.two_way(
            Some(model),
            Some("Value".into()),
            Some(Rc::new(move || {
                e.set_numeric_value(m.value());
            })),
            Some(element.clone().upcast()),
            Some(RibbonSlider::value_property().as_property()),
            Some(Rc::new(move || {
                let m = &mm;
                let e = &ee;
                m.set_value(e.value());
            })),
        );
        element.upcast()
    }
}

impl RibbonElementFactory {
    pub(super) fn apply_flyout_style(flyout: &ferroui_controls::Flyout) {
        let app =
            ferroui_controls::Application::current().expect("NullReference: Application.Current");
        if let Some(value) = app
            .resources()
            .try_get_resource(&"RibbonFlyoutPresenterStyle".into(), None)
            .flatten()
        {
            let theme = ValueTypes::as_object(&*value)
                .and_then(|o| o.cast::<ferroui_base::styling::ControlTheme>())
                .expect("InvalidCast: RibbonFlyoutPresenterStyle");
            flyout.set_flyout_presenter_theme(theme);
        }
    }
}
