//! Typed lists for markup: the declaration form of a list of items that markup can name,
//! add to, index and deliver to an items control.

/// Declares the notifying list of `Item` values (`FerroList<Item>`) to markup: the
/// counterpart of an `ObservableCollection<T>` / `IList<T>` property type of the managed
/// original.
///
/// ```ignore
/// // The lists of a view model:
/// ferro_markup_list!(pub CursorList: Rc<StandardCursorModel>);
///
/// ferro_markup_type!(class CursorPageViewModel {
///     ..
///     properties: [StandardCursors: FerroList<Rc<StandardCursorModel>> { get: .. }],
/// });
///
/// // In the crate's `register_types()`:
/// CursorList::register();
/// ```
///
/// The declaration states the element type, which is what the XAML compiler needs to
/// infer the data type of an item template from the collection its items control is bound
/// to (`[InheritDataTypeFromItems]`) and to type an indexer of a binding path. It
/// generates a carrier type named as given, with the markup metadata of the list (the
/// instantiation ``FerroList`1`` of the item type: a constructor, `Count`, `Add`, the
/// indexer) and `register()`, which makes the metadata known and lets bindings deliver the
/// list to an items source property on every thread.
#[macro_export]
macro_rules! ferro_markup_list {
    ($visibility:vis $carrier:ident : $item:ty) => {
        /// Carries the markup metadata of a typed list; see `ferro_markup_list!`.
        $visibility struct $carrier;

        ::ferroui_base::ferro_markup_type!(class $carrier as "FerroList`1" {
            namespace: "FerroUI.Collections",
            handles: [
                ::ferroui_base::collections::FerroList<$item>,
                ::std::option::Option<::ferroui_base::collections::FerroList<$item>>
            ],
            this: ::ferroui_base::collections::FerroList<$item>,
            generic: "FerroList`1" [$item],
            constructors: [() => ::ferroui_base::collections::FerroList::<$item>::new],
            properties: [
                Count: i32 { get: |list: &::ferroui_base::collections::FerroList<$item>| list.count() as i32 },
            ],
            methods: [
                fn Add($item) => |list: &::ferroui_base::collections::FerroList<$item>, item: $item| list.add(item),
            ],
            indexers: [
                (i32) -> $item {
                    try_get: |list: &::ferroui_base::collections::FerroList<$item>, index: i32| {
                        if index < 0 || index as usize >= list.count() {
                            return ::std::result::Result::Err(
                                "Index was out of range. Must be non-negative and less than the size of the collection.",
                            );
                        }
                        ::std::result::Result::Ok(list.get(index as usize))
                    }
                },
            ],
        });

        impl $carrier {
            /// Makes the list known to markup and deliverable to items source properties.
            /// Called from the `register_types()` of the declaring crate.
            $visibility fn register() {
                ::ferroui_base::metadata::MarkupType::register(
                    <$carrier as ::ferroui_base::metadata::MarkupTyped>::MARKUP,
                );
                ::ferroui_base::data::core::ValueTypes::register_global(|| {
                    ::ferroui_base::data::core::ValueTypes::register_nullable::<
                        ::ferroui_base::collections::FerroList<$item>,
                    >();
                    $crate::ItemsSource::register_binding_conversion_with::<
                        ::ferroui_base::collections::FerroList<$item>,
                    >(|list| ::std::rc::Rc::new(list.clone()).into());
                });
            }
        }
    };
}

// The lists of this crate that are delivered to items source properties.
ferro_markup_list!(pub(crate) PipsList: i32);

#[cfg(test)]
mod tests {
    use crate::ItemsSource;
    use ferroui_base::collections::FerroList;
    use ferroui_base::data::core::{ValueType, ValueTypes};
    use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupType, MarkupTyped};
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    #[derive(Debug, PartialEq)]
    struct ListTestItem(i32);

    ferro_markup_list!(ListTestItems: Rc<ListTestItem>);

    #[test]
    fn a_declared_list_states_its_item_type_and_members() {
        let markup = <ListTestItems as MarkupTyped>::MARKUP;
        let generic = markup.generic.expect("an instantiation");
        assert_eq!(generic.definition, "FerroList`1");
        assert_eq!((generic.arguments[0])(), ValueType::of::<Rc<ListTestItem>>());
        assert_eq!(markup.handle(), Some(ValueType::of::<FerroList<Rc<ListTestItem>>>()));

        let list = (markup.constructors[0].invoke)(&[]).expect("a list");
        let add = markup.find_methods("Add").next().expect("Add");
        (add.invoke)(&[list.clone(), into_markup_value(Rc::new(ListTestItem(7)))]).expect("added");
        let count = markup.find_property("Count").expect("Count");
        let counted = (count.get.expect("a getter"))(&[list.clone()]).expect("count");
        assert_eq!(from_markup_value::<i32>(&counted), Some(1));

        let indexer = &markup.indexers[0];
        let item = (indexer.get.expect("a getter"))(&[list.clone(), into_markup_value(0i32)]).expect("the item");
        assert_eq!(from_markup_value::<Rc<ListTestItem>>(&item).map(|item| item.0), Some(7));
        assert!((indexer.get.expect("a getter"))(&[list, into_markup_value(1i32)]).is_err());
    }

    #[test]
    fn a_registered_list_is_delivered_to_an_items_source_on_every_thread() {
        ListTestItems::register();
        assert!(MarkupType::find_by_handle(std::any::TypeId::of::<FerroList<Rc<ListTestItem>>>()).is_some());

        let deliver = || {
            let list = FerroList::<Rc<ListTestItem>>::new();
            list.add(Rc::new(ListTestItem(1)));
            let boxed: BoxedValue = Rc::new(list);
            let converted = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Option<ItemsSource>>());
            converted
                .flatten()
                .and_then(|value| value.downcast_ref::<Option<ItemsSource>>().cloned().flatten())
                .map(|source| source.count())
        };
        assert_eq!(deliver(), Some(1));
        assert_eq!(std::thread::spawn(deliver).join().expect("the thread"), Some(1));
    }
}
