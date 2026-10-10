//! Upstream's `Composition/OpenGlCompositionInteropTests.cs`: no test of it
//! is ported yet, because none can be built or run in this test crate.
//!
//! Every test of the file starts from a compositor whose graphics are the
//! software OpenGL of upstream's `MesaSoftwareRenderer` (a native library of
//! llvmpipe behind EGL, which the upstream test project takes from a
//! package) and whose surface is upstream's `FboReadbackGlPlatformSurface`;
//! one test asks for the software Vulkan of the same class and does nothing
//! on macOS. What a port of the tests needs and the test environment of the
//! port does not have:
//!
//! - the OpenGL crate of the port (`ferroui-opengl`: the composition GL
//!   context, its textures and leases, the OpenGL control base) is not a
//!   dependency of this crate;
//! - an OpenGL platform graphics for the tests: the port has the EGL platform
//!   graphics, and no EGL implementation to load on the platforms the tests
//!   run on (no software renderer library, no offscreen context);
//! - the Skia backend of the port renders through Metal on Apple platforms
//!   and is built without an OpenGL back end there, so a compositor over an
//!   OpenGL context cannot create its render interface;
//! - the readback surfaces of upstream's `Mesa` directory, which are not
//!   ported;
//! - a Vulkan platform graphics, which the port does not have.
