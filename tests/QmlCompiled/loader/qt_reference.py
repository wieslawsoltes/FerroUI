"""Exercise the shared Loader and Repeater fixtures on the published Qt Quick engine without a display."""
import os
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from PySide6.QtCore import QUrl, qVersion
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlComponent, QQmlEngine


def main():
    assert qVersion() == "6.12.0", qVersion()
    app = QGuiApplication([])
    engine = QQmlEngine()
    def load(path):
        component = QQmlComponent(engine, QUrl.fromLocalFile(str(path.resolve())))
        root = component.create()
        if root is None:
            raise RuntimeError("\n".join(e.toString() for e in component.errors()))
        return component, root

    folder = Path(__file__).parent
    component, root = load(folder / "Main.qml")
    assert root.report() == "0/0/0/0/0"
    root.inlineOn()
    assert root.report() == "1/40/12/3/1"
    root.deactivate()
    root.enqueue(12)
    assert root.report() == "0/40/12/0/1"
    root.activate()
    assert root.report() == "1/30/20/12/2"
    root.enqueue(13)
    assert root.report() == "1/30/20/13/3"
    root.missing()
    assert root.report() == "3/30/20/0/3"
    root.removeItem()
    assert root.report() == "0/30/20/0/3"
    print("Qt 6.12.0 Loader lifecycle and geometry reference passed")
    component, root = load(folder.parent / "delegates/Main.qml")
    assert root.multipleRoles() == "A/3/true/true"
    root.renameThroughDelegate()
    assert root.multipleRoles() == "Z/3/true/true"
    root.growRoles()
    assert root.rowsSeen() == "A/undefined/B/undefined/17"
    root.reattach()
    assert root.rowsSeen() == "A/17/B/19/17"
    assert root.summary() == "3/2/2/0=one/1=two"
    root.enableEdit()
    assert root.summary() == "30/4/2/0=one/1=two"
    root.changeSeed()
    assert root.summary() == "50/6/2/0=one/1=two"
    root.disableEdit()
    assert root.summary() == "5/2/2/0=one/1=two"
    root.remember()
    root.moveRow()
    assert root.savedAt(1) and root.summary() == "5/2/2/0=two/1=one"
    root.renameRow()
    assert root.savedAt(1) and root.summary() == "5/2/2/0=two/1=updated"
    root.eraseRow()
    assert root.savedAt(0) and root.summary() == "5/2/1/0=updated/-"
    print("Qt 6.12.0 multi-target Binding, Repeater order and delegate identity passed")
    component, root = load(folder.parent / "anchors/Main.qml")
    assert root.geometry() == "63/42/132/52/124/23", root.geometry()
    root.moveMarker()
    assert root.geometry() == "83/52/112/42/134/33", root.geometry()
    root.resetLeft()
    assert root.geometry() == "3/52/192/42/94/33", root.geometry()
    print("Qt 6.12.0 sibling anchor geometry passed")

    component, root = load(folder.parent / "anchors/Mirrored.qml")
    assert root.frames() == "-2/19/7/92/42", root.frames()
    assert root.inherited() == "true/false/true", root.inherited()
    root.inheritLocal()
    assert root.inherited() == "true/false/false", root.inherited()
    root.resetLocal()
    assert root.inherited() == "true/true/true", root.inherited()
    root.change()
    assert root.frames() == "32/21/2/92/50", root.frames()
    print("Qt 6.12.0 baseline anchors and inherited mirroring passed")
    component, root = load(folder.parent / "anchors/Positioners.qml")
    assert root.positions() == "70/48/30/18/10", root.positions()
    root.reverseDirections()
    for item in root.children():
        if hasattr(item, "forceLayout"):
            item.forceLayout()
    assert root.positions() == "0/12/0/22/0", root.positions()
    print("Qt 6.12.0 mirrored Row and wrapped Flow positions passed")

    component, root = load(folder.parent / "nested_roles/Main.qml")
    root.remember()
    root.edit()
    assert root.nestedEdit() == "2/A/BB/true/7", root.nestedEdit()
    root.replace()
    root.addNested()
    root.appendGroup()
    assert root.replaced() == "2/X/Y/true/9", root.replaced()
    root.clearData()
    print("Qt 6.12.0 nested role identity, replacement and delegates passed")

    from PySide6.QtQuick import QQuickWindow
    component, root = load(folder.parent / "views/Main.qml")
    window = QQuickWindow()
    window.resize(100, 60)
    root.setParentItem(window.contentItem())
    window.show()
    app.processEvents()
    root.scrollMiddle()
    assert root.report() == "1000000/20000000/10000000/500000/500000/true", root.report()
    root.changeCurrent()
    assert root.selection() == "0/updated/updated", root.selection()
    window.hide()
    root.setParentItem(None)
    print("Qt 6.12.0 ListView scrolling, model edits and selection passed")
    component, root = load(folder.parent / "views/Decorated.qml")
    window.resize(160, 160)
    root.setParentItem(window.contentItem())
    window.show()
    app.processEvents()
    assert root.report() == "A/161/0/-154/-31/-88/0/-1/AAB/-54", root.report()
    root.regroup()
    app.processEvents()
    assert root.report() == "A/161/0/-154/-31/-88/0/-1/ABB/-65", root.report()
    window.hide()
    root.setParentItem(None)
    print("Qt 6.12.0 reversed ListView headers, footers and section role edits passed")
    component, root = load(folder.parent / "views/Reuse.qml")
    window.resize(40, 40)
    root.setParentItem(window.contentItem())
    window.show()
    app.processEvents()
    for index in (30, 80, 140, 10):
        root.jump(index)
        app.processEvents()
    assert root.valid(), (root.property("pooled"), root.property("reused"), root.property("invalid"))
    window.hide()
    root.setParentItem(None)
    print("Qt 6.12.0 delegate pooling and reuse lifecycle passed")
    component, root = load(folder.parent / "buttons/Main.qml")
    assert root.report() == "2/false/0/false/false/false/0/0/", root.report()
    root.choose()
    root.choose()
    root.setSecond()
    root.nextCheck()
    assert root.report() == "2/false/1/false/true/false/1/0/TCGCGK", root.report()
    root.nextCheck()
    root.flip()
    root.groupAll()
    assert root.report() == "2/false/2/true/true/true/2/1/TCGCGKKS", root.report()
    print("Qt 6.12.0 grouped buttons, tri-state checkboxes and switch activation passed")
    component, root = load(folder.parent / "ranges/Main.qml")
    assert root.report() == "6/0.4/0.6/0.4/6/6/4/0/0", root.report()
    root.step()
    assert root.report() == "9/0.1/0.9/0.1/3/3/4/0/0", root.report()
    root.clamp()
    assert root.report() == "10/0/1/0/10/10/4/0/0", root.report()
    print("Qt 6.12.0 descending ranges, step methods and spin wrapping passed")
    del window, root, component, engine, app


if __name__ == "__main__":
    main()
