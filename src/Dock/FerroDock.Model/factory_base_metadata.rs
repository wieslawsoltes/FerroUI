// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    pub(crate) fn copy_document_extras(source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) {
        if let Some(source) = source.clone().as_document_dock_content() {
            if let Some(target) = target.clone().as_document_dock_content() {
                target.set_document_template(source.document_template());
            }
        }
        if let Some(source) = source.clone().as_items_source_dock() {
            if let Some(target) = target.clone().as_items_source_dock() {
                target.set_document_item_container_theme(source.document_item_container_theme());
                target
                    .set_document_item_template_selector(source.document_item_template_selector());
                target.set_can_update_items_source_on_unregister(
                    source.can_update_items_source_on_unregister(),
                );
            }
        }
        if let Some(source) = source.as_document_dock_factory() {
            if let Some(target) = target.as_document_dock_factory() {
                target.set_document_factory(source.document_factory());
            }
        }
    }

    pub(crate) fn copy_tool_extras(source: Rc<dyn IDockable>, target: Rc<dyn IDockable>) {
        if let Some(source) = source.clone().as_tool_dock_content() {
            if let Some(target) = target.clone().as_tool_dock_content() {
                target.set_tool_template(source.tool_template());
            }
        }
        if let Some(source) = source.as_tool_items_source_dock() {
            if let Some(target) = target.as_tool_items_source_dock() {
                target.set_tool_item_container_theme(source.tool_item_container_theme());
                target.set_tool_item_template_selector(source.tool_item_template_selector());
                target.set_can_update_items_source_on_unregister(
                    source.can_update_items_source_on_unregister(),
                );
            }
        }
    }
}
