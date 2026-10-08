//! The public paths the fixture states for its types.

ferroui_base::ferro_rust_paths! {
    classes: [
        crate::Border,
        crate::controls::grid::Grid,
    ],
    types: [crate::Dock],
    contracts: [crate::IBrush],
    generics: [(crate::Listed<::ferroui_base::Ref<crate::Border>>, "fixture::Listed<::ferroui_base::Ref<::fixture::Border>>")],
    generic_contracts: [],
}
