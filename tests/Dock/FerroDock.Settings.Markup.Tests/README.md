# Independent settings consumer

This Cargo package depends on the native settings and model libraries plus the
published FerroUI framework. It does not depend on FerroDock.Ferro or its themes.
The standard Model compiler consumes dependency-exported metadata; no runtime
reflection or alternate XAML compiler is used.

The Headless case loads an unprefixed DockProperties object through the settings
assembly XML namespace, constructs DockSettingsOptions with original core/settings
enums and a KeyGesture, reads a static settings field, and exercises a compiled
binding that inherits the attached property's original TwoWay default. It also
checks inherited drag/drop flags/group and the explicit adorner host. This proves
the settings dependency can supply its own metadata before a docking UI exists.
