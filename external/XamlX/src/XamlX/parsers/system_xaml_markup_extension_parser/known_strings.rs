//! Port of `Parsers/SystemXamlMarkupExtensionParser/KnownStrings.cs`.
// Licensed to the .NET Foundation under one or more agreements.
// The .NET Foundation licenses this file to you under the MIT license.

pub struct MeScannerKnownStrings;

#[allow(dead_code)]
impl MeScannerKnownStrings {
    // Built-in strings.
    pub const XML_PREFIX: &'static str = "xml";
    pub const XML_NS_PREFIX: &'static str = "xmlns";

    pub const PRESERVE: &'static str = "preserve";
    pub const DEFAULT: &'static str = "default";

    pub const URI_CLR_NAMESPACE: &'static str = "clr-namespace";
    pub const URI_ASSEMBLY: &'static str = "assembly";

    pub const STRING_TYPE: &'static str = "String";
    pub const OBJECT_TYPE: &'static str = "Object";

    pub const GET: &'static str = "Get";
    pub const SET: &'static str = "Set";
    pub const ADD: &'static str = "Add";
    pub const HANDLER: &'static str = "Handler";
    pub const EXTENSION: &'static str = "Extension";
    pub const IS_READ_ONLY: &'static str = "IsReadOnly";
    pub const SHOULD_SERIALIZE: &'static str = "ShouldSerialize";

    pub const FRAMEWORK_ELEMENT: &'static str = "FrameworkElement";
    pub const TYPE_EXTENSION: &'static str = "TypeExtension";

    pub const GRAVE_QUOTE: char = '`';
    pub const NESTED_TYPE_DELIMITER: char = '+';

    pub const GET_ENUMERATOR: &'static str = "GetEnumerator";
    pub const I_COLLECTION_OF_T: &'static str = "System.Collections.Generic.ICollection`1";
    pub const I_DICTIONARY: &'static str = "System.Collections.IDictionary";
    pub const I_DICTIONARY_OF_KT: &'static str = "System.Collections.Generic.IDictionary`2";

    pub const NULLABLE_OF_T: &'static str = "Nullable`1";
    pub const KEY_VALUE_PAIR_OF_TT: &'static str = "KeyValuePair`2";

    pub const AMBIENT_PROPERTY_ATTRIBUTE: &'static str = "AmbientPropertyAttribute";
    pub const DEPENDENCY_PROPERTY_SUFFIX: &'static str = "Property";

    pub const XPS_NAMESPACE: &'static str = "http://schemas.microsoft.com/xps/2005/06";

    pub const LOCAL_PREFIX: &'static str = "local";
    pub const DEFAULT_PREFIX: &'static str = "p";

    pub const REFERENCE_NAME: &'static str = "__ReferenceID";
    pub const WHITESPACE_CHARS: [char; 5] = [' ', '\t', '\n', '\r', '\u{000C}'];
    pub const SPACE_CHAR: char = ' ';
    pub const TAB_CHAR: char = '\t';
    pub const NEWLINE_CHAR: char = '\n';
    pub const RETURN_CHAR: char = '\r';

    pub const CLR_NAMESPACE_FORMAT: &'static str = "clr-namespace:{0};assembly={1}";
    pub const CREATE_DELEGATE_HELPER: &'static str = "_CreateDelegate";
    pub const CREATE_DELEGATE: &'static str = "CreateDelegate";
    pub const INVOKE_MEMBER: &'static str = "InvokeMember";
    pub const GET_TYPE_FROM_HANDLE: &'static str = "GetTypeFromHandle";

    pub const MEMBER: &'static str = "Member";
    pub const PROPERTY: &'static str = "Property";
}
