import {PreviewerFrame, PreviewerServerConnection} from "src/PreviewerServerConnection";
import {PointerPressedEventMessage} from "src/Models/Input/PointerPressedEventMessage";
import {PointerReleasedEventMessage} from "src/Models/Input/PointerReleasedEventMessage";
import {PointerMovedEventMessage} from "src/Models/Input/PointerMovedEventMessage";
import {ScrollEventMessage} from "src/Models/Input/ScrollEventMessage";

interface PreviewerPresenterProps {
    conn: PreviewerServerConnection;
}

// The canvas that shows the frames of the previewer and sends the mouse input over it. It works on
// the document itself: the constructor creates the element, render() hands it out, and whoever
// puts it into the document calls componentDidMount() afterwards.
export class PreviewerPresenter {
    public readonly props: PreviewerPresenterProps;
    private canvas: HTMLCanvasElement;
    private scale: number = 1;

    constructor(props: PreviewerPresenterProps) {
        this.props = props;
        this.canvas = document.createElement("canvas");
        this.componentDidUpdate({
            conn: null!
        });

        this.handleMouseDown = this.handleMouseDown.bind(this);
        this.handleMouseUp = this.handleMouseUp.bind(this);
        this.handleMouseMove = this.handleMouseMove.bind(this);
        this.handleWheel = this.handleWheel.bind(this);

        this.canvas.addEventListener('mousedown', this.handleMouseDown);
        this.canvas.addEventListener('mouseup', this.handleMouseUp);
        this.canvas.addEventListener('mousemove', this.handleMouseMove);
        // Not passive: the handler keeps the page from scrolling.
        this.canvas.addEventListener('wheel', this.handleWheel, {passive: false});
    }

    componentDidMount(): void {
        this.updateCanvas(this.canvas, this.props.conn.currentFrame);

        if (parent != null && parent !== window)
        {
            window.addEventListener('message', (event) => {
                if (!event.origin.startsWith('vscode-webview://')) {
                    return;
                }

                // Handle previewer dpi changed event
                if (event.data.type === 'scaling') {
                    const scale = Number(event.data.scale);
                    if (Number.isFinite(scale) && scale > 0) {
                        this.scale = event.data.scale;
                    }
                }
            });
        }
    }

    componentDidUpdate(prevProps: Readonly<PreviewerPresenterProps>): void {
        if(prevProps.conn != this.props.conn)
        {
            if(prevProps.conn)
                prevProps.conn.removeFrameListener(this.frameHandler);
            if(this.props.conn)
                this.props.conn.addFrameListener(this.frameHandler);
        }
    }

    private frameHandler = (frame: PreviewerFrame | null)=>{
        this.updateCanvas(this.canvas, frame);
    };


    updateCanvas(canvas: HTMLCanvasElement | null, frame: PreviewerFrame | null) {
        if (!canvas)
            return;

        const oldWidth = canvas.style.width;
        const oldHeight = canvas.style.height;

        if (frame == null){
            canvas.width = canvas.height = 1;
            canvas.style.width = canvas.style.height = `1px`;
            canvas.getContext('2d')!.clearRect(0,0,1,1);
        }
        else {
            canvas.width = frame.data.width;
            canvas.height = frame.data.height;

            // To prevent blurry images on high-DPI screens
            canvas.style.width = `${frame.data.width / window.devicePixelRatio}px`;
            canvas.style.height = `${frame.data.height / window.devicePixelRatio}px`;

            const ctx = canvas.getContext('2d')!;
            ctx.putImageData(frame.data, 0,0);
        }

        if (oldWidth !== canvas.style.width || oldHeight !== canvas.style.height)
        {
            // Inform the parent page (a VSCode extention) the canvas size has changed
            parent?.postMessage({
                type: 'resize',
                width: canvas.width / window.devicePixelRatio,
                height: canvas.height / window.devicePixelRatio,
            }, '*');
        }
    }

    handleMouseDown(e: MouseEvent) {
        e.preventDefault();
        const pointerPressedEventMessage = new PointerPressedEventMessage(e, this.scale / window.devicePixelRatio);
        this.props.conn.sendMouseEvent(pointerPressedEventMessage);
    }

    handleMouseUp(e: MouseEvent) {
        e.preventDefault();
        const pointerReleasedEventMessage = new PointerReleasedEventMessage(e, this.scale / window.devicePixelRatio);
        this.props.conn.sendMouseEvent(pointerReleasedEventMessage);
    }

    handleMouseMove(e: MouseEvent) {
        e.preventDefault();
        const pointerMovedEventMessage = new PointerMovedEventMessage(e, this.scale / window.devicePixelRatio);
        this.props.conn.sendMouseEvent(pointerMovedEventMessage);
    }

    handleWheel(e: WheelEvent) {
        e.preventDefault();
        const scrollEventMessage = new ScrollEventMessage(e, this.scale / window.devicePixelRatio);
        this.props.conn.sendMouseEvent(scrollEventMessage);
    }

    render(): HTMLCanvasElement {
        return this.canvas;
    }
}
