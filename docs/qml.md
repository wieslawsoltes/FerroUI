# QML for FerroUI

The language reference for this implementation is Qt QML 6.12.0. The parser, object model, compilation pipeline and native adapters are written in Rust from the public language documentation. They do not link Qt.

## Architecture

- The compiler owns the source grammar, byte spans and a component representation shared by source loading and ahead-of-time compilation.
- The runtime owns component instances, scoped identifiers, property values, dependency tracking and dispatch. Rust hosts supply type registrations.
- The controls adapter projects instances onto FerroUI controls and routes native events back into the component.
- Cargo build scripts compile QML during the build. Applications can include the generated module without opening a QML file at runtime.

The compatibility target includes the QML language and object engine. The visual integration uses FerroUI controls and its layout, rendering and event systems. Each supported feature should have an executable test. The compatibility table will distinguish implemented behavior from outstanding work.

## Public references

- [Qt QML](https://doc.qt.io/qt-6/qtqml-index.html)
- [QML language reference](https://doc.qt.io/qt-6/qmlreference.html)
- [Object attributes](https://doc.qt.io/qt-6/qtqml-syntax-objectattributes.html)
- [Property binding](https://doc.qt.io/qt-6/qtqml-syntax-propertybinding.html)
- [Module syntax](https://doc.qt.io/qt-6/qtqml-syntax-imports.html)
