from PyQt5.QtGui import QColor
from PyQt5.QtWidgets import QApplication, QColorDialog

app = QApplication(["ferro-qt-interop"])
assert app.platformName() == "wayland", app.platformName()
app.setDesktopFileName("org.ferroui.NativeQtColor")
dialog = QColorDialog(QColor("#427CAD"))
dialog.setOption(QColorDialog.DontUseNativeDialog)
dialog.setWindowTitle("Select Color — Qt Widgets")
dialog.show()
app.exec()
