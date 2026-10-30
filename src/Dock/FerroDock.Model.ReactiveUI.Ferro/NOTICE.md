# Model metadata integration

Copyright (c) Wiesław Šoltés. All rights reserved.
Licensed under the MIT license in the workspace LICENSE.TXT.

This UI integration layer declares the original Dock.Model.ReactiveUI class shapes,
constructors and inheritance through the shared FerroUI metadata/compiler pipeline.
Class references retain the actual enclosing source engine objects and their state.
It owns no replacement model or docking engine. The model/domain crates remain free
of UI dependencies; the general control library does not depend on this backend SDK.
The metadata source files were moved from the earlier UI-library integration and
remain traced by docs/porting/dock/ports.json to their original source classes.
