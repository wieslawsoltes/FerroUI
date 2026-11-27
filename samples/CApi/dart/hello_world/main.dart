// Hello, World! in Dart through the generated bindings of FerroUI (bindings/README.md): a window
// with a text and a button whose clicks change its text, the Dart version of hello_world.c.
//
//   make -C samples/CApi run-dart
//
// FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds. A standalone Dart program does
// not run on the main thread of the process, so this runs on Linux and Windows and not on macOS.

import 'ferroui/ferroui.dart';

void main() {
  runApp(() {
    final text = TextBlock()
      ..text = 'Hello, World!'
      ..fontSize = 28;

    var clicks = 0;
    final button = Button()
      ..horizontalAlignment = HorizontalAlignment.center
      ..content = 'Click me';
    button.onClick((sender) {
      clicks += 1;
      sender.content = clicks == 1 ? 'Clicked 1 time' : 'Clicked $clicks times';
    });

    final panel = StackPanel()
      ..horizontalAlignment = HorizontalAlignment.center
      ..verticalAlignment = VerticalAlignment.center
      ..spacing = 12
      ..addChild(text)
      ..addChild(button);

    final window = Window()
      ..title = 'Hello, World!'
      ..width = 400
      ..height = 240
      ..addChild(panel)
      ..show();

    // The window holds its tree and the platform holds the open window.
    for (final FerroObject object in [text, button, panel, window]) {
      object.release();
    }
  });
}
