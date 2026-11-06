# FerroBoard on-screen keyboard

The reusable keyboard is isolated in [components/FerroBoard](../../components/FerroBoard/README.md).
Its independent workspace contains Core, Core.Ferro markup integration, controls,
resources, the full sample, native launchers and tests. Applications use normal path dependencies; ControlCatalog demonstrates the public host and
input properties in its **Text → FerroBoard** page.

See the component's [integration boundaries](../../components/FerroBoard/docs/PORTING.md),
[dependencies](../../components/FerroBoard/docs/DEPENDENCIES.md) and
[validation record](../../components/FerroBoard/docs/VALIDATION.md).
No keyboard engine logic is added to framework controls or other ports. The
component can be moved out by removing the documented parent integration seams.
