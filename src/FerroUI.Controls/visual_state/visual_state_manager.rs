//! WinUI's `VisualStateManager` (`Microsoft.UI.Xaml.VisualStateManager`;
//! `dxaml/xcp/dxaml/lib/VisualStateManager_Partial.cpp`, `dxaml/xcp/core/core/elements/VisualStateManager.cpp`
//! and the engine `dxaml/xcp/components/vsm/CVisualStateManager2.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! An addition (docs/porting/winui-calculator-gaps.md, section 5): upstream has no visual state
//! manager. It runs only for elements that have visual state groups, beside the pseudo-class
//! styles, whose behaviour it does not change.

use super::{VisualState, VisualStateChangedEventArgs, VisualStateGroup};
use crate::{ContentControl, ContentPage, Control};
use ferroui_base::collections::FerroList;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, AttachedProperty, FerroObject, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Ref,
};

/// The groups of visual states of an element (WinUI's `IVector<VisualStateGroup>`).
pub type VisualStateGroupCollection = FerroList<Ref<VisualStateGroup>>;

/// Manages the visual states of controls and the transitions between them.
///
/// The visual states of an element are the [`visual_state_groups_property`](Self::visual_state_groups_property)
/// attached to the root of its visual states: for a templated control the root of its template,
/// for a user control (or any content control, or content page) whose template has none, the
/// control that is its content. [`go_to_state`](Self::go_to_state) makes a state the current state of its group.
///
/// A custom manager derives from this class, overrides
/// [`go_to_state_core`](VisualStateManagerImpl::go_to_state_core) and is set on the root with
/// [`set_custom_visual_state_manager`](Self::set_custom_visual_state_manager).
#[repr(C)]
pub struct VisualStateManager {
    base: FerroObject,
}

ferro_class! {
    VisualStateManager: FerroObject, virtuals VisualStateManagerImpl: FerroObjectImpl {
        /// Changes the state of `group` to `state` (WinUI's overridable `GoToStateCore`).
        ///
        /// `template_root` is the element that has the groups; `group` and `state` are the
        /// group and the state named `state_name`, or `None` when no group has a state of that
        /// name (a custom manager is called either way). The base method changes the state when
        /// both are given and returns whether it did.
        fn go_to_state_core(
            this,
            control: &Ref<Control>,
            template_root: &Ref<Control>,
            state_name: &str,
            group: Option<&Ref<VisualStateGroup>>,
            state: Option<&Ref<VisualState>>,
            use_transitions: bool,
        ) -> bool;
    }
}
ferro_class_info!(VisualStateManager {
    new: VisualStateManager::new,
    markup: {
        namespace: "FerroUI.Controls",
        methods: [
            // The getter creates the collection, so that property element syntax adds the groups
            // to it (it replaces the plain accessor projected for the registered property).
            static fn GetVisualStateGroups(Ref<Control>) -> VisualStateGroupCollection =>
                |element: Ref<Control>| VisualStateManager::get_visual_state_groups(&element),
            static fn SetVisualStateGroups(Ref<Control>, Option<VisualStateGroupCollection>) =>
                |element: Ref<Control>, value: Option<VisualStateGroupCollection>| {
                    VisualStateManager::set_visual_state_groups(&element, value)
                },
            static fn GetCustomVisualStateManager(Ref<Control>) -> Option<Ref<VisualStateManager>> =>
                |element: Ref<Control>| VisualStateManager::get_custom_visual_state_manager(&element),
            static fn SetCustomVisualStateManager(Ref<Control>, Option<Ref<VisualStateManager>>) =>
                |element: Ref<Control>, value: Option<Ref<VisualStateManager>>| {
                    VisualStateManager::set_custom_visual_state_manager(&element, value)
                },
        ],
    },
});

impl FerroObjectImpl for VisualStateManager {}

impl VisualStateManagerImpl for VisualStateManager {
    fn go_to_state_core(
        _this: &Self,
        control: &Ref<Control>,
        template_root: &Ref<Control>,
        _state_name: &str,
        group: Option<&Ref<VisualStateGroup>>,
        state: Option<&Ref<VisualState>>,
        use_transitions: bool,
    ) -> bool {
        match (group, state) {
            (Some(group), Some(state)) => Self::change_state(control, template_root, group, Some(state), use_transitions),
            _ => false,
        }
    }
}

ferro_properties! { impl VisualStateManager {
    /// Defines the attached `VisualStateGroups` property: the groups of visual states of an
    /// element. [`get_visual_state_groups`](VisualStateManager::get_visual_state_groups)
    /// creates the collection when the element has none, which is what markup's property
    /// element syntax (`<VisualStateManager.VisualStateGroups>`) adds the groups to.
    pub fn visual_state_groups_property() -> AttachedProperty<Option<VisualStateGroupCollection>> {
        FerroProperty::register_attached::<VisualStateManager, Control, Option<VisualStateGroupCollection>>(
            "VisualStateGroups",
            None,
        )
    }

    /// Defines the attached `CustomVisualStateManager` property: the manager that changes the
    /// states of the groups of the element it is set on.
    pub fn custom_visual_state_manager_property() -> AttachedProperty<Option<Ref<VisualStateManager>>> {
        FerroProperty::register_attached::<VisualStateManager, Control, Option<Ref<VisualStateManager>>>(
            "CustomVisualStateManager",
            None,
        )
    }
} }

impl VisualStateManager {
    fn static_constructor() {
        Self::visual_state_groups_property()
            .changed()
            .add_class_handler::<Control>(Self::on_visual_state_groups_changed);
    }

    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The groups of visual states of `element`; an empty collection is created and set when the
    /// element has none (WinUI's `GetVisualStateGroups`).
    pub fn get_visual_state_groups(element: &Control) -> VisualStateGroupCollection {
        if let Some(groups) = element.get_value(Self::visual_state_groups_property()) {
            return groups;
        }
        let groups = VisualStateGroupCollection::new();
        element.set_value(Self::visual_state_groups_property(), Some(groups.clone()));
        groups
    }

    /// Sets the groups of visual states of `element`.
    pub fn set_visual_state_groups(element: &Control, value: Option<VisualStateGroupCollection>) {
        element.set_value(Self::visual_state_groups_property(), value);
    }

    /// The custom manager set on `element`, if any.
    pub fn get_custom_visual_state_manager(element: &Control) -> Option<Ref<VisualStateManager>> {
        element.get_value(Self::custom_visual_state_manager_property())
    }

    /// Sets the custom manager of the groups of `element`.
    pub fn set_custom_visual_state_manager(element: &Control, value: Option<Ref<VisualStateManager>>) {
        element.set_value(Self::custom_visual_state_manager_property(), value);
    }

    /// Makes the state named `state_name` the current state of its group (WinUI's `GoToState`).
    ///
    /// The groups are those of the root of the visual states of `control` (see the type). Returns
    /// `false`, and changes nothing, when there is no such root or no group has a state of that
    /// name (names are case-sensitive); a custom manager on the root decides instead. Entering
    /// the current state again with `use_transitions == false` applies its values again; with
    /// transitions it does nothing. Either way it returns `true`.
    pub fn go_to_state(control: &Control, state_name: &str, use_transitions: bool) -> bool {
        let control = control.to_ref();
        let Some(root) = Self::implementation_root(&control) else {
            return false;
        };
        let found = root
            .get_value(Self::visual_state_groups_property())
            .and_then(|groups| Self::find(&groups, state_name));
        let (group, state) = match &found {
            Some((group, state)) => (Some(group), Some(state)),
            None => (None, None),
        };
        match Self::get_custom_visual_state_manager(&root) {
            Some(custom) => custom.go_to_state_core(&control, &root, state_name, group, state, use_transitions),
            None => match (group, state) {
                (Some(group), Some(state)) => Self::change_state(&control, &root, group, Some(state), use_transitions),
                _ => false,
            },
        }
    }

    /// Raises `CurrentStateChanging` of `group` (for derived managers; WinUI's protected
    /// `RaiseCurrentStateChanging`).
    pub fn raise_current_state_changing(
        &self,
        group: &Ref<VisualStateGroup>,
        old_state: Option<Ref<VisualState>>,
        new_state: Option<Ref<VisualState>>,
        control: Option<Ref<Control>>,
    ) {
        group.raise_current_state_changing(&VisualStateChangedEventArgs::new(old_state, new_state, control));
    }

    /// Raises `CurrentStateChanged` of `group` (for derived managers; WinUI's protected
    /// `RaiseCurrentStateChanged`).
    pub fn raise_current_state_changed(
        &self,
        group: &Ref<VisualStateGroup>,
        old_state: Option<Ref<VisualState>>,
        new_state: Option<Ref<VisualState>>,
        control: Option<Ref<Control>>,
    ) {
        group.raise_current_state_changed(&VisualStateChangedEventArgs::new(old_state, new_state, control));
    }

    /// The root of the visual states of `control`: the first visual child (the root of the
    /// template, WinUI's `ImplementationRoot`) when it has groups or a custom manager, else the
    /// content of a content control when that is a control with groups.
    ///
    /// Deviation (docs/porting/winui-controls.md, VisualStateManager): in WinUI a user control has
    /// no template and its content is its first child; FerroUI's user control is templated, so its
    /// content is looked at too.
    pub(crate) fn implementation_root(control: &Ref<Control>) -> Option<Ref<Control>> {
        let first_child = control
            .visual_children()
            .snapshot()
            .first()
            .and_then(|child| child.cast::<Control>());
        if let Some(first_child) = &first_child {
            if Self::has_states(first_child) {
                return Some(first_child.clone());
            }
        }
        let content = if let Some(content_control) = control.downcast_ref::<ContentControl>() {
            content_control.content()
        } else if let Some(page) = control.downcast_ref::<ContentPage>() {
            page.content()
        } else {
            None
        };
        if let Some(content) = content.and_then(|content| Control::from_boxed(&content)).filter(Self::has_states) {
            return Some(content);
        }
        first_child
    }

    fn has_states(element: &Ref<Control>) -> bool {
        element.get_value(Self::visual_state_groups_property()).is_some_and(|groups| groups.count() > 0)
            || Self::get_custom_visual_state_manager(element).is_some()
    }

    fn find(groups: &VisualStateGroupCollection, state_name: &str) -> Option<(Ref<VisualStateGroup>, Ref<VisualState>)> {
        groups
            .snapshot()
            .iter()
            .find_map(|group| group.find_state(state_name).map(|state| (group.clone(), state)))
    }

    /// Changes the state of a group (WinUI's `CVisualStateManager2::GoToStateOptimizedImpl` and
    /// `VisualStateManagerActuator::ChangeVisualState`).
    pub(crate) fn change_state(
        control: &Ref<Control>,
        template_root: &Ref<Control>,
        group: &Ref<VisualStateGroup>,
        state: Option<&Ref<VisualState>>,
        use_transitions: bool,
    ) -> bool {
        let old_state = group.current_state();
        if old_state.as_ref() == state {
            if !use_transitions {
                group.reapply_current_state(Some(template_root));
            }
            return true;
        }
        let args = VisualStateChangedEventArgs::new(old_state, state.cloned(), Some(control.clone()));
        Self::raise_queued(group, args.clone(), true);
        group.set_state(state, Some(template_root));
        Self::raise_queued(group, args, false);
        true
    }

    /// Raises an event of a group on the dispatcher, as WinUI's event manager raises them
    /// (asynchronously, in order).
    fn raise_queued(group: &Ref<VisualStateGroup>, args: VisualStateChangedEventArgs, changing: bool) {
        let group = group.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if changing {
                    group.raise_current_state_changing(&args);
                } else {
                    group.raise_current_state_changed(&args);
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    /// The groups of an element were replaced: the values of the groups it had are removed, and
    /// the triggers of the new ones are followed.
    fn on_visual_state_groups_changed(element: &Control, change: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(Some(old)) = change.get_old_value::<Option<VisualStateGroupCollection>>() {
            for group in old.snapshot().iter() {
                group.detach();
            }
        }
        let new = change.get_new_value::<Option<VisualStateGroupCollection>>();
        super::state_triggers::install(element, new.as_ref());
    }
}
