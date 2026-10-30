// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Explicit application model registrations over original live model contracts.
use crate::{catalog, properties};
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::controls::*;
use ferrodock_serializer_system_text_json::{
    JsonDerivedType, JsonError, JsonErrorKind, JsonPropertyInfo, JsonTypeInfo,
};
use std::{any::TypeId, collections::HashSet, rc::Rc};

/// One explicitly configured application class and its original polymorphic families.
/// Constructors and accessors operate on the enclosing model's actual Rc identity.
/// Own properties precede inherited fields; callers must resolve hiding/overrides and
/// exclude ignored members before supplying these typed property descriptors.
pub struct DockJsonModelRegistration {
    name: String,
    type_id: TypeId,
    pub(crate) metadata: Vec<Rc<JsonTypeInfo>>,
    pub(crate) dockable: JsonDerivedType<Rc<dyn IDockable>>,
    pub(crate) dock: Option<JsonDerivedType<Rc<dyn IDock>>>,
    pub(crate) root: Option<JsonDerivedType<Rc<dyn IRootDock>>>,
}

impl DockJsonModelRegistration {
    fn from_dockable<T: IDockable + 'static>(name: String, info: Rc<JsonTypeInfo>) -> Self {
        Self {
            dockable: JsonDerivedType::new::<Rc<T>>(name.clone(), catalog::concrete, |v| {
                v as Rc<dyn IDockable>
            }),
            name,
            type_id: TypeId::of::<Rc<T>>(),
            metadata: vec![info, JsonTypeInfo::optional::<Rc<T>>()],
            dock: None,
            root: None,
        }
    }
    fn from_dock<T: IDock + 'static>(name: String, info: Rc<JsonTypeInfo>) -> Self {
        let mut registration = Self::from_dockable::<T>(name.clone(), info);
        registration.dock = Some(JsonDerivedType::new::<Rc<T>>(
            name,
            |v: &Rc<dyn IDock>| catalog::concrete(&(v.clone() as Rc<dyn IDockable>)),
            |v| v as Rc<dyn IDock>,
        ));
        registration
    }
    /// Registers a concrete application DockBase subclass in IDockable and IDock.
    pub fn dock<T: IDock + IDockableDockingRestrictions + IDockSelectorInfo + 'static>(
        name: impl Into<String>,
        create: impl Fn() -> Rc<T> + 'static,
        mut own_properties: Vec<JsonPropertyInfo<Rc<T>>>,
    ) -> Self {
        own_properties.extend(properties::concrete_dock::<T>());
        Self::from_dock::<T>(name.into(), catalog::dock(create, own_properties))
    }
    /// Registers an application Document subclass with its inherited MDI/dockable fields.
    pub fn document<
        T: IMdiDocument + IDockableDockingRestrictions + IDockSelectorInfo + 'static,
    >(
        name: impl Into<String>,
        create: impl Fn() -> Rc<T> + 'static,
        mut own_properties: Vec<JsonPropertyInfo<Rc<T>>>,
    ) -> Self {
        own_properties.extend(properties::mdi::<T>());
        own_properties.extend(properties::concrete_dockable::<T>());
        let info = catalog::dockable(create, own_properties);
        let info =
            JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDocument>>(info, |v| v.clone());
        let info =
            JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IMdiDocument>>(info, |v| v.clone());
        Self::from_dockable::<T>(name.into(), info)
    }
    /// Registers an application Tool subclass, including its Document/MDI reference views.
    pub fn tool<
        T: ITool + IMdiDocument + IDockableDockingRestrictions + IDockSelectorInfo + 'static,
    >(
        name: impl Into<String>,
        create: impl Fn() -> Rc<T> + 'static,
        mut own_properties: Vec<JsonPropertyInfo<Rc<T>>>,
    ) -> Self {
        own_properties.extend(properties::mdi::<T>());
        own_properties.extend(properties::concrete_dockable::<T>());
        let info = catalog::dockable(create, own_properties);
        let info = JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn ITool>>(info, |v| v.clone());
        let info =
            JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDocument>>(info, |v| v.clone());
        let info =
            JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IMdiDocument>>(info, |v| v.clone());
        Self::from_dockable::<T>(name.into(), info)
    }
    /// Registers an application DocumentDock subclass and retains its actual constructor.
    /// Ignored commands are reconstructed by that constructor rather than read from JSON.
    pub fn document_dock<
        T: IDocumentDock + IDockableDockingRestrictions + IDockSelectorInfo + 'static,
    >(
        name: impl Into<String>,
        create: impl Fn() -> Rc<T> + 'static,
        mut own_properties: Vec<JsonPropertyInfo<Rc<T>>>,
    ) -> Self {
        own_properties.extend(properties::document_dock_own::<T>());
        own_properties.extend(properties::concrete_dock::<T>());
        let info = JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDocumentDock>>(
            catalog::dock(create, own_properties),
            |v| v.clone(),
        );
        Self::from_dock::<T>(name.into(), info)
    }
    /// Registers an application RootDock subclass in all three original dock families.
    pub fn root<T: IRootDock + IDockableDockingRestrictions + IDockSelectorInfo + 'static>(
        name: impl Into<String>,
        create: impl Fn() -> Rc<T> + 'static,
        mut own_properties: Vec<JsonPropertyInfo<Rc<T>>>,
    ) -> Self {
        own_properties.extend(properties::root_own::<T>());
        own_properties.extend(properties::concrete_dock::<T>());
        let info = JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IRootDock>>(
            catalog::dock(create, own_properties),
            |v| v.clone(),
        );
        let name = name.into();
        let mut registration = Self::from_dock::<T>(name.clone(), info);
        registration.root = Some(JsonDerivedType::new::<Rc<T>>(
            name,
            |v: &Rc<dyn IRootDock>| catalog::concrete(&(v.clone() as Rc<dyn IDockable>)),
            |v| v as Rc<dyn IRootDock>,
        ));
        registration
    }
}

pub(crate) fn validate(models: &[DockJsonModelRegistration]) -> SerializationResult<()> {
    let mut names = HashSet::new();
    let mut types = HashSet::new();
    macro_rules! standard {
        ($($t:ty),* $(,)?) => {$(
            names.insert(concat!("Dock.Model.ReactiveUI.Controls.", stringify!($t)));
            types.insert(TypeId::of::<Rc<$t>>());
        )*};
    }
    standard!(
        DockDock,
        Document,
        DocumentDock,
        GridDock,
        GridDockSplitter,
        ProportionalDock,
        ProportionalDockSplitter,
        RootDock,
        SplitViewDock,
        StackDock,
        Tool,
        ToolDock,
        UniformGridDock,
        WrapDock
    );
    for model in models {
        if model.name.trim().is_empty()
            || !names.insert(&model.name)
            || !types.insert(model.type_id)
        {
            return Err(DockSerializationError::new(JsonError { kind: JsonErrorKind::Argument,
                message: "Application docking registrations require unique nonempty names and model types.".into() }));
        }
    }
    Ok(())
}
