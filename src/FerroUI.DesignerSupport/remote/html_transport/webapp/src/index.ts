import {PreviewerPresenter} from './FramePresenter'
import {PreviewerServerConnection} from "src/PreviewerServerConnection";

const loc = window.location;
const conn = new PreviewerServerConnection((loc.protocol === "https:" ? "wss" : "ws") + "://" + loc.host + "/ws");

// The application: one element as wide as the page, with the presenter in it.
const App = function(): HTMLElement {
    const root = document.createElement("div");
    root.style.width = '100%';
    const presenter = new PreviewerPresenter({conn: conn});
    root.appendChild(presenter.render());
    presenter.componentDidMount();
    return root;
};

// It replaces what the element of the page showed while the script loaded.
const app = document.getElementById("app")!;
while (app.firstChild)
    app.removeChild(app.firstChild);
app.appendChild(App());
