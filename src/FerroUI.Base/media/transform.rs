use crate::animation::Animatable;
use crate::media::immutable::ImmutableTransform;
use crate::media::MatrixTransform;
use crate::media::ref_adapter::RefAdapter;
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::drawing::{CompositorResourceHolder, ICompositionRenderResource};
use crate::rendering::composition::generated::ServerCompositionSimpleTransformProps;
use crate::rendering::composition::server::{IServerObject, ServerCompositionSimpleTransform, ServerObjectId};
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use crate::utilities::{FormatError, HandlerList};
use crate::{ferro_class, FerroObject, FerroObjectImpl, Matrix, Ref, Upcast};
use std::fmt;
use std::rc::Rc;

/// Represents a transform on a visual or brush. Abstract base class of the
/// mutable transforms.
#[repr(C)]
pub struct Transform {
    base: Animatable,
    changed: HandlerList<dyn Fn()>,
    resource: CompositorResourceHolder,
}

ferro_class! {
    Transform: Animatable, virtuals TransformImpl: FerroObjectImpl {
        /// The transform's matrix.
        fn value(this) -> Matrix;
    }
}
crate::ferro_class_info!(Transform { interfaces: [std::rc::Rc<dyn crate::media::ITransform>] });

crate::ferro_impl_classes!(Transform: FerroObjectImpl);

impl TransformImpl for Transform {
    fn value(_this: &Self) -> Matrix {
        panic!("Transform is abstract: 'value' must be implemented by the deriving class")
    }
}

impl Transform {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), changed: HandlerList::new(), resource: CompositorResourceHolder::new() }
    }

    /// Parses a matrix string (`m11,m12,m21,m22,m31,m32` or a transform
    /// operation list) to a transform.
    pub fn parse(s: &str) -> Result<Ref<Transform>, FormatError> {
        Ok(MatrixTransform::with_matrix(Matrix::parse(s)?).upcast())
    }

    /// Subscribes to changes of the transform. Disposing the returned handle
    /// unsubscribes.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let object: &FerroObject = self.upcast();
        let weak = object.to_weak();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                if let Some(this) = this.downcast_ref::<Transform>() {
                    this.changed.remove(token);
                }
            }
        })
    }

    /// Raises the changed notification.
    pub fn raise_changed(&self) {
        if self.changed.is_empty() {
            return;
        }
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }

    /// The object as something a compositor serializes: the adapter keeps
    /// the object alive while it is queued.
    pub(crate) fn as_compositor_serializable(&self) -> Rc<dyn ICompositorSerializable> {
        Rc::new(RefAdapter(self.to_ref()))
    }

    /// Converts the transform to an immutable transform.
    pub fn to_immutable(&self) -> ImmutableTransform {
        ImmutableTransform::new(self.value())
    }
}

impl ICompositionRenderResource for Transform {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        let owner = self.as_compositor_serializable();
        self.resource.create_or_add_ref(c, Some(owner), |cc| {
            cc.create_server_object(|server, _| ServerCompositionSimpleTransform::new(server) as Rc<dyn IServerObject>)
        });
    }

    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        self.resource.release(c);
    }

    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        self.resource.get_for_compositor(c)
    }
}

impl ICompositorSerializable for RefAdapter<Transform> {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.0.resource.try_get_for_compositor(c)
    }

    fn serialization_key(&self) -> *const () {
        RefAdapter::reference_id(self)
    }

    fn serialize_changes(&self, _c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        ServerCompositionSimpleTransformProps::serialize_all_changes(writer, self.0.value());
    }
}

/// Writes the transform's matrix.
impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value(), f)
    }
}
