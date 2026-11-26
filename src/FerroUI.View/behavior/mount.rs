use super::{ActionKind, Behavior, Trigger};
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement};
use ferroui_controls::Control;
use ferroui_xaml_interactions::{
    CallMethodAction, ChangePropertyAction, DataTriggerBehavior, EventTriggerBehavior,
    InvokeCommandAction,
};
use ferroui_xaml_interactions_custom::{AddClassAction, RemoveClassAction};
use ferroui_xaml_interactivity::{
    ActionCollection, BehaviorCollection, Interaction, InvokeCommandActionBase,
};
use std::rc::Rc;

pub(super) fn install(owner: &Ref<Control>, behaviors: Rc<[Behavior]>) {
    if owner.is_attached_to_logical_tree() {
        let root = owner
            .get_self_and_logical_ancestors()
            .last()
            .expect("the owner is in its scope");
        attach(owner, &root, &behaviors);
    }
    let weak = owner.downgrade();
    owner
        .attached_to_logical_tree(move |args| {
            if let Some(owner) = weak.upgrade() {
                attach(&owner, args.root(), &behaviors);
            }
        })
        .detach();
    let weak = owner.downgrade();
    owner
        .detached_from_logical_tree(move |_| {
            if let Some(owner) = weak.upgrade() {
                Interaction::set_behaviors(&owner, None);
            }
        })
        .detach();
}

fn attach(owner: &Ref<Control>, root: &StyledElement, behaviors: &[Behavior]) {
    let scope = Scope { owner, root };
    let collection = BehaviorCollection::new();
    for behavior in behaviors {
        if let Some(behavior) = scope.behavior(behavior) {
            collection.add(behavior);
        }
    }
    Interaction::set_behaviors(owner, Some(collection));
}

struct Scope<'a> {
    owner: &'a Ref<Control>,
    root: &'a StyledElement,
}

impl Scope<'_> {
    fn target(&self, name: Option<&str>) -> Option<Option<Ref<StyledElement>>> {
        let Some(name) = name else { return Some(None) };
        if let Some(target) = NameScopeExtensions::find_name_scope(self.owner)
            .and_then(|scope| scope.find_as::<StyledElement>(name))
        {
            return Some(Some(target));
        }
        let mut matches = self
            .root
            .get_self_and_logical_descendants()
            .filter(|control| control.name().as_deref() == Some(name));
        if let Some(target) = matches.next() {
            if matches.next().is_none() {
                return Some(Some(target));
            }
        }
        crate::diagnostics::report(crate::Diagnostic::BehaviorTarget { name: name.into() });
        None
    }

    fn bind(&self, target: &FerroObject, property: &'static FerroProperty, path: &str) {
        let owner = self.owner.clone().upcast::<FerroObject>();
        let binding = ReflectionBinding::new(path);
        if let Some(scope) = NameScopeExtensions::find_name_scope(self.owner) {
            binding.set_name_scope(Some(Rc::downgrade(&scope.0)));
        }
        target.bind_binding_with_anchor(property, &*binding, Some(&owner));
    }

    fn behavior(&self, behavior: &Behavior) -> Option<Ref<FerroObject>> {
        let actions = ActionCollection::new();
        for action in &behavior.actions {
            if let Some(action) = self.action(&action.0) {
                actions.add(action);
            }
        }
        let control = match &behavior.trigger {
            Trigger::Event { name, source } => {
                let source = self.target(source.as_deref())?;
                let trigger = EventTriggerBehavior::new();
                trigger.set_event_name(Some(name.clone()));
                trigger.set_source_object(object(source));
                trigger.set_actions(Some(actions));
                trigger.upcast()
            }
            Trigger::Data {
                path,
                comparison,
                value,
                revert_on_false,
            } => {
                let trigger = DataTriggerBehavior::new();
                trigger.set_comparison_condition(*comparison);
                trigger.set_trigger_value(Some(Rc::new(value.clone())));
                trigger.set_revert_on_false(*revert_on_false);
                trigger.set_actions(Some(actions));
                self.bind(
                    &trigger,
                    DataTriggerBehavior::binding_property().as_property(),
                    path,
                );
                trigger.upcast()
            }
        };
        Some(control)
    }

    fn action(&self, action: &ActionKind) -> Option<Ref<FerroObject>> {
        Some(match action {
            ActionKind::Property {
                target,
                property,
                value,
            } => {
                let action = ChangePropertyAction::new();
                action.set_target_object(object(self.target(target.as_deref())?));
                action.set_property_name(Some(property.clone()));
                action.set_action_value(Some(Rc::new(value.clone())));
                action.upcast()
            }
            ActionKind::Command { path, parameter } => {
                let action = InvokeCommandAction::new();
                action.set_command_parameter(
                    parameter
                        .as_ref()
                        .map(|value| Rc::new(value.clone()) as BoxedValue),
                );
                self.bind(
                    &action,
                    InvokeCommandActionBase::command_property().as_property(),
                    path,
                );
                action.upcast()
            }
            ActionKind::Method { target, name } => {
                let action = CallMethodAction::new();
                action.set_target_object(object(self.target(target.as_deref())?));
                action.set_method_name(Some(name.clone()));
                action.upcast()
            }
            ActionKind::Class {
                target,
                name,
                remove,
            } => {
                let target = self.target(target.as_deref())?;
                if *remove {
                    let action = RemoveClassAction::new();
                    action.set_styled_element(target);
                    action.set_class_name(Some(name.clone()));
                    action.upcast()
                } else {
                    let action = AddClassAction::new();
                    action.set_styled_element(target);
                    action.set_class_name(Some(name.clone()));
                    action.upcast()
                }
            }
        })
    }
}

fn object(element: Option<Ref<StyledElement>>) -> Option<BoxedValue> {
    element.map(|value| Rc::new(value.upcast::<FerroObject>()) as BoxedValue)
}
