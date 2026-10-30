// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiled generated metadata consumer over actual original engine models.
extern crate self as ferrodock_serializer_generated_tests;
include!(concat!(env!("OUT_DIR"), "/generated_json.rs"));
mod payload_models;
/// Native model view used by independent compiled receiver-projection consumers.
pub mod receiver_models;
/// Public name for the original fixture model retained by the native view.
pub use payload_models::Payload as ReceiverPayload;
include!("json_registration.rs");
/// Generated context over the original application's custom object payload schemas.
pub mod payloads {
    include!(concat!(env!("OUT_DIR"), "/payloads.rs"));
}
/// Independent generated consumer with a readonly typed Result property.
pub mod readonly_probe {
    include!(concat!(env!("OUT_DIR"), "/readonly_probe.rs"));
}
/// Independent generated consumer retaining a configured callback's captured state.
pub mod stateful_probe {
    include!(concat!(env!("OUT_DIR"), "/stateful_probe.rs"));
}
/// Generated serialization through native class-view accessors on original model handles.
pub mod receiver_probe {
    include!(concat!(env!("OUT_DIR"), "/receiver_probe.rs"));
}
/// Generated extraction of the same model returned by an original constructor callback.
pub mod receiver_constructor_probe {
    include!(concat!(env!("OUT_DIR"), "/receiver_constructor_probe.rs"));
}
/// Generated receiver projection retaining an original accessor's error handle.
pub mod receiver_error_probe {
    include!(concat!(env!("OUT_DIR"), "/receiver_error_probe.rs"));
}
#[cfg(test)]
mod tests {
    use super::*;
    use ferrodock_model::{controls::*, core::*};
    use ferrodock_model_reactiveui::{
        controls::{Document, DocumentDock, RootDock},
        Factory,
    };
    use ferrodock_serializer_system_text_json::JsonListType;
    use std::rc::Rc;
    #[test]
    fn emitted_receiver_projection_matches_source_wire_aliases_and_live_original_models() {
        let original = payloads::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let projected = receiver_probe::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let wire = include_str!("../data/serializer-json-audit.generated.holder.json");
        let model = original
            .deserialize::<Rc<crate::payload_models::Holder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(projected.serialize(model).unwrap(), wire);
        let read = projected
            .deserialize::<Rc<crate::payload_models::Holder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(projected.serialize(read.clone()).unwrap(), wire);
        let typed = read.typed.borrow().clone().unwrap();
        assert!(Rc::ptr_eq(&typed, &typed.own.borrow().clone().unwrap()));
        for name in ["first", "second"] {
            let value = crate::payload_models::Payload::new();
            value.name.replace(Some(name.into()));
            assert!(projected.serialize(value.clone()).unwrap().contains(name));
            value.name.replace(Some("changed".into()));
            assert!(projected.serialize(value).unwrap().contains("changed"));
        }
    }
    #[test]
    fn emitted_receiver_constructor_extracts_same_original_instance_and_invokes_once() {
        assert!(include_str!("../data/serializer-json-audit.accessors.json")
            .contains("constructor-same-instance"));
        let serializer =
            receiver_constructor_probe::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let first = serializer
            .deserialize::<Rc<crate::payload_models::Payload>>(Some("{}"))
            .unwrap()
            .unwrap();
        assert_eq!(first.name.borrow().as_deref(), Some("1"));
        let second = serializer
            .deserialize::<Rc<crate::payload_models::Payload>>(Some("{}"))
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(second.name.borrow().as_deref(), Some("2"));
    }
    #[test]
    fn emitted_receiver_projection_preserves_original_error_handle_across_operations() {
        assert!(include_str!("../data/serializer-json-audit.accessors.json")
            .contains("persistent-error"));
        let serializer =
            receiver_error_probe::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let model = crate::payload_models::Payload::new();
        let first = serializer.serialize(model.clone()).unwrap_err();
        let second = serializer.serialize(model).unwrap_err();
        assert!(first.ptr_eq(&second));
        assert_eq!(first.to_string(), "original-accessor-failure");
    }
    #[test]
    fn emitted_mixed_accessor_constructs_original_callback_once_and_retains_its_state() {
        assert!(include_str!("../data/serializer-json-audit.accessors.json")
            .contains("persistent-getter"));
        let serializer = stateful_probe::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let value = crate::payload_models::Payload::new();
        let first = serializer.serialize(value.clone()).unwrap();
        let second = serializer.serialize(value).unwrap();
        assert!(!first.contains("\"Self\""));
        assert!(second.contains("\"Self\"") && second.contains("\"$ref\""));
    }
    #[test]
    fn emitted_fallible_readonly_accessor_keeps_original_values_and_skips_input_setters() {
        let wire = include_str!("../data/serializer-json-audit.generated.named-root.json");
        let model = crate::payload_models::TypeNamed::new();
        model.kind.replace(Some("payload-type".into()));
        model.name.replace(Some("named".into()));
        let readonly = readonly_probe::DockSystemTextJsonGenerated::create_serializer().unwrap();
        assert_eq!(readonly.serialize(model).unwrap(), wire);
        let read = readonly
            .deserialize::<Rc<crate::payload_models::TypeNamed>>(Some("{\"Name\":\"ignored\"}"))
            .unwrap()
            .unwrap();
        assert!(read.kind.borrow().is_none());
        assert!(read.name.borrow().is_none());
    }
    #[test]
    fn emitted_metadata_roundtrips_original_model_projection_and_initializes_the_same_engine() {
        let serializer = DockSystemTextJsonGenerated::create_serializer().unwrap();
        let wire = include_str!("../data/serializer-json-audit.model.root.json");
        let root = serializer
            .deserialize::<Rc<RootDock>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
        let floating = root.windows().unwrap().get(0).unwrap();
        assert_eq!(floating.window_state().to_raw(), 123);
        assert_eq!(floating.owner_mode().to_raw(), -19);
        assert!(floating.topmost() && floating.is_modal());
        assert_eq!(floating.show_in_taskbar(), Some(false));
        assert!(Rc::ptr_eq(
            &floating.parent_window().unwrap(),
            &root.window().unwrap()
        ));
        let docs = root
            .visible_dockables()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<DocumentDock>>()
            .unwrap()
            .clone();
        let document = docs
            .visible_dockables()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<Document>>()
            .unwrap()
            .clone();
        assert_eq!(root.dock().to_raw(), -19);
        assert_eq!(root.pinned_dock_display_mode().to_raw(), 123);
        assert_eq!(root.floating_window_host_mode().to_raw(), -17);
        assert_eq!(docs.dock().to_raw(), i32::MIN);
        assert_eq!(document.dock().to_raw(), 123);
        assert!(root.pinned_dock_display_mode_override().is_none());
        assert_eq!(
            docs.pinned_dock_display_mode_override(),
            Some(PinnedDockDisplayMode::Inline)
        );
        assert_eq!(
            document
                .pinned_dock_display_mode_override()
                .map(|v| v.to_raw()),
            Some(i32::MIN)
        );
        assert!(Rc::ptr_eq(
            &document.owner().unwrap(),
            &(docs.clone() as Rc<dyn IDockable>)
        ));
        assert_eq!(document.proportion(), 0.125);
        assert_eq!(document.max_width(), f64::INFINITY);
        assert_eq!(document.docking_state(), DockingWindowState::DOCUMENT);
        assert_eq!(document.allowed_drop_operations().bits(), i32::MIN | 1);
        assert!(document
            .allowed_dock_operations()
            .allows(DockOperation::OuterLeft));
        assert!(!document
            .allowed_dock_operations()
            .allows(DockOperation::Right));
        assert!(document
            .allowed_drop_operations()
            .allows(DockOperation::Fill));
        assert!(!document
            .allowed_drop_operations()
            .allows(DockOperation::Window));
        let pinned = document.pinned_bounds().unwrap();
        assert_eq!(pinned.x.to_bits(), (-0.0f64).to_bits());
        assert_eq!(
            (pinned.y, pinned.width, pinned.height),
            (0.125, 512.5, 256.25)
        );
        assert!(Rc::ptr_eq(
            &root.root_dock_capability_policy().unwrap(),
            &docs.dock_capability_policy().unwrap()
        ));
        assert_eq!(
            document.dock_capability_overrides().unwrap().can_close(),
            Some(true)
        );
        let factory = Factory::new();
        factory.init_layout(root.clone());
        assert_eq!(document.dock().to_raw(), 123);
        assert_eq!(
            document
                .pinned_dock_display_mode_override()
                .map(|v| v.to_raw()),
            Some(i32::MIN)
        );
        assert_eq!(root.floating_window_host_mode().to_raw(), -17);
        assert!(Rc::ptr_eq(
            &document.factory().unwrap(),
            &(factory as Rc<dyn IFactory>)
        ));
        let capability = ferrodock_model::DockCapabilityResolver::evaluate(
            Some(document.clone()),
            DockCapability::Close,
        );
        assert!(capability.effective_value());
        assert_eq!(
            capability.effective_source(),
            DockCapabilityValueSource::DockableOverride
        );
        root.root_dock_capability_policy()
            .unwrap()
            .set_can_pin(Some(false));
        assert_eq!(
            docs.dock_capability_policy().unwrap().can_pin(),
            Some(false)
        );
        assert!(!ferrodock_model::DockCapabilityResolver::is_enabled(
            Some(document),
            DockCapability::Pin,
            Some(docs)
        ));
        root.root_dock_capability_policy()
            .unwrap()
            .set_can_pin(Some(true));
        assert_eq!(serializer.serialize(root.clone()).unwrap(), wire);
        assert!(Rc::ptr_eq(
            &root.window().unwrap().layout().unwrap(),
            &(root.clone() as Rc<dyn IRootDock>)
        ));
    }
    #[test]
    fn emitted_factory_list_overload_uses_the_configured_original_list_creator() {
        let serializer = DockSystemTextJsonGenerated::create_serializer_with_list_type(Some(
            JsonListType::list(),
        ))
        .unwrap();
        let wire = include_str!("../data/serializer-json-audit.model.root.json");
        let root = serializer
            .deserialize::<Rc<RootDock>>(Some(wire))
            .unwrap()
            .unwrap();
        assert!(root
            .visible_dockables()
            .unwrap()
            .as_observable_collection()
            .is_none());
        assert!(root.windows().unwrap().as_observable_collection().is_none());
        assert_eq!(serializer.serialize(root).unwrap(), wire);
        assert!(DockSystemTextJsonGenerated::create_serializer_with_list_type(None).is_err());
    }
    #[test]
    fn emitted_payload_metadata_preserves_source_inner_reference_scopes_and_closed_generics() {
        let serializer = payloads::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let wire = include_str!("../data/serializer-json-audit.generated.holder.json");
        let holder = serializer
            .deserialize::<Rc<crate::payload_models::Holder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(serializer.serialize(holder.clone()).unwrap(), wire);
        let first = holder
            .content
            .borrow()
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<crate::payload_models::Payload>>()
            .unwrap()
            .clone();
        let second = holder
            .other
            .borrow()
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<crate::payload_models::Payload>>()
            .unwrap()
            .clone();
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(Rc::ptr_eq(&first, first.own.borrow().as_ref().unwrap()));
        first.own.take();
        second.own.take();
        holder.typed.borrow().as_ref().unwrap().own.take();
        let wire = include_str!("../data/serializer-json-audit.generated.generic.json");
        let holder = serializer
            .deserialize::<Rc<crate::payload_models::Holder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(serializer.serialize(holder.clone()).unwrap(), wire);
        assert_eq!(
            *holder
                .content
                .borrow()
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<crate::payload_models::Generic<i32>>>()
                .unwrap()
                .value
                .borrow(),
            37
        );
    }
    #[test]
    fn emitted_custom_readonly_setonly_and_duplicate_property_converters_match_original_wire() {
        let serializer = payloads::DockSystemTextJsonGenerated::create_serializer().unwrap();
        let wire = include_str!("../data/serializer-json-audit.generated.custom.json");
        let custom = serializer
            .deserialize::<Rc<crate::payload_models::CustomHolder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(serializer.serialize(custom).unwrap(), wire);
        assert_eq!(
            serializer
                .serialize(crate::payload_models::ReadonlyHolder::new())
                .unwrap(),
            include_str!("../data/serializer-json-audit.generated.readonly.json")
        );
        let write=serializer.deserialize::<Rc<crate::payload_models::WriteOnlyHolder>>(Some("{\"Content\":{\"$type\":\"GeneratedPayload\",\"Name\":\"writeonly\"},\"Number\":37}")).unwrap().unwrap();
        assert_eq!(write.number.get(), 37);
        assert_eq!(
            write
                .content
                .borrow()
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<crate::payload_models::Payload>>()
                .unwrap()
                .name
                .borrow()
                .as_deref(),
            Some("writeonly")
        );
        assert_eq!(
            serializer.serialize(write).unwrap(),
            include_str!("../data/serializer-json-audit.generated.writeonly.json")
        );
        let wire = include_str!("../data/serializer-json-audit.generated.duplicate.json");
        let holder = serializer
            .deserialize::<Rc<crate::payload_models::Holder>>(Some(wire))
            .unwrap()
            .unwrap();
        assert_eq!(serializer.serialize(holder).unwrap(), wire);
        let holder = crate::payload_models::Holder::new();
        holder
            .content
            .replace(Some(Rc::new(Rc::new(crate::payload_models::ArrayPayload))));
        assert!(serializer.serialize(holder).is_err());
    }
}
