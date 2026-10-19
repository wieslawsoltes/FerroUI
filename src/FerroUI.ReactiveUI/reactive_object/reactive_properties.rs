//! The macro [`reactive_properties!`](crate::reactive_properties): the notifying properties of a
//! view model, the counterpart of the `[Reactive]` attribute of ReactiveUI.SourceGenerators.
//!
//! Not an upstream file: upstream writes the setter by hand
//! (`set => this.RaiseAndSetIfChanged(ref field, value)`) or has a source generator write it.

/// Declares a view model's struct with its notifying properties
/// (ReactiveUI.SourceGenerators' `[Reactive]`).
///
/// The macro takes a struct declaration. Each field marked `#[reactive]` becomes a
/// `RefCell<Type>` field with two methods:
/// - a getter `name(&self) -> Type`, which clones the value (the type is `Clone`);
/// - a setter `set_name(&self, value: Type)`, which calls
///   [`raise_and_set_if_changed`](crate::IReactiveObjectExt::raise_and_set_if_changed) with the
///   name of the property in PascalCase (`search_term` is `SearchTerm`, the name that
///   [`prop!`](ferroui_system::prop) derives from the getter). The type is `PartialEq`; a shared
///   object compares by identity when its `PartialEq` does.
///
/// The other fields are kept as they are. `#[reactive]` comes before the other attributes of its
/// field; the struct is not generic. The accessors are `pub`, whatever the visibility of the
/// field. The struct holds a [`ReactiveObject`](crate::ReactiveObject) (or a
/// [`ReactiveRecord`](crate::ReactiveRecord)) and [`reactive_object!`](crate::reactive_object) is
/// invoked for it, which gives the setters the raising methods.
///
/// ```
/// use ferroui_reactiveui::{reactive_object, reactive_properties, IReactiveNotifyPropertyChanged, ReactiveObject};
/// use ferroui_reactive::ObservableExt;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// reactive_properties! {
///     /// The state of a search screen.
///     #[derive(Default)]
///     pub struct Search {
///         reactive: ReactiveObject,
///         #[reactive]
///         search_term: String,
///         #[reactive]
///         page: u32,
///         history: RefCell<Vec<String>>,
///     }
/// }
/// reactive_object!(Search);
///
/// let search = Rc::new(Search::default());
/// let changes = Rc::new(RefCell::new(Vec::new()));
/// let log = changes.clone();
/// let _subscription = search.changed().subscribe_next(move |change| log.borrow_mut().push(change.property_name));
///
/// search.set_search_term("reactive".to_owned());
/// search.set_page(2);
/// search.set_page(2);
/// search.history.borrow_mut().push(search.search_term());
///
/// assert_eq!(search.search_term(), "reactive");
/// assert_eq!(search.page(), 2);
/// assert_eq!(*changes.borrow(), ["SearchTerm", "Page"]);
/// ```
#[macro_export]
macro_rules! reactive_properties {
    (
        $(#[$struct_attr:meta])*
        $vis:vis struct $name:ident {
            $($body:tt)*
        }
    ) => {
        $crate::reactive_properties!(@field { $(#[$struct_attr])* } { $vis } $name [] [] $($body)*);
    };

    // A notifying property.
    (@field $attrs:tt $vis:tt $name:ident [$($fields:tt)*] [$($accessors:tt)*]
        #[reactive] $(#[$field_attr:meta])* $field_vis:vis $field:ident : $field_ty:ty $(, $($rest:tt)*)?
    ) => {
        $crate::reactive_properties!(@field $attrs $vis $name
            [$($fields)* $(#[$field_attr])* $field_vis $field: ::core::cell::RefCell<$field_ty>,]
            [$($accessors)* { $field : $field_ty }]
            $($($rest)*)?
        );
    };

    // Another field, kept as it is.
    (@field $attrs:tt $vis:tt $name:ident [$($fields:tt)*] [$($accessors:tt)*]
        $(#[$field_attr:meta])* $field_vis:vis $field:ident : $field_ty:ty $(, $($rest:tt)*)?
    ) => {
        $crate::reactive_properties!(@field $attrs $vis $name
            [$($fields)* $(#[$field_attr])* $field_vis $field: $field_ty,]
            [$($accessors)*]
            $($($rest)*)?
        );
    };

    // The struct and the accessors of its notifying properties.
    (@field { $($attrs:tt)* } { $vis:vis } $name:ident [$($fields:tt)*] [$({ $field:ident : $field_ty:ty })*]) => {
        $($attrs)*
        $vis struct $name {
            $($fields)*
        }

        $crate::__private::paste! {
            impl $name {
                $(
                    #[allow(dead_code)]
                    pub fn $field(&self) -> $field_ty {
                        ::core::clone::Clone::clone(&*self.$field.borrow())
                    }

                    #[allow(dead_code)]
                    pub fn [<set_ $field>](&self, value: $field_ty) {
                        let _ = $crate::IReactiveObjectExt::raise_and_set_if_changed(
                            self,
                            &self.$field,
                            value,
                            ::core::stringify!([<$field:camel>]),
                        );
                    }
                )*
            }
        }
    };
}
