// The tests of the Dart bindings over the library (no window, no display):
//
//   make -C samples/CApi test-dart CARGO_FLAGS="--no-default-features --features type-tables"

import 'dart:io' show exit, stderr;

import '../../../../bindings/dart/full/ferroui.dart';

var checks = 0;

void check(bool condition, String what) {
  checks += 1;
  if (!condition) {
    stderr.writeln('failed: $what');
    exit(1);
  }
}

void testValues() {
  final text = TextBlock()
    ..text = 'Hello'
    ..fontSize = 20
    ..opacity = 0.5
    ..isVisible = false;
  check(text.text == 'Hello', 'text');
  check(text.fontSize == 20, 'font size');
  check(text.opacity == 0.5, 'a property of a base');
  check(!text.isVisible, 'a boolean');

  final longer = 'x' * 500;
  text.text = longer;
  check(text.text == longer, 'a text longer than the first buffer');
  text.text = 'Zażółć';
  check(text.text == 'Zażółć', 'UTF-8');
  text.text = '';
  check(text.text == '', 'an empty text');
  text.release();

  final button = Button()..horizontalAlignment = HorizontalAlignment.center;
  check(button.horizontalAlignment == HorizontalAlignment.center, 'an enumeration');
  button.release();
}

void testContentAndChildren() {
  final button = Button()..content = 'Click me';
  check(button.content == 'Click me', 'content as text');

  final inner = TextBlock();
  button.contentObject = inner;
  final held = button.contentObject;
  check(held.handle == inner.handle, 'content as an object');
  held.release();

  final panel = StackPanel()..spacing = 4;
  final child = Border();
  panel.addChild(child);
  panel.removeChild(child);
  for (final FerroObject object in [button, inner, panel, child]) {
    object.release();
  }
}

void testAttachedProperties() {
  final grid = Grid();
  final child = Border();
  grid.addChild(child);
  Grid.setRow(child, 2);
  Grid.setColumn(child, 1);
  check(Grid.getRow(child) == 2, 'attached row');
  check(Grid.getColumn(child) == 1, 'attached column');
  child.release();
  grid.release();
}

void testWrappers() {
  final border = Border()..width = 10;
  final control = Control.of(border);
  check(control.handle == border.handle, 'the same handle');
  check(control.width == 10, 'through another wrapper');
  border.release();
}

void testEvents() {
  final button = Button();
  final click = eventId('Click');
  var clicks = 0;
  button.onClick((sender) {
    clicks += 1;
    sender.content = 'Clicked';
  });
  raiseEvent(button, click);
  check(clicks == 1, 'the handler ran');
  check(button.content == 'Clicked', 'with its sender');

  var counted = 0;
  final subscription = button.onClick((_) => counted += 1);
  raiseEvent(button, click);
  check(counted == 1, 'a second handler');
  subscription.cancel();
  raiseEvent(button, click);
  check(counted == 1, 'cancelled');
  check(clicks == 3, 'the first one still runs');

  // What a handler throws stops at the boundary of the C ABI.
  button.onClick((_) => throw StateError('from the handler'));
  raiseEvent(button, click);
  check(clicks == 4, 'after a throw');
  button.release();
}

void testErrors() {
  final border = Border();
  var thrown = false;
  try {
    // `Text` is not a property of a border.
    TextBlock.of(border).text = 'no';
  } on FerroError catch (error) {
    thrown = true;
    check(error.message.contains('Text'), 'with the message of the library');
  }
  check(thrown, 'a typed error');
  border.release();
}

void main() {
  init();
  checkLibrary();
  testValues();
  testContentAndChildren();
  testAttachedProperties();
  testWrappers();
  testEvents();
  testErrors();
  print('ok: $checks checks');
}
