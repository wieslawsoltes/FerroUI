//! Drag and drop between the view and the system: the drags that enter the
//! view, from this or another application, and the drags the application
//! starts (`AndroidDragDrop` of the reference).

use super::android_drag_drop_state::{DragDropState, DragStep};
use super::clip_data_to_data_transfer_wrapper::ClipDataToDataTransferWrapper;
use super::clip_description_to_data_transfer_wrapper::ClipDescriptionToDataTransferWrapper;
use super::skia_platform::TopLevelImpl;
use super::specific::helpers::android_motion_events_helper::IMotionEventsTopLevel;
use super::specific::helpers::clip_data_helper::ClipDataHelper;
use crate::interop::java::{
    call_boolean, call_object, call_static_object, call_void, is_instance_of, JavaClass, JavaObject, JavaRef, JavaValue,
};
use crate::interop::natives::FERRO_DRAG_LISTENER;
use ferroui_base::input::raw::{IDragDropDevice, RawDragEvent, RawDragEventType};
use ferroui_base::input::{DragDropEffects, IAsyncDataTransfer, IDataTransfer, LocalBoxFuture, RawInputModifiers};
use ferroui_base::{FerroLocator, Point};
use ferroui_system::task::oneshot;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(crate) struct AndroidDragDrop {
    top_level: Weak<TopLevelImpl>,
    /// The surface view and its `FerroDragListener`, until disposed.
    view: RefCell<Option<(JavaObject, JavaObject)>>,
    /// The context of the view: the activity that asks for the permissions
    /// of a drop, and the context the data of a drop is read with.
    context: JavaObject,
    state: RefCell<DragDropState>,
    /// The drag the application started, until the system ends it.
    current_drag: RefCell<Option<oneshot::Sender<DragDropEffects>>>,
}

impl AndroidDragDrop {
    /// Listens to the drags over `view`, the surface view of `top_level`,
    /// which the listener calls back with the handle of the top-level.
    pub(crate) fn new(top_level: Weak<TopLevelImpl>, handle: i64, view: &JavaObject, context: &JavaObject) -> Rc<Self> {
        let listener = call_static_object(
            &JavaClass::find(FERRO_DRAG_LISTENER),
            "attach",
            "(Landroid/view/View;J)Lorg/ferroui/android/FerroDragListener;",
            &[JavaValue::Object(Some(view)), JavaValue::Long(handle)],
        )
        .map(|listener| listener.to_global());
        Rc::new(AndroidDragDrop {
            top_level,
            view: RefCell::new(listener.map(|listener| (view.clone(), listener))),
            context: context.clone(),
            state: RefCell::new(DragDropState::default()),
            current_drag: RefCell::new(None),
        })
    }

    /// Stops listening; a drag the application started ends with no effect.
    pub(crate) fn dispose(&self) {
        let current_drag = self.current_drag.borrow_mut().take();
        if let Some(current_drag) = current_drag {
            let _ = current_drag.send(DragDropEffects::NONE);
        }
        let view = self.view.borrow_mut().take();
        if let Some((view, listener)) = view {
            call_void(&listener, "detach", "(Landroid/view/View;)V", &[JavaValue::Object(Some(&view))]);
        }
    }

    /// A drag event of the system (`View.OnDragListener.onDrag`); `x` and
    /// `y` are in pixels of the view, `event` is valid during the call only.
    pub(crate) fn on_drag(&self, action: i32, x: f32, y: f32, result: bool, is_own_drag: bool, event: &dyn JavaRef) {
        let Some(top_level) = self.top_level.upgrade() else {
            return;
        };
        let scaling = top_level.motion_render_scaling();
        let position = Point::new(f64::from(x) / scaling, f64::from(y) / scaling);

        let step = self.state.borrow_mut().on_drag(action, position, result, is_own_drag);
        match step {
            DragStep::None => {}
            DragStep::Ended(effects) => {
                let current_drag = self.current_drag.borrow_mut().take();
                if let Some(current_drag) = current_drag {
                    let _ = current_drag.send(effects);
                }
            }
            DragStep::Raise(event_type, position) => {
                let description = call_object(event, "getClipDescription", "()Landroid/content/ClipDescription;", &[]);
                let data_transfer =
                    ClipDescriptionToDataTransferWrapper::new(description.as_ref().map(|d| d as &dyn JavaRef));
                Self::raise(&top_level, event_type, position, data_transfer);
            }
            DragStep::Drop(position) => {
                if !is_instance_of(&self.context, "android/app/Activity") {
                    return;
                }
                let Some(clip_data) = call_object(event, "getClipData", "()Landroid/content/ClipData;", &[]) else {
                    return;
                };
                // The content URIs of another application are readable while the permissions
                // are held: during the drop.
                let permissions = call_object(
                    &self.context,
                    "requestDragAndDropPermissions",
                    "(Landroid/view/DragEvent;)Landroid/view/DragAndDropPermissions;",
                    &[JavaValue::Object(Some(event))],
                );
                let data_transfer =
                    ClipDataToDataTransferWrapper::new(clip_data.to_global(), Some(self.context.clone()));
                let _release = ReleasePermissions(permissions.map(|permissions| permissions.to_global()));
                Self::raise(&top_level, RawDragEventType::Drop, position, data_transfer);
            }
        }
    }

    fn raise(
        top_level: &TopLevelImpl,
        event_type: RawDragEventType,
        position: Point,
        data_transfer: Rc<dyn IDataTransfer>,
    ) {
        let (Some(device), Some(input_root)) =
            (FerroLocator::current().get_service::<dyn IDragDropDevice>(), top_level.input_root())
        else {
            return;
        };
        let event = RawDragEvent::new(
            device,
            event_type,
            input_root,
            position,
            data_transfer,
            DragDropEffects::COPY,
            RawInputModifiers::NONE,
        );
        top_level.dispatch_input(Rc::new(event));
    }

    /// Starts a drag of `data_transfer` from the view; the future ends with
    /// the effect when the system ends the drag. A drag that is still going
    /// on ends with no effect.
    ///
    /// The reference attaches a `DragStartHelper` of AndroidX, which starts
    /// the drag at the next drag gesture of the pointer that is down; the
    /// port starts it when the clip data is made, while that pointer is
    /// still down.
    pub(crate) fn start_drag_drop(
        self: &Rc<Self>,
        data_transfer: Rc<dyn IDataTransfer>,
        async_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> LocalBoxFuture<DragDropEffects> {
        let this = Rc::downgrade(self);
        Box::pin(async move {
            let effects = Self::drag(this, async_transfer).await;
            data_transfer.dispose();
            effects
        })
    }

    async fn drag(this: Weak<Self>, async_transfer: Rc<dyn IAsyncDataTransfer>) -> DragDropEffects {
        let Ok(Some(clip_data)) = ClipDataHelper::get_clip_data(async_transfer).await else {
            return DragDropEffects::NONE;
        };
        let Some(this) = this.upgrade() else {
            return DragDropEffects::NONE;
        };

        let (sender, receiver) = oneshot::channel();
        let previous = this.current_drag.borrow_mut().replace(sender);
        if let Some(previous) = previous {
            let _ = previous.send(DragDropEffects::NONE);
        }

        let started = this.view.borrow().as_ref().is_some_and(|(view, listener)| {
            call_boolean(
                listener,
                "startDrag",
                "(Landroid/view/View;Landroid/content/ClipData;)Z",
                &[JavaValue::Object(Some(view)), JavaValue::Object(Some(&clip_data))],
            )
        });
        if !started {
            this.current_drag.borrow_mut().take();
            return DragDropEffects::NONE;
        }
        drop(this);

        receiver.await.unwrap_or(DragDropEffects::NONE)
    }
}

/// Releases the permissions of a drop when dropped (`finally` of the
/// reference).
struct ReleasePermissions(Option<JavaObject>);

impl Drop for ReleasePermissions {
    fn drop(&mut self) {
        if let Some(permissions) = &self.0 {
            call_void(permissions, "release", "()V", &[]);
        }
    }
}
