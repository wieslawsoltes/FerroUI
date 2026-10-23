//! The contracts every UI-thread composition object of the WinUI port
//! implements the same way, as a macro: what deriving from
//! `CompositionObject` gives a class.

/// Implements the composition object contracts of a class that holds a
/// `this: Weak<Self>` and an `object: CompositionObject`:
/// `IRegisterForSerialization`, `ICompositionObjectHost`,
/// `ICompositionObjectAnimations`, `AsCompositionObject`,
/// `ICompositionObject` and `ICompositorSerializable`.
///
/// `serialize` writes the changes of the class (the generated
/// `serialize_changes_core` of its property block), `start_animation`
/// connects an animation with a property (the generated `start_animation`),
/// `composition_property` resolves a property name (the generated
/// `get_composition_property` of the server block).
macro_rules! composition_object_impls {
    (
        $ty:ty, $type_name:literal,
        serialize: |$ser_this:ident, $writer:ident| $serialize:expr,
        start_animation: |$start_this:ident, $property_name:ident, $animation:ident, $final_value:ident| $start:expr,
        composition_property: |$prop_this:ident, $prop_name:ident| $prop:expr $(,)?
    ) => {
        impl $crate::rendering::composition::transport::IRegisterForSerialization for $ty {
            fn register_for_serialization(&self) {
                self.object.register_for_serialization(|| {
                    self.this.upgrade().map(|this| {
                        this as ::std::rc::Rc<dyn $crate::rendering::composition::ICompositorSerializable>
                    })
                });
            }
        }

        impl $crate::rendering::composition::ICompositionObjectHost for $ty {
            fn server(&self) -> $crate::rendering::composition::server::ServerObjectId {
                self.object.required_server()
            }

            fn pending_animations(&self) -> &$crate::rendering::composition::PendingAnimations {
                self.object.pending_animations()
            }

            fn implicit_animation(
                &self,
                property_name: &str,
            ) -> Option<::std::rc::Rc<dyn $crate::rendering::composition::animations::ICompositionAnimationBase>> {
                self.object.implicit_animation(property_name)
            }

            fn start_animation_group(
                &self,
                grp: &::std::rc::Rc<dyn $crate::rendering::composition::animations::ICompositionAnimationBase>,
                target: &str,
                final_value: $crate::rendering::composition::expressions::ExpressionVariant,
            ) -> bool {
                $crate::rendering::composition::ICompositionObjectAnimations::start_animation_group_for(
                    self,
                    &**grp,
                    target,
                    final_value,
                )
            }
        }

        impl $crate::rendering::composition::ICompositionObjectAnimations for $ty {
            fn try_start_animation(
                &self,
                property_name: &str,
                animation: &dyn $crate::rendering::composition::animations::ICompositionAnimation,
                final_value: Option<$crate::rendering::composition::expressions::ExpressionVariant>,
            ) -> bool {
                let $start_this = self;
                let $property_name = property_name;
                let $animation = animation;
                let $final_value = final_value;
                $start
            }

            fn get_composition_property(
                &self,
                property_name: &str,
            ) -> Option<&'static $crate::rendering::composition::server::CompositionProperty> {
                let $prop_this = self;
                let $prop_name = property_name;
                $prop
            }

            fn composition_object(&self) -> &$crate::rendering::composition::CompositionObject {
                &self.object
            }
        }

        impl $crate::rendering::composition::AsCompositionObject for $ty {
            fn as_composition_object(&self) -> &$crate::rendering::composition::CompositionObject {
                &self.object
            }

            fn composition_type_name(&self) -> &'static str {
                $type_name
            }
        }

        impl $crate::rendering::composition::ICompositionObject for $ty {
            fn server(&self) -> $crate::rendering::composition::server::ServerObjectId {
                self.object.required_server()
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn into_any_rc(self: ::std::rc::Rc<Self>) -> ::std::rc::Rc<dyn ::std::any::Any> {
                self
            }
        }

        impl $crate::rendering::composition::ICompositorSerializable for $ty {
            fn try_get_server(
                &self,
                c: &$crate::rendering::composition::Compositor,
            ) -> Option<$crate::rendering::composition::server::ServerObjectId> {
                self.object.try_get_server(c)
            }

            fn serialize_changes(
                &self,
                c: &$crate::rendering::composition::Compositor,
                writer: &mut $crate::rendering::composition::transport::BatchStreamWriter<'_>,
            ) {
                self.object.begin_serialize_changes(c);
                let $ser_this = self;
                let $writer = writer;
                $serialize
            }
        }
    };
}

pub(crate) use composition_object_impls;
