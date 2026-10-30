# Model metadata integration

Copyright (c) Wiesław Šoltés. All rights reserved.
Licensed under the workspace MIT LICENSE.TXT.

The original Dock.Model enum/flags metadata uses the unchanged domain value types.
Source declaration generation and converter metadata were moved from the previous
settings integration. The shared native compiler and runtime metadata pipeline are
retained. The domain crate has no UI dependency. Remaining source contract metadata
is tracked separately; this integration is not a replacement docking model/engine.
