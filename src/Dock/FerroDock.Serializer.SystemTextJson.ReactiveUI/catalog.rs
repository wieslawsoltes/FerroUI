// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::properties::{self, fields};
use ferrodock_model::{collections::ListHandle, controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, core::DockWindow};
use ferrodock_serializer_system_text_json::{
    core_model_type_metadata, JsonDerivedType, JsonError, JsonErrorKind, JsonPropertyInfo,
    JsonTypeInfo, JsonUnknownDerivedTypeHandling, JsonValue,
};
use std::{any::TypeId, rc::Rc};
fn identity<T: ?Sized>(v: &Rc<T>) -> usize {
    Rc::as_ptr(v) as *const () as usize
}
pub(crate) fn concrete<T: 'static>(v: &Rc<dyn IDockable>) -> Option<Rc<T>> {
    v.clone().boxed_reference().downcast_ref::<Rc<T>>().cloned()
}
fn runtime(v: &Rc<dyn IDockable>) -> TypeId {
    v.clone().boxed_reference().as_ref().type_id()
}
pub(crate) fn dockable<T: IDockable + 'static>(
    create: impl Fn() -> Rc<T> + 'static,
    fields: Vec<JsonPropertyInfo<Rc<T>>>,
) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDockable>>(
        JsonTypeInfo::object(create, identity, fields),
        |v| v.clone(),
    )
}
pub(crate) fn dock<T: IDock + 'static>(
    create: impl Fn() -> Rc<T> + 'static,
    fields: Vec<JsonPropertyInfo<Rc<T>>>,
) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::with_reference_cast::<Rc<T>, Rc<dyn IDock>>(dockable(create, fields), |v| {
        v.clone()
    })
}
fn closed_enum<T: Copy + TryFrom<i32> + 'static>(
    bits: impl Fn(&T) -> i32 + 'static,
) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::converter(
        move |v, _| Ok(JsonValue::number(i64::from(bits(v)))),
        |value, _| {
            if let JsonValue::Number(number) = value {
                if let Ok(bits) = number.as_str().parse::<i32>() {
                    if let Ok(value) = T::try_from(bits) {
                        return Ok(Some(value));
                    }
                }
            }
            Err(DockSerializationError::new(JsonError {
                kind: JsonErrorKind::InvalidJson,
                message: "Unsupported docking enumeration value.".into(),
            }))
        },
    )
}
/// Supplies compiled descriptors for the standard ReactiveUI layout models and dependencies.
/// Properties follow original declaration order and omit transient/ignored members.
/// Application-specific classes need additional explicit descriptors and registrations.
/// Closed Rust layout enums currently accept their declared values only.
pub fn model_type_metadata() -> Vec<Rc<JsonTypeInfo>> {
    compiled_model_metadata(Vec::new())
}

pub(crate) fn compiled_model_metadata(
    models: Vec<crate::DockJsonModelRegistration>,
) -> Vec<Rc<JsonTypeInfo>> {
    let mut application_dockables = Vec::new();
    let mut application_docks = Vec::new();
    let mut application_roots = Vec::new();
    let mut application_metadata = Vec::new();
    for model in models {
        application_metadata.extend(model.metadata);
        application_dockables.push(model.dockable);
        application_docks.extend(model.dock);
        application_roots.extend(model.root);
    }
    let mut types = core_model_type_metadata();
    types.push(JsonTypeInfo::optional::<String>());
    let mut fields = fields!(DockDock; "LastChildFill" => last_child_fill, set_last_child_fill;);
    fields.extend(properties::concrete_dock::<DockDock>());
    let schema_dockdock = dock(DockDock::new, fields);
    types.push(schema_dockdock);
    types.push(JsonTypeInfo::optional::<Rc<DockDock>>());
    let mut fields = properties::mdi::<Document>();
    fields.extend(properties::concrete_dockable::<Document>());
    let schema_document = dockable(Document::new, fields);
    let schema_document = JsonTypeInfo::with_reference_cast::<Rc<Document>, Rc<dyn IDocument>>(
        schema_document,
        |v| v.clone(),
    );
    let schema_document = JsonTypeInfo::with_reference_cast::<Rc<Document>, Rc<dyn IMdiDocument>>(
        schema_document,
        |v| v.clone(),
    );
    types.push(schema_document);
    types.push(JsonTypeInfo::optional::<Rc<Document>>());
    let mut fields = properties::document_dock_own::<DocumentDock>();
    fields.extend(properties::concrete_dock::<DocumentDock>());
    let schema_documentdock = dock(DocumentDock::new, fields);
    let schema_documentdock = JsonTypeInfo::with_reference_cast::<
        Rc<DocumentDock>,
        Rc<dyn IDocumentDock>,
    >(schema_documentdock, |v| v.clone());
    types.push(schema_documentdock);
    types.push(JsonTypeInfo::optional::<Rc<DocumentDock>>());
    let mut fields = fields!(GridDock; "ColumnDefinitions" => column_definitions, set_column_definitions;"RowDefinitions" => row_definitions, set_row_definitions;);
    fields.extend(properties::concrete_dock::<GridDock>());
    let schema_griddock = dock(GridDock::new, fields);
    types.push(schema_griddock);
    types.push(JsonTypeInfo::optional::<Rc<GridDock>>());
    let mut fields =
        fields!(GridDockSplitter; "ResizeDirection" => resize_direction, set_resize_direction;);
    fields.extend(properties::concrete_dockable::<GridDockSplitter>());
    let schema_griddocksplitter = dockable(GridDockSplitter::new, fields);
    types.push(schema_griddocksplitter);
    types.push(JsonTypeInfo::optional::<Rc<GridDockSplitter>>());
    let mut fields = fields!(ProportionalDock; "Orientation" => orientation, set_orientation;);
    fields.extend(properties::concrete_dock::<ProportionalDock>());
    let schema_proportionaldock = dock(ProportionalDock::new, fields);
    types.push(schema_proportionaldock);
    types.push(JsonTypeInfo::optional::<Rc<ProportionalDock>>());
    let mut fields = fields!(ProportionalDockSplitter; "CanResize" => can_resize, set_can_resize;"ResizePreview" => resize_preview, set_resize_preview;);
    fields.extend(properties::concrete_dockable::<ProportionalDockSplitter>());
    let schema_proportionaldocksplitter = dockable(ProportionalDockSplitter::new, fields);
    types.push(schema_proportionaldocksplitter);
    types.push(JsonTypeInfo::optional::<Rc<ProportionalDockSplitter>>());
    let mut fields = properties::root_own::<RootDock>();
    fields.extend(properties::concrete_dock::<RootDock>());
    let schema_rootdock = dock(RootDock::new, fields);
    let schema_rootdock = JsonTypeInfo::with_reference_cast::<Rc<RootDock>, Rc<dyn IRootDock>>(
        schema_rootdock,
        |v| v.clone(),
    );
    types.push(schema_rootdock);
    types.push(JsonTypeInfo::optional::<Rc<RootDock>>());
    let mut fields = fields!(SplitViewDock; "CompactPaneLength" => compact_pane_length, set_compact_pane_length;"DisplayMode" => display_mode, set_display_mode;"IsPaneOpen" => is_pane_open, set_is_pane_open;"OpenPaneLength" => open_pane_length, set_open_pane_length;"PanePlacement" => pane_placement, set_pane_placement;"UseLightDismissOverlayMode" => use_light_dismiss_overlay_mode, set_use_light_dismiss_overlay_mode;"PaneDockable" => pane_dockable, set_pane_dockable;"ContentDockable" => content_dockable, set_content_dockable;);
    fields.extend(properties::concrete_dock::<SplitViewDock>());
    let schema_splitviewdock = dock(SplitViewDock::new, fields);
    types.push(schema_splitviewdock);
    types.push(JsonTypeInfo::optional::<Rc<SplitViewDock>>());
    let mut fields = fields!(StackDock; "Orientation" => orientation, set_orientation;"Spacing" => spacing, set_spacing;);
    fields.extend(properties::concrete_dock::<StackDock>());
    let schema_stackdock = dock(StackDock::new, fields);
    types.push(schema_stackdock);
    types.push(JsonTypeInfo::optional::<Rc<StackDock>>());
    let mut fields = properties::mdi::<Tool>();
    fields.extend(properties::concrete_dockable::<Tool>());
    let schema_tool = dockable(Tool::new, fields);
    let schema_tool =
        JsonTypeInfo::with_reference_cast::<Rc<Tool>, Rc<dyn ITool>>(schema_tool, |v| v.clone());
    let schema_tool =
        JsonTypeInfo::with_reference_cast::<Rc<Tool>, Rc<dyn IDocument>>(schema_tool, |v| {
            v.clone()
        });
    let schema_tool =
        JsonTypeInfo::with_reference_cast::<Rc<Tool>, Rc<dyn IMdiDocument>>(schema_tool, |v| {
            v.clone()
        });
    types.push(schema_tool);
    types.push(JsonTypeInfo::optional::<Rc<Tool>>());
    let mut fields = properties::tool_dock_own::<ToolDock>();
    fields.extend(properties::concrete_dock::<ToolDock>());
    let schema_tooldock = dock(ToolDock::new, fields);
    let schema_tooldock = JsonTypeInfo::with_reference_cast::<Rc<ToolDock>, Rc<dyn IToolDock>>(
        schema_tooldock,
        |v| v.clone(),
    );
    types.push(schema_tooldock);
    types.push(JsonTypeInfo::optional::<Rc<ToolDock>>());
    let mut fields =
        fields!(UniformGridDock; "Rows" => rows, set_rows;"Columns" => columns, set_columns;);
    fields.extend(properties::concrete_dock::<UniformGridDock>());
    let schema_uniformgriddock = dock(UniformGridDock::new, fields);
    types.push(schema_uniformgriddock);
    types.push(JsonTypeInfo::optional::<Rc<UniformGridDock>>());
    let mut fields = fields!(WrapDock; "Orientation" => orientation, set_orientation;);
    fields.extend(properties::concrete_dock::<WrapDock>());
    let schema_wrapdock = dock(WrapDock::new, fields);
    types.push(schema_wrapdock);
    types.push(JsonTypeInfo::optional::<Rc<WrapDock>>());
    let window = JsonTypeInfo::with_reference_cast::<Rc<DockWindow>, Rc<dyn IDockWindow>>(
        JsonTypeInfo::object(
            DockWindow::new,
            identity,
            properties::window::<DockWindow>(),
        ),
        |v| v.clone(),
    );
    types.push(window);
    types.push(JsonTypeInfo::optional::<Rc<DockWindow>>());
    let fallback = JsonTypeInfo::abstract_object(
        identity::<dyn IDockable>,
        properties::dockable::<dyn IDockable>(),
    );
    types.push(JsonTypeInfo::polymorphic_with_policy::<Rc<dyn IDockable>>(
        vec![
            JsonDerivedType::new::<Rc<DockDock>>(
                "Dock.Model.ReactiveUI.Controls.DockDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<Document>>(
                "Dock.Model.ReactiveUI.Controls.Document",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<DocumentDock>>(
                "Dock.Model.ReactiveUI.Controls.DocumentDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<GridDock>>(
                "Dock.Model.ReactiveUI.Controls.GridDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<GridDockSplitter>>(
                "Dock.Model.ReactiveUI.Controls.GridDockSplitter",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<ProportionalDock>>(
                "Dock.Model.ReactiveUI.Controls.ProportionalDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<ProportionalDockSplitter>>(
                "Dock.Model.ReactiveUI.Controls.ProportionalDockSplitter",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<RootDock>>(
                "Dock.Model.ReactiveUI.Controls.RootDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<SplitViewDock>>(
                "Dock.Model.ReactiveUI.Controls.SplitViewDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<StackDock>>(
                "Dock.Model.ReactiveUI.Controls.StackDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<Tool>>(
                "Dock.Model.ReactiveUI.Controls.Tool",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<ToolDock>>(
                "Dock.Model.ReactiveUI.Controls.ToolDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<UniformGridDock>>(
                "Dock.Model.ReactiveUI.Controls.UniformGridDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
            JsonDerivedType::new::<Rc<WrapDock>>(
                "Dock.Model.ReactiveUI.Controls.WrapDock",
                concrete,
                |v| v as Rc<dyn IDockable>,
            ),
        ]
        .into_iter()
        .chain(application_dockables)
        .collect(),
        Some(fallback),
        true,
        JsonUnknownDerivedTypeHandling::FallBackToBaseType,
        runtime,
    ));
    let mut fallback_fields = properties::dock_own::<dyn IDock>();
    fallback_fields.remove(4); // IsActive is ignored by the original interface-wide union.
    fallback_fields.extend(properties::dockable::<dyn IDock>());
    let fallback = JsonTypeInfo::abstract_object(identity::<dyn IDock>, fallback_fields);
    types.push(JsonTypeInfo::polymorphic_with_policy::<Rc<dyn IDock>>(
        vec![
            JsonDerivedType::new::<Rc<DockDock>>(
                "Dock.Model.ReactiveUI.Controls.DockDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<DocumentDock>>(
                "Dock.Model.ReactiveUI.Controls.DocumentDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<GridDock>>(
                "Dock.Model.ReactiveUI.Controls.GridDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<ProportionalDock>>(
                "Dock.Model.ReactiveUI.Controls.ProportionalDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<RootDock>>(
                "Dock.Model.ReactiveUI.Controls.RootDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<SplitViewDock>>(
                "Dock.Model.ReactiveUI.Controls.SplitViewDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<StackDock>>(
                "Dock.Model.ReactiveUI.Controls.StackDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<ToolDock>>(
                "Dock.Model.ReactiveUI.Controls.ToolDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<UniformGridDock>>(
                "Dock.Model.ReactiveUI.Controls.UniformGridDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
            JsonDerivedType::new::<Rc<WrapDock>>(
                "Dock.Model.ReactiveUI.Controls.WrapDock",
                |v: &Rc<dyn IDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
                |v| v as Rc<dyn IDock>,
            ),
        ]
        .into_iter()
        .chain(application_docks)
        .collect(),
        Some(fallback),
        true,
        JsonUnknownDerivedTypeHandling::FallBackToBaseType,
        |v| runtime(&(v.clone() as Rc<dyn IDockable>)),
    ));
    let mut root_fields = properties::root_own::<dyn IRootDock>();
    let mut dock_fields = properties::dock_own::<dyn IRootDock>();
    dock_fields.remove(4);
    root_fields.extend(dock_fields);
    root_fields.extend(properties::dockable::<dyn IRootDock>());
    let fallback = JsonTypeInfo::abstract_object(identity::<dyn IRootDock>, root_fields);
    types.push(JsonTypeInfo::polymorphic_with_policy::<Rc<dyn IRootDock>>(
        vec![JsonDerivedType::new::<Rc<RootDock>>(
            "Dock.Model.ReactiveUI.Controls.RootDock",
            |v: &Rc<dyn IRootDock>| concrete(&(v.clone() as Rc<dyn IDockable>)),
            |v| v as Rc<dyn IRootDock>,
        )]
        .into_iter()
        .chain(application_roots)
        .collect(),
        Some(fallback),
        true,
        JsonUnknownDerivedTypeHandling::FallBackToNearestAncestor,
        |v| runtime(&(v.clone() as Rc<dyn IDockable>)),
    ));
    let fallback = JsonTypeInfo::abstract_object(
        identity::<dyn IDockWindow>,
        properties::window::<dyn IDockWindow>(),
    );
    types.push(
        JsonTypeInfo::polymorphic_with_policy::<Rc<dyn IDockWindow>>(
            vec![JsonDerivedType::new::<Rc<DockWindow>>(
                "Dock.Model.ReactiveUI.Core.DockWindow",
                |v: &Rc<dyn IDockWindow>| {
                    v.clone()
                        .boxed_reference()
                        .downcast_ref::<Rc<DockWindow>>()
                        .cloned()
                },
                |v| v as Rc<dyn IDockWindow>,
            )],
            Some(fallback),
            true,
            JsonUnknownDerivedTypeHandling::FallBackToBaseType,
            |v| v.clone().boxed_reference().as_ref().type_id(),
        ),
    );
    let mut tool_fields = properties::tool_dock_own::<dyn IToolDock>();
    tool_fields.extend(properties::dock_own::<dyn IToolDock>());
    tool_fields.extend(properties::dockable::<dyn IToolDock>());
    types.push(JsonTypeInfo::abstract_object(
        identity::<dyn IToolDock>,
        tool_fields,
    ));
    types.push(JsonTypeInfo::list::<Option<Rc<dyn IDockable>>>());
    types.push(JsonTypeInfo::optional::<
        ListHandle<Option<Rc<dyn IDockable>>>,
    >());
    types.push(JsonTypeInfo::list::<Option<Rc<dyn IDockWindow>>>());
    types.push(JsonTypeInfo::optional::<
        ListHandle<Option<Rc<dyn IDockWindow>>>,
    >());
    types.push(JsonTypeInfo::optional::<Rc<dyn IDockable>>());
    types.push(JsonTypeInfo::optional::<Rc<dyn IDock>>());
    types.push(JsonTypeInfo::optional::<Rc<dyn IRootDock>>());
    types.push(JsonTypeInfo::optional::<Rc<dyn IToolDock>>());
    types.push(closed_enum::<Alignment>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<Alignment>());
    types.push(closed_enum::<GripMode>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<GripMode>());
    types.push(closed_enum::<Orientation>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<Orientation>());
    types.push(closed_enum::<GridResizeDirection>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<GridResizeDirection>());
    types.push(closed_enum::<SplitViewDisplayMode>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<SplitViewDisplayMode>());
    types.push(closed_enum::<SplitViewPanePlacement>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<SplitViewPanePlacement>());
    types.push(closed_enum::<DocumentTabLayout>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<DocumentTabLayout>());
    types.push(closed_enum::<DocumentLayoutMode>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<DocumentLayoutMode>());
    types.push(closed_enum::<DocumentCloseButtonShowMode>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<DocumentCloseButtonShowMode>());
    types.push(closed_enum::<MdiWindowState>(|v| *v as i32));
    types.push(JsonTypeInfo::optional::<MdiWindowState>());
    types.extend(application_metadata);
    types
}
