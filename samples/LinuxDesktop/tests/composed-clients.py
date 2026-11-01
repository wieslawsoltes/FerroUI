#!/usr/bin/python3
"""Independent GTK and Qt Wayland clients, and capture probes for ferro-compositor.sh."""
import sys


def gtk_client():
    import gi
    gi.require_version("Gtk", "3.0")
    gi.require_version("Gdk", "3.0")
    from gi.repository import Gdk, Gtk

    window = Gtk.Window(title="Project notes")
    window.set_default_size(630, 400)
    window.connect("destroy", Gtk.main_quit)
    header = Gtk.HeaderBar(title="Project notes", subtitle="GTK 3")
    header.set_show_close_button(True)
    window.set_titlebar(header)

    css = Gtk.CssProvider()
    css.load_from_data(b"#note-tag {background-color: #d76a3d; color: white; padding: 16px;}")
    Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), css,
                                           Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    column = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14, margin=22)
    for text in ("<big><b>Desktop integration</b></big>",
                 "Independent clients share the same FerroUI composition target."):
        label = Gtk.Label(xalign=0)
        label.set_markup(text)
        column.pack_start(label, False, False, 0)
    details = Gtk.Entry()
    details.set_placeholder_text("Add a project note…")
    column.pack_start(details, False, False, 0)
    style = Gtk.Label(label="   Native Wayland surface   ·   GTK", xalign=0)
    style.set_name("note-tag")
    column.pack_start(style, False, False, 0)
    column.pack_start(Gtk.CheckButton(label="Track updates on this output"), False, False, 0)
    window.add(column)
    window.show_all()
    Gtk.main()


def qt_client():
    from PyQt5.QtCore import QEvent, QObject, QPoint, Qt
    from PyQt5.QtWidgets import QApplication, QWidget, QVBoxLayout, QLabel, QLineEdit, QMenu

    app = QApplication(sys.argv[:1])
    app.setApplicationName("Ferro color inspector")
    app.setDesktopFileName("org.ferroui.ColorInspector")
    window = QWidget()
    window.setWindowTitle("Color inspector — Qt Widgets")
    window.resize(380, 250)
    layout = QVBoxLayout(window)
    layout.setContentsMargins(20, 16, 20, 16)
    layout.setSpacing(10)
    layout.addWidget(QLabel("<h2>Color inspector</h2>"))
    layout.addWidget(QLabel("Active workspace accent"))
    swatch = QLabel()
    swatch.setMinimumHeight(68)
    layout.addWidget(swatch)
    value = QLineEdit()
    value.setReadOnly(True)
    value.textEdited.connect(lambda text: print("label " + text, flush=True))
    layout.addWidget(value)
    layout.addWidget(QLabel("Qt Widgets   ·   Native Wayland client"))
    popup = QMenu(window)
    popup.setStyleSheet("QMenu { background: #f5ce52; color: #212529; padding: 8px; } "
                        "QMenu::item { padding: 6px 14px; }")
    popup.addAction("Use workspace accent")
    popup.addAction("Restore original color")

    def color(rgb):
        swatch.setStyleSheet("background: " + rgb)
        value.setText(rgb.upper())

    class Keys(QObject):
        def eventFilter(self, obj, event):
            if event.type() == QEvent.KeyPress and not popup.isVisible():
                if not value.isReadOnly():
                    if event.key() in (Qt.Key_Return, Qt.Key_Enter):
                        value.setReadOnly(True)
                    else:
                        return False
                if event.key() == Qt.Key_F2:
                    value.setReadOnly(False)
                    value.setFocus()
                    value.selectAll()
                    return True
                if event.key() == Qt.Key_U:
                    color("#1ba48e")
                    print("updated", flush=True)
                    return True
                if event.key() == Qt.Key_P:
                    popup.popup(swatch.mapToGlobal(QPoint(12, swatch.height() - 10)))
                    return True
            return False

    keys = Keys()
    app.installEventFilter(keys)
    color("#397abb")
    window.show()
    app.exec_()


def probe(path, color, test):
    from PIL import Image, ImageChops
    im = Image.open(path).convert("RGB")
    # The opaque, unfiltered widget interiors survive both the client and
    # compositor color conversion. Permit minor driver rounding differences.
    if color == "picker":
        # The middle of the Appearance panel contains rows with opaque
        # themed surfaces. Wait for the new theme to paint before capturing.
        from PIL import ImageStat
        level = ImageStat.Stat(im.crop((500, 470, 780, 610)).convert("L")).mean[0]
        return level < 110 if test == "dark" else level > 170
    if color == "subsurface":
        # The red child uses translucent pixels composited over the green
        # client root. Test its red-channel dominance instead of assuming an
        # exact destination color underneath the client.
        mask = im.convert("L", (1, -2, 0, 0)).point([255 * (v > 70) for v in range(256)])
        return mask.histogram()[255] < 200 if test == "absent" else mask.histogram()[255] > 3000
    rgb = bytes.fromhex(color)
    masks = [band.point([255 * (abs(i - c) <= 3) for i in range(256)])
             for band, c in zip(im.split(), rgb)]
    mask = ImageChops.multiply(ImageChops.multiply(*masks[:2]), masks[2])
    count = mask.histogram()[255]
    if test == "absent":
        return count < 80
    if count < 500:
        return False
    if test == "point":
        left, top, right, bottom = mask.getbbox()
        # Choose a real interior pixel at the right edge of the visible swatch,
        # even if another client partially covers its left half.
        for y in range(top + 3, bottom - 3, 2):
            for x in range(right - 6, left + 4, -2):
                if mask.getpixel((x, y)) and mask.getpixel((x - 3, y)):
                    print(x, y)
                    return True
        return False
    return True


if __name__ == "__main__":
    if sys.argv[1] == "gtk":
        gtk_client()
    elif sys.argv[1] == "qt":
        qt_client()
    else:
        sys.exit(0 if probe(*sys.argv[1:]) else 1)
