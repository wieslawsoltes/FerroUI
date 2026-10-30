// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Explicit source application model registrations for ordinary layout persistence.
use crate::services::ILayoutSerialization;
use crate::view_models::{
    docks::CustomDocumentDock,
    documents::DocumentViewModel,
    tools::*,
    views::{DashboardViewModel, HomeViewModel},
};
use ferrodock_model::core::SerializationResult;
use ferrodock_model::{controls::IRootDock, core::IDockSerializer};
use ferrodock_serializer_system_text_json::DockSerializer;
use ferrodock_serializer_system_text_json_reactiveui::{
    create_serializer_with_models, DockJsonModelRegistration as Model,
};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct LayoutJsonSerialization {
    serializer: Rc<DockSerializer>,
}
impl LayoutJsonSerialization {
    pub(crate) fn new() -> SerializationResult<Rc<Self>> {
        Ok(Rc::new(Self {
            serializer: create_layout_serializer()?,
        }))
    }
}
impl ILayoutSerialization for LayoutJsonSerialization {
    fn serialize(&self, layout: Option<Rc<dyn IRootDock>>) -> SerializationResult<String> {
        self.serializer.serialize(layout)
    }
    fn deserialize(&self, json: &str) -> SerializationResult<Option<Rc<dyn IRootDock>>> {
        self.serializer.deserialize(Some(json))
    }
    fn save_bytes(&self, layout: Option<Rc<dyn IRootDock>>) -> SerializationResult<Vec<u8>> {
        let stream = ferrodock_model::io::MemoryStream::new();
        self.serializer.save(Some(stream.clone()), layout)?;
        Ok(stream.to_array())
    }
    fn load_bytes(&self, bytes: Vec<u8>) -> SerializationResult<Option<Rc<dyn IRootDock>>> {
        let stream = ferrodock_model::io::MemoryStream::from_buffer(
            Rc::new(RefCell::new(bytes.into_boxed_slice())),
            false,
        );
        self.serializer.load(Some(stream))
    }
}

pub(crate) fn create_layout_serializer() -> SerializationResult<Rc<DockSerializer>> {
    create_serializer_with_models(vec![
        Model::document(
            "DockReactiveUISample.ViewModels.Documents.DocumentViewModel",
            DocumentViewModel::new,
            Vec::new(),
        ),
        Model::document_dock(
            "DockReactiveUISample.ViewModels.Docks.CustomDocumentDock",
            CustomDocumentDock::new,
            Vec::new(),
        ),
        Model::dock(
            "DockReactiveUISample.ViewModels.Views.DashboardViewModel",
            DashboardViewModel::new,
            Vec::new(),
        ),
        Model::root(
            "DockReactiveUISample.ViewModels.Views.HomeViewModel",
            HomeViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool1ViewModel",
            Tool1ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool2ViewModel",
            Tool2ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool3ViewModel",
            Tool3ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool4ViewModel",
            Tool4ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool5ViewModel",
            Tool5ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool6ViewModel",
            Tool6ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool7ViewModel",
            Tool7ViewModel::new,
            Vec::new(),
        ),
        Model::tool(
            "DockReactiveUISample.ViewModels.Tools.Tool8ViewModel",
            Tool8ViewModel::new,
            Vec::new(),
        ),
    ])
}
