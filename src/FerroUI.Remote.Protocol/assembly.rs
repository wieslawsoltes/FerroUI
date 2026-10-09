//! The table of the exported types of an assembly: what the resolver of the
//! original gets from `Assembly.ExportedTypes` and from the identifier
//! attribute of each type.
//!
//! A crate that declares message classes of its own has such a table and
//! passes it to [`DefaultMessageTypeResolver::new`](crate::DefaultMessageTypeResolver::new).

use crate::design_messages::{StartDesignerSessionMessage, UpdateXamlMessage, UpdateXamlResultMessage};
use crate::ferro_remote_message_guid_attribute::{FerroRemoteMessage, FerroRemoteMessageGuidAttribute};
use crate::input_messages::{
    KeyEventMessage, PointerMovedEventMessage, PointerPressedEventMessage, PointerReleasedEventMessage,
    ScrollEventMessage, TextInputEventMessage,
};
use crate::metsys_bson::ClassType;
use crate::transport_messages::HtmlTransportStartedMessage;
use crate::viewport_messages::{
    ClientRenderInfoMessage, ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage,
    FrameReceivedMessage, MeasureViewportMessage, RequestViewportResizeMessage,
};

/// An exported type of an assembly with the identifier attribute it carries.
pub struct ExportedType {
    pub class: &'static ClassType,
    pub attribute: Option<FerroRemoteMessageGuidAttribute>,
}

impl ExportedType {
    /// The entry of a message class.
    pub const fn of<T: FerroRemoteMessage>() -> ExportedType {
        ExportedType { class: T::CLASS, attribute: Some(T::ATTRIBUTE) }
    }
}

/// An assembly, as far as a resolver reads one.
pub struct Assembly {
    pub name: &'static str,
    pub exported_types: &'static [ExportedType],
}

/// This library: every message class of the protocol.
pub static ASSEMBLY: Assembly = Assembly {
    name: "FerroUI.Remote.Protocol",
    exported_types: &[
        // ViewportMessages
        ExportedType::of::<MeasureViewportMessage>(),
        ExportedType::of::<ClientViewportAllocatedMessage>(),
        ExportedType::of::<RequestViewportResizeMessage>(),
        ExportedType::of::<ClientSupportedPixelFormatsMessage>(),
        ExportedType::of::<ClientRenderInfoMessage>(),
        ExportedType::of::<FrameReceivedMessage>(),
        ExportedType::of::<FrameMessage>(),
        // InputMessages
        ExportedType::of::<PointerMovedEventMessage>(),
        ExportedType::of::<PointerPressedEventMessage>(),
        ExportedType::of::<PointerReleasedEventMessage>(),
        ExportedType::of::<ScrollEventMessage>(),
        ExportedType::of::<KeyEventMessage>(),
        ExportedType::of::<TextInputEventMessage>(),
        // DesignMessages
        ExportedType::of::<UpdateXamlMessage>(),
        ExportedType::of::<UpdateXamlResultMessage>(),
        ExportedType::of::<StartDesignerSessionMessage>(),
        // TransportMessages
        ExportedType::of::<HtmlTransportStartedMessage>(),
    ],
};

// Tests of the port: the table stands for what reflection finds in the
// original, so it is compared with the list of the upstream message classes.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use super::*;
    use crate::default_message_type_resolver::DefaultMessageTypeResolver;
    use crate::design_messages::ExceptionDetails;
    use crate::guid::Guid;
    use crate::i_message_type_resolver::IMessageTypeResolver;
    use crate::input_messages::{InputEventMessageBase, InputModifiers, MouseButton, PointerEventMessageBase};
    use crate::key::Key;
    use crate::metsys_bson::{BinaryReader, BsonClass, BsonObject, Deserializer, Serializer, Type, Value, ValueRef};
    use crate::physical_key::PhysicalKey;
    use crate::viewport_messages::PixelFormat;

    /// A message as a connection carries one.
    type Message = Arc<dyn BsonObject>;

    /// Every class of the upstream library that carries the identifier
    /// attribute, with the text of the attribute as it is written there.
    const UPSTREAM_MESSAGES: &[(&str, &str)] = &[
        ("MeasureViewportMessage", "6E3C5310-E2B1-4C3D-8688-01183AA48C5B"),
        ("ClientViewportAllocatedMessage", "BD7A8DE6-3DB8-4A13-8583-D6D4AB189A31"),
        ("RequestViewportResizeMessage", "9B47B3D8-61DF-4C38-ACD4-8C1BB72554AC"),
        ("ClientSupportedPixelFormatsMessage", "63481025-7016-43FE-BADC-F2FD0F88609E"),
        ("ClientRenderInfoMessage", "7A3c25d3-3652-438D-8EF1-86E942CC96C0"),
        ("FrameReceivedMessage", "68014F8A-289D-4851-8D34-5367EDA7F827"),
        ("FrameMessage", "F58313EE-FE69-4536-819D-F52EDF201A0E"),
        ("PointerMovedEventMessage", "6228F0B9-99F2-4F62-A621-414DA2881648"),
        ("PointerPressedEventMessage", "7E9E2818-F93F-411A-800E-6B1AEB11DA46"),
        ("PointerReleasedEventMessage", "4ADC84EE-E7C8-4BCF-986C-DE3A2F78EDE4"),
        ("ScrollEventMessage", "79301A05-F02D-4B90-BB39-472563B504AE"),
        ("KeyEventMessage", "1C3B691E-3D54-4237-BFB0-9FEA83BC1DB8"),
        ("TextInputEventMessage", "C174102E-7405-4594-916F-B10B8248A17D"),
        ("UpdateXamlMessage", "9AEC9A2E-6315-4066-B4BA-E9A9EFD0F8CC"),
        ("UpdateXamlResultMessage", "B7A70093-0C5D-47FD-9261-22086D43A2E2"),
        ("StartDesignerSessionMessage", "854887CF-2694-4EB6-B499-7461B6FB96C7"),
        ("HtmlTransportStartedMessage", "53778004-78fa-4381-8ec3-176a6f2328b6"),
    ];

    fn modifiers() -> InputEventMessageBase {
        InputEventMessageBase { modifiers: Some(vec![InputModifiers::Control, InputModifiers::MiddleMouseButton]) }
    }

    fn pointer() -> PointerEventMessageBase {
        PointerEventMessageBase { base: modifiers(), x: 12.5, y: -3.25 }
    }

    /// One object of every message class, with no property at its default.
    fn samples() -> Vec<Message> {
        vec![
            Arc::new(MeasureViewportMessage { width: 100.5, height: 200.25 }),
            Arc::new(ClientViewportAllocatedMessage { width: 640.0, height: 480.0, dpi_x: 96.0, dpi_y: 192.0 }),
            Arc::new(RequestViewportResizeMessage { width: 1.5, height: 2.5 }),
            Arc::new(ClientSupportedPixelFormatsMessage {
                formats: Some(vec![PixelFormat::Bgra8888, PixelFormat::Rgba8888]),
            }),
            Arc::new(ClientRenderInfoMessage { dpi_x: 120.0, dpi_y: 144.0 }),
            Arc::new(FrameReceivedMessage { sequence_id: 1 << 40 }),
            Arc::new(FrameMessage {
                sequence_id: 42,
                format: PixelFormat::Bgra8888,
                data: Some(vec![1, 2, 3, 4, 0, 255]),
                width: 3,
                height: 2,
                stride: 12,
                dpi_x: 96.0,
                dpi_y: 97.0,
            }),
            Arc::new(PointerMovedEventMessage { base: pointer() }),
            Arc::new(PointerPressedEventMessage { base: pointer(), button: MouseButton::Middle }),
            Arc::new(PointerReleasedEventMessage { base: pointer(), button: MouseButton::Right }),
            Arc::new(ScrollEventMessage { base: pointer(), delta_x: 0.5, delta_y: -1.0 }),
            Arc::new(KeyEventMessage {
                base: modifiers(),
                is_down: true,
                key: Key::Escape,
                physical_key: PhysicalKey::Backquote,
                key_symbol: Some("`".to_string()),
            }),
            Arc::new(TextInputEventMessage { base: modifiers(), text: "text".to_string() }),
            Arc::new(UpdateXamlMessage {
                xaml: Some("<Window />".to_string()),
                assembly_path: Some("/bin/app".to_string()),
                xaml_file_project_path: Some("/Views/Main.xaml".to_string()),
            }),
            Arc::new(UpdateXamlResultMessage {
                error: Some("error".to_string()),
                handle: Some("1234".to_string()),
                exception: Some(ExceptionDetails {
                    exception_type: Some("Exception".to_string()),
                    message: Some("Here".to_string()),
                    line_number: Some(5),
                    line_position: Some(6),
                }),
            }),
            Arc::new(StartDesignerSessionMessage { session_id: Some("session".to_string()) }),
            Arc::new(HtmlTransportStartedMessage { uri: Some("http://127.0.0.1:5000/".to_string()) }),
        ]
    }

    fn round_trip(message: &Message) -> Value {
        let bytes = Serializer::serialize_object(ValueRef::Object(&**message)).unwrap();
        Deserializer::deserialize_type(&mut BinaryReader::new(&bytes), &Type::Class(message.get_type()), None).unwrap()
    }

    fn property_names(class: &ClassType) -> Vec<&'static str> {
        class.properties.iter().map(|p| p.name).collect()
    }

    #[test]
    fn every_upstream_message_class_is_in_the_table_with_its_identifier() {
        assert_eq!(UPSTREAM_MESSAGES.len(), 17);
        assert_eq!(ASSEMBLY.exported_types.len(), UPSTREAM_MESSAGES.len());
        for (name, guid) in UPSTREAM_MESSAGES {
            let exported = ASSEMBLY
                .exported_types
                .iter()
                .find(|t| t.class.name == *name)
                .unwrap_or_else(|| panic!("{} is not in the table", name));
            assert_eq!(exported.attribute.unwrap().guid(), Guid::parse(guid).unwrap(), "{}", name);
        }
        let guids: HashSet<Guid> = ASSEMBLY.exported_types.iter().map(|t| t.attribute.unwrap().guid()).collect();
        assert_eq!(guids.len(), UPSTREAM_MESSAGES.len());
    }

    #[test]
    fn the_resolver_maps_every_identifier_to_its_class_and_back() {
        let resolver = DefaultMessageTypeResolver::new(&[]);
        for (name, guid) in UPSTREAM_MESSAGES {
            let guid = Guid::parse(guid).unwrap();
            let class = resolver.get_by_guid(guid).unwrap();
            assert_eq!(class.name, *name);
            assert_eq!(resolver.get_guid(class).unwrap(), guid);
        }
        assert!(matches!(resolver.get_by_guid(Guid::EMPTY), Err(crate::Error::KeyNotFound(_))));
        assert!(matches!(resolver.get_guid(ExceptionDetails::CLASS), Err(crate::Error::KeyNotFound(_))));
        assert_eq!(MeasureViewportMessage::GUID, Guid::parse("6E3C5310-E2B1-4C3D-8688-01183AA48C5B").unwrap());
    }

    #[test]
    fn every_message_class_round_trips_with_values_that_are_not_the_defaults() {
        let samples = samples();
        let sampled: HashSet<&str> = samples.iter().map(|m| m.get_type().name).collect();
        let listed: HashSet<&str> = UPSTREAM_MESSAGES.iter().map(|(name, _)| *name).collect();
        assert_eq!(sampled, listed);
        for message in &samples {
            // A sample that equals the default of its class would prove nothing.
            let default = (message.get_type().create_instance)();
            let helper = crate::metsys_bson::TypeHelper::get_helper_for_type(message.get_type());
            for property in helper.get_properties() {
                assert_ne!(
                    property.getter(&**message).to_value(),
                    property.getter(&*default).to_value(),
                    "{}.{} of the sample is the default",
                    message.get_type().name,
                    property.name()
                );
            }
            match round_trip(message) {
                Value::Object(read) => assert!(read.equals(&**message), "{:?} was read as {:?}", message, read),
                other => panic!("{:?} was read as {:?}", message, other),
            }
        }
    }

    #[test]
    fn every_message_class_round_trips_with_its_defaults() {
        for exported in ASSEMBLY.exported_types {
            let message: Message = Arc::from((exported.class.create_instance)());
            match round_trip(&message) {
                Value::Object(read) => assert!(read.equals(&*message), "{:?} was read as {:?}", message, read),
                other => panic!("{:?} was read as {:?}", message, other),
            }
        }
    }

    #[test]
    fn properties_are_written_under_the_upstream_names_derived_class_first() {
        assert_eq!(property_names(MeasureViewportMessage::CLASS), ["Width", "Height"]);
        assert_eq!(property_names(ClientViewportAllocatedMessage::CLASS), ["Width", "Height", "DpiX", "DpiY"]);
        assert_eq!(property_names(RequestViewportResizeMessage::CLASS), ["Width", "Height"]);
        assert_eq!(property_names(ClientSupportedPixelFormatsMessage::CLASS), ["Formats"]);
        assert_eq!(property_names(ClientRenderInfoMessage::CLASS), ["DpiX", "DpiY"]);
        assert_eq!(property_names(FrameReceivedMessage::CLASS), ["SequenceId"]);
        assert_eq!(
            property_names(FrameMessage::CLASS),
            ["SequenceId", "Format", "Data", "Width", "Height", "Stride", "DpiX", "DpiY"]
        );
        assert_eq!(property_names(PointerMovedEventMessage::CLASS), ["X", "Y", "Modifiers"]);
        assert_eq!(property_names(PointerPressedEventMessage::CLASS), ["Button", "X", "Y", "Modifiers"]);
        assert_eq!(property_names(PointerReleasedEventMessage::CLASS), ["Button", "X", "Y", "Modifiers"]);
        assert_eq!(property_names(ScrollEventMessage::CLASS), ["DeltaX", "DeltaY", "X", "Y", "Modifiers"]);
        assert_eq!(property_names(KeyEventMessage::CLASS), ["IsDown", "Key", "PhysicalKey", "KeySymbol", "Modifiers"]);
        assert_eq!(property_names(TextInputEventMessage::CLASS), ["Text", "Modifiers"]);
        assert_eq!(property_names(UpdateXamlMessage::CLASS), ["Xaml", "AssemblyPath", "XamlFileProjectPath"]);
        assert_eq!(property_names(UpdateXamlResultMessage::CLASS), ["Error", "Handle", "Exception"]);
        assert_eq!(property_names(StartDesignerSessionMessage::CLASS), ["SessionId"]);
        assert_eq!(property_names(HtmlTransportStartedMessage::CLASS), ["Uri"]);
        assert_eq!(property_names(ExceptionDetails::CLASS), ["ExceptionType", "Message", "LineNumber", "LinePosition"]);
    }

    #[test]
    fn the_document_of_a_message_with_base_class_properties() {
        let message = PointerPressedEventMessage {
            base: PointerEventMessageBase { base: InputEventMessageBase { modifiers: None }, x: 1.0, y: 2.0 },
            button: MouseButton::Left,
        };
        let bytes = Serializer::serialize(&message).unwrap();
        assert_eq!(
            bytes,
            [
                0x32, 0, 0, 0, // the document
                0x10, 0x42, 0x75, 0x74, 0x74, 0x6F, 0x6E, 0, 1, 0, 0, 0, // Button
                0x01, 0x58, 0, 0, 0, 0, 0, 0, 0, 0xF0, 0x3F, // X
                0x01, 0x59, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, // Y
                0x0A, 0x4D, 0x6F, 0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x73, 0, // Modifiers
                0
            ]
        );
        assert_eq!(Deserializer::deserialize::<PointerPressedEventMessage>(&bytes, None).unwrap(), message);
    }

    #[test]
    fn the_document_of_a_message_with_null_properties() {
        let message = UpdateXamlResultMessage { error: None, handle: Some("h".to_string()), exception: None };
        let bytes = Serializer::serialize(&message).unwrap();
        assert_eq!(
            bytes,
            [
                0x25, 0, 0, 0, // the document
                0x0A, 0x45, 0x72, 0x72, 0x6F, 0x72, 0, // Error
                0x02, 0x48, 0x61, 0x6E, 0x64, 0x6C, 0x65, 0, 2, 0, 0, 0, 0x68, 0, // Handle
                0x0A, 0x45, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6F, 0x6E, 0, // Exception
                0
            ]
        );
        assert_eq!(Deserializer::deserialize::<UpdateXamlResultMessage>(&bytes, None).unwrap(), message);
    }

    #[test]
    fn exception_details_of_an_error() {
        let error = crate::Error::InvalidOperation("Here".to_string());
        let details = ExceptionDetails::from_exception("InvalidOperationException", &error, Some((5, 6)));
        assert_eq!(details.exception_type.as_deref(), Some("InvalidOperationException"));
        assert_eq!(details.message.as_deref(), Some("Here"));
        assert_eq!((details.line_number, details.line_position), (Some(5), Some(6)));
        let details = ExceptionDetails::from_exception("Error", &error, None);
        assert_eq!((details.line_number, details.line_position), (None, None));
        assert_eq!(ExceptionDetails::new(), ExceptionDetails::default());
    }
}
