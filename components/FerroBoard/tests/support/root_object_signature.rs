// Generated from unchanged .NET System.Object.ToString reflection.
// IsVirtual has no IXamlMethod counterpart; this signature row remains partial.
macro_rules! check_root_object_signature {
    ($method:expr) => {{
        let method = $method;
        assert_eq!(method.name(), "ToString");
        assert_eq!(method.declaring_type().full_name(), "System.Object");
        assert_eq!(method.return_type().full_name(), "System.String");
        assert_eq!(method.parameters().len(), 0);
        assert!(!method.is_static());
        assert!(method.is_public());
        assert!(!method.is_private());
        assert!(!method.is_family());
        assert!(!method.is_generic_method());
        assert!(!method.is_generic_method_definition());
        assert!(!method.contains_generic_parameters());
        11usize
    }};
}
