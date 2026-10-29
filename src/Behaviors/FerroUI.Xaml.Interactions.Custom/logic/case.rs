//! Port of `Logic/Case.cs`.

use ferroui_base::collections::FerroList;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, StyledElementImplExt,
    StyledProperty,
};
use ferroui_xaml_interactivity::{ActionCollection, StyledElementAction};
use std::ops::Deref;

/// A case of a [`SwitchCaseAction`](crate::SwitchCaseAction) or a
/// [`SwitchCaseBehavior`](crate::SwitchCaseBehavior): the actions (the content of the case in
/// markup) executed when the switch value equals [`value`](Self::value).
///
/// While the case is in the logical tree its styled element actions are its logical children
/// (one way).
#[repr(C)]
pub struct Case {
    base: StyledElement,
}

ferro_class!(Case: StyledElement);
ferro_class_info!(Case {
    new: Case::new,
    markup: {
        content: Actions,
    },
});

ferro_properties! {
    impl Case {
        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Case, _>("Value", None)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<Case, _>("Actions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for Case {
        /// The constructor body: the case starts with an empty collection of actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::actions_property().as_property() {
                let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
                let new_actions = change.get_new_value::<Option<ActionCollection>>();

                if old_actions.is_some() && this.is_attached_to_logical_tree() {
                    attach_actions::detach(old_actions.as_ref(), this);
                }

                if new_actions.is_some() && this.is_attached_to_logical_tree() {
                    attach_actions::attach(new_actions.as_ref(), this);
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementImpl for Case {
        fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            Self::parent_on_attached_to_logical_tree(this, e);
            attach_actions::attach(this.actions().as_ref(), this);
        }

        fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            attach_actions::detach(this.actions().as_ref(), this);
            Self::parent_on_detached_from_logical_tree(this, e);
        }
    }
}

/// Not from the reference: the attachment of the styled element actions of a collection to the
/// logical tree of their parent, which each class of the folder writes as private methods
/// (`AttachActionsToLogicalTree`, `DetachActionsFromLogicalTree`).
pub(crate) mod attach_actions {
    use super::*;

    /// Makes the styled element actions of the collection logical children of `parent`.
    pub(crate) fn attach(actions: Option<&ActionCollection>, parent: &StyledElement) {
        let Some(actions) = actions else {
            return;
        };

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.attach_action_to_logical_tree(parent);
            }
        }
    }

    /// Removes the styled element actions of the collection from the logical tree of `parent`.
    pub(crate) fn detach(actions: Option<&ActionCollection>, parent: &StyledElement) {
        let Some(actions) = actions else {
            return;
        };

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.detach_action_from_logical_tree(parent);
            }
        }
    }
}

impl Case {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    /// Creates a case.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the value compared with the switch value. This is a property of the object
    /// model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value compared with the switch value (named apart from the setter of every
    /// property of the object, as the condition of the object model names its own).
    pub fn set_case_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets the actions executed when the case matches. This is a property of the object
    /// model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }
}

/// The cases of a [`SwitchCaseAction`](crate::SwitchCaseAction) or a
/// [`SwitchCaseBehavior`](crate::SwitchCaseBehavior): a list of [`Case`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference types the cases as the generic list of
/// the framework of `Case`; here they are a named list (as the class setters of the responsive
/// behaviors), so that markup can add the cases of a document to it.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct CaseCollection(FerroList<Ref<Case>>);

impl CaseCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<Case>> {
        &self.0
    }
}

impl Default for CaseCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for CaseCollection {
    type Target = FerroList<Ref<Case>>;

    fn deref(&self) -> &FerroList<Ref<Case>> {
        &self.0
    }
}

impl std::fmt::Debug for CaseCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CaseCollection({})", self.0.count())
    }
}

ferroui_base::ferro_markup_type!(class CaseCollection {
    handles: [CaseCollection, Option<CaseCollection>],
    constructors: [() => CaseCollection::new],
    properties: [
        Count: i32 { get: |list: &CaseCollection| list.count() as i32 },
    ],
    methods: [
        fn Add(Ref<Case>) => |list: &CaseCollection, item: Ref<Case>| list.add(item)
    ],
});

/// Not from the reference: the attachment of the cases of a list to the logical tree of their
/// switch, and the subscription to the changes of the list, which the two switch classes write
/// alike (`AttachCasesToLogicalTree`, `DetachCasesFromLogicalTree`, `OnCasesCollectionChanged`).
pub(crate) mod attach_cases {
    use super::*;
    use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};

    /// Makes the cases logical children of `parent`.
    pub(crate) fn attach(cases: &[Ref<Case>], parent: &StyledElement) {
        for case_item in cases {
            case_item.set_parent(parent.to_ref());
        }
    }

    /// Removes the cases from the logical tree.
    pub(crate) fn detach(cases: &[Ref<Case>]) {
        for case_item in cases {
            case_item.set_parent(None);
        }
    }

    /// `OnCasesCollectionChanged`: follows the additions, removals and replacements of the list
    /// while `parent` is in the logical tree.
    pub(crate) fn on_cases_collection_changed(parent: &StyledElement, e: &NotifyCollectionChangedEventArgs<'_, Ref<Case>>) {
        if !parent.is_attached_to_logical_tree() {
            return;
        }

        match e.action {
            NotifyCollectionChangedAction::Add => attach(e.new_items, parent),
            NotifyCollectionChangedAction::Remove => detach(e.old_items),
            NotifyCollectionChangedAction::Replace => {
                detach(e.old_items);
                attach(e.new_items, parent);
            }
            // The reference does not track the items to detach them on a reset.
            NotifyCollectionChangedAction::Reset | NotifyCollectionChangedAction::Move => {}
        }
    }
}
