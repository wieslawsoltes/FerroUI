use crate::remote::UiThreadHandle;
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::DrawingContext;
use ferroui_base::platform::{PixelFormat, PixelFormats};
use ferroui_base::threading::DispatcherPriority;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, PixelSize, Rect, Ref,
    StyledElementImpl, Vector, VisualImpl, VisualImplExt, WeakRef,
};
use ferroui_remote_protocol::viewport::{
    ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage, FrameReceivedMessage,
    PixelFormat as ProtocolPixelFormat,
};
use ferroui_remote_protocol::{message_handler, IFerroRemoteTransportConnection, Message};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// `RemoteWidget.SizingMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizingMode {
    Local,
    Remote,
}

/// Shows the frames received from a connection of the remote protocol.
/// (Internal upstream; public here for the crates the upstream assembly is
/// visible to.)
#[repr(C)]
pub struct RemoteWidget {
    base: Control,
    connection: Arc<dyn IFerroRemoteTransportConnection>,
    /// `_lastFrame`: the message as it was received, which is a
    /// [`FrameMessage`].
    last_frame: RefCell<Option<Message>>,
    bitmap: RefCell<Option<WriteableBitmap>>,
    mode: Cell<SizingMode>,
}

ferro_class!(RemoteWidget: Control);
ferroui_base::ferro_class_info!(RemoteWidget {});
ferro_impl_classes!(RemoteWidget: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

/// `new PixelFormat((PixelFormatEnum)format)`: the first three members of
/// the two enumerations are the same.
fn platform_pixel_format(format: ProtocolPixelFormat) -> PixelFormat {
    match format {
        ProtocolPixelFormat::Rgb565 => PixelFormats::RGB565,
        ProtocolPixelFormat::Rgba8888 => PixelFormats::RGBA8888,
        ProtocolPixelFormat::Bgra8888 => PixelFormats::BGRA8888,
    }
}

impl FerroObjectImpl for RemoteWidget {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // `_connection.OnMessage += (t, msg) => Dispatcher.UIThread.Post(OnMessage, msg)`:
        // the handler runs on the reader thread of the connection and posts
        // the message, which is plain data, to the UI thread, where the
        // widget is looked up again.
        let handle: UiThreadHandle<WeakRef<RemoteWidget>> =
            UiThreadHandle::register(this.to_ref().downgrade(), |widget| widget.upgrade().is_some());
        this.connection.on_message(message_handler(move |_, msg| {
            let msg = msg.clone();
            handle.post(
                move |widget| {
                    if let Some(widget) = widget.upgrade() {
                        widget.on_message(&msg);
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }));
        this.connection.send(Arc::new(ClientSupportedPixelFormatsMessage {
            formats: Some(vec![ProtocolPixelFormat::Bgra8888, ProtocolPixelFormat::Rgba8888]),
        }));
    }
}

impl LayoutableImpl for RemoteWidget {
    fn arrange_core(this: &Self, final_rect: Rect) {
        if this.mode.get() == SizingMode::Local {
            this.connection.send(Arc::new(ClientViewportAllocatedMessage {
                width: final_rect.width,
                height: final_rect.height,
                dpi_x: 10.0 * 96.0,
                dpi_y: 10.0 * 96.0, //TODO: Somehow detect the actual DPI
            }));
        }

        Self::parent_arrange_core(this, final_rect);
    }
}

impl VisualImpl for RemoteWidget {
    fn render(this: &Self, context: &mut DrawingContext) {
        let last_frame = this.last_frame.borrow().clone();
        let last_frame = last_frame.as_ref().and_then(|frame| frame.downcast_ref::<FrameMessage>());
        if let Some(last_frame) = last_frame {
            if last_frame.width != 0 && last_frame.height != 0 {
                let fmt = platform_pixel_format(last_frame.format);
                let mut bitmap = this.bitmap.borrow_mut();
                let stale = match bitmap.as_ref() {
                    None => true,
                    Some(bitmap) => {
                        bitmap.pixel_size().width != last_frame.width || bitmap.pixel_size().height != last_frame.height
                    }
                };
                if stale {
                    if let Some(old) = bitmap.take() {
                        old.dispose();
                    }
                    *bitmap = Some(WriteableBitmap::new(
                        PixelSize::new(last_frame.width, last_frame.height),
                        Vector::new(96.0, 96.0),
                        Some(fmt),
                        None,
                    ));
                }
                let bitmap = bitmap.as_ref().expect("the bitmap was created above");
                {
                    let l = bitmap.lock();
                    let line_len = ((if fmt == PixelFormat::RGB565 { 2 } else { 4 }) * last_frame.width) as usize;
                    let row_bytes = l.row_bytes() as usize;
                    let stride = last_frame.stride as usize;
                    // `Marshal.Copy` of a null array is an
                    // `ArgumentNullException`, and a frame shorter than it
                    // says is out of range: both panic here.
                    let data = last_frame.data.as_deref().expect("Value cannot be null. (Parameter 'source')");
                    l.with_data(&mut |dest| {
                        for y in 0..last_frame.height as usize {
                            dest[row_bytes * y..row_bytes * y + line_len]
                                .copy_from_slice(&data[y * stride..y * stride + line_len]);
                        }
                    });
                    l.dispose();
                }
                context.draw_image_with_rects(
                    bitmap,
                    Rect::new(0.0, 0.0, f64::from(bitmap.pixel_size().width), f64::from(bitmap.pixel_size().height)),
                    Rect::from_size(this.bounds().size()),
                );
            }
        }
        Self::parent_render(this, context);
    }
}

impl RemoteWidget {
    /// Field initialisation only.
    pub fn construct(connection: Arc<dyn IFerroRemoteTransportConnection>) -> Self {
        Self {
            base: Control::construct(),
            connection,
            last_frame: RefCell::new(None),
            bitmap: RefCell::new(None),
            mode: Cell::new(SizingMode::Local),
        }
    }

    /// `new RemoteWidget(connection)`.
    pub fn new(connection: Arc<dyn IFerroRemoteTransportConnection>) -> Ref<Self> {
        instantiate(Self::construct(connection))
    }

    pub fn mode(&self) -> SizingMode {
        self.mode.get()
    }

    pub fn set_mode(&self, value: SizingMode) {
        self.mode.set(value)
    }

    fn on_message(&self, msg: &Message) {
        if let Some(frame) = msg.downcast_ref::<FrameMessage>() {
            self.connection.send(Arc::new(FrameReceivedMessage { sequence_id: frame.sequence_id }));
            *self.last_frame.borrow_mut() = Some(msg.clone());
            self.invalidate_visual();
        }
    }

    /// The bitmap the last frame was copied into (`_bitmap`).
    #[cfg(test)]
    pub(crate) fn with_bitmap<R>(&self, f: impl FnOnce(Option<&WriteableBitmap>) -> R) -> R {
        f(self.bitmap.borrow().as_ref())
    }

    /// The last frame received (`_lastFrame`).
    #[cfg(test)]
    pub(crate) fn last_frame(&self) -> Option<Message> {
        self.last_frame.borrow().clone()
    }
}
